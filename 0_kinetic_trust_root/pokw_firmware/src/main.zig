const std = @import("std");
const pokw = @import("pokw_generator.zig");

fn parsePubkeyHex(input: []const u8) ![32]u8 {
    var out: [32]u8 = undefined;
    if (input.len != 64) return error.InvalidPubkeyLength;
    _ = try std.fmt.hexToBytes(&out, input);
    return out;
}

fn buildDemoDmaFrames() [4]pokw.SensorFrame {
    return .{
        .{ .timestamp_ns = 1_000_000_000, .torque_nm = 2.2, .angular_vel_rads = 3.1, .accel_x = 0.01, .accel_y = 0.02, .accel_z = 9.80, .thermal_noise = 1731 },
        .{ .timestamp_ns = 1_001_000_000, .torque_nm = 2.4, .angular_vel_rads = 3.3, .accel_x = 0.03, .accel_y = 0.01, .accel_z = 9.79, .thermal_noise = 8472 },
        .{ .timestamp_ns = 1_002_000_000, .torque_nm = 2.0, .angular_vel_rads = 3.0, .accel_x = 0.01, .accel_y = 0.00, .accel_z = 9.81, .thermal_noise = 421 },
        .{ .timestamp_ns = 1_003_000_000, .torque_nm = 2.8, .angular_vel_rads = 3.5, .accel_x = 0.02, .accel_y = 0.01, .accel_z = 9.80, .thermal_noise = 9233 },
    };
}

pub fn main() !void {
    var args = std.process.args();
    _ = args.skip();

    const pubkey_hex = args.next() orelse {
        std.debug.print("Usage: pokw_firmware <agent_pubkey_hex_64> <challenge_nonce>\n", .{});
        return;
    };

    const nonce_str = args.next() orelse {
        std.debug.print("Usage: pokw_firmware <agent_pubkey_hex_64> <challenge_nonce>\n", .{});
        return;
    };

    const agent_pubkey = try parsePubkeyHex(pubkey_hex);
    const challenge_nonce = try std.fmt.parseInt(u64, nonce_str, 10);
    var dma_frames = buildDemoDmaFrames();

    const proof = pokw.generate_pokw_hash(agent_pubkey, challenge_nonce, &dma_frames);

    try std.json.stringify(proof, .{}, std.io.getStdOut().writer());
    try std.io.getStdOut().writer().writeByte('\n');
}
