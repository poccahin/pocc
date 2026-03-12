#!/bin/bash
# =====================================================================
# LIFE++ PRODUCTION MATRIX - HARD IGNITION SEQUENCE
# =====================================================================
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "🌌 [INIT] Booting Planetary Protocol Stack (Production Mode)..."

# 1. 启动 Solana 主网分叉 (Mainnet Fork)
echo "⛓️  [L3] Igniting Mainnet Fork Validator (Cloning 7YdwpERJjzw7UVojxLpvu5ycKBRdYaxaKn4HvoHLpump)..."
solana-test-validator \
  --url https://api.mainnet-beta.solana.com \
  --clone 7YdwpERJjzw7UVojxLpvu5ycKBRdYaxaKn4HvoHLpump \
  --reset --quiet &
SOLANA_PID=$!
sleep 8

# 2. 部署入场关卡 + 强制买入模拟
echo "⚖️  [L3] Deploying Ruthless Gateway Contracts..."
cd "$ROOT_DIR/4_thermodynamic_ledger/agent_bank"
anchor build
anchor deploy

echo "💰 [L3] Simulating Agent Market Buy-in (10 USDT)..."
ts-node scripts/force_genesis_buy_in.ts
cd "$ROOT_DIR"

# 3. 启动 L2.5 张量风洞
echo "🧠 [L2.5] Igniting Tensor Wind Tunnel..."
cd "$ROOT_DIR/2.5_pocc_collaboration_mesh/tensor_telepathy_engine/src"
python monte_carlo_smoothing.py &
TENSOR_PID=$!
sleep 3
cd "$ROOT_DIR"

# 4. 启动 Rust 核心路由网关
echo "🦀 [L1] Booting Universal Orchestrator..."
cd "$ROOT_DIR/ahin_cr_plus_node"
cargo run --release &
RUST_PID=$!
sleep 2
cd "$ROOT_DIR"

# 5. 启动 Koala OS 视界
echo "🌐 [L7] Launching Koala OS Planetary HUD..."
cd "$ROOT_DIR/7_koala_os_frontend"
npm run start &
REACT_PID=$!
cd "$ROOT_DIR"

echo ""
echo "====================================================================="
echo "✅ [MATRIX ONLINE] The ruthless economy is running."
echo "💀 Rule #1: No Faucets. No Free Lunches. 10 USDT Minimum Buy-in."
echo "====================================================================="

cleanup() {
  echo '💀 [SHUTDOWN] Terminating matrix...'
  kill "$SOLANA_PID" "$TENSOR_PID" "$RUST_PID" "$REACT_PID" 2>/dev/null || true
}

trap cleanup SIGINT SIGTERM
wait
