const std = @import("std");
const Sha256 = std.crypto.hash.sha2.Sha256;

/// 物理传感器帧 (128 bytes 对齐，适配 DMA 总线)
pub const SensorFrame = extern struct {
    timestamp_ns: u64,
    torque_nm: f32,
    angular_vel_rads: f32,
    accel_x: f32,
    accel_y: f32,
    accel_z: f32,
    thermal_noise: u32,
    _padding: [92]u8 = [_]u8{0} ** 92,
};
comptime {
    std.debug.assert(@alignOf(SensorFrame) == 4);
    std.debug.assert(@sizeOf(SensorFrame) == 128);
}

/// 最终的动力学工作证明载荷
pub const PokwProof = struct {
    agent_pubkey: [32]u8,
    challenge_nonce: u64,
    total_joules: f32,
    signature_hash: [32]u8,
};

pub fn generate_pokw_hash(
    agent_pubkey: [32]u8,
    challenge_nonce: u64,
    dma_buffer: []const SensorFrame,
) PokwProof {
    var hasher = Sha256.init(.{});
    var total_work_joules: f32 = 0.0;

    hasher.update(std.mem.asBytes(&challenge_nonce));
    hasher.update(&agent_pubkey);

    var prev_timestamp_ns: ?u64 = null;
    for (dma_buffer) |frame| {
        const dt_s: f32 = if (prev_timestamp_ns) |prev| blk: {
            const delta_ns = if (frame.timestamp_ns >= prev) frame.timestamp_ns - prev else 0;
            break :blk @as(f32, @floatFromInt(delta_ns)) / 1_000_000_000.0;
        } else 0.0;
        prev_timestamp_ns = frame.timestamp_ns;

        const instantaneous_power = frame.torque_nm * frame.angular_vel_rads;
        const delta_work = instantaneous_power * dt_s;
        if (delta_work > 0.0001) {
            total_work_joules += delta_work;
        }

        hasher.update(std.mem.asBytes(&frame.timestamp_ns));
        hasher.update(std.mem.asBytes(&frame.torque_nm));
        hasher.update(std.mem.asBytes(&frame.angular_vel_rads));
        hasher.update(std.mem.asBytes(&frame.thermal_noise));
    }

    hasher.update(std.mem.asBytes(&total_work_joules));

    var final_hash: [32]u8 = undefined;
    hasher.final(&final_hash);

    return PokwProof{
        .agent_pubkey = agent_pubkey,
        .challenge_nonce = challenge_nonce,
        .total_joules = total_work_joules,
        .signature_hash = final_hash,
    };
}

test "pokw hash is deterministic for same input" {
    const key = [_]u8{0xAA} ** 32;
    const nonce: u64 = 0x1122334455667788;
    const frames = [_]SensorFrame{
        .{ .timestamp_ns = 1_000_000_000, .torque_nm = 10.0, .angular_vel_rads = 2.0, .accel_x = 0.1, .accel_y = 0.2, .accel_z = 0.3, .thermal_noise = 0xA1 },
        .{ .timestamp_ns = 1_001_000_000, .torque_nm = 10.0, .angular_vel_rads = 2.0, .accel_x = 0.1, .accel_y = 0.2, .accel_z = 0.3, .thermal_noise = 0xB2 },
    };

    const p1 = generate_pokw_hash(key, nonce, &frames);
    const p2 = generate_pokw_hash(key, nonce, &frames);

    try std.testing.expectEqual(p1.total_joules, p2.total_joules);
    try std.testing.expectEqualSlices(u8, &p1.signature_hash, &p2.signature_hash);
}

test "pokw total_joules uses timestamp delta and ignores tiny noise" {
    const key = [_]u8{0x01} ** 32;
    const nonce: u64 = 7;
    const frames = [_]SensorFrame{
        .{ .timestamp_ns = 100, .torque_nm = 0.001, .angular_vel_rads = 0.001, .accel_x = 0, .accel_y = 0, .accel_z = 0, .thermal_noise = 1 },
        .{ .timestamp_ns = 1_000_000_100, .torque_nm = 10.0, .angular_vel_rads = 2.0, .accel_x = 0, .accel_y = 0, .accel_z = 0, .thermal_noise = 2 },
    };

    const p = generate_pokw_hash(key, nonce, &frames);
    try std.testing.expectApproxEqAbs(@as(f32, 20.0), p.total_joules, 0.0001);
}
