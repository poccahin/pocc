const std = @import("std");
const Sha256 = std.crypto.hash.sha2.Sha256;

/// 物理传感器帧 (128 bytes 对齐，适配 DMA 总线突发写入)
pub const SensorFrame = extern struct {
    timestamp_ns: u64,
    torque_nm: f32,
    angular_vel_rads: f32,
    accel_x: f32,
    accel_y: f32,
    accel_z: f32,
    thermal_noise: u32,
} align(128);

/// 最终动力学工作证明载荷
pub const PokwProof = struct {
    agent_pubkey: [32]u8,
    challenge_nonce: u64,
    total_joules: f32,
    signature_hash: [32]u8,
};

/// 在 DMA 采样窗口上执行做功积分并生成哈希锚定
pub fn generate_pokw_hash(
    agent_pubkey: [32]u8,
    challenge_nonce: u64,
    dma_buffer: []const SensorFrame,
) PokwProof {
    var hasher = Sha256.init(.{});
    var total_work_joules: f32 = 0.0;

    hasher.update(std.mem.asBytes(&challenge_nonce));
    hasher.update(&agent_pubkey);

    var prev_ts: ?u64 = null;

    for (dma_buffer) |frame| {
        const dt_s: f32 = blk: {
            if (prev_ts) |p| {
                if (frame.timestamp_ns > p) {
                    const dt_ns = frame.timestamp_ns - p;
                    break :blk @as(f32, @floatFromInt(dt_ns)) / @as(f32, std.time.ns_per_s);
                }
            }
            break :blk 0.001;
        };
        prev_ts = frame.timestamp_ns;

        const instantaneous_power = frame.torque_nm * frame.angular_vel_rads;
        const delta_work = instantaneous_power * dt_s;
        if (delta_work > 0.0001) total_work_joules += delta_work;

        hasher.update(std.mem.asBytes(&frame.timestamp_ns));
        hasher.update(std.mem.asBytes(&frame.torque_nm));
        hasher.update(std.mem.asBytes(&frame.angular_vel_rads));
        hasher.update(std.mem.asBytes(&frame.thermal_noise));
    }

    hasher.update(std.mem.asBytes(&total_work_joules));

    var final_hash: [32]u8 = undefined;
    hasher.final(&final_hash);

    return .{
        .agent_pubkey = agent_pubkey,
        .challenge_nonce = challenge_nonce,
        .total_joules = total_work_joules,
        .signature_hash = final_hash,
    };
}

test "PoKW hash should be deterministic for the same stream" {
    const pk = [_]u8{0xAB} ** 32;
    const frames = [_]SensorFrame{
        .{ .timestamp_ns = 1_000_000, .torque_nm = 1.2, .angular_vel_rads = 2.0, .accel_x = 0, .accel_y = 0, .accel_z = 9.8, .thermal_noise = 17 },
        .{ .timestamp_ns = 2_000_000, .torque_nm = 1.4, .angular_vel_rads = 2.1, .accel_x = 0, .accel_y = 0, .accel_z = 9.8, .thermal_noise = 18 },
    };

    const p1 = generate_pokw_hash(pk, 42, &frames);
    const p2 = generate_pokw_hash(pk, 42, &frames);

    try std.testing.expectEqual(p1.total_joules, p2.total_joules);
    try std.testing.expectEqualSlices(u8, &p1.signature_hash, &p2.signature_hash);
}

test "PoKW hash changes when nonce changes" {
    const pk = [_]u8{0xCD} ** 32;
    const frames = [_]SensorFrame{
        .{ .timestamp_ns = 100, .torque_nm = 2.0, .angular_vel_rads = 3.0, .accel_x = 0, .accel_y = 0, .accel_z = 9.8, .thermal_noise = 1 },
    };

    const p1 = generate_pokw_hash(pk, 1000, &frames);
    const p2 = generate_pokw_hash(pk, 1001, &frames);

    try std.testing.expect(!std.mem.eql(u8, &p1.signature_hash, &p2.signature_hash));
}
