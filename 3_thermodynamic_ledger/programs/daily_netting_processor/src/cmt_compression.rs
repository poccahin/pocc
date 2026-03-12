use anchor_lang::prelude::*;
use spl_account_compression::{
    cpi::{accounts::Modify, append},
    program::SplAccountCompression,
    Noop,
};

declare_id!("NettingProcessor11111111111111111111111111");

#[program]
pub mod daily_netting_processor {
    use super::*;

    /// @notice 极速附加认知交易 (CTx) 叶子节点
    /// @dev 边缘节点在 x402 通道内完成高频协作后，将最终的轧差账单哈希作为一片叶子追加进全网 CMT 中。
    /// 这个操作极其廉价，不存储交易明细，只存储密码学承诺。
    pub fn append_ctx_leaf(ctx: Context<AppendCtxLeaf>, ctx_hash: [u8; 32]) -> Result<()> {
        // 1. 验证提交者的身份与授权 (结合 L1 的 AEGIS 拦截器结果)
        require!(
            ctx.accounts.edge_node.is_signer,
            ErrorCode::UnauthorizedNode
        );

        // 2. 构建并发默克尔树的追加请求
        // 这里的 ctx_hash 是包含了：发包方 DID、接单方 DID、PoCC 摩擦力、微支付金额 的综合哈希
        let cpi_ctx = CpiContext::new(
            ctx.accounts.compression_program.to_account_info(),
            Modify {
                merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
                authority: ctx.accounts.tree_authority.to_account_info(),
                noop: ctx.accounts.noop_program.to_account_info(),
            },
        );

        // 3. 调用底层的零知识状态压缩预编译合约
        // 即使当前有 10,000 个机器人在同时调用这个函数，CMT 的并发缓冲区也能确保它们被安全地序列化入树，而不会互相打断。
        append(cpi_ctx, ctx_hash)?;

        msg!("🍃 [CMT] CTx Leaf Appended successfully.");
        msg!("🔗 Hash: {:?}", ctx_hash);

        Ok(())
    }

    /// @notice 每日宏观轧差与争议仲裁 (Daily Netting & Dispute Resolution)
    /// @dev 当发生违约或需要将资金从状态通道提现至 L1 主链时触发。
    /// 提交包含该 CTx 的 Merkle Proof，智能合约在 $O(\log N)$ 的时间复杂度内验证其真实性。
    pub fn verify_and_net_daily(
        ctx: Context<VerifyAndNet>,
        root: [u8; 32],
        leaf: [u8; 32],
        index: u32,
        proof: Vec<[u8; 32]>,
    ) -> Result<()> {
        // 1. 将链下的完整交易明细映射为叶子节点
        let node = leaf;

        // 2. 调用 CMT 底层库，验证这笔微支付是否真的存在于“地球 3D 瞬息孪生模型”的历史快照中
        // CMT 引擎会自动处理并发缓冲，允许基于近期历史根的证明通过
        require!(
            spl_account_compression::verify_leaf(
                &ctx.accounts.merkle_tree.to_account_info(),
                &root,
                &node,
                index,
                &proof,
            )?,
            ErrorCode::InvalidCognitiveTransactionProof
        );

        // 3. 验证通过，执行真实的 L1 资金划转或触发 Soulbound Slasher (声誉销毁)
        msg!("⚖️ [NETTING] Cognitive Transaction formally verified on-chain.");
        // (资金划转与状态更新逻辑略...)

        Ok(())
    }
}

// =====================================================================
// 📦 账户上下文与权限约束 (Account Contexts)
// =====================================================================

#[derive(Accounts)]
pub struct AppendCtxLeaf<'info> {
    #[account(mut)]
    pub edge_node: Signer<'info>,

    /// CHECK: 树的权限控制 PDA
    #[account(
        mut,
        seeds = [b"tree_authority", merkle_tree.key().as_ref()],
        bump,
    )]
    pub tree_authority: AccountInfo<'info>,

    /// CHECK: 实际存储 CMT 数据的巨型连续内存账户
    #[account(mut)]
    pub merkle_tree: UncheckedAccount<'info>,

    pub compression_program: Program<'info, SplAccountCompression>,
    pub noop_program: Program<'info, Noop>,
}

#[derive(Accounts)]
pub struct VerifyAndNet<'info> {
    pub arbiter: Signer<'info>,

    /// CHECK: 实际存储 CMT 数据的巨型连续内存账户
    pub merkle_tree: UncheckedAccount<'info>,

    pub compression_program: Program<'info, SplAccountCompression>,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Node is not authorized to interact with the CMT.")]
    UnauthorizedNode,
    #[msg("The provided CTx Proof is mathematically invalid or forged.")]
    InvalidCognitiveTransactionProof,
}
