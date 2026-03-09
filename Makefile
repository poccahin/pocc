# ==============================================================================
# 🪐 Life++ Planetary Core - Polyglot Build Orchestrator
# Supported Toolchains: Zig (L0), Rust (L1/L3/L4), Python/Mojo (L2/L6), Node.js (L7)
# ==============================================================================

.PHONY: all help init build-hardware build-network build-cognitive build-omnisphere build-planetary test spin-devnet clean

# 默认目标
all: help

help:
	@echo "🪐 Life++ Protocol Build System"
	@echo "-------------------------------------------------------"
	@echo "Commands:"
	@echo "  make init                 - Install all polyglot dependencies (rustup, zig, npm, pip)"
	@echo "  make build-planetary      - Build the entire Kardashev Type-I Stack (L0 to L7)"
	@echo "  make spin-devnet          - Ignite the local planetary simulator (AHIN + Koala OS)"
	@echo "  make test                 - Run deterministic tests across all layers"
	@echo "  make clean                - Wipe all build artifacts and sensory waste"
	@echo "-------------------------------------------------------"

init:
	@echo "📦 [Init] Bootstrapping Life++ toolchains..."
	@# 实际工程中这里会检查并拉取必要的编译器版本
	@cargo --version || (echo "Rust missing. Install via rustup" && exit 1)
	@zig version || (echo "Zig missing. Please install Zig 0.11+" && exit 1)
	@npm --version || (echo "Node.js missing. Please install Node 18+" && exit 1)
	@echo "✅ All core toolchains present."

build-hardware:
	@echo "🛡️ [L0] Compiling Physical Trust Root & Chopping Watchdogs (Zig)..."
	cd 1_kinetic_trust_root/hardware_chopping_estop && zig build -Doptimize=ReleaseFast
	cd 1_kinetic_trust_root/puf_heterogeneous_multisig && zig build -Doptimize=ReleaseSafe

build-network:
	@echo "🕸️ [L1-L4] Compiling AHIN, PoCC, and Thermodynamic Ledger (Rust)..."
	cargo build --release --workspace

build-cognitive:
	@echo "🧠 [L2/L6] Setting up Cognitive Cortex & Planetary Defense (Python/Mojo)..."
	pip install -r 3_cai_cognitive_cortex/requirements.txt
	pip install -r 6_planetary_defense/requirements.txt

build-omnisphere:
	@echo "🌍 [L7] Bundling Koala OS & 3D Gaussian Splatting Engine (TypeScript/WebGL)..."
	cd 7_koala_os_frontend && npm install && npm run build

build-planetary: build-hardware build-network build-cognitive build-omnisphere
	@echo "✨ [Success] Planetary Core compiled successfully. You are ready to ignite."

test:
	@echo "🧪 Running cross-domain deterministic tests..."
	cd 1_kinetic_trust_root/hardware_chopping_estop && zig build test
	cargo test --workspace --locked
	pytest 3_cai_cognitive_cortex/tests/

spin-devnet: build-planetary
	@echo "🚀 [Ignition] Spinning up Life++ Local Planetary Simulator..."
	@echo "-> Booting AHIN Tracker Node (Port 8000)..."
	./target/release/ahin_node --network devnet &
	@echo "-> Booting CAI Cognitive Cortex..."
	python 3_cai_cognitive_cortex/gateway.py &
	@echo "-> Launching Koala OS Earth Twin (http://localhost:3000)..."
	cd 7_koala_os_frontend && npm run dev

clean:
	@echo "🧹 Wiping build artifacts and purging holographic distillations..."
	cargo clean
	rm -rf 7_koala_os_frontend/node_modules
	rm -rf 7_koala_os_frontend/.next
	find . -type d -name "__pycache__" -exec rm -r {} +
	@echo "✅ Clean complete."
