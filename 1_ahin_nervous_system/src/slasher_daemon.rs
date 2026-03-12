use sha2::{Digest, Sha256};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signature, Signer},
    transaction::Transaction,
};
use std::{str::FromStr, sync::Arc, time::Duration};
use tokio::sync::mpsc::Receiver;

/// 来自 L0/L2.5 的协议违规信号。
#[derive(Debug, Clone)]
pub enum ProtocolViolation {
    TensorSemanticDrift {
        agent_pubkey: Pubkey,
        variance: f64,
    },
    KinematicThermalForgery {
        agent_pubkey: Pubkey,
        expected_joules: u32,
        reported_joules: u32,
    },
}

impl ProtocolViolation {
    pub fn agent_pubkey(&self) -> Pubkey {
        match self {
            Self::TensorSemanticDrift { agent_pubkey, .. }
            | Self::KinematicThermalForgery { agent_pubkey, .. } => *agent_pubkey,
        }
    }

    pub fn reason(&self) -> &'static str {
        match self {
            Self::TensorSemanticDrift { .. } => "Semantic Drift",
            Self::KinematicThermalForgery { .. } => "Thermal/Kinematic Forgery",
        }
    }
}

/// slash 指令所需账户，按 Anchor 程序预期顺序填写。
#[derive(Debug, Clone)]
pub struct SlashAccounts {
    pub executioner: Pubkey,
    pub reputation_account: Pubkey,
    pub stake_vault: Pubkey,
    pub burn_vault: Pubkey,
    pub token_program: Pubkey,
}

impl SlashAccounts {
    pub fn to_account_metas(&self) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new_readonly(self.executioner, true),
            AccountMeta::new(self.reputation_account, false),
            AccountMeta::new(self.stake_vault, false),
            AccountMeta::new(self.burn_vault, false),
            AccountMeta::new_readonly(self.token_program, false),
        ]
    }
}

#[derive(Debug, Clone)]
pub struct SlasherConfig {
    pub rpc_url: String,
    pub ws_url: Option<String>,
    pub program_id: Pubkey,
    pub request_timeout: Duration,
}

impl SlasherConfig {
    pub fn new(rpc_url: impl Into<String>, program_id: impl AsRef<str>) -> Result<Self, String> {
        let program_id = Pubkey::from_str(program_id.as_ref())
            .map_err(|err| format!("invalid program_id: {err}"))?;

        Ok(Self {
            rpc_url: rpc_url.into(),
            ws_url: None,
            program_id,
            request_timeout: Duration::from_secs(8),
        })
    }
}

/// L1 独立斩首守护进程。
pub struct SlasherDaemon {
    rpc_client: Arc<RpcClient>,
    config: SlasherConfig,
    executioner: Arc<Keypair>,
    slash_accounts: SlashAccounts,
}

impl SlasherDaemon {
    pub fn new(
        config: SlasherConfig,
        keypair_path: impl AsRef<std::path::Path>,
        slash_accounts: SlashAccounts,
    ) -> Result<Self, String> {
        println!("💀 [SLASHER] Awakening executioner daemon...");

        let executioner = read_keypair_file(keypair_path)
            .map_err(|err| format!("failed to load executioner keypair: {err}"))?;

        let rpc_client = RpcClient::new_with_timeout_and_commitment(
            config.rpc_url.clone(),
            config.request_timeout,
            CommitmentConfig::confirmed(),
        );

        Ok(Self {
            rpc_client: Arc::new(rpc_client),
            config,
            executioner: Arc::new(executioner),
            slash_accounts,
        })
    }

    /// 主循环：监听上游违规事件并尝试立即执行 slash。
    pub async fn patrol_and_execute(&self, mut rx: Receiver<ProtocolViolation>) {
        println!("👁️  [SLASHER] Patrol mode engaged.");
        if let Some(ws_url) = &self.config.ws_url {
            println!("🔭 [SLASHER] websocket telemetry endpoint: {ws_url}");
        }

        while let Some(violation) = rx.recv().await {
            match &violation {
                ProtocolViolation::TensorSemanticDrift {
                    agent_pubkey,
                    variance,
                } => {
                    println!(
                        "🚨 [VIOLATION] Tensor drift variance={variance:.6}, agent={agent_pubkey}"
                    );
                }
                ProtocolViolation::KinematicThermalForgery {
                    agent_pubkey,
                    expected_joules,
                    reported_joules,
                } => {
                    println!(
                        "🚨 [VIOLATION] Thermal mismatch expected={expected_joules}, reported={reported_joules}, agent={agent_pubkey}"
                    );
                }
            }

            if let Err(err) = self.execute_on_chain_slash(&violation).await {
                eprintln!("❌ [SLASHER] failed to slash agent: {err}");
            }
        }
    }

    async fn execute_on_chain_slash(
        &self,
        violation: &ProtocolViolation,
    ) -> Result<Signature, String> {
        let agent_pubkey = violation.agent_pubkey();
        let reason = violation.reason();
        println!("⚡ [EXECUTION] slashing {agent_pubkey}, reason={reason}");

        let data = build_trigger_soulbound_slash_data(reason);
        let instruction = Instruction {
            program_id: self.config.program_id,
            accounts: self.slash_accounts.to_account_metas(),
            data,
        };

        let latest_blockhash = self
            .rpc_client
            .get_latest_blockhash()
            .map_err(|err| format!("get_latest_blockhash failed: {err}"))?;

        let tx = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.executioner.pubkey()),
            &[self.executioner.as_ref()],
            latest_blockhash,
        );

        let sig = self
            .rpc_client
            .send_and_confirm_transaction(&tx)
            .map_err(|err| format!("send_and_confirm_transaction failed: {err}"))?;

        println!("🔥 [BURNED] slash transaction confirmed: {sig}");
        Ok(sig)
    }
}

/// Anchor 指令 `trigger_soulbound_slash([u8;32])` 编码：
/// 8-byte discriminator + 32-byte evidence hash。
fn build_trigger_soulbound_slash_data(reason: &str) -> Vec<u8> {
    let mut data = Vec::with_capacity(40);
    data.extend_from_slice(&anchor_discriminator("trigger_soulbound_slash"));
    data.extend_from_slice(&reason_digest(reason));
    data
}

fn anchor_discriminator(ix_name: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(format!("global:{ix_name}").as_bytes());
    let digest = hasher.finalize();

    let mut out = [0_u8; 8];
    out.copy_from_slice(&digest[..8]);
    out
}

fn reason_digest(reason: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(reason.as_bytes());
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::{anchor_discriminator, build_trigger_soulbound_slash_data};

    #[test]
    fn builds_expected_anchor_discriminator() {
        let discriminator = anchor_discriminator("trigger_soulbound_slash");
        assert_eq!(discriminator, [130, 135, 131, 181, 188, 37, 8, 70]);
    }

    #[test]
    fn slash_data_is_discriminator_plus_hash() {
        let data = build_trigger_soulbound_slash_data("Semantic Drift");
        assert_eq!(data.len(), 40);
        assert_eq!(&data[..8], &[130, 135, 131, 181, 188, 37, 8, 70]);
    }
}
