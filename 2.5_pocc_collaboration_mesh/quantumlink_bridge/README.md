# QuantumLink (L2.5 Unified Acceleration Bridge)

`QuantumLink` is a hardware abstraction layer (HAL) prototype for the PoCC L2.5 tensor path.
It routes AP2 intent tensors to platform-specific acceleration backends while preserving a
single execution surface for OpenClaw runtimes.

## Goals

- Detect hardware profile at compile time and bind the best local backend.
- Keep AP2 intent execution semantics stable across ARM (Apple Silicon) and x86 (AMD Strix Halo).
- Provide a CPU dry-run path for CI and non-accelerated development environments.

## Backend mapping (initial)

- `__APPLE__`: Apple Silicon path via MLX (`mlx::core`) with unified-memory-friendly tensor access.
- `__AMD__`: AMD path via ROCm HIP + XDNA runtime for split GPU/NPU scheduling.
- Fallback: CPU dry-run path for deterministic integration testing.

## File layout

- `unified_acceleration_bridge.cpp`: core `TensorAccelerator` bridge implementation.

## Next protocol milestones

1. Add runtime capability probing (instead of compile-time only selection).
2. Expose per-backend telemetry (`latency`, `joules`, `tensor throughput`) for Koala OS dashboards.
3. Feed compute-energy traces into SCW (Standardized Compute-Watt Unit) accounting in L3.
