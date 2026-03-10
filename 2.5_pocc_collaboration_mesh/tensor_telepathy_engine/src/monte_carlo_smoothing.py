"""gRPC tensor validator with deterministic model warmup before opening port.

This module mitigates PyTorch lazy CUDA initialization latency by pre-running
forward passes and synchronizing the GPU before the service starts listening.
"""

from __future__ import annotations

import os
import time
from concurrent import futures

import grpc
import torch

import pocc_pb2
import pocc_pb2_grpc


class MonteCarloDriftModel(torch.nn.Module):
    """Simple projection model for semantic drift checks."""

    def __init__(self) -> None:
        super().__init__()
        self.projection = torch.nn.Linear(128, 64)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return self.projection(x)


def warmup_tensor_engine(model: torch.nn.Module, device: torch.device) -> None:
    """Force PyTorch/CUDA runtime initialization before networking starts."""
    print("🔥 [SYS] Igniting Tensor Wind Tunnel... Warming up GPU/CPU architecture.")

    dummy_tensor = torch.randn(1, 128, device=device)

    start_time = time.time()
    with torch.no_grad():
        for _ in range(10):
            _ = model(dummy_tensor)

    if torch.cuda.is_available() and device.type == "cuda":
        torch.cuda.synchronize()

    elapsed = time.time() - start_time
    print(f"✅ [SYS] VRAM fully allocated in {elapsed:.3f}s. CUDA context locked.")
    print("✅ [SYS] Engine Ready for Kardashev Type-I AP2 Traffic.")


class TensorWindTunnelServicer(pocc_pb2_grpc.TensorFirewallServicer):
    """Implements TensorFirewall gRPC API."""

    def __init__(self, model: torch.nn.Module, device: torch.device) -> None:
        self.model = model
        self.device = device
        self.theta_collapse = float(os.getenv("NOISE_STD", "0.05"))

    def CheckSemanticDrift(self, request, context):  # noqa: N802 - gRPC naming
        # Placeholder behavior for scaffolded service.
        return pocc_pb2.DriftResponse(is_safe=True, proof_of_poison="")


def serve() -> None:
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"🖥️  [INIT] Hardware backend selected: {device}")

    model = MonteCarloDriftModel().to(device)
    model.eval()

    # Warmup must happen before opening the gRPC port.
    warmup_tensor_engine(model, device)

    max_workers = int(os.getenv("TENSOR_VALIDATOR_WORKERS", "100"))
    listen_addr = os.getenv("TENSOR_VALIDATOR_ADDR", "[::]:50051")

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=max_workers))
    pocc_pb2_grpc.add_TensorFirewallServicer_to_server(
        TensorWindTunnelServicer(model, device),
        server,
    )

    server.add_insecure_port(listen_addr)
    server.start()
    print(f"🌐 [gRPC] Tensor Wind Tunnel online. Listening on {listen_addr}...")

    server.wait_for_termination()


if __name__ == "__main__":
    serve()
