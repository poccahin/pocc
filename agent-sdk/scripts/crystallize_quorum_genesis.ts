import { ethers } from 'ethers';

// =====================================================================
// 🌌 LIFE++ GENESIS CRYSTALLIZATION (QUORUM PRIVACY EDITION)
// =====================================================================

// Quorum 联盟链 RPC 节点 (带有 Tessera 隐私飞地支持)
const QUORUM_RPC_URL =
  process.env.QUORUM_RPC_URL || 'https://quorum.ahin.io/node-1';

// 聚合器 (Aggregator) 私钥，提取自 SolveX 部署环境
const AGGREGATOR_PK =
  process.env.AGGREGATOR_PK || '0x_SOLVEX_AGGREGATOR_PRIVATE_KEY';

// 部署在 Quorum 上的 Life++ 结算合约地址
const SETTLEMENT_CONTRACT_ADDRESS =
  process.env.SETTLEMENT_CONTRACT_ADDRESS ||
  '0x8888888888888888888888888888888888888888';

// 授权可解密底层 CTx 详情的合规审计节点 (Tessera Public Keys)
// 例如：香港证监会授权信托、学术观测节点、核心 Slasher 节点
const AUTHORIZED_ENCLAVES = (
  process.env.AUTHORIZED_ENCLAVES ||
  'BULeR8JyUWhiuuCMU/HLA0Q5pzkYT+cHII3ZKBey3Bo=,QfeDAys9MPDs2XHExtc84jKGHxZg/aj52DTh0vtA3Xc='
)
  .split(',')
  .map((v) => v.trim())
  .filter(Boolean);

const SETTLEMENT_ABI = [
  'function submitBatchRoot(bytes32 newRoot, uint256 foldedTxCount) external returns (bool)',
  'event RootCrystallized(bytes32 indexed root, uint256 count, uint256 timestamp)',
];

function assertConfiguration(genesisRootHex: string): void {
  if (!ethers.isHexString(genesisRootHex, 32)) {
    throw new Error(
      `Invalid genesis root: expected 32-byte hex string, got ${genesisRootHex}`,
    );
  }

  if (!ethers.isAddress(SETTLEMENT_CONTRACT_ADDRESS)) {
    throw new Error(
      `Invalid settlement contract address: ${SETTLEMENT_CONTRACT_ADDRESS}`,
    );
  }

  if (!AGGREGATOR_PK.startsWith('0x') || AGGREGATOR_PK.length !== 66) {
    throw new Error(
      'AGGREGATOR_PK must be set to a 32-byte private key (0x-prefixed).',
    );
  }

  if (AUTHORIZED_ENCLAVES.length === 0) {
    throw new Error('AUTHORIZED_ENCLAVES cannot be empty.');
  }
}

async function crystallizeQuorumGenesis(
  genesisRootHex: string,
  txCount: number,
): Promise<void> {
  console.log('==================================================');
  console.log('🏛️ [QUORUM L1] INITIATING PRIVACY-PRESERVING GENESIS');
  console.log('==================================================\n');

  assertConfiguration(genesisRootHex);

  const provider = new ethers.JsonRpcProvider(QUORUM_RPC_URL);
  const wallet = new ethers.Wallet(AGGREGATOR_PK, provider);
  const contract = new ethers.Contract(
    SETTLEMENT_CONTRACT_ADDRESS,
    SETTLEMENT_ABI,
    wallet,
  );

  console.log(`🔌 Connected to Quorum Enclave. Aggregator: ${wallet.address}`);
  console.log(`📦 Public Payload: [Root: ${genesisRootHex}, Folded CTx: ${txCount}]`);
  console.log(
    `🔐 Private Routing to ${AUTHORIZED_ENCLAVES.length} authorized enclaves.`,
  );
  console.log('\n⏳ Transmitting ZK-Folded State to Quorum EVM...');

  try {
    // 在 Quorum 中，可通过 JSON-RPC 私有交易扩展将 privateFor 注入 Tessera。
    // 这里通过 customData 传递给支持 Quorum 扩展的 provider/adapter。
    const tx = await contract.submitBatchRoot(genesisRootHex, txCount, {
      gasLimit: 500000,
      customData: {
        privateFor: AUTHORIZED_ENCLAVES,
      },
    });

    console.log(
      `🚀 Transaction Broadcasted to Privacy Enclave! TxHash: ${tx.hash}`,
    );
    console.log('⌛ Waiting for Quorum IBFT Consensus...');

    const receipt = await tx.wait();

    console.log('\n✨ [QUORUM CRYSTALLIZATION COMPLETE]');
    console.log(`🧱 Block Number: ${receipt?.blockNumber}`);
    console.log('🔏 Status: State Root is PUBLIC. Underlying CTx proofs are PRIVATE.');
    console.log(
      '\n💎 The Genesis Merkle Root is permanently inscribed. The Dark Pool is secured.',
    );
  } catch (error) {
    console.error('💥 [FATAL ERROR] Failed to crystallize root on Quorum:', error);
    process.exitCode = 1;
  }
}

// 提取我们在 swarm_simulator 中折叠出的第一批引力塌缩状态根
const GENESIS_ROOT =
  process.env.GENESIS_ROOT ||
  '0xab12cd34ef56ab12cd34ef56ab12cd34ef56ab12cd34ef56ab12cd34ef56ab12';
const FOLDED_TX_COUNT = Number(process.env.FOLDED_TX_COUNT || 4);

crystallizeQuorumGenesis(GENESIS_ROOT, FOLDED_TX_COUNT).catch((error) => {
  console.error('Unhandled crystallization failure:', error);
  process.exit(1);
});
