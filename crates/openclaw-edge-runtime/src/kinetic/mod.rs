use std::collections::VecDeque;
use tokio::sync::Mutex;
use std::sync::Arc;
use thiserror::Error;
use tee_foundation::TeeReport;

#[derive(Error, Debug)]
pub enum KineticError {
    #[error("Safety constraint violation")]
    SafetyViolation,
    #[error("TEE attestation required: command carries no attestation report")]
    AttestationMissing,
    #[error("TEE attestation invalid: {0}")]
    AttestationInvalid(String),
}

/// A kinetic command dispatched to a physical actuator.
///
/// Every command that passes through the three-stage safety pipeline must carry
/// a valid TEE attestation report.  This ensures that the intent originated
/// from a genuine, unmodified Life++ enclave and was not injected by a
/// compromised process outside the trusted execution boundary.
#[derive(Debug, Clone)]
pub struct KineticCommand {
    pub actuator_id: String,
    pub target_state: Vec<f64>,
    pub priority: u8,
    /// Remote attestation report produced by the enclave that generated this
    /// command.  `None` commands are rejected at the TEE gate before they
    /// reach the actuator limit or blacklist checks.
    pub tee_attestation: Option<TeeReport>,
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
