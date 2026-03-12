use std::collections::VecDeque;
use tokio::sync::Mutex;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KineticError {
    #[error("Safety constraint violation")]
    SafetyViolation,
}

#[derive(Debug, Clone)]
pub struct KineticCommand {
    pub actuator_id: String,
    pub target_state: Vec<f64>,
    pub priority: u8,
}

pub struct LaneQueue {
    lanes: Arc<Mutex<VecDeque<KineticCommand>>>,
}

impl Default for LaneQueue {
    fn default() -> Self { Self::new() }
}

impl LaneQueue {
    pub fn new() -> Self {
        Self { lanes: Arc::new(Mutex::new(VecDeque::new())) }
    }
    pub async fn enqueue_command(&self, cmd: KineticCommand) -> Result<(), KineticError> {
        self.lanes.lock().await.push_back(cmd);
        Ok(())
    }
    pub async fn dequeue_command(&self) -> Option<KineticCommand> {
        self.lanes.lock().await.pop_front()
    }
}
