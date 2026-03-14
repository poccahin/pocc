//! RWA Securitization Bridge (rwa-securitization-bridge)
//!
//! Capital leverage and hardware capacity bridging layer for the POCC
//! governance stack (L4).
//!
//! Real-World Assets (RWA) in the Life++ context are primarily **hardware
//! capacity bonds**: tokenised representations of committed CPU/GPU/NPU
//! compute capacity that edge operators pledge to the network in exchange for
//! an upfront LIFE++ capital allocation.
//!
//! ```text
//!  Hardware Operator
//!       │  pledges capacity (specs + duration)
//!       ▼
//!  RwaSecuritizationBridge::originate_bond(params)
//!       │
//!       ▼
//!  HardwareCapacityBond { bond_id, face_value, maturity_epoch, status: Active }
//!       │
//!  Buyer acquires bond ──► RwaSecuritizationBridge::transfer_bond()
//!       │
//!  Bond matures ──────────► RwaSecuritizationBridge::redeem_bond()
//!                                   │
//!                              Net settlement amount → DailyNettingProcessor
//! ```

use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RwaError {
    #[error("Bond not found: {0}")]
    BondNotFound(String),
    #[error("Bond has already matured or been redeemed")]
    BondAlreadySettled,
    #[error("Bond is not yet matured (current epoch {current}, maturity {maturity})")]
    NotYetMatured { current: u64, maturity: u64 },
    #[error("Insufficient face value {available:.4} for redemption amount {requested:.4}")]
    InsufficientFaceValue { available: f64, requested: f64 },
    #[error("Transfer rejected: seller {0} is not the current bond holder")]
    UnauthorisedTransfer(String),
}

// ─────────────────────────────────────────────────────────────────────────────
// Bond lifecycle state
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum BondStatus {
    /// Bond is active and capacity is being committed to the network.
    Active,
    /// Bond has been redeemed by the holder at or after maturity.
    Redeemed,
    /// Bond was force-slashed due to operator misconduct.
    Slashed,
}

// ─────────────────────────────────────────────────────────────────────────────
// Hardware capacity bond
// ─────────────────────────────────────────────────────────────────────────────

/// Specification of the hardware capacity being securitised.
#[derive(Debug, Clone)]
pub struct HardwareSpec {
    /// Human-readable description (e.g. "AMD XDNA NPU 16 TOPS, 128 GB RAM").
    pub description: String,
    /// Compute throughput in TOPS (Tera-Operations Per Second).
    pub tops: f64,
    /// Memory bandwidth in GB/s.
    pub memory_bandwidth_gbs: f64,
    /// Geographic region (for latency SLAs).
    pub region: String,
}

/// A tokenised hardware capacity bond.
#[derive(Debug, Clone)]
pub struct HardwareCapacityBond {
    pub bond_id: String,
    /// DID of the hardware operator who originated the bond.
    pub operator_did: String,
    /// DID of the current bond holder (may differ after transfers).
    pub holder_did: String,
    pub hardware_spec: HardwareSpec,
    /// Face value in LIFE++ tokens at maturity.
    pub face_value: f64,
    pub token_symbol: String,
    /// Amount of capital pre-allocated to operator at origination.
    pub advance_amount: f64,
    /// Unix epoch at which the bond matures and can be redeemed.
    pub maturity_epoch: u64,
    pub status: BondStatus,
}

impl HardwareCapacityBond {
    pub fn is_active(&self) -> bool {
        self.status == BondStatus::Active
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// RWA Securitization Bridge
// ─────────────────────────────────────────────────────────────────────────────

pub struct RwaSecuritizationBridge {
    bonds: HashMap<String, HardwareCapacityBond>,
    /// Total capital advanced to operators (tracks protocol exposure).
    total_capital_advanced: f64,
}

impl Default for RwaSecuritizationBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl RwaSecuritizationBridge {
    pub fn new() -> Self {
        Self {
            bonds: HashMap::new(),
            total_capital_advanced: 0.0,
        }
    }

    /// Originate a new hardware capacity bond.
    ///
    /// `advance_ratio` controls what fraction of `face_value` is advanced to
    /// the operator immediately (typical range 0.5–0.8).
    pub fn originate_bond(
        &mut self,
        operator_did: &str,
        hardware_spec: HardwareSpec,
        face_value: f64,
        token_symbol: &str,
        maturity_epoch: u64,
        advance_ratio: f64,
    ) -> String {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(operator_did.as_bytes());
        h.update(hardware_spec.description.as_bytes());
        h.update(face_value.to_be_bytes());
        h.update(maturity_epoch.to_be_bytes());
        let bond_id = hex::encode(h.finalize());

        let advance_amount = face_value * advance_ratio.clamp(0.0, 1.0);
        self.total_capital_advanced += advance_amount;

        let bond = HardwareCapacityBond {
            bond_id: bond_id.clone(),
            operator_did: operator_did.to_string(),
            holder_did: operator_did.to_string(),
            hardware_spec,
            face_value,
            token_symbol: token_symbol.to_string(),
            advance_amount,
            maturity_epoch,
            status: BondStatus::Active,
        };
        self.bonds.insert(bond_id.clone(), bond);
        bond_id
    }

    /// Transfer bond ownership from `from_did` to `to_did` (secondary market).
    pub fn transfer_bond(
        &mut self,
        bond_id: &str,
        from_did: &str,
        to_did: &str,
    ) -> Result<(), RwaError> {
        let bond = self
            .bonds
            .get_mut(bond_id)
            .ok_or_else(|| RwaError::BondNotFound(bond_id.to_string()))?;

        if !bond.is_active() {
            return Err(RwaError::BondAlreadySettled);
        }
        if bond.holder_did != from_did {
            return Err(RwaError::UnauthorisedTransfer(from_did.to_string()));
        }

        bond.holder_did = to_did.to_string();
        Ok(())
    }

    /// Redeem a matured bond and return the settlement amount.
    ///
    /// The settlement amount is `face_value - advance_amount` (the residual
    /// due to the holder at maturity, net of what was already advanced).
    pub fn redeem_bond(
        &mut self,
        bond_id: &str,
        holder_did: &str,
        current_epoch: u64,
    ) -> Result<f64, RwaError> {
        let bond = self
            .bonds
            .get_mut(bond_id)
            .ok_or_else(|| RwaError::BondNotFound(bond_id.to_string()))?;

        if !bond.is_active() {
            return Err(RwaError::BondAlreadySettled);
        }
        if bond.holder_did != holder_did {
            return Err(RwaError::UnauthorisedTransfer(holder_did.to_string()));
        }
        if current_epoch < bond.maturity_epoch {
            return Err(RwaError::NotYetMatured {
                current: current_epoch,
                maturity: bond.maturity_epoch,
            });
        }

        let settlement = (bond.face_value - bond.advance_amount).max(0.0);
        bond.status = BondStatus::Redeemed;
        Ok(settlement)
    }

    /// Slash a bond (e.g. due to operator's hardware going offline).
    /// Called by `SoulboundSlasher` during cross-layer extermination.
    pub fn slash_bond(&mut self, bond_id: &str) -> Result<f64, RwaError> {
        let bond = self
            .bonds
            .get_mut(bond_id)
            .ok_or_else(|| RwaError::BondNotFound(bond_id.to_string()))?;
        if !bond.is_active() {
            return Err(RwaError::BondAlreadySettled);
        }
        let confiscated = bond.advance_amount;
        bond.status = BondStatus::Slashed;
        Ok(confiscated)
    }

    pub fn get_bond(&self, bond_id: &str) -> Option<&HardwareCapacityBond> {
        self.bonds.get(bond_id)
    }

    pub fn total_capital_advanced(&self) -> f64 {
        self.total_capital_advanced
    }

    pub fn active_bonds(&self) -> impl Iterator<Item = &HardwareCapacityBond> {
        self.bonds.values().filter(|b| b.is_active())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> HardwareSpec {
        HardwareSpec {
            description: "AMD XDNA NPU 32 TOPS".into(),
            tops: 32.0,
            memory_bandwidth_gbs: 800.0,
            region: "ap-southeast-1".into(),
        }
    }

    fn bridge_with_bond() -> (RwaSecuritizationBridge, String) {
        let mut bridge = RwaSecuritizationBridge::new();
        let id = bridge.originate_bond(
            "did:operator:amd_node_01",
            spec(),
            10_000.0,
            "LIFE++",
            1_000,
            0.7,
        );
        (bridge, id)
    }

    #[test]
    fn originate_bond_advances_capital() {
        let (bridge, id) = bridge_with_bond();
        let bond = bridge.get_bond(&id).unwrap();
        assert!((bond.advance_amount - 7_000.0).abs() < f64::EPSILON);
        assert!((bridge.total_capital_advanced() - 7_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn redeem_after_maturity_returns_residual() {
        let (mut bridge, id) = bridge_with_bond();
        // advance = 7000, face = 10000, residual = 3000
        let settlement = bridge
            .redeem_bond(&id, "did:operator:amd_node_01", 1_001)
            .unwrap();
        assert!((settlement - 3_000.0).abs() < f64::EPSILON);
        assert_eq!(bridge.get_bond(&id).unwrap().status, BondStatus::Redeemed);
    }

    #[test]
    fn premature_redemption_is_rejected() {
        let (mut bridge, id) = bridge_with_bond();
        assert!(matches!(
            bridge.redeem_bond(&id, "did:operator:amd_node_01", 500),
            Err(RwaError::NotYetMatured { .. })
        ));
    }

    #[test]
    fn transfer_and_redeem_by_new_holder() {
        let (mut bridge, id) = bridge_with_bond();
        bridge
            .transfer_bond(&id, "did:operator:amd_node_01", "did:investor:fund_a")
            .unwrap();
        let settlement = bridge
            .redeem_bond(&id, "did:investor:fund_a", 1_001)
            .unwrap();
        assert!((settlement - 3_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn slash_confiscates_advance() {
        let (mut bridge, id) = bridge_with_bond();
        let confiscated = bridge.slash_bond(&id).unwrap();
        assert!((confiscated - 7_000.0).abs() < f64::EPSILON);
        assert_eq!(bridge.get_bond(&id).unwrap().status, BondStatus::Slashed);
    }

    #[test]
    fn double_redeem_is_rejected() {
        let (mut bridge, id) = bridge_with_bond();
        bridge
            .redeem_bond(&id, "did:operator:amd_node_01", 1_001)
            .unwrap();
        assert!(matches!(
            bridge.redeem_bond(&id, "did:operator:amd_node_01", 1_002),
            Err(RwaError::BondAlreadySettled)
        ));
    }
}
