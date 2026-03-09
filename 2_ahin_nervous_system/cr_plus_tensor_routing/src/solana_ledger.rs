use std::str::FromStr;

use solana_client::{client_error::ClientError, rpc_client::RpcClient};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_token_legacy::instruction::burn;

pub struct ThermodynamicLedger {
    rpc_client: RpcClient,
    life_plus_mint: Pubkey,
    system_keypair: Keypair,
}

impl ThermodynamicLedger {
    pub fn new(rpc_url: &str, mint_address: &str, secret_key_bytes: &[u8]) -> anyhow::Result<Self> {
        let rpc_client =
            RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());
        let life_plus_mint = Pubkey::from_str(mint_address)?;
        let system_keypair = Keypair::from_bytes(secret_key_bytes)
            .map_err(|e| anyhow::anyhow!("invalid system keypair bytes: {e}"))?;

        println!("🔗 [L3 Ledger] Connected to Solana RPC at: {rpc_url}");
        println!("🪙 [L3 Ledger] Anchored to LIFE++ Mint: {life_plus_mint}");

        Ok(Self {
            rpc_client,
            life_plus_mint,
            system_keypair,
        })
    }

    pub fn get_staked_balance(&self, robot_token_account_str: &str) -> f64 {
        let Ok(account_pubkey) = Pubkey::from_str(robot_token_account_str) else {
            return 0.0;
        };

        match self.rpc_client.get_token_account_balance(&account_pubkey) {
            Ok(balance) => balance.ui_amount.unwrap_or(0.0),
            Err(_) => 0.0,
        }
    }

    pub fn execute_slashing_burn(
        &self,
        malicious_token_account_str: &str,
        amount_to_burn: u64,
    ) -> Result<(), ClientError> {
        let malicious_account = match Pubkey::from_str(malicious_token_account_str) {
            Ok(account) => account,
            Err(_) => {
                println!(
                    "⚠️ [SLASHING ABORTED] Invalid malicious token account: {malicious_token_account_str}"
                );
                return Ok(());
            }
        };

        let burn_ix = burn(
            &spl_token_legacy::id(),
            &malicious_account,
            &self.life_plus_mint,
            &self.system_keypair.pubkey(),
            &[&self.system_keypair.pubkey()],
            amount_to_burn,
        )
        .expect("burn instruction should be constructible with valid accounts");

        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        let tx = Transaction::new_signed_with_payer(
            &[burn_ix],
            Some(&self.system_keypair.pubkey()),
            &[&self.system_keypair],
            recent_blockhash,
        );

        match self.rpc_client.send_and_confirm_transaction(&tx) {
            Ok(sig) => {
                println!("🔥 [SLASHING SUCCESS] {amount_to_burn} LIFE++ burned! Tx: {sig}");
                Ok(())
            }
            Err(e) => {
                eprintln!("⚠️ [SLASHING FAILED] Could not execute burn: {e}");
                Err(e)
            }
        }
    }
}
