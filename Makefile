# Life++ Planetary Core - Unified Build System

.PHONY: all boot-l0 compile-l1-l4 train-l2.5 build-l7 ignite-testnet ignite-local generate-mtls-certs check-docker

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
	cd 7_koala_os_frontend && npm ci --legacy-peer-deps && npm run build

ignite-testnet: check-docker
	@echo "🔥 Igniting the Promethean Crucible Testnet..."
	cd tests/pocc_resonance && docker-compose up --build

# Docker availability check — used by ignite-testnet
check-docker:
	@command -v docker >/dev/null 2>&1 || { \
	  echo "❌ Docker is not installed or not in PATH."; \
	  echo "   Install Docker Desktop: https://docs.docker.com/get-docker/"; \
	  echo "   Or run 'make ignite-local' to start services without Docker."; \
	  exit 1; \
	}
	@docker info >/dev/null 2>&1 || { \
	  echo "❌ Docker daemon is not running."; \
	  echo "   Start Docker Desktop or the Docker service, then retry."; \
	  echo "   Or run 'make ignite-local' to start services without Docker."; \
	  exit 1; \
	}

# Docker-free local development — starts each service process directly
ignite-local:
	@echo "🔥 [LOCAL] Starting Planetary Protocol Stack (no Docker required)..."
	@echo ""
	@echo "⛓️  [L3] Note: Start solana-test-validator manually if needed:"
	@echo "   solana-test-validator --reset --quiet &"
	@echo ""
	@echo "🧠 [L2.5] Starting Tensor Wind Tunnel..."
	@cd 2.5_pocc_collaboration_mesh/tensor_telepathy_engine/src && \
	  pip install -r ../../requirements.txt --quiet && \
	  python monte_carlo_smoothing.py &
	@sleep 2
	@echo "🦀 [L1] Starting AHIN CR+ Router..."
	@cd 2_ahin_nervous_system/cr_plus_tensor_routing && cargo run --release &
	@sleep 2
	@echo "🌐 [L7] Starting Koala OS Frontend (dev server)..."
	@cd 7_koala_os_frontend && npm ci --legacy-peer-deps --quiet && npm run dev &
	@echo ""
	@echo "====================================================================="
	@echo "✅ [LOCAL MATRIX ONLINE] Services started in the background."
	@echo "   L2.5 Tensor Validator : http://localhost:50051 (gRPC)"
	@echo "   L1 AHIN Router        : http://localhost:8000 / ws://localhost:9000"
	@echo "   L7 Koala OS Frontend  : http://localhost:5173 (Vite dev server)"
	@echo "   To stop all services:  pkill -f monte_carlo_smoothing.py;"
	@echo "                          pkill -f cr_plus_tensor_routing;"
	@echo "                          pkill -f 'vite'"
	@echo "====================================================================="


generate-mtls-certs:
	@echo "🔐 Generating mTLS certificates for PoCC wind tunnel..."
	mkdir -p certs && cd certs && \
	openssl req -x509 -newkey rsa:4096 -days 3650 -nodes -keyout ca.key -out ca.crt -subj "/CN=LifePlus_Root_CA" && \
	openssl req -newkey rsa:4096 -nodes -keyout server.key -out server.csr -subj "/CN=localhost" && \
	openssl x509 -req -in server.csr -CA ca.crt -CAkey ca.key -CAcreateserial -out server.crt -days 3650 && \
	openssl req -newkey rsa:4096 -nodes -keyout client.key -out client.csr -subj "/CN=LifePlus_Rust_Gateway" && \
	openssl x509 -req -in client.csr -CA ca.crt -CAkey ca.key -CAcreateserial -out client.crt -days 3650 && \
	rm -f *.csr
