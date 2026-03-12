const std = @import("std");
const HmacSha256 = std.crypto.auth.hmac.HmacSha256;

/// Mock device secret key — simulates a key derived from the Secure Enclave / TPM.
/// In production: provision this via ARM TrustZone, Apple SEP, or read from
/// a protected environment variable (DEVICE_SECRET_KEY).
const DEVICE_SECRET_KEY: [32]u8 = [_]u8{
    0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE,
    0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE,
    0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE,
    0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE,
};

/// 动力学采样点：包含微秒级时间戳、关节电流与空间加速度
const KinematicSample = struct {
    timestamp_us: i64,
    joint_id: u8,
    current_ma: i32, // 伺服电机电流 (毫安)
    accel_z_mg: i32, // Z 轴加速度 (毫 G)
};

/// 可签名的核心有效载荷字段（不含签名自身）
const PoKWCore = struct {
    intent_hash: [32]u8,
    noise_seed: u64,
    samples: []KinematicSample,
    energy_joules_estimate: u32,
};

/// 核心数据结构：动力学工作证明 (PoKW)
/// device_signature 是用设备密钥对 PoKWCore JSON 做 HMAC-SHA256 的结果，
/// 可防止来自第三方的遥测数据伪造攻击。
const PoKWPayload = struct {
    intent_hash: [32]u8,
    noise_seed: u64,
    samples: []KinematicSample,
    energy_joules_estimate: u32,
    device_signature: [HmacSha256.mac_length]u8,
};

fn readServoCurrent(random: std.rand.Random, joint_id: u8) i32 {
    _ = joint_id;
    return 1500 + random.intRangeAtMost(i32, -100, 100);
}

fn readImuZAxis(random: std.rand.Random) i32 {
    return 981 + random.intRangeAtMost(i32, -10, 10);
}

fn parseIntentHashHex(input: []const u8) ![32]u8 {
    var out: [32]u8 = undefined;
    if (input.len != 64) return error.InvalidIntentHashLength;
    _ = try std.fmt.hexToBytes(&out, input);
    return out;
}

fn captureKinematicWaveform(
    allocator: std.mem.Allocator,
    duration_ms: u64,
    noise_seed: u64,
) ![]KinematicSample {
    var waveform = std.ArrayList(KinematicSample).init(allocator);
    errdefer waveform.deinit();

    var prng = std.rand.DefaultPrng.init(noise_seed);
    const random = prng.random();

    const start_time = std.time.microTimestamp();
    const end_time = start_time + @as(i64, @intCast(duration_ms * 1000));

    // 受 noise_seed 控制的微秒级采样扰动
    while (std.time.microTimestamp() < end_time) {
        const jitter = random.intRangeAtMost(i32, -50, 50);
        const base: i32 = 1000;
        const sleep_us_i32 = if (base + jitter < 50) 50 else base + jitter;
        const sleep_us = @as(u64, @intCast(sleep_us_i32));

        std.time.sleep(sleep_us * std.time.ns_per_us);

        try waveform.append(.{
            .timestamp_us = std.time.microTimestamp(),
            .joint_id = 1,
            .current_ma = readServoCurrent(random, 1),
            .accel_z_mg = readImuZAxis(random),
        });
    }

    return waveform.toOwnedSlice();
}

fn estimateEnergyJoules(samples: []const KinematicSample) u32 {
    var total: u32 = 0;
    for (samples) |s| {
        total += @as(u32, @intCast(@abs(s.current_ma))) * 24 / 1000;
    }
    return total;
}

/// 对 PoKWCore JSON 载荷计算 HMAC-SHA256，返回设备签名。
/// 这可防止第三方通过伪造 JSON 输出来绕过物理 PoKW 共识。
fn computeDeviceSignature(
    allocator: std.mem.Allocator,
    core: PoKWCore,
) ![HmacSha256.mac_length]u8 {
    // 将核心载荷序列化为 JSON（不含签名字段）
    var buf = std.ArrayList(u8).init(allocator);
    defer buf.deinit();
    try std.json.stringify(core, .{}, buf.writer());

    // 用设备密钥对序列化后的 JSON 计算 HMAC-SHA256
    var mac: [HmacSha256.mac_length]u8 = undefined;
    HmacSha256.create(&mac, buf.items, &DEVICE_SECRET_KEY);
    return mac;
}

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const args = try std.process.argsAlloc(allocator);
    defer std.process.argsFree(allocator, args);

    if (args.len < 3) {
        std.debug.print("Usage: pokw_sensor <intent_hash_hex_64> <noise_seed>\n", .{});
        return;
    }

    const intent_hash = try parseIntentHashHex(args[1]);
    const noise_seed = try std.fmt.parseInt(u64, args[2], 10);

    std.debug.print("🤖 [L0 Kinematics] Noise seed injected: 0x{x}\n", .{noise_seed});

    const samples = try captureKinematicWaveform(allocator, 500, noise_seed);
    defer allocator.free(samples);

    const energy = estimateEnergyJoules(samples);

    // 先构建核心载荷，再对其签名
    const core = PoKWCore{
        .intent_hash = intent_hash,
        .noise_seed = noise_seed,
        .samples = samples,
        .energy_joules_estimate = energy,
    };
    const signature = try computeDeviceSignature(allocator, core);

    const payload = PoKWPayload{
        .intent_hash = intent_hash,
        .noise_seed = noise_seed,
        .samples = samples,
        .energy_joules_estimate = energy,
        .device_signature = signature,
    };

    // 实际工程里建议用 CBOR/FlatBuffers; 这里用 JSON 便于 L1 直接解析。
    var writer = std.io.getStdOut().writer();
    try std.json.stringify(payload, .{}, writer);
    try writer.writeAll("\n");
}
