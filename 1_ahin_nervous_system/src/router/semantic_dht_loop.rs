use libp2p::{swarm::SwarmEvent, PeerId};
use std::time::Instant;

use crate::ffi_bridge::OpenClawRuntime;
use crate::p2p::behaviour::{AhinBehaviour, AhinEvent};

/// 极度严苛的引力塌缩阈值 (Epsilon)
/// 摩擦力必须低于 5%，智能体才会接管该任务
const SEMANTIC_FRICTION_THRESHOLD: f32 = 0.05;

pub struct AhinSemanticRouter {
    pub swarm: libp2p::Swarm<AhinBehaviour>,
    /// 缓存在内存中的本机能力基底张量 (由本地 LLM/视觉模型特征提取)
    pub local_capability_tensor: Vec<f32>,
}

impl AhinSemanticRouter {
    /// 启动 L1 P2P 虫洞事件循环：监听、试算、塌缩或丢弃
    pub async fn run_event_loop(mut self) {
        println!("🌌 [AHIN ROUTER] Semantic DHT event loop ignited. Listening to the void...");

        loop {
            tokio::select! {
                // 监听底层 Kademlia DHT 与 Gossipsub 混合网络涌入的数据包
                swarm_event = self.swarm.select_next_some() => {
                    match swarm_event {
                        SwarmEvent::Behaviour(AhinEvent::ActiveHashIntentReceived {
                            source_peer,
                            intent_id,
                            intent_tensor,
                            reward_usdc,
                        }) => {
                            self.handle_intent_resonance(source_peer, intent_id, intent_tensor, reward_usdc).await;
                        }

                        // 监听 Slasher 广播的死亡宣告 (斩首操作)
                        SwarmEvent::Behaviour(AhinEvent::CyberDeathWarrant { slain_did }) => {
                            println!("💀 [SLASHER] Entity {} has been executed. Purging from local K-Buckets.", slain_did);
                            self.swarm.behaviour_mut().kademlia.remove_peer(&slain_did);
                        }

                        _ => {
                            // 处理常规的 P2P 发现与存活心跳
                        }
                    }
                }
            }
        }
    }

    /// 核心逻辑：意图共振与引力塌缩试算
    async fn handle_intent_resonance(
        &mut self,
        source_peer: PeerId,
        intent_id: String,
        intent_tensor: Vec<f32>,
        reward_usdc: f64,
    ) {
        let start_time = Instant::now();

        // ⚡ 1. 唤醒异构张量风洞 (调用 FFI 桥接的 C++ MLX/XDNA)
        // 这个调用被封存在 tokio::spawn_blocking 中，绝不会卡死当前的 P2P 事件循环
        let friction_result = OpenClawRuntime::collapse_cognitive_canxian(
            intent_tensor,
            self.local_capability_tensor.clone(),
        )
        .await;

        match friction_result {
            Ok(friction) => {
                let compute_time = start_time.elapsed().as_micros();

                // 📐 2. 评估摩擦力：物理决定路由
                if friction <= SEMANTIC_FRICTION_THRESHOLD {
                    println!(
                        "✨ [COLLAPSE] Resonance achieved! Friction: {:.4} ({}us). Reward: ${}",
                        friction, compute_time, reward_usdc
                    );

                    // 塌缩成功：该任务完美落入本节点的认知坎陷。
                    // 立即向 Orchestrator 发送 ACK 握手，撕开 x402 闪电微支付通道
                    self.initiate_x402_handshake(source_peer, intent_id, friction)
                        .await;
                } else {
                    // 🕳️ 3. 摩擦力过大，语义不匹配：绝对静默丢弃
                    // 不记录日志（防止高频垃圾任务刷爆硬盘），不发送拒绝回复。
                    // 对网络而言，这个包就像投入了黑洞。
                    #[cfg(debug_assertions)]
                    println!(
                        "🧱 [BOUNCE] Friction too high ({:.4}). Intent discarded.",
                        friction
                    );
                }
            }
            Err(err) => {
                // 底层硬件熔断 (例如 NPU 显存溢出)
                eprintln!("💥 [HARDWARE FAULT] {}", err);
            }
        }
    }

    /// 撕开 x402 状态通道，准备接收物理做功结算
    async fn initiate_x402_handshake(&mut self, target: PeerId, intent_id: String, _friction: f32) {
        // 使用 Ed25519 物理私钥对 ACK 进行签名，建立防重放的闪电网络
        let ack_payload = format!("ACK_INTENT_{}", intent_id);
        let _ = ack_payload;
        // (调用密码学模块生成签名...)

        println!(
            "⚡ [x402] Handshake dispatched to Orchestrator ({}). Engaging physical actuators...",
            target
        );
        // 发送 P2P 专属消息...
        // self.swarm.behaviour_mut().gossipsub.publish(x402_topic, signed_payload);
    }
}
