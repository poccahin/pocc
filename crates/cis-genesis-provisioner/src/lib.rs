//! CIS Genesis Provisioner
//!
//! Zero-trust factory-line identity bootstrapping for Cognitive Intelligence
//! Systems (CIS) edge compute nodes.
//!
//! This crate implements the **Silicon Awakening** ceremony described in the
//! CIS Enterprise Integration & Delivery Blueprint v1.0. It runs once on the
//! factory production line (Line-End Station) to:
//!
//! 1. Simulate hardware-entropy extraction (SRAM PUF seed derivation).
//! 2. Derive Ed25519 and EVM (secp256k1) cryptographic key pairs from the seed.
//! 3. Build an ERC-8004 agent capability manifest.
//! 4. Sign the genesis registration transaction internally (private key never leaves the device).
//! 5. Simulate OTP fuse burning to permanently lock JTAG/UART debug interfaces.
//!
//! # Zero-Trust Principle
//!
//! Private keys are generated **on-device** from physical silicon entropy and
//! are never transmitted off-device. The signed registration transaction is the
//! only artefact forwarded to the factory gateway for broadcast.

pub mod crypto;
pub mod hardware;
pub mod manifest;
pub mod provisioner;

pub use provisioner::MEASUREMENT_DISPLAY_LENGTH;
