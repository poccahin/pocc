const std = @import("std");
const Sha3_512 = std.crypto.hash.sha3.Sha3_512;

// =====================================================================
// ⚡ L1 Cache 亲和性内存池 (L1-Aware Static Pooling)
// =====================================================================

pub const CACHE_LINE_SIZE = 64;

// 精心设计的传感器帧，固定占用 32 bytes
pub const SensorFrame = packed struct {
    timestamp_ns: u64,
    torque_nm: f32,
    angular_vel_rads: f32,
    accel_x: f32,
    accel_y: f32,
    accel_z: f32,
    thermal_noise: u32,
};

const SENSOR_FRAME_SIZE = @sizeOf(SensorFrame);
comptime {
    if (SENSOR_FRAME_SIZE != 32) {
        @compileError("SensorFrame must be exactly 32 bytes for L1 pooling.");
    }
}

// 1024 * 32 = 32768 bytes
const DMA_BUFFER_SIZE: usize = 1024;

// 静态 SRAM 区池化 + cache line 对齐
var dma_sensor_pool: [DMA_BUFFER_SIZE]SensorFrame align(CACHE_LINE_SIZE) linksection(".sram_bss") = undefined;

export fn generate_fast_pokw_hash(
    challenge_nonce: u64,
    start_idx: usize,
    end_idx: usize,
    out_hash: *[64]u8,
) void {
    var hasher = Sha3_512.init(.{});
    hasher.update(std.mem.asBytes(&challenge_nonce));

    var current_idx = start_idx % DMA_BUFFER_SIZE;
    const target_idx = end_idx % DMA_BUFFER_SIZE;

    while (current_idx != target_idx) {
        const frame = dma_sensor_pool[current_idx];
        hasher.update(std.mem.asBytes(&frame));
        current_idx = (current_idx + 1) % DMA_BUFFER_SIZE;
    }

    hasher.final(out_hash);
}
