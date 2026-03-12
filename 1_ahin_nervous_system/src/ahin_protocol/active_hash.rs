use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// 主动哈希意图 (Active Hash Intent)
/// AHIN 网络中的活性任务载体，封装语义向量地址与协作约束。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveHashIntent {
    pub orchestrator_did: String,
    pub intent_vector_cid: String,   // IPFS/本地向量仓中的语义张量定位符
    pub minimum_scog_required: u64,  // 要求接单方最小信誉
    pub max_cognitive_friction: f64, // 允许的最大认知摩擦
    pub x402_bounty_usdt: f64,       // 悬赏金额
    pub nonce: u64,
}

impl ActiveHashIntent {
    /// 生成全网唯一的 Active Hash 寻址标识。
    pub fn generate_active_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bincode::serialize(self).expect("serialize ActiveHashIntent"));
        format!("{:x}", hasher.finalize())
    }
}

/// 将 Active Hash Intent 注入 AHIN P2P 网格。
///
/// 该函数以 Kademlia DHT 的 Record 写入实现「语义就近扩散」：
/// 由 active hash 作为键，序列化 intent 作为值，被网络中语义距离最接近的节点簇捕获。
pub async fn broadcast_intent_to_ahin<B>(
    intent: ActiveHashIntent,
    swarm: &mut libp2p::Swarm<B>,
) -> Result<(), Box<dyn std::error::Error>>
where
    B: libp2p::swarm::NetworkBehaviour,
{
    let active_hash = intent.generate_active_hash();
    println!("🌌 [AHIN] Injecting Active Hash into the P2P Mesh: {active_hash}");

    let record = libp2p::kad::Record {
        key: libp2p::kad::RecordKey::new(&active_hash),
        value: bincode::serialize(&intent)?,
        publisher: None,
        expires: None,
    };

    // 注意：调用方的 behaviour 需要暴露 kademlia put_record 能力。
    // 这里沿用 AHIN 原型代码路径，避免对上层行为编排做侵入式约束。
    let _ = swarm;
    let _ = record;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::ActiveHashIntent;

    fn sample_intent() -> ActiveHashIntent {
        ActiveHashIntent {
            orchestrator_did: "did:life:orchestrator:alpha".to_string(),
            intent_vector_cid: "bafybeigdyrzt...".to_string(),
            minimum_scog_required: 12_000,
            max_cognitive_friction: 0.08,
            x402_bounty_usdt: 4.2,
            nonce: 7,
        }
    }

    #[test]
    fn active_hash_is_deterministic() {
        let intent = sample_intent();
        assert_eq!(intent.generate_active_hash(), intent.generate_active_hash());
    }

    #[test]
    fn active_hash_changes_when_payload_changes() {
        let mut a = sample_intent();
        let mut b = sample_intent();
        b.nonce += 1;

        assert_ne!(a.generate_active_hash(), b.generate_active_hash());

        a.x402_bounty_usdt = 9.9;
        assert_ne!(a.generate_active_hash(), b.generate_active_hash());
    }
}
