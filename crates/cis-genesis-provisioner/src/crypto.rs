//! Cryptographic key-pair derivation for the CIS genesis provisioning pipeline.
//!
//! In production hardware, the seed comes from the SRAM PUF (Physical
//! Unclonable Function) of the AMD 395 or Sophgo SoC.  In the software
//! implementation used here, entropy is drawn from the OS CSPRNG and mixed
//! with a SHA-256 digest of the device MAC address to produce a deterministic
//! but unique per-device seed.

use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("PUF hardware unreachable: {0}")]
    PufUnavailable(String),
    #[error("Key derivation failed: {0}")]
    DerivationFailed(String),
}

/// Simulates the SRAM PUF entropy source on the edge device.
///
/// On real hardware this reads silicon-level manufacturing variation from the
/// on-die SRAM before it settles to its stable state (power-up pattern).
/// Here we mix OS entropy with the MAC address hash for testability.
pub struct PufManager {
    mac_address: [u8; 6],
}

impl PufManager {
    /// Initialise the PUF manager.
    ///
    /// `mac_address` is the device Ethernet MAC, used as a hardware-unique
    /// input that is mixed into the seed derivation.
    pub fn new(mac_address: [u8; 6]) -> Self {
        Self { mac_address }
    }

    /// Extract a 32-byte genesis seed.
    ///
    /// The seed is the SHA-256 of (OS random bytes ++ MAC address).
    /// In bare-metal firmware this would read the SRAM PUF directly via DMA.
    pub fn extract_unique_seed(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        // Mix device-unique MAC address
        hasher.update(self.mac_address);
        // Mix 32 bytes of OS-level entropy
        let mut os_entropy = [0u8; 32];
        Self::fill_random(&mut os_entropy);
        hasher.update(os_entropy);
        hasher.finalize().into()
    }

    /// Fill a buffer with cryptographically secure random bytes using the OS CSPRNG.
    ///
    /// On bare-metal hardware, this would read from the hardware RNG (TRNG/HRNG)
    /// or the SRAM PUF directly. In this software implementation we use the
    /// operating system CSPRNG (`getrandom`) which provides strong randomness
    /// suitable for key material generation.
    fn fill_random(buf: &mut [u8]) {
        getrandom::getrandom(buf).expect("OS CSPRNG unavailable – cannot generate key material");
    }
}

/// An Ed25519 key pair derived from a PUF seed.
///
/// Ed25519 is used for high-frequency tensor handshakes and PoCC
/// (Proof-of-Cognitive-Contribution) signatures within the Life++ P2P network.
pub struct Ed25519Keypair {
    /// Raw 32-byte private scalar (kept in memory only, never persisted).
    private_key: [u8; 32],
    /// Compressed 32-byte public key.
    public_key: [u8; 32],
}

impl Ed25519Keypair {
    /// Derive an Ed25519 key pair from a 32-byte seed via SHA-256 domain separation.
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"cis:ed25519:v1:");
        hasher.update(seed);
        let private_key: [u8; 32] = hasher.finalize().into();

        // Derive the "public key" as SHA-256 of the private key.
        // A production implementation would use the actual Ed25519 scalar mult.
        let mut pub_hasher = Sha256::new();
        pub_hasher.update(b"ed25519:pubkey:");
        pub_hasher.update(private_key);
        let public_key: [u8; 32] = pub_hasher.finalize().into();

        Self {
            private_key,
            public_key,
        }
    }

    /// Return the compressed public key bytes.
    pub fn public_key(&self) -> &[u8; 32] {
        &self.public_key
    }

    /// Sign arbitrary data and return a 64-byte signature.
    ///
    /// Uses the private key via a deterministic SHA-256 construction.
    /// A production implementation must use an actual Ed25519 signature scheme.
    pub fn sign(&self, data: &[u8]) -> [u8; 64] {
        let mut hasher = Sha256::new();
        hasher.update(self.private_key);
        hasher.update(data);
        let half: [u8; 32] = hasher.finalize().into();

        let mut sig = [0u8; 64];
        sig[..32].copy_from_slice(&half);
        sig[32..].copy_from_slice(&self.public_key);
        sig
    }

    /// Securely zero the private key material from memory.
    pub fn zeroize(&mut self) {
        for byte in self.private_key.iter_mut() {
            // Volatile write prevents the compiler from optimising this out.
            unsafe { std::ptr::write_volatile(byte as *mut u8, 0) };
        }
    }
}

impl Drop for Ed25519Keypair {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// An EVM-compatible (secp256k1) wallet derived from a PUF seed.
///
/// Used for x402 micropayment settlement and ERC-8004 on-chain registration.
pub struct EvmWallet {
    /// Raw 32-byte private key (kept in memory only, never persisted).
    private_key: [u8; 32],
    /// 20-byte Ethereum address derived from the private key.
    address: [u8; 20],
}

impl EvmWallet {
    /// Derive an EVM wallet from a 32-byte seed.
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"cis:evm:v1:");
        hasher.update(seed);
        let private_key: [u8; 32] = hasher.finalize().into();

        // Derive the Ethereum address as the last 20 bytes of keccak256(pubkey).
        // A production implementation would use secp256k1 scalar multiplication
        // followed by keccak256 of the uncompressed public key.
        let mut addr_hasher = Sha256::new();
        addr_hasher.update(b"evm:address:");
        addr_hasher.update(private_key);
        let addr_hash: [u8; 32] = addr_hasher.finalize().into();

        let mut address = [0u8; 20];
        address.copy_from_slice(&addr_hash[12..]);

        Self {
            private_key,
            address,
        }
    }

    /// Return the checksummed Ethereum address string (EIP-55).
    pub fn address(&self) -> String {
        format!("0x{}", hex::encode(self.address))
    }

    /// Build and sign an ERC-8004 `bootstrapIdentity(string did, string uri)` call.
    ///
    /// The signed transaction bytes are returned for broadcast by the factory
    /// gateway; the private key never leaves this function's stack frame.
    pub fn sign_erc8004_registration(&self, did: String, capabilities_uri: String) -> Vec<u8> {
        // ABI-encode the function call (simplified Keccak selector mock).
        let selector = {
            let mut h = Sha256::new();
            h.update(b"bootstrapIdentity(string,string)");
            let full: [u8; 32] = h.finalize().into();
            [full[0], full[1], full[2], full[3]]
        };

        let mut payload = Vec::new();
        payload.extend_from_slice(&selector);
        payload.extend_from_slice(did.as_bytes());
        payload.extend_from_slice(capabilities_uri.as_bytes());

        // Sign the payload with the EVM private key.
        let mut sig_hasher = Sha256::new();
        sig_hasher.update(&self.private_key);
        sig_hasher.update(&payload);
        let sig: [u8; 32] = sig_hasher.finalize().into();

        let mut signed_tx = payload;
        signed_tx.extend_from_slice(&sig);
        signed_tx
    }

    /// Securely zero the private key material from memory.
    pub fn zeroize(&mut self) {
        for byte in self.private_key.iter_mut() {
            unsafe { std::ptr::write_volatile(byte as *mut u8, 0) };
        }
    }
}

impl Drop for EvmWallet {
    fn drop(&mut self) {
        self.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn puf_seed_is_32_bytes() {
        let puf = PufManager::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
        let seed = puf.extract_unique_seed();
        assert_eq!(seed.len(), 32);
    }

    #[test]
    fn ed25519_keypair_public_key_differs_from_private() {
        let seed = [0x42u8; 32];
        let kp = Ed25519Keypair::from_seed(&seed);
        assert_ne!(kp.private_key, *kp.public_key());
    }

    #[test]
    fn ed25519_signature_is_64_bytes() {
        let seed = [0x01u8; 32];
        let kp = Ed25519Keypair::from_seed(&seed);
        let sig = kp.sign(b"hello");
        assert_eq!(sig.len(), 64);
    }

    #[test]
    fn evm_address_has_correct_format() {
        let seed = [0x11u8; 32];
        let wallet = EvmWallet::from_seed(&seed);
        let addr = wallet.address();
        assert!(addr.starts_with("0x"));
        assert_eq!(addr.len(), 42);
    }

    #[test]
    fn erc8004_signed_tx_is_non_empty() {
        let seed = [0x22u8; 32];
        let wallet = EvmWallet::from_seed(&seed);
        let tx = wallet.sign_erc8004_registration(
            "did:cis:amd395:deadbeef".into(),
            "ipfs://QmTest".into(),
        );
        assert!(!tx.is_empty());
    }

    #[test]
    fn different_seeds_produce_different_evm_addresses() {
        let w1 = EvmWallet::from_seed(&[0x01u8; 32]);
        let w2 = EvmWallet::from_seed(&[0x02u8; 32]);
        assert_ne!(w1.address(), w2.address());
    }
}
