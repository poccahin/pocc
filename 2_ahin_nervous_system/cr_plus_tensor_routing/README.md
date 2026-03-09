# CR+ Tensor Routing Engine

`cr_plus_tensor_routing` now contains a runnable AHIN daemon prototype with:

- L1 async TCP ingress loop on Tokio.
- L2.5 tensor robustness gate (currently wired to a mock validator shim).
- L3 Solana Devnet economic settlement hooks for stake queries and slashing burn execution.

## Running locally

```bash
cargo run
```

The daemon listens on `0.0.0.0:8000` and expects JSON intents over TCP.
