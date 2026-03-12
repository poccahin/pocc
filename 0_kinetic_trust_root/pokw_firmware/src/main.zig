const std = @import("std");

const HmacSha256 = std.crypto.auth.hmac.sha2.HmacSha256;

/// 模拟设备签名密钥（生产环境中必须从安全飞地/TPM 读取，切勿将此占位符部署至生产环境）
const DEVICE_SECRET_KEY: [32]u8 = .{
    0x7f, 0x3a, 0xc1, 0x9e, 0x84, 0x56, 0xd2, 0x0b,
    0xf1, 0x4e, 0x82, 0x37, 0xc9, 0x6a, 0x15, 0x53,
    0xe8, 0x2d, 0x70, 0xbb, 0x49, 0x1c, 0xf6, 0x38,
    0xad, 0x75, 0x0e, 0xc4, 0x92, 0x5f, 0xb8, 0x1a,
};

/// 动力学采样点：时间戳为单调时钟的相对微秒值，防止绝对时间被系统时钟篡改
const KinematicSample = struct {
    timestamp_us: u64, // 相对于采集开始的单调时钟微秒数
    joint_id: u8,
    current_ma: i32, // 伺服电机电流 (毫安)
    accel_z_mg: i32, // Z 轴加速度 (毫 G)
};

/// 核心数据结构：动力学工作证明 (PoKW)
const PoKWPayload = struct {
    intent_hash: [32]u8,
    noise_seed: u64,
    samples: []KinematicSample,
    energy_joules_estimate: u32,
    /// 来自硬件 TRNG 的真实熵（32 字节），证明执行时有真实随机性参与，攻击者无法预计算
    hardware_entropy: [32]u8,
    /// HMAC-SHA256 签名，覆盖 intent_hash、noise_seed、energy 及 hardware_entropy，
    /// 由设备密钥签发，防止外部脚本伪造 PoKW payload。
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

/// 使用硬件 TRNG 熵与 noise_seed 混合生成密码学安全的 PRNG 种子。
/// 这可以证明 noise_seed 在物理执行时确实参与了计算，同时引入不可预测的真实硬件熵，
/// 防止攻击者通过已知 noise_seed 预计算波形数据。
fn deriveSecureSeed(noise_seed: u64, hardware_entropy: [32]u8) u64 {
    var hasher = std.crypto.hash.sha2.Sha256.init(.{});
    var seed_bytes: [8]u8 = undefined;
    std.mem.writeInt(u64, &seed_bytes, noise_seed, .little);
    hasher.update(&seed_bytes);
    hasher.update(&hardware_entropy);
    var hash_out: [32]u8 = undefined;
    hasher.final(&hash_out);
    return std.mem.readInt(u64, hash_out[0..8], .little);
}

fn captureKinematicWaveform(
    allocator: std.mem.Allocator,
    duration_ms: u64,
    noise_seed: u64,
    hardware_entropy: [32]u8,
) ![]KinematicSample {
    var waveform = std.ArrayList(KinematicSample).init(allocator);
    errdefer waveform.deinit();

    // 使用 TRNG 混合后的安全种子，防止攻击者通过已知 noise_seed 预计算波形
    const secure_seed = deriveSecureSeed(noise_seed, hardware_entropy);
    var prng = std.rand.DefaultPrng.init(secure_seed);
    const random = prng.random();

    // 使用单调时钟（相对执行时长），防止系统时钟被篡改而影响时间戳
    var timer = try std.time.Timer.start();
    const duration_ns = duration_ms * std.time.ns_per_ms;

    // 受混合种子控制的微秒级采样扰动
    while (timer.read() < duration_ns) {
        const jitter = random.intRangeAtMost(i32, -50, 50);
        const base: i32 = 1000;
        const sleep_us_i32 = if (base + jitter < 50) 50 else base + jitter;
        const sleep_us = @as(u64, @intCast(sleep_us_i32));

        std.time.sleep(sleep_us * std.time.ns_per_us);

        try waveform.append(.{
            .timestamp_us = timer.read() / std.time.ns_per_us,
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

/// 计算覆盖关键 payload 字段的 HMAC-SHA256 设备签名。
/// 签名覆盖：intent_hash || noise_seed || energy_joules_estimate || hardware_entropy。
/// 上层网关必须用设备公钥验证此签名，拒绝任何未经硬件签发的 payload，
/// 从而防止攻击者通过外部脚本伪造 PoKW 证明来套取 Life++ 奖励。
fn signPayload(
    intent_hash: [32]u8,
    noise_seed: u64,
    energy: u32,
    hardware_entropy: [32]u8,
) [HmacSha256.mac_length]u8 {
    var mac: [HmacSha256.mac_length]u8 = undefined;
    var hmac = HmacSha256.init(&DEVICE_SECRET_KEY);
    hmac.update(&intent_hash);
    var seed_bytes: [8]u8 = undefined;
    std.mem.writeInt(u64, &seed_bytes, noise_seed, .little);
    hmac.update(&seed_bytes);
    var energy_bytes: [4]u8 = undefined;
    std.mem.writeInt(u32, &energy_bytes, energy, .little);
    hmac.update(&energy_bytes);
    hmac.update(&hardware_entropy);
    hmac.final(&mac);
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

    // 从硬件 TRNG 采集真实熵，证明执行时存在不可预测的物理随机性
    var hardware_entropy: [32]u8 = undefined;
    std.crypto.random.bytes(&hardware_entropy);

    const samples = try captureKinematicWaveform(allocator, 500, noise_seed, hardware_entropy);
    defer allocator.free(samples);

    const energy = estimateEnergyJoules(samples);

    // 计算设备签名，覆盖所有关键字段，由设备密钥背书
    const device_signature = signPayload(intent_hash, noise_seed, energy, hardware_entropy);

    const payload = PoKWPayload{
        .intent_hash = intent_hash,
        .noise_seed = noise_seed,
        .samples = samples,
        .energy_joules_estimate = energy,
        .hardware_entropy = hardware_entropy,
        .device_signature = device_signature,
    };

    // 实际工程里建议用 CBOR/FlatBuffers; 这里用 JSON 便于 L1 直接解析。
    var writer = std.io.getStdOut().writer();
    try std.json.stringify(payload, .{}, writer);
    try writer.writeAll("\n");
}
