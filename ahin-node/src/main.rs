use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;

mod x402_channel;
use x402_channel::X402Channel;

#[tokio::main]
async fn main() {
    println!("🌌 [ORCHESTRATOR] x402 State Machine Initialized.");
    let intent_id = "INTENT_9982_CLEAN_WAREHOUSE";

    // 模拟边缘节点 (Worker) 的行为：生成签名
    let mut csprng = OsRng;
    let worker_keypair = SigningKey::generate(&mut csprng);
    let worker_pubkey_hex = hex::encode(worker_keypair.verifying_key().as_bytes());

    let ack_payload = format!("ACK_INTENT_{}", intent_id);
    let signature = worker_keypair.sign(ack_payload.as_bytes());
    let sig_hex = hex::encode(signature.to_bytes());

    println!("🤖 [WORKER] Emitting signed ACK for Intent...");

    // 模拟 Orchestrator 的行为：验证握手
    let mut channel = X402Channel::new(intent_id);

    match channel.process_ack(&worker_pubkey_hex, &sig_hex) {
        Ok(_) => {
            println!(
                "🚀 [NETWORK] Thermodynamic channel established. Ready for high-frequency micropayments."
            );
        }
        Err(e) => {
            eprintln!("{}", e);
        }
    }
}
