use pqcrypto_falcon::falcon1024::{detached_sign, keypair};
use pqcrypto_kyber::kyber1024;
use pqcrypto_traits::{kem::Ciphertext as _, sign::DetachedSignature as _, sign::PublicKey as _};

mod x402_channel;
use x402_channel::X402Channel;

#[tokio::main]
async fn main() {
    println!("🌌 [ORCHESTRATOR] x402 State Machine Initialized (PQC).");
    let intent_id = "INTENT_9982_CLEAN_WAREHOUSE";

    // 模拟边缘节点 (Worker) 的行为：生成 Falcon-1024 签名
    let (worker_pk, worker_sk) = keypair();
    let worker_pubkey_hex = hex::encode(worker_pk.as_bytes());

    let ack_payload = format!("ACK_INTENT_{}", intent_id);
    let signature = detached_sign(ack_payload.as_bytes(), &worker_sk);
    let sig_hex = hex::encode(signature.as_bytes());

    println!("🤖 [WORKER] Emitting PQC-signed ACK for Intent...");

    // 模拟 Orchestrator 的行为：验证握手
    let mut channel = X402Channel::new(intent_id);

    match channel.process_pqc_ack(&worker_pubkey_hex, &sig_hex) {
        Ok(_) => {
            println!(
                "🚀 [NETWORK] Quantum-safe thermodynamic channel established. Ready for high-frequency micropayments."
            );
        }
        Err(e) => {
            eprintln!("{}", e);
        }
    }

    // 建立抗量子虫洞隧道 (ML-KEM/Kyber)
    let (kem_pk, kem_sk) = kyber1024::keypair();
    let (shared_secret_a, ciphertext) = kyber1024::encapsulate(&kem_pk);
    let shared_secret_b = kyber1024::decapsulate(&ciphertext, &kem_sk);

    if shared_secret_a == shared_secret_b {
        println!(
            "🌌 [AHIN TUNNEL] Post-Quantum shared secret established via Kyber-1024. Ciphertext bytes: {}",
            ciphertext.as_bytes().len()
        );
    }
}
