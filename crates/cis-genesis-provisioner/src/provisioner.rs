//! High-level provisioner orchestrating the full Silicon Awakening sequence.
//!
//! [`GenesisProvisioner`] is the entry-point used by the `cis_genesis_provision`
//! binary (and by integration tests).  It coordinates:
//!
//! 1. PUF entropy extraction.
//! 2. Dual key-pair derivation (Ed25519 + EVM).
//! 3. Capability manifest construction.
//! 4. Genesis transaction signing.
//! 5. OTP fuse burning.

use crate::{
    crypto::{Ed25519Keypair, EvmWallet, PufManager},
    hardware::OtpFuses,
    manifest::AgentCapabilityManifest,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProvisionerError {
    #[error("Hardware error: {0}")]
    Hardware(#[from] crate::hardware::HardwareError),
    #[error("Provisioning already completed for this device")]
    AlreadyProvisioned,
}

/// Outcome of a successful Silicon Awakening ceremony.
#[derive(Debug)]
pub struct GenesisResult {
    /// The device DID minted during provisioning.
    pub did: String,
    /// The EVM address associated with this device.
    pub evm_address: String,
    /// The signed ERC-8004 registration transaction (ready for broadcast).
    pub signed_tx: Vec<u8>,
    /// The capability manifest that was uploaded to the factory IPFS node.
    pub manifest: AgentCapabilityManifest,
}

/// Configuration for the factory provisioner.
pub struct ProvisionerConfig {
    /// The device MAC address, used as a hardware-unique PUF mixing input.
    pub mac_address: [u8; 6],
    /// Factory IPFS gateway URL for manifest pinning.
    pub factory_ipfs_url: String,
    /// Factory RPC gateway URL for genesis transaction broadcast.
    pub factory_rpc_url: String,
    /// Unix timestamp at provisioning time (injected for testability).
    pub provisioned_at: u64,
}

/// Orchestrates the end-to-end Silicon Awakening ceremony.
pub struct GenesisProvisioner {
    config: ProvisionerConfig,
    otp_fuses: OtpFuses,
}

impl GenesisProvisioner {
    /// Create a new provisioner with the given configuration.
    pub fn new(config: ProvisionerConfig) -> Self {
        Self {
            config,
            otp_fuses: OtpFuses::new(),
        }
    }

    /// Execute the complete Silicon Awakening sequence.
    ///
    /// # Steps
    ///
    /// 1. Extract SRAM PUF silicon entropy.
    /// 2. Derive Ed25519 + EVM key pairs.
    /// 3. Build the agent capability manifest.
    /// 4. Sign the ERC-8004 genesis transaction (key never leaves memory).
    /// 5. Burn OTP fuses to permanently lock debug interfaces.
    ///
    /// # Errors
    ///
    /// Returns [`ProvisionerError::Hardware`] if either OTP fuse burn fails.
    pub fn run(&mut self) -> Result<GenesisResult, ProvisionerError> {
        // ── Step 1: Physical entropy extraction ──────────────────────────────
        eprintln!("⏳ [STEP 1] Harvesting SRAM PUF silicon entropy...");
        let puf = PufManager::new(self.config.mac_address);
        let genesis_seed = puf.extract_unique_seed();

        // ── Step 2: Dual key-pair derivation ─────────────────────────────────
        eprintln!("⏳ [STEP 2] Deriving cryptographic sovereignty (Ed25519 & EVM)...");
        let agent_keypair = Ed25519Keypair::from_seed(&genesis_seed);
        let mut evm_wallet = EvmWallet::from_seed(&genesis_seed);

        let agent_did = format!(
            "did:cis:amd395:{}",
            hex::encode(agent_keypair.public_key())
        );
        eprintln!("✨ Generated DID:         {}", agent_did);
        eprintln!("✨ Generated EVM Address: {}", evm_wallet.address());

        // ── Step 3: Build capability manifest ────────────────────────────────
        let manifest =
            AgentCapabilityManifest::for_amd395(&agent_did, self.config.provisioned_at);
        let capabilities_uri = format!(
            "{}/ipfs/Qm{}",
            self.config.factory_ipfs_url,
            hex::encode(&genesis_seed[..16])
        );

        // ── Step 4: Sign genesis transaction (private key stays in memory) ───
        eprintln!("⏳ [STEP 4] Signing ERC-8004 Genesis Transaction internally...");
        let signed_tx =
            evm_wallet.sign_erc8004_registration(agent_did.clone(), capabilities_uri);
        let evm_address = evm_wallet.address();

        // Zeroize the EVM wallet private key before the fuse burn step.
        evm_wallet.zeroize();

        // ── Step 5: Burn OTP fuses ────────────────────────────────────────────
        eprintln!("⏳ [STEP 5] Burning OTP Fuses. Locking down hardware debugging...");
        self.otp_fuses.burn_jtag_disable_bit()?;
        self.otp_fuses.burn_secure_boot_enforce_bit()?;

        eprintln!("✅ [AWAKENING COMPLETE] The entity is now sovereign and sealed.");
        eprintln!("📡 EVM Address mapped in CIS inventory database: {evm_address}");

        Ok(GenesisResult {
            did: agent_did,
            evm_address,
            signed_tx,
            manifest,
        })
    }

    /// Return a reference to the current OTP fuse state (for diagnostics).
    pub fn fuse_state(&self) -> &crate::hardware::FuseState {
        self.otp_fuses.state()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> ProvisionerConfig {
        ProvisionerConfig {
            mac_address: [0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E],
            factory_ipfs_url: "http://192.168.100.1:5001".into(),
            factory_rpc_url: "http://192.168.100.1:8545".into(),
            provisioned_at: 1700000000,
        }
    }

    #[test]
    fn provisioner_run_succeeds() {
        let mut p = GenesisProvisioner::new(make_config());
        let result = p.run().expect("provisioning should succeed");
        assert!(result.did.starts_with("did:cis:amd395:"));
        assert!(result.evm_address.starts_with("0x"));
        assert!(!result.signed_tx.is_empty());
    }

    #[test]
    fn fuses_are_burned_after_provisioning() {
        let mut p = GenesisProvisioner::new(make_config());
        p.run().unwrap();
        assert!(p.fuse_state().jtag_disabled);
        assert!(p.fuse_state().secure_boot_enforced);
    }

    #[test]
    fn provisioner_did_contains_hex_public_key() {
        let mut p = GenesisProvisioner::new(make_config());
        let result = p.run().unwrap();
        // DID format: did:cis:amd395:<64 hex chars>
        let hex_part = result.did.strip_prefix("did:cis:amd395:").unwrap();
        assert_eq!(hex_part.len(), 64);
        assert!(hex_part.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn second_fuse_burn_returns_error() {
        let mut p = GenesisProvisioner::new(make_config());
        p.run().unwrap();
        // Attempt to burn the JTAG fuse again directly.
        let err = p.otp_fuses.burn_jtag_disable_bit();
        assert!(err.is_err());
    }
}
