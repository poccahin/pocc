use crate::kinetic::{KineticCommand, KineticError};
use std::collections::HashSet;

pub struct ObjectiveDrivenEngine {
    /// Maximum permissible absolute value for any component of `target_state`.
    /// Commands whose state vector contains a component exceeding this value
    /// are rejected as potentially unsafe actuator commands.
    safety_threshold: f64,
    /// DIDs whose hardware is banned from receiving commands (e.g. slashed nodes).
    hardware_blacklist: HashSet<String>,
}

impl ObjectiveDrivenEngine {
    pub fn new(safety_threshold: f64) -> Self {
        Self { safety_threshold, hardware_blacklist: HashSet::new() }
    }

    /// Rehearse a kinetic command against local safety rules before committing
    /// it to the physical lane queue.
    ///
    /// Safety gates (applied in order):
    /// 1. **Hardware blacklist** - reject commands targeting a slashed DID.
    /// 2. **Actuator limit check** - reject commands where any component of
    ///    `target_state` exceeds `safety_threshold` in absolute value.
    ///
    /// Returns `Ok(())` only when both gates pass.
    pub async fn psychological_rehearsal(&self, cmd: &KineticCommand) -> Result<(), KineticError> {
        // Gate 1: hardware blacklist
        if self.hardware_blacklist.contains(&cmd.actuator_id) {
            return Err(KineticError::SafetyViolation);
        }

        // Gate 2: actuator limit — no state component may exceed the threshold
        let exceeds_limit = cmd
            .target_state
            .iter()
            .any(|v| v.abs() > self.safety_threshold);
        if exceeds_limit {
            return Err(KineticError::SafetyViolation);
        }

        Ok(())
    }

    pub fn inject_hardware_blacklist(&mut self, did: &str) {
        self.hardware_blacklist.insert(did.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(actuator_id: &str, target_state: Vec<f64>) -> KineticCommand {
        KineticCommand {
            actuator_id: actuator_id.to_string(),
            target_state,
            priority: 1,
        }
    }

    #[tokio::test]
    async fn safe_command_passes_rehearsal() {
        let engine = ObjectiveDrivenEngine::new(0.8);
        let result = engine.psychological_rehearsal(&cmd("arm-1", vec![0.1, 0.5, -0.3])).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn blacklisted_actuator_is_rejected() {
        let mut engine = ObjectiveDrivenEngine::new(0.8);
        engine.inject_hardware_blacklist("did:node:evil");
        let result = engine.psychological_rehearsal(&cmd("did:node:evil", vec![0.0])).await;
        assert!(matches!(result, Err(KineticError::SafetyViolation)));
    }

    #[tokio::test]
    async fn state_exceeding_threshold_is_rejected() {
        let engine = ObjectiveDrivenEngine::new(0.8);
        // 0.9 > 0.8 safety threshold
        let result = engine.psychological_rehearsal(&cmd("arm-2", vec![0.1, 0.9, -0.3])).await;
        assert!(matches!(result, Err(KineticError::SafetyViolation)));
    }

    #[tokio::test]
    async fn state_at_exact_threshold_passes() {
        let engine = ObjectiveDrivenEngine::new(0.8);
        // 0.8 is not strictly greater than 0.8 → should pass
        let result = engine.psychological_rehearsal(&cmd("arm-3", vec![0.8, -0.8])).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn non_blacklisted_actuator_is_not_rejected_by_gate1() {
        let mut engine = ObjectiveDrivenEngine::new(0.8);
        engine.inject_hardware_blacklist("did:node:other");
        let result = engine.psychological_rehearsal(&cmd("arm-safe", vec![0.1])).await;
        assert!(result.is_ok());
    }
}
