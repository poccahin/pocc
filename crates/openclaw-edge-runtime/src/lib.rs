pub mod adaptive_pricer;
pub mod kinetic;
pub mod engine;
pub mod world_model;

use kinetic::{KineticCommand, LaneQueue};
use engine::ObjectiveDrivenEngine;
use world_model::AmiWorldModelBridge;
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
    pub async fn submit_intent(&self, cmd: KineticCommand) {
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
