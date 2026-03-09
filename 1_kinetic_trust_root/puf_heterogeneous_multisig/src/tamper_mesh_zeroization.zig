//! Life++ L0 Hardware Trust Root - Active Tamper Mesh & Scorched Earth Protocol
//! FIPS 140-3 Level 4 style bare-metal tamper response flow.
//!
//! 目标：在物理攻击事件发生时，通过 NMI 级别中断触发主动归零与硬件熔毁流程。

const std = @import("std");

// --- Active Tamper Mesh 寄存器映射 ---
const TAMPER_STATUS_REG: *volatile u32 = @ptrFromInt(0x400A_0000);
const TAMPER_MESH_CTRL: *volatile u32 = @ptrFromInt(0x400A_0004);

// --- Zeroization 执行通道 ---
const ZEROIZATION_TRIGGER: *volatile u32 = @ptrFromInt(0x400B_0000);
const SECURE_SRAM_BANK: *volatile [1024]u32 = @ptrFromInt(0x5000_0000);

// --- NMI 路由控制寄存器（平台相关，需与 SoC 手册核对）---
const NMI_ENABLE_REG: *volatile u32 = @ptrFromInt(0xE000_ED14);

const EVENT_EMFI_GLITCH: u32 = 1 << 0;
const EVENT_MESH_BROKEN: u32 = 1 << 1;
const EVENT_TEMP_ANOMALY: u32 = 1 << 2;

pub const TamperEventMask: u32 = EVENT_EMFI_GLITCH | EVENT_MESH_BROKEN | EVENT_TEMP_ANOMALY;

/// 早期启动入口：任何 PUF 或密钥派生前必须先布防。
export fn _start() noreturn {
    arm_active_tamper_mesh();

    while (true) {
        asm volatile ("wfi");
    }
}

/// 启用物理防区网并将篡改事件路由到 NMI。
pub inline fn arm_active_tamper_mesh() void {
    NMI_ENABLE_REG.* = 1;
    TAMPER_MESH_CTRL.* = 0x01;

    asm volatile ("" ::: "memory");
}

/// NMI handler: 检测到篡改后立即执行焦土归零。
///
/// 注意：真实量产固件中该符号通常需要链接脚本绑定到 NMI vector。
export fn tamper_nmi_handler() noreturn {
    const status = TAMPER_STATUS_REG.*;

    if ((status & TamperEventMask) != 0) {
        scorch_secure_sram();

        // 触发硬件放电路径（高压电容/熔丝，平台实现决定具体行为）。
        ZEROIZATION_TRIGGER.* = 0xFFFF_FFFF;

        asm volatile ("" ::: "memory");

        // 锁死 CPU，避免任何后续可观测行为。
        while (true) {
            asm volatile ("wfi");
        }
    }

    // NMI 被触发即视作不可恢复事件，误触场景也采用 fail-closed。
    while (true) {
        asm volatile ("wfi");
    }
}

inline fn scorch_secure_sram() void {
    var i: usize = 0;
    while (i < SECURE_SRAM_BANK.len) : (i += 1) {
        SECURE_SRAM_BANK.*[i] = 0xDEAD_BEEF;
    }
}
