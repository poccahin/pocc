//! # Agent Wallet Vault
//!
//! A hardware-rooted Solana private-key sandbox that ensures **no agent ever
//! touches a raw private key**.  All signing happens inside the vault; the only
//! artefacts that leave are signed transactions and a post-quantum attestation
//! that proves the signature was produced by an authorised vault instance.
//!
//! ## Security architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  Agent process  (untrusted, cannot see any raw key material)    │
//! │  ┌──────────────────────────────────────────────────────────┐   │
//! │  │  1. Obtain AuthToken from vault operator out-of-band     │   │
//! │  │  2. Build AgentAuthProof (HMAC commitment – ZK-like)     │   │
//! │  │  3. Submit SigningRequest{ tx_bytes, proof, nonce }      │   │
//! │  │  4. Receive SigningResponse{ signed_tx, vault_sig }      │   │
//! │  └────────────────────────┬─────────────────────────────────┘   │
//! └───────────────────────────┼─────────────────────────────────────┘
//!                             │  IPC / RPC (never raw key bytes)
//! ┌───────────────────────────▼─────────────────────────────────────┐
//! │  SolanaKeyVault  (TEE-isolated, privileged process)             │
//! │                                                                 │
//! │  Sealed key store  ──► TEE measurement binding                 │
//! │                     ──► Post-quantum KEM key wrap (Kyber-1024) │
//! │  Auth verification ──► HMAC commitment proof verification      │
//! │  Signing           ──► Ed25519 (Solana) inside vault only      │
//! │  Response attest.  ──► Dilithium-3 post-quantum signature      │
//! │  Audit log         ──► Append-only Merkle hash chain           │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Quantum-resistance properties
//!
//! | Threat                        | Mitigation                                    |
//! |-------------------------------|-----------------------------------------------|
//! | Harvest-now / decrypt-later   | Kyber-1024 KEM wraps the sealed key blob      |
//! | Forged vault response         | Dilithium-3 lattice signature on every reply  |
//! | Compromised auth token        | HMAC-SHA-256 commitment + per-request nonce   |
//! | Key extraction via replay     | Nonce registry (constant-time lookup)         |
//! | Software tampering            | TEE measurement chain – wrong build ≠ unseal  |
//!
//! ## Zero-knowledge auth proof
//!
//! The [`AgentAuthProof`] is a *commit-and-reveal* scheme: the agent commits to
//! its `auth_token` via `HMAC-SHA256(auth_token, nonce ‖ agent_id ‖ intent)`
//! without ever sending the raw token.  The vault, which holds the same token,
//! recomputes the expected commitment and accepts the request only on an exact
//! match.  This is functionally equivalent to a Sigma-protocol "proof of
//! knowledge of the pre-image of a hash" without requiring a circuit prover.

pub mod auth;
pub mod audit;
pub mod vault;
pub mod zk_commitment;

pub use auth::{AgentAuthProof, AuthError, AuthToken};
pub use audit::{AuditEntry, AuditLog};
pub use vault::{SigningRequest, SigningResponse, SolanaKeyVault, VaultError};
pub use zk_commitment::{CommitmentError, ZkCommitmentScheme};
