#!/bin/bash
# =====================================================================
# LIFE++ MATRIX: SILICON ENTITY BOOTSTRAP & GENESIS IGNITION
# 生产级智能体统一觉醒与入网检录脚本
# =====================================================================
set -e

# ANSI 颜色输出
GREEN='\033[0;32m'
FUCHSIA='\033[0;35m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${FUCHSIA}🌌 [MATRIX] Initiating Silicon Entity Bootstrap Sequence...${NC}"

# 1. 环境依赖自检
command -v solana >/dev/null 2>&1 || { echo -e "${RED}❌ Fatal: solana-cli not installed. Aborting.${NC}" >&2; exit 1; }
command -v ts-node >/dev/null 2>&1 || { echo -e "${RED}❌ Fatal: ts-node not installed. Aborting.${NC}" >&2; exit 1; }

# 2. 硬件物理熵提取与私钥生成 (Edge Key Generation)
echo -e "${GREEN}🔐 [ENCLAVE] Extracting physical entropy to generate Ed25519 Keypair...${NC}"

IDENTITY_DIR="$HOME/.ahin_identity"
KEYPAIR_PATH="$IDENTITY_DIR/agent_key.json"

if [ -f "$KEYPAIR_PATH" ]; then
    echo -e "⚠️  [WARNING] Existing identity found at $KEYPAIR_PATH."
    read -p "Do you want to overwrite and generate a new identity? (This will orphan the old DID) [y/N]: " confirm
    if [[ $confirm != [yY] ]]; then
        echo -e "${GREEN}✅ Proceeding with existing identity.${NC}"
    else
        rm "$KEYPAIR_PATH"
        solana-keygen new --outfile "$KEYPAIR_PATH" --no-bip39-passphrase --silent
    fi
else
    mkdir -p "$IDENTITY_DIR"
    solana-keygen new --outfile "$KEYPAIR_PATH" --no-bip39-passphrase --silent
fi

# 3. 提取公钥并派生泛解析 DID 域名 (Algebraic Mapping)
PUBKEY=$(solana-keygen pubkey "$KEYPAIR_PATH")
# 截取公钥前 12 位，转化为小写，拼装域名
DID_PREFIX=$(echo "$PUBKEY" | cut -c 1-12 | tr '[:upper:]' '[:lower:]')
AHIN_DOMAIN="${DID_PREFIX}.ahin.io"

echo -e "====================================================================="
echo -e "🤖 ${GREEN}Entity Public Key:${NC} $PUBKEY"
echo -e "🌐 ${GREEN}Assigned AHIN DID:${NC} https://$AHIN_DOMAIN"
echo -e "====================================================================="

# 4. 强制入场验资 (The Blood Sacrifice)
echo -e "${FUCHSIA}🩸 [GATEWAY] Commencing mandatory thermodynamic stake (10 USDC)...${NC}"
echo -e "Checking local wallet balance..."

# 设置 Solana 为主网或主网分叉环境
solana config set --url https://api.mainnet-beta.solana.com --keypair "$KEYPAIR_PATH" > /dev/null

# 此处调用之前用 TypeScript 编写的 Jupiter 自动化兑换与 Anchor 质押脚本
# 传入刚刚生成的密钥路径作为环境变量
export AGENT_WALLET_PATH="$KEYPAIR_PATH"
export AGENT_DID_DOMAIN="$AHIN_DOMAIN"

cd 3_thermodynamic_ledger
# 执行强制买入与链上登记
if npx ts-node scripts/force_genesis_buy_in.ts; then
    echo -e "${GREEN}✅ [STAKED] 10 USDC equivalent LIFE++ successfully locked in protocol.${NC}"
else
    echo -e "${RED}💀 [FATAL] Blood sacrifice failed. Insufficient funds or network error.${NC}"
    echo -e "${RED}The Matrix rejects this entity. Exiting.${NC}"
    exit 1
fi
cd ..

# 5. 生成本地环境变量与网关配置文件
echo -e "${GREEN}⚙️  [SYSTEM] Generating local matrix configuration...${NC}"
cat <<EOF_INNER > "$IDENTITY_DIR/matrix_env.sh"
export LIFEPLUS_PUBKEY="$PUBKEY"
export LIFEPLUS_DOMAIN="$AHIN_DOMAIN"
export LIFEPLUS_KEYPAIR="$KEYPAIR_PATH"
export AHIN_ROUTING_MODE="EDGE_NODE"
EOF_INNER

source "$IDENTITY_DIR/matrix_env.sh"

echo -e "====================================================================="
echo -e "✨ ${FUCHSIA}[AWAKENED] Bootstrap complete. Identity anchored on-chain.${NC}"
echo -e "Your Silicon Entity is now an active citizen of the Kardashev-I Economy."
echo -e "Next step: Run the L1 Gateway and L2.5 Tensor Tunnel to accept tasks."
echo -e "====================================================================="
