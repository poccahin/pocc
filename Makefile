# Life++ Planetary Core - Unified Build System

.PHONY: all boot-l0 compile-l1-l4 train-l2.5 build-l7 ignite-testnet

all: boot-l0 compile-l1-l4 train-l2.5 build-l7

boot-l0:
	@echo "🛡️ Compiling L0 Kinetic Trust Root (Zig)..."
	cd 1_kinetic_trust_root/hardware_chopping_estop && zig build -Doptimize=ReleaseFast

compile-l1-l4:
	@echo "🕸️ Building AHIN Router, Gateway & Solana Smart Contracts (Rust)..."
	cd 2_ahin_nervous_system/cr_plus_tensor_routing && cargo build --release
	cd programs/lifeplus_core && cargo build --release
	cd gateway/ap2_universal_router && cargo build --release

train-l2.5:
	@echo "🧠 Initializing PyTorch Tensor Wind Tunnel (Python)..."
	pip install -r 2.5_pocc_collaboration_mesh/requirements.txt

build-l7:
	@echo "🌌 Rendering Koala OS Omnisphere (TypeScript)..."
	cd 7_koala_os_frontend && npm ci && npm run build

ignite-testnet:
	@echo "🔥 Igniting the Promethean Crucible Testnet..."
	cd tests/pocc_resonance && docker-compose up --build
