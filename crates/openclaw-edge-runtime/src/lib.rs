pub mod kinetic;
pub mod engine;

use kinetic::{KineticCommand, LaneQueue};
use engine::ObjectiveDrivenEngine;
use std::sync::Arc;

pub struct EdgeRuntime {
    lane_queue: Arc<LaneQueue>,
    engine: Arc<ObjectiveDrivenEngine>,
}

impl EdgeRuntime {
    pub fn new() -> Self {
        Self {
            lane_queue: Arc::new(LaneQueue::new()),
            engine: Arc::new(ObjectiveDrivenEngine::new(0.8)),
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
}
