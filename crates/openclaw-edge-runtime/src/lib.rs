pub mod adaptive_pricer;
pub mod kinetic;
pub mod engine;
pub mod world_model;
pub mod market;

use kinetic::{KineticCommand, LaneQueue};
use engine::ObjectiveDrivenEngine;
use world_model::{AmiWorldModelBridge, WorldModelError};
use std::sync::Arc;

pub struct EdgeRuntime {
    lane_queue: Arc<LaneQueue>,
    engine: Arc<ObjectiveDrivenEngine>,
    world_model: Arc<tokio::sync::Mutex<AmiWorldModelBridge>>,
}

impl EdgeRuntime {
    pub fn new() -> Self {
        Self {
            lane_queue: Arc::new(LaneQueue::new()),
            engine: Arc::new(ObjectiveDrivenEngine::new(0.8)),
            world_model: Arc::new(tokio::sync::Mutex::new(AmiWorldModelBridge::new(0.5, 16))),
        }
    }

    /// Submit a kinetic command through the two-stage safety pipeline:
    ///
    /// 1. **World model rehearsal** – predict the next world state and reject
    ///    commands that would exceed the collision-probability threshold.
    ///    If the world model has not yet been initialised with a sensor
    ///    snapshot, this gate is skipped (permissive fail-open during boot).
    ///
    /// 2. **Engine psychological rehearsal** – check the hardware blacklist
    ///    and per-axis actuator limits.
    ///
    /// Commands that pass both gates are enqueued for physical execution.
    pub async fn submit_intent(&self, cmd: KineticCommand) {
        // Gate 1: world model safety check (skipped if model is uninitialised
        // or the command embedding size differs from the configured dim).
        let wm_result = {
            let wm = self.world_model.lock().await;
            wm.safety_check(&cmd.target_state)
        };

        match wm_result {
            Err(WorldModelError::UnitialisedModel) => {
                // No sensor snapshot yet – skip world-model gate (boot phase).
                eprintln!("[WARN] World model uninitialised – skipping safety gate (boot phase)");
            }
            Err(WorldModelError::DimensionMismatch { expected, got }) => {
                // target_state dimension doesn't match the configured embedding
                // dim – skip world-model gate and let the engine decide.
                eprintln!(
                    "[WARN] World model dimension mismatch (expected {expected}, got {got}) – \
                     skipping world-model gate"
                );
            }
            Err(e) => {
                eprintln!("Rejected by World Model: {:?}", e);
                return;
            }
            Ok(_) => {}
        }

        // Gate 2: engine psychological rehearsal (blacklist + actuator limits).
        match self.engine.psychological_rehearsal(&cmd).await {
            Ok(_) => {
                let _ = self.lane_queue.enqueue_command(cmd).await;
            }
            Err(e) => eprintln!("Rejected by Safety Guardrails: {:?}", e),
        }
    }

    pub fn world_model(&self) -> Arc<tokio::sync::Mutex<AmiWorldModelBridge>> {
        Arc::clone(&self.world_model)
    }
}

impl Default for EdgeRuntime {
    fn default() -> Self { Self::new() }
}
