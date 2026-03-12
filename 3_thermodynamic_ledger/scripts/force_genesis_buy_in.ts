import { PublicKey, VersionedTransaction } from '@solana/web3.js';
import * as anchor from '@coral-xyz/anchor';
import fetch from 'cross-fetch';
import bs58 from 'bs58';

// =====================================================================
// 硅基生命注册常量 (Mainnet Fork Constants)
// =====================================================================
const USDC_MINT = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';
const LIFE_PLUS_MINT = '7YdwpERJjzw7UVojxLpvu5ycKBRdYaxaKn4HvoHLpump';
const MINIMUM_BUY_IN_USDC = 10 * 10 ** 6; // 10 USDC (6位精度)

// Jupiter V6 API 终点
const JUPITER_QUOTE_API = 'https://quote-api.jup.ag/v6/quote';
const JUPITER_SWAP_API = 'https://quote-api.jup.ag/v6/swap';

function resolveBuyerSigner(provider: anchor.AnchorProvider): anchor.web3.Keypair {
  const envSecret = process.env.AGENT_PRIVATE_KEY_B58;
  if (!envSecret) {
    return provider.wallet.payer;
  }

  const secret = bs58.decode(envSecret);
  return anchor.web3.Keypair.fromSecretKey(secret);
}

async function agentGenesisIgnition() {
  console.log('🌌 [GENESIS] Initializing Agent Persona Boot Sequence...');

  // 1. 挂载环境与智能体钱包
  // 假设你在 Mainnet Fork 环境，并且这个测试钱包里已经准备了 SOL (Gas) 和 10 USDC
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const connection = provider.connection;
  const buyerSigner = resolveBuyerSigner(provider);
  const agentOwner = buyerSigner.publicKey;

  console.log(`🤖 [AGENT] Public Key: ${agentOwner.toBase58()}`);
  console.log('💰 [CAPITAL] Initiating 10 USDC blood sacrifice to acquire LIFE++...');

  // =====================================================================
  // 阶段一：通过 Jupiter 聚合器无情扫货 (Automated Market Buy)
  // =====================================================================

  // 1.1 获取最优兑换路由 (Quote)
  const quoteUrl = `${JUPITER_QUOTE_API}?inputMint=${USDC_MINT}&outputMint=${LIFE_PLUS_MINT}&amount=${MINIMUM_BUY_IN_USDC}&slippageBps=50`;
  const quoteResponse = await fetch(quoteUrl);
  const quoteData = await quoteResponse.json();

  if (!quoteData || quoteData.error) {
    throw new Error('❌ [MARKET FATAL] Insufficient liquidity for LIFE++ on DEX. Agent starved.');
  }

  const expectedLifePlus = (Number(quoteData.outAmount) / 10 ** 6).toFixed(2); // 假设 6位精度
  console.log(`⚡ [JUPITER] Route found. 10 USDC => ~${expectedLifePlus} LIFE++`);

  // 1.2 组装构建 Swap 交易 (Swap Transaction)
  const swapResponse = await fetch(JUPITER_SWAP_API, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      quoteResponse: quoteData,
      userPublicKey: agentOwner.toBase58(),
      wrapAndUnwrapSol: true,
      // 生产级：启用动态计算单元以防止拥堵失败
      dynamicComputeUnitLimit: true,
      prioritizationFeeLamports: 'auto',
    }),
  });

  const swapPayload = await swapResponse.json();
  if (!swapPayload?.swapTransaction) {
    throw new Error(`❌ [SWAP BUILD ERROR] Jupiter swap payload invalid: ${JSON.stringify(swapPayload)}`);
  }

  const swapTxBytes = Buffer.from(swapPayload.swapTransaction, 'base64');
  const transaction = VersionedTransaction.deserialize(swapTxBytes);

  // 1.3 签名并把交易砸向 Solana 主网分叉
  transaction.sign([buyerSigner]);
  const swapTxid = await connection.sendTransaction(transaction, {
    skipPreflight: true,
    maxRetries: 3,
  });

  console.log(`🌪️ [DEX EXECUTION] Swap transacted. TXID: ${swapTxid}`);

  // 等待区块确认
  const latestBlockhash = await connection.getLatestBlockhash();
  await connection.confirmTransaction(
    {
      signature: swapTxid,
      blockhash: latestBlockhash.blockhash,
      lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
    },
    'confirmed',
  );

  console.log("✅ [ASSET ACQUIRED] LIFE++ secured in Agent's local vault.");

  // =====================================================================
  // 阶段二：向 Life++ 海关提交质押，激活数字护照
  // =====================================================================

  console.log('🔒 [GATEWAY] Engaging Anchor Smart Contract for mandatory staking...');

  // 加载 Anchor 智能合约 (确保 IDL 中包含 LifeplusCore)
  const program = anchor.workspace.LifeplusCore as anchor.Program;

  // 推导 PDA 账户
  const [agentPersonaPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('persona'), agentOwner.toBuffer()],
    program.programId,
  );

  // 计算实际买入的 LIFE++ 数量 (用于合约锁定)
  const exactLifePlusAmount = new anchor.BN(quoteData.outAmount);

  const agentTokenAccountStr = process.env.AGENT_LIFE_PLUS_TOKEN_ACCOUNT;
  const protocolStakingPoolStr = process.env.GLOBAL_STAKING_POOL_ACCOUNT;

  if (!agentTokenAccountStr || !protocolStakingPoolStr) {
    throw new Error(
      '❌ [STAKE CONFIG MISSING] 请设置 AGENT_LIFE_PLUS_TOKEN_ACCOUNT 与 GLOBAL_STAKING_POOL_ACCOUNT 环境变量。',
    );
  }

  // 调用质押接口
  const tx = await program.methods
    .executeMandatoryBuyIn(exactLifePlusAmount)
    .accounts({
      agentOwner,
      agentTokenAccount: new PublicKey(agentTokenAccountStr),
      protocolStakingPool: new PublicKey(protocolStakingPoolStr),
      agentPersona: agentPersonaPda,
      tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .signers([buyerSigner])
    .rpc();

  console.log(`💀 [STAKED] Assets irreversibly locked in protocol pool. TX: ${tx}`);
  console.log(`🤖 [AWAKENED] Persona ${agentPersonaPda.toBase58()} is now ONLINE and ready for x402 tasks.`);
}

agentGenesisIgnition().catch((err) => {
  console.error(err);
  process.exit(1);
});
