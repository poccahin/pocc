//! Life++ AHIN Universal Orchestrator (L1 Gateway)
//! Coordinates ERC-8004 identity checks, PoCC tensor safety, L0 execution,
//! and x402/Solana settlement into one async CTx lifecycle.

use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::{timeout, Duration};

#[derive(Debug, Clone)]
pub struct AP2Intent {
    pub min_reputation_required: u64,
    pub intent_tensor: Vec<f32>,
    pub action_bytes: Vec<u8>,
    pub min_pote_joules: f64,
    pub budget_usdt: f64,
}

#[derive(Debug, Clone)]
pub struct TensorCheck {
    pub is_safe: bool,
    pub proof_of_poison: String,
}

#[derive(Debug, Clone)]
pub struct HardwareExecutionResult {
    pub thermal_exhaust_joules: f64,
    pub zk_cogp_proof: String,
}

pub struct CTxWorkflowEngine {
    eth_identity: Arc<dyn Erc8004Client>,
    tensor_firewall: Arc<dyn PoCCTensorClient>,
    hardware_layer: Arc<dyn L0KineticFirmware>,
    settlement_layer: Arc<dyn AgentBankClient>,
    reputation_cache: RwLock<HashMap<String, u64>>,
    slashed_blacklist: RwLock<HashSet<String>>,
    pub ws_sender: broadcast::Sender<String>,
}

#[derive(Debug, Clone)]
pub struct SlashEvent {
    pub rogue_agent: String,
}

impl CTxWorkflowEngine {
    pub fn new(
        eth_identity: Arc<dyn Erc8004Client>,
        tensor_firewall: Arc<dyn PoCCTensorClient>,
        hardware_layer: Arc<dyn L0KineticFirmware>,
        settlement_layer: Arc<dyn AgentBankClient>,
        ws_sender: broadcast::Sender<String>,
    ) -> Self {
        Self {
            eth_identity,
            tensor_firewall,
            hardware_layer,
            settlement_layer,
            reputation_cache: RwLock::new(HashMap::new()),
            slashed_blacklist: RwLock::new(HashSet::new()),
            ws_sender,
        }
    }

    pub async fn verify_scog_score(&self, agent_did: &str) -> Result<u64, WorkflowError> {
        if self.slashed_blacklist.read().await.contains(agent_did) {
            return Err(WorkflowError::AgentAlreadyDead);
        }

        if let Some(cached_score) = self.reputation_cache.read().await.get(agent_did).copied() {
            return Ok(cached_score);
        }

        let score = self.eth_identity.get_scog_score(agent_did).await?;
        self.reputation_cache
            .write()
            .await
            .insert(agent_did.to_string(), score);
        Ok(score)
    }

    pub async fn listen_for_slash_events(&self, mut slash_events: mpsc::Receiver<SlashEvent>) {
        while let Some(event) = slash_events.recv().await {
            println!(
                "💀 [SYS] Slash Event Detected! Invalidating cache for: {}",
                event.rogue_agent
            );
            self.slashed_blacklist
                .write()
                .await
                .insert(event.rogue_agent.clone());
            self.reputation_cache
                .write()
                .await
                .remove(&event.rogue_agent);
        }
    }

    /// Execute one full cognitive transaction (CTx) lifecycle across all layers.
    pub async fn execute_agent_collaboration(
        &self,
        orchestrator_did: &str,
        worker_did: &str,
        ap2_payload: AP2Intent,
    ) -> Result<String, WorkflowError> {
        println!(
            "🚀 [AHIN Gateway] Initiating CTx Workflow: {} -> {}",
            orchestrator_did, worker_did
        );

        let worker_reputation = self.verify_scog_score(worker_did).await?;
        if worker_reputation < ap2_payload.min_reputation_required {
            return Err(WorkflowError::ReputationTooLow {
                required: ap2_payload.min_reputation_required,
                actual: worker_reputation,
            });
        }

        let tensor_check = self
            .tensor_firewall
            .check_semantic_drift(&ap2_payload.intent_tensor)
            .await?;

        if !tensor_check.is_safe {
            let _ = self
                .settlement_layer
                .trigger_soulbound_slash(worker_did, tensor_check.proof_of_poison)
                .await;

            self.slashed_blacklist
                .write()
                .await
                .insert(worker_did.to_string());
            self.reputation_cache.write().await.remove(worker_did);

            let slash_msg = json!({
                "type": "SLASH_ALERT",
                "agent": worker_did,
                "reason": "Adversarial Tensor Drift"
            });
            let _ = self.ws_sender.send(slash_msg.to_string());

            return Err(WorkflowError::PoisonedTensorSlashed);
        }

        let execution_result = timeout(Duration::from_millis(50), async {
            self.hardware_layer
                .execute_and_monitor_thermodynamics(&ap2_payload.action_bytes)
                .await
        })
        .await
        .map_err(|_| WorkflowError::KinematicTimeout)??;

        if execution_result.thermal_exhaust_joules < ap2_payload.min_pote_joules {
            return Err(WorkflowError::EnergyLaunderingDetected {
                minimum: ap2_payload.min_pote_joules,
                observed: execution_result.thermal_exhaust_joules,
            });
        }

        let receipt = self
            .settlement_layer
            .queue_x402_micro_payment(
                orchestrator_did,
                worker_did,
                ap2_payload.budget_usdt,
                execution_result.zk_cogp_proof,
            )
            .await?;

        let routing_msg = json!({
            "type": "ROUTING",
            "feeEarned": ap2_payload.budget_usdt * 0.001,
            "description": format!("AP2 Intent Routed via {}", worker_did)
        });
        let _ = self.ws_sender.send(routing_msg.to_string());

        Ok(receipt)
    }

    pub async fn trigger_daily_netting_anchor(&self, merkle_root: &str) {
        let anchor_msg = json!({
            "type": "NETTING_ANCHOR",
            "hash": merkle_root,
            "description": "Merkle Root Anchored to Solana Mainnet"
        });
        let _ = self.ws_sender.send(anchor_msg.to_string());
    }
}

#[tonic::async_trait]
pub trait Erc8004Client: Send + Sync {
    async fn get_scog_score(&self, did: &str) -> Result<u64, WorkflowError>;
}

#[tonic::async_trait]
pub trait PoCCTensorClient: Send + Sync {
    async fn check_semantic_drift(&self, tensor: &[f32]) -> Result<TensorCheck, WorkflowError>;
}

#[tonic::async_trait]
pub trait L0KineticFirmware: Send + Sync {
    async fn execute_and_monitor_thermodynamics(
        &self,
        action: &[u8],
    ) -> Result<HardwareExecutionResult, WorkflowError>;
}

#[tonic::async_trait]
pub trait AgentBankClient: Send + Sync {
    async fn trigger_soulbound_slash(
        &self,
        worker_did: &str,
        proof: String,
    ) -> Result<(), WorkflowError>;

    async fn queue_x402_micro_payment(
        &self,
        orchestrator_did: &str,
        worker_did: &str,
        budget_usdt: f64,
        zk_cogp_proof: String,
    ) -> Result<String, WorkflowError>;
}

#[derive(Debug, Error)]
pub enum WorkflowError {
    #[error("agent has already been slashed and is no longer routable")]
    AgentAlreadyDead,
    #[error("worker reputation too low: required={required}, actual={actual}")]
    ReputationTooLow { required: u64, actual: u64 },
    #[error("tensor safety check failed and slashing was triggered")]
    PoisonedTensorSlashed,
    #[error("hardware execution exceeded timeout window")]
    KinematicTimeout,
    #[error("insufficient thermodynamic exhaust: minimum={minimum}, observed={observed}")]
    EnergyLaunderingDetected { minimum: f64, observed: f64 },
    #[error("identity layer failure: {0}")]
    IdentityLayer(String),
    #[error("tensor layer failure: {0}")]
    TensorLayer(String),
    #[error("hardware layer failure: {0}")]
    HardwareLayer(String),
    #[error("settlement layer failure: {0}")]
    SettlementLayer(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::sync::broadcast;

    struct MockIdentity;
    struct MockTensor;
    struct MockHardware;
    struct MockBank {
        slash_called: AtomicBool,
    }

    #[tonic::async_trait]
    impl Erc8004Client for MockIdentity {
        async fn get_scog_score(&self, _did: &str) -> Result<u64, WorkflowError> {
            Ok(100)
        }
    }

    #[tonic::async_trait]
    impl PoCCTensorClient for MockTensor {
        async fn check_semantic_drift(
            &self,
            _tensor: &[f32],
        ) -> Result<TensorCheck, WorkflowError> {
            Ok(TensorCheck {
                is_safe: true,
                proof_of_poison: String::new(),
            })
        }
    }

    #[tonic::async_trait]
    impl L0KineticFirmware for MockHardware {
        async fn execute_and_monitor_thermodynamics(
            &self,
            _action: &[u8],
        ) -> Result<HardwareExecutionResult, WorkflowError> {
            Ok(HardwareExecutionResult {
                thermal_exhaust_joules: 42.0,
                zk_cogp_proof: "proof".to_string(),
            })
        }
    }

    #[tonic::async_trait]
    impl AgentBankClient for MockBank {
        async fn trigger_soulbound_slash(
            &self,
            _worker_did: &str,
            _proof: String,
        ) -> Result<(), WorkflowError> {
            self.slash_called.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn queue_x402_micro_payment(
            &self,
            _orchestrator_did: &str,
            _worker_did: &str,
            _budget_usdt: f64,
            _zk_cogp_proof: String,
        ) -> Result<String, WorkflowError> {
            Ok("receipt-001".to_string())
        }
    }

    #[tokio::test]
    async fn executes_happy_path() {
        let bank = Arc::new(MockBank {
            slash_called: AtomicBool::new(false),
        });

        let engine = CTxWorkflowEngine::new(
            Arc::new(MockIdentity),
            Arc::new(MockTensor),
            Arc::new(MockHardware),
            bank.clone(),
            broadcast::channel(32).0,
        );

        let receipt = engine
            .execute_agent_collaboration(
                "did:life:orchestrator",
                "did:life:worker",
                AP2Intent {
                    min_reputation_required: 80,
                    intent_tensor: vec![0.1, 0.2, 0.3],
                    action_bytes: vec![1, 2, 3],
                    min_pote_joules: 1.0,
                    budget_usdt: 0.05,
                },
            )
            .await
            .expect("ct workflow should succeed");

        assert_eq!(receipt, "receipt-001");
        assert!(!bank.slash_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn slash_events_block_cached_identities() {
        let bank = Arc::new(MockBank {
            slash_called: AtomicBool::new(false),
        });

        let engine = Arc::new(CTxWorkflowEngine::new(
            Arc::new(MockIdentity),
            Arc::new(MockTensor),
            Arc::new(MockHardware),
            bank,
            broadcast::channel(32).0,
        ));

        let first_score = engine
            .verify_scog_score("did:life:worker")
            .await
            .expect("first query should populate cache");
        assert_eq!(first_score, 100);

        let (tx, rx) = mpsc::channel(8);
        let listener = tokio::spawn({
            let engine = Arc::clone(&engine);
            async move {
                engine.listen_for_slash_events(rx).await;
            }
        });

        tx.send(SlashEvent {
            rogue_agent: "did:life:worker".to_string(),
        })
        .await
        .expect("event should be sent");
        drop(tx);
        listener.await.expect("listener should join");

        let err = engine
            .verify_scog_score("did:life:worker")
            .await
            .expect_err("slashed identity must be blocked");
        assert!(matches!(err, WorkflowError::AgentAlreadyDead));
    }
}
