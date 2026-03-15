//! `cis_genesis_provision` – CIS Silicon Awakening binary.
//!
//! This binary is cross-compiled and deployed to the Line-End Station
//! (LES) on the CIS factory production floor.  It runs exactly once on
//! each edge compute node before the device is packaged for shipment.
//!
//! # Usage
//!
//! ```text
//! cis_genesis_provision [--mac <AA:BB:CC:DD:EE:FF>] [--rpc <url>] [--ipfs <url>]
//! ```
//!
//! All flags are optional; defaults target the factory-internal network
//! (192.168.100.x).

use cis_genesis_provisioner::provisioner::{GenesisProvisioner, ProvisionerConfig};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    println!("==================================================");
    println!("🔥 [CIS FACTORY] INITIATING SILICON AWAKENING");
    println!("==================================================\n");

    let mac_address = parse_mac_from_env_or_default();
    let factory_rpc_url =
        std::env::var("CIS_FACTORY_RPC").unwrap_or_else(|_| "http://192.168.100.1:8545".into());
    let factory_ipfs_url =
        std::env::var("CIS_FACTORY_IPFS").unwrap_or_else(|_| "http://192.168.100.1:5001".into());

    let provisioned_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let config = ProvisionerConfig {
        mac_address,
        factory_ipfs_url,
        factory_rpc_url,
        provisioned_at,
    };

    let mut provisioner = GenesisProvisioner::new(config);

    match provisioner.run() {
        Ok(result) => {
            println!("\n📋 [GENESIS RESULT]");
            println!("   DID:         {}", result.did);
            println!("   EVM Address: {}", result.evm_address);
            println!(
                "   Signed TX:   {} bytes (ready for gateway broadcast)",
                result.signed_tx.len()
            );
            println!("\n✅ [PASS] Station 7 Complete. Box is ready for packaging.");
        }
        Err(e) => {
            eprintln!("❌ [ERROR] Silicon Awakening failed: {e}");
            std::process::exit(1);
        }
    }
}

/// Parse the MAC address from the `CIS_MAC_ADDRESS` environment variable,
/// or fall back to a placeholder `AA:BB:CC:DD:EE:FF`.
fn parse_mac_from_env_or_default() -> [u8; 6] {
    let raw = std::env::var("CIS_MAC_ADDRESS").unwrap_or_else(|_| "AA:BB:CC:DD:EE:FF".into());
    let parts: Vec<&str> = raw.split(':').collect();
    if parts.len() == 6 {
        let mut mac = [0u8; 6];
        let mut valid = true;
        for (i, part) in parts.iter().enumerate() {
            match u8::from_str_radix(part, 16) {
                Ok(b) => mac[i] = b,
                Err(_) => {
                    valid = false;
                    break;
                }
            }
        }
        if valid {
            return mac;
        }
    }
    eprintln!("⚠️  Invalid CIS_MAC_ADDRESS; using default placeholder.");
    [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]
}
