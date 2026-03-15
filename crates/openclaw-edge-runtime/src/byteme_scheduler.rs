//! Hybrid Cognitive Engine – `byteme_scheduler`
//!
//! Implements a two-tier intent dispatcher for the `openclaw_edge_daemon`.
//! High-value intents that closely match a registered [`BytemeAvatar`]'s
//! expertise are routed to cloud-backed AI via an external API call; all
//! remaining intents fall back to the local AMD XDNA NPU queue for low-cost
//! processing.
//!
//! # Scheduling philosophy
//!
//! ```text
//! for each incoming intent:
//!   1. Scan all Byteme avatars for semantic friction F (cosine distance in
//!      16-D embedding space).
//!   2. If F < 0.10 AND offered_reward > 3× API base cost AND avatar is idle
//!      → spawn an async cloud task on the avatar.
//!   3. Otherwise fall back to the local NPU queue.
//! ```
//!
//! This ensures the expensive cloud API quota is consumed only for tasks where
//! the expected profit margin is at least 3× the call cost.

use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;

/// Base cost (USD) of a single cloud cognitive API call.
const CLOUD_API_BASE_COST_USD: f64 = 0.05;

/// Maximum cosine friction below which a Byteme avatar is considered a match.
const FRICTION_THRESHOLD: f32 = 0.10;

/// Minimum profit multiplier over the API base cost required to wake an avatar.
const MIN_PROFIT_MULTIPLIER: f64 = 3.0;

// ─────────────────────────────────────────────────────────────────────────────
// Intent
// ─────────────────────────────────────────────────────────────────────────────

/// A network intent submitted for routing by the hybrid cognitive engine.
#[derive(Debug, Clone)]
pub struct Intent {
    /// Unique identifier for this intent.
    pub id: String,
    /// 16-dimensional semantic embedding of the intent's task content.
    pub tensor: [f32; 16],
}

// ─────────────────────────────────────────────────────────────────────────────
// BytemeAvatar
// ─────────────────────────────────────────────────────────────────────────────

/// An expert AI worker backed by a cloud cognitive API.
///
/// Each avatar specialises in a specific domain (e.g. financial analysis,
/// medical consultation, creative design) represented by a 16-dimensional
/// expertise tensor.  Avatars are spawned asynchronously and tracked via an
/// `is_busy` mutex so the scheduler can skip occupied workers.
pub struct BytemeAvatar {
    /// Unique avatar identifier (e.g. `"BM-FIN-001"`).
    pub avatar_id: String,
    /// 16-D semantic embedding describing this avatar's area of expertise.
    pub expertise_tensor: [f32; 16],
    is_busy: Arc<Mutex<bool>>,
}

impl BytemeAvatar {
    /// Create a new [`BytemeAvatar`] with the given `id` and `expertise` tensor.
    pub fn new(id: &str, expertise: [f32; 16]) -> Self {
        Self {
            avatar_id: id.to_string(),
            expertise_tensor: expertise,
            is_busy: Arc::new(Mutex::new(false)),
        }
    }

    /// Execute a cloud cognitive task for the given intent using `api_key`.
    ///
    /// Resets `is_busy` to `false` on completion (the caller is responsible
    /// for setting it to `true` before spawning this task).
    /// Returns the raw response payload on success.
    async fn execute_cloud_cognitive_task(
        &self,
        intent: &Intent,
        api_key: &str,
    ) -> Result<Vec<u8>, &'static str> {
        println!(
            "🤖 [BYTEME {}] Waking up for intent {}. API key: {}…",
            self.avatar_id,
            intent.id,
            &api_key[..api_key.len().min(4)],
        );

        // Simulate asynchronous cloud API round-trip.
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        *self.is_busy.lock().await = false;

        Ok(vec![0u8; 256])
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// HybridCognitiveEngine
// ─────────────────────────────────────────────────────────────────────────────

/// A two-tier cognitive dispatcher that routes intents between cloud-backed
/// [`BytemeAvatar`]s and a local NPU queue.
///
/// # Construction
///
/// ```
/// use tokio::sync::mpsc;
/// use openclaw_edge_runtime::byteme_scheduler::{HybridCognitiveEngine, BytemeAvatar};
///
/// let (tx, _rx) = mpsc::channel(64);
/// let engine = HybridCognitiveEngine::new(
///     tx,
///     vec![
///         BytemeAvatar::new("BM-FIN-001", [0.9, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
///                                          0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
///     ],
///     "my-api-key".to_string(),
/// );
/// ```
pub struct HybridCognitiveEngine {
    /// Sender side of the local AMD XDNA NPU task queue (fallback path).
    local_npu_queue: mpsc::Sender<Intent>,
    /// Registered Byteme expert avatars.
    expert_avatars: Vec<Arc<BytemeAvatar>>,
    /// Cloud API key used to authenticate avatar calls.
    cloud_api_key: String,
}

impl HybridCognitiveEngine {
    /// Create a new engine with the given NPU queue sender, avatars, and API key.
    pub fn new(
        npu_tx: mpsc::Sender<Intent>,
        avatars: Vec<BytemeAvatar>,
        cloud_api_key: String,
    ) -> Self {
        Self {
            local_npu_queue: npu_tx,
            expert_avatars: avatars.into_iter().map(Arc::new).collect(),
            cloud_api_key,
        }
    }

    /// Route `intent` to the best available expert avatar or fall back to the
    /// local NPU queue.
    ///
    /// An avatar is chosen when **all three** of the following hold:
    /// 1. Semantic friction between the intent and the avatar is below
    ///    [`FRICTION_THRESHOLD`].
    /// 2. `offered_reward_usd` exceeds `CLOUD_API_BASE_COST_USD ×
    ///    MIN_PROFIT_MULTIPLIER`.
    /// 3. The avatar is currently idle (non-blocking `try_lock` succeeds and
    ///    `is_busy == false`).
    ///
    /// If no avatar is selected the intent is sent to the local NPU queue.
    pub async fn dispatch_intent(&self, intent: Intent, offered_reward_usd: f64) {
        let min_reward = CLOUD_API_BASE_COST_USD * MIN_PROFIT_MULTIPLIER;

        for avatar in &self.expert_avatars {
            let friction =
                calculate_semantic_friction(&intent.tensor, &avatar.expertise_tensor);

            if friction < FRICTION_THRESHOLD && offered_reward_usd > min_reward {
                // Non-blocking trylock: skip if avatar is already busy.
                if let Ok(mut busy_flag) = avatar.is_busy.try_lock() {
                    if !*busy_flag {
                        // Mark busy optimistically before spawning.
                        *busy_flag = true;
                        drop(busy_flag);

                        let avatar_clone = Arc::clone(avatar);
                        let api_key_clone = self.cloud_api_key.clone();
                        let intent_clone = intent.clone();

                        tokio::spawn(async move {
                            match avatar_clone
                                .execute_cloud_cognitive_task(&intent_clone, &api_key_clone)
                                .await
                            {
                                Ok(_) => println!(
                                    "✅ [PROFIT] Byteme {} completed intent {}. Earned {:.3} USD.",
                                    avatar_clone.avatar_id,
                                    intent_clone.id,
                                    offered_reward_usd
                                ),
                                Err(e) => eprintln!(
                                    "❌ Byteme {} execution failed: {}",
                                    avatar_clone.avatar_id, e
                                ),
                            }
                        });

                        return; // Successfully dispatched.
                    }
                }
            }
        }

        // Fallback: route to local NPU queue.
        println!(
            "⬇️ [FALLBACK] Routing intent {} to local NPU queue.",
            intent.id
        );
        let _ = self.local_npu_queue.send(intent).await;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Semantic friction
// ─────────────────────────────────────────────────────────────────────────────

/// Compute the cosine distance (friction) between two 16-D unit vectors.
///
/// Returns a value in `[0, 1]` where 0 means identical direction and 1 means
/// orthogonal (completely unrelated).  NaN-safe: returns `1.0` when either
/// vector has zero magnitude.
pub fn calculate_semantic_friction(intent: &[f32; 16], capability: &[f32; 16]) -> f32 {
    let dot: f32 = intent.iter().zip(capability.iter()).map(|(a, b)| a * b).sum();
    let mag_i: f32 = intent.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_c: f32 = capability.iter().map(|x| x * x).sum::<f32>().sqrt();

    if mag_i < f32::EPSILON || mag_c < f32::EPSILON {
        return 1.0;
    }

    let cosine_similarity = dot / (mag_i * mag_c);
    // Clamp to [-1, 1] to guard against floating-point drift.
    1.0 - cosine_similarity.clamp(-1.0, 1.0)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    fn zero_tensor() -> [f32; 16] {
        [0.0f32; 16]
    }

    fn unit_tensor(idx: usize) -> [f32; 16] {
        let mut t = [0.0f32; 16];
        t[idx] = 1.0;
        t
    }

    fn intent_with_tensor(id: &str, tensor: [f32; 16]) -> Intent {
        Intent { id: id.to_string(), tensor }
    }

    // ── calculate_semantic_friction ───────────────────────────────────────────

    #[test]
    fn identical_tensors_have_zero_friction() {
        let t = unit_tensor(0);
        let f = calculate_semantic_friction(&t, &t);
        assert!(f.abs() < 1e-6, "identical vectors should have friction ≈ 0, got {f}");
    }

    #[test]
    fn orthogonal_tensors_have_unit_friction() {
        let a = unit_tensor(0);
        let b = unit_tensor(1);
        let f = calculate_semantic_friction(&a, &b);
        assert!((f - 1.0).abs() < 1e-6, "orthogonal vectors should have friction ≈ 1, got {f}");
    }

    #[test]
    fn zero_magnitude_tensor_returns_one() {
        let z = zero_tensor();
        let t = unit_tensor(3);
        let f = calculate_semantic_friction(&z, &t);
        assert!((f - 1.0).abs() < 1e-6, "zero-magnitude vector should return friction = 1, got {f}");
    }

    // ── dispatch_intent (cloud path) ──────────────────────────────────────────

    #[tokio::test]
    async fn high_value_matched_intent_routes_to_cloud() {
        let (tx, mut rx) = mpsc::channel::<Intent>(8);

        let expertise = unit_tensor(0); // avatar specialises in dim-0
        let avatars = vec![BytemeAvatar::new("BM-TEST-001", expertise)];
        let engine = HybridCognitiveEngine::new(tx, avatars, "test-key".to_string());

        // Intent closely aligned with the avatar (friction ≈ 0) and generous reward.
        let intent = intent_with_tensor("cloud-intent", unit_tensor(0));
        engine.dispatch_intent(intent, 1.0).await;

        // Give the spawned task a moment to complete.
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Nothing should have been sent to the local NPU queue.
        assert!(rx.try_recv().is_err(), "cloud-routed intent should NOT reach the NPU queue");
    }

    // ── dispatch_intent (NPU fallback) ────────────────────────────────────────

    #[tokio::test]
    async fn low_reward_intent_falls_back_to_npu() {
        let (tx, mut rx) = mpsc::channel::<Intent>(8);

        let expertise = unit_tensor(0);
        let avatars = vec![BytemeAvatar::new("BM-TEST-002", expertise)];
        let engine = HybridCognitiveEngine::new(tx, avatars, "test-key".to_string());

        // Reward is below the 3× minimum threshold (3 × 0.05 = 0.15).
        let intent = intent_with_tensor("cheap-intent", unit_tensor(0));
        engine.dispatch_intent(intent, 0.10).await;

        let received = rx.try_recv();
        assert!(received.is_ok(), "low-reward intent should fall back to NPU queue");
        assert_eq!(received.unwrap().id, "cheap-intent");
    }

    #[tokio::test]
    async fn unmatched_intent_falls_back_to_npu() {
        let (tx, mut rx) = mpsc::channel::<Intent>(8);

        let expertise = unit_tensor(0);   // avatar specialises in dim-0
        let avatars = vec![BytemeAvatar::new("BM-TEST-003", expertise)];
        let engine = HybridCognitiveEngine::new(tx, avatars, "test-key".to_string());

        // Intent is orthogonal to the avatar (high friction).
        let intent = intent_with_tensor("orthogonal-intent", unit_tensor(1));
        engine.dispatch_intent(intent, 10.0).await;

        let received = rx.try_recv();
        assert!(received.is_ok(), "unmatched intent should fall back to NPU queue");
        assert_eq!(received.unwrap().id, "orthogonal-intent");
    }

    // ── BytemeAvatar ──────────────────────────────────────────────────────────

    #[test]
    fn avatar_creation_stores_id_and_tensor() {
        let t = unit_tensor(5);
        let avatar = BytemeAvatar::new("BM-ART-003", t);
        assert_eq!(avatar.avatar_id, "BM-ART-003");
        assert_eq!(avatar.expertise_tensor, t);
    }
}
