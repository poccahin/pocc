# ZK Compressor (L2.5 Regional Gateway)

This module scaffolds a RISC Zero based compressor that folds many x402 settlement events into a single proof artifact.

- `methods/guest/src/main.rs`: guest circuit that validates signatures and computes a rolling root.
- `host/src/main.rs`: host runner that packages a batch, generates a receipt, and prepares payloads for Quorum AppChain submission.

> Note: this is a blueprint scaffold and requires the full RISC Zero workspace wiring (`methods` crate generation and prover configuration) before production use.
