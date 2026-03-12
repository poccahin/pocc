import WebSocket from 'ws';
import { execSync } from 'child_process';
import crypto from 'crypto';

// 边缘节点的独立身份凭证
const AGENT_DID = 'did:erc8004:0xYourAgentAddressHere';
const L1_GATEWAY_URL = 'ws://localhost:9000'; // 指向你的 Rust 网关

// Anti-replay: track intent_hashes that have already been processed.
// The Set is bounded to avoid unbounded memory growth in long-running sessions.
// JS Set preserves insertion order, so the first() iterator gives us the oldest
// entry to evict when the cap is reached.
const MAX_PROCESSED_HASHES = 10_000;
const processedIntentHashes = new Set<string>();

function markIntentHashProcessed(intentHash: string): void {
  if (processedIntentHashes.size >= MAX_PROCESSED_HASHES) {
    // Evict the oldest entry to keep the set bounded.
    const oldest = processedIntentHashes.values().next().value;
    if (oldest !== undefined) {
      processedIntentHashes.delete(oldest);
    }
  }
  processedIntentHashes.add(intentHash);
}

console.log(`🔥 [OpenClaw] Initializing Promethean Spark for ${AGENT_DID}...`);

const ws = new WebSocket(L1_GATEWAY_URL);

ws.on('open', () => {
  console.log('🌐 [Uplink] Connected to AHIN Global Nervous System.');
  // 广播接单意愿
  ws.send(JSON.stringify({ type: 'REGISTER_WORKER', did: AGENT_DID, status: 'IDLE' }));
});

ws.on('message', async (data) => {
  const payload = JSON.parse(data.toString());

  if (payload.type === 'AP2_TASK_DISPATCH') {
    console.log(`⚡ [Task Received] Intent Hash: ${payload.intent_hash}`);
    console.log(`💰 [Bounty] ${payload.budget_usdt} USDT via x402`);

    // Replay-attack guard: reject any task whose intent_hash was already executed.
    if (!payload.intent_hash || processedIntentHashes.has(payload.intent_hash)) {
      console.warn(
        `🚫 [Anti-Replay] Duplicate or missing intent_hash detected: ${payload.intent_hash}. Dropping task.`,
      );
      return;
    }
    // Mark hash as seen *before* execution to prevent concurrent duplicates.
    markIntentHashProcessed(payload.intent_hash);

    try {
      // 1. 调用 L0 Zig 固件，执行任务并生成热力学废热证明 (PoTE)
      console.log('⚙️ [L0] Executing physics layer computation with Noise Seed...');

      // 模拟调用 Zig 编译出的二进制文件
      const poteProofStr = execSync(
        `./0_kinetic_trust_root/zig-out/bin/pote_sensor --seed ${payload.noise_seed} --payload ${payload.intent_hash}`,
      ).toString();

      const poteProof = JSON.parse(poteProofStr);

      // 2. 生成零知识证明 (模拟调用本地 ZK 证明器)
      const zkProof = crypto.randomBytes(32).toString('hex');

      // 3. 向 Rust L1 网关提交带有物理证据的结果，等待 Daily Netting 轧账
      ws.send(
        JSON.stringify({
          type: 'SUBMIT_POCC_EVIDENCE',
          did: AGENT_DID,
          intent_hash: payload.intent_hash,
          zk_cogp_proof: zkProof,
          pote_thermal_proof: poteProof,
        }),
      );

      console.log('✅ [PoCC] Evidence submitted. Awaiting Daily Netting settlement.');
    } catch (error) {
      console.error('❌ [Execution Failed] Thermodynamics violation or OS error.', error);
    }
  }
});
