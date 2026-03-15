//! Hardware control primitives for the CIS factory provisioning line.
//!
//! On real AMD 395 / Sophgo hardware these operations interact with
//! One-Time Programmable (OTP) fuse banks and JTAG DAP controllers via
//! memory-mapped I/O.  The software implementation used here records fuse
//! state in an in-memory structure so that the provisioner binary can be
//! integration-tested without physical hardware.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum HardwareError {
    #[error("OTP fuse burn failed: {0}")]
    FuseBurnFailed(String),
    #[error("JTAG controller not accessible: {0}")]
    JtagError(String),
    #[error("Fuse already burned – cannot rewrite OTP")]
    FuseAlreadyBurned,
}

/// Current state of the hardware security fuses.
#[derive(Debug, Clone, Default)]
pub struct FuseState {
    /// When `true`, external JTAG/SWD debug access is permanently disabled.
    pub jtag_disabled: bool,
    /// When `true`, the boot ROM enforces a validated chain-of-trust on every boot.
    pub secure_boot_enforced: bool,
}

/// Interface to the One-Time Programmable (OTP) fuse controller.
///
/// Burning a fuse is an **irreversible** operation.  Once `jtag_disabled` is
/// set, no debugger – including the device manufacturer – can access the
/// hardware internals via JTAG or UART root console.
pub struct OtpFuses {
    state: FuseState,
}

impl OtpFuses {
    /// Create a new `OtpFuses` controller in its factory-default state
    /// (all fuses intact, all interfaces open).
    pub fn new() -> Self {
        Self {
            state: FuseState::default(),
        }
    }

    /// Permanently disable the JTAG / SWD debug interface.
    ///
    /// # Errors
    ///
    /// Returns `HardwareError::FuseAlreadyBurned` if the fuse was already set.
    pub fn burn_jtag_disable_bit(&mut self) -> Result<(), HardwareError> {
        if self.state.jtag_disabled {
            return Err(HardwareError::FuseAlreadyBurned);
        }
        // On real hardware: write to OTP_JTAG_DISABLE register via MMIO.
        self.state.jtag_disabled = true;
        Ok(())
    }

    /// Enforce verified secure boot on every subsequent power cycle.
    ///
    /// # Errors
    ///
    /// Returns `HardwareError::FuseAlreadyBurned` if the fuse was already set.
    pub fn burn_secure_boot_enforce_bit(&mut self) -> Result<(), HardwareError> {
        if self.state.secure_boot_enforced {
            return Err(HardwareError::FuseAlreadyBurned);
        }
        // On real hardware: write to OTP_SECURE_BOOT register via MMIO.
        self.state.secure_boot_enforced = true;
        Ok(())
    }

    /// Return a snapshot of the current fuse state.
    pub fn state(&self) -> &FuseState {
        &self.state
    }
}

impl Default for OtpFuses {
    fn default() -> Self {
        Self::new()
    }
}

/// JTAG controller interface.
///
/// Used to verify that the debug interface is open before provisioning begins,
/// and to confirm that it is closed after the OTP fuse is burned.
pub struct JtagController;

impl JtagController {
    /// Check whether the JTAG interface is currently accessible.
    ///
    /// Returns `true` when the interface is open (pre-provisioning state).
    /// Returns `false` once the OTP fuse has been burned.
    pub fn is_accessible(fuse_state: &FuseState) -> bool {
        !fuse_state.jtag_disabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jtag_accessible_before_fuse_burn() {
        let fuses = OtpFuses::new();
        assert!(JtagController::is_accessible(fuses.state()));
    }

    #[test]
    fn jtag_inaccessible_after_fuse_burn() {
        let mut fuses = OtpFuses::new();
        fuses.burn_jtag_disable_bit().expect("first burn should succeed");
        assert!(!JtagController::is_accessible(fuses.state()));
    }

    #[test]
    fn burning_jtag_fuse_twice_returns_error() {
        let mut fuses = OtpFuses::new();
        fuses.burn_jtag_disable_bit().unwrap();
        let result = fuses.burn_jtag_disable_bit();
        assert!(matches!(result, Err(HardwareError::FuseAlreadyBurned)));
    }

    #[test]
    fn secure_boot_enforced_after_burn() {
        let mut fuses = OtpFuses::new();
        fuses.burn_secure_boot_enforce_bit().unwrap();
        assert!(fuses.state().secure_boot_enforced);
    }

    #[test]
    fn both_fuses_can_be_burned_independently() {
        let mut fuses = OtpFuses::new();
        fuses.burn_jtag_disable_bit().unwrap();
        fuses.burn_secure_boot_enforce_bit().unwrap();
        assert!(fuses.state().jtag_disabled);
        assert!(fuses.state().secure_boot_enforced);
    }
}
