//! Agent capability manifest builder for CIS edge nodes.
//!
//! Each provisioned device publishes a JSON capabilities manifest that
//! describes its hardware specification (compute substrate, NPU TOPS,
//! sensor interfaces) and the Life++ protocol features it supports.
//! This manifest is pinned to IPFS during factory provisioning and its
//! CID is recorded on-chain as part of the ERC-8004 `bootstrapIdentity`
//! call.

use serde::{Deserialize, Serialize};

/// Compute substrate classification for a CIS edge node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComputeSubstrate {
    /// AMD Ryzen AI (Strix Halo / Phoenix) with integrated XDNA NPU.
    AmdRyzenAi { model: String, tops: f32 },
    /// Sophgo RISC-V / TPU architecture.
    Sophgo { model: String, tops: f32 },
    /// Generic x86-64 edge device (no dedicated NPU).
    GenericX86 { cores: u32 },
}

/// Hardware telemetry interfaces available on the device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryInterfaces {
    /// I²C bus for thermal sensors and current monitors.
    pub i2c: bool,
    /// SPI bus for high-speed sensor data.
    pub spi: bool,
    /// CAN bus for actuator feedback (robotics use-case).
    pub can: bool,
}

/// Hardware root-of-trust mechanism.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrustRoot {
    /// Dedicated TPM 2.0 security chip.
    Tpm20,
    /// On-die SRAM Physical Unclonable Function.
    SramPuf,
}

/// Full capability manifest for one CIS edge compute node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilityManifest {
    /// Globally unique decentralised identity (DID).
    pub did: String,
    /// Human-readable device model string.
    pub device_model: String,
    /// Compute substrate description.
    pub compute: ComputeSubstrate,
    /// Hardware root of trust.
    pub trust_root: TrustRoot,
    /// Available telemetry interfaces.
    pub telemetry: TelemetryInterfaces,
    /// Life++ protocol features supported by this node.
    pub supported_protocols: Vec<String>,
    /// Firmware version string.
    pub firmware_version: String,
    /// Factory provisioning timestamp (Unix seconds).
    pub provisioned_at: u64,
}

impl AgentCapabilityManifest {
    /// Build the default capability manifest for a CIS AMD-395 node.
    pub fn for_amd395(did: &str, provisioned_at: u64) -> Self {
        Self {
            did: did.to_string(),
            device_model: "CIS-Edge-AMD395-v1".to_string(),
            compute: ComputeSubstrate::AmdRyzenAi {
                model: "AMD Ryzen AI 395".to_string(),
                tops: 50.0,
            },
            trust_root: TrustRoot::SramPuf,
            telemetry: TelemetryInterfaces {
                i2c: true,
                spi: true,
                can: false,
            },
            supported_protocols: vec![
                "life++/pokw-v1".to_string(),
                "x402/micropayment-v1".to_string(),
                "erc-8004/identity-v1".to_string(),
                "quorum/private-tx-v1".to_string(),
            ],
            firmware_version: env!("CARGO_PKG_VERSION").to_string(),
            provisioned_at,
        }
    }

    /// Serialise the manifest to a compact JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_serialises_to_json() {
        let m = AgentCapabilityManifest::for_amd395("did:cis:amd395:deadbeef", 1700000000);
        let json = m.to_json();
        assert!(json.contains("did:cis:amd395:deadbeef"));
        assert!(json.contains("life++/pokw-v1"));
    }

    #[test]
    fn manifest_has_sram_puf_trust_root() {
        let m = AgentCapabilityManifest::for_amd395("did:cis:test:01", 0);
        assert_eq!(m.trust_root, TrustRoot::SramPuf);
    }

    #[test]
    fn manifest_has_correct_compute_substrate() {
        let m = AgentCapabilityManifest::for_amd395("did:cis:test:02", 0);
        assert!(matches!(
            m.compute,
            ComputeSubstrate::AmdRyzenAi { .. }
        ));
    }
}
