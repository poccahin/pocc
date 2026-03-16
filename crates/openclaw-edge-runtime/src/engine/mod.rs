use crate::kinetic::{KineticCommand, KineticError};
use std::collections::HashSet;

pub struct ObjectiveDrivenEngine {
    /// Maximum permissible absolute value for any component of `target_state`.
    /// Commands whose state vector contains a component exceeding this value
    /// are rejected as potentially unsafe actuator commands.
    safety_threshold: f64,
    /// DIDs whose hardware is banned from receiving commands (e.g. slashed nodes).
    hardware_blacklist: HashSet<String>,
    /// Expected TEE measurement digest.  When `Some`, every command must carry
    /// an attestation report whose `measurement` field matches this value.
    /// When `None`, TEE measurement pinning is disabled (useful for tests that
    /// do not know the exact measurement at compile time, but TEE presence is
    /// still required).
    expected_measurement: Option<[u8; 32]>,
}

impl ObjectiveDrivenEngine {
    pub fn new(safety_threshold: f64) -> Self {
        Self {
            safety_threshold,
            hardware_blacklist: HashSet::new(),
            expected_measurement: None,
        }
    }

    /// Pin the engine to only accept commands whose TEE attestation report
    /// carries exactly `measurement`.
    ///
    /// Call this during node initialisation after the full software stack has
    /// been loaded and the expected measurement is known (e.g. from the
    /// factory-provisioned genesis manifest).
    pub fn set_expected_measurement(&mut self, measurement: [u8; 32]) {
        self.expected_measurement = Some(measurement);
    }

    /// Rehearse a kinetic command against local safety rules before committing
    /// it to the physical lane queue.
    ///
    /// Safety gates (applied in order):
    /// 0. **TEE attestation** – the command must carry a valid remote
    ///    attestation report produced inside a genuine Life++ enclave.  If an
    ///    expected measurement is pinned, the report's measurement must match.
    /// 1. **Hardware blacklist** - reject commands targeting a slashed DID.
    /// 2. **Actuator limit check** - reject commands where any component of
    ///    `target_state` exceeds `safety_threshold` in absolute value.
    ///
    /// Returns `Ok(())` only when all gates pass.
    pub async fn psychological_rehearsal(&self, cmd: &KineticCommand) -> Result<(), KineticError> {
        // Gate 0: TEE attestation
        match &cmd.tee_attestation {
            None => return Err(KineticError::AttestationMissing),
            Some(report) => {
                report
                    .verify()
                    .map_err(|e| KineticError::AttestationInvalid(e.to_string()))?;

                if let Some(expected) = &self.expected_measurement {
                    report
                        .verify_measurement(expected)
                        .map_err(|e| KineticError::AttestationInvalid(e.to_string()))?;
                }
            }
        }

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
    use tee_foundation::{MeasurementChain, TeeContext, TeeVendor};

    fn make_report() -> tee_foundation::TeeReport {
        let mut ctx = TeeContext::new(TeeVendor::Software);
        ctx.extend_measurement(b"openclaw-edge-runtime:v0.1.0");
        ctx.generate_report(b"test-nonce").expect("report generation should succeed")
    }

    fn cmd(actuator_id: &str, target_state: Vec<f64>) -> KineticCommand {
        KineticCommand {
            actuator_id: actuator_id.to_string(),
            target_state,
            priority: 1,
            tee_attestation: Some(make_report()),
        }
    }

    fn cmd_no_tee(actuator_id: &str, target_state: Vec<f64>) -> KineticCommand {
        KineticCommand {
            actuator_id: actuator_id.to_string(),
            target_state,
            priority: 1,
            tee_attestation: None,
        }
    }

    #[tokio::test]
    async fn safe_command_passes_rehearsal() {
        let engine = ObjectiveDrivenEngine::new(0.8);
        let result = engine.psychological_rehearsal(&cmd("arm-1", vec![0.1, 0.5, -0.3])).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn command_without_tee_attestation_is_rejected() {
        let engine = ObjectiveDrivenEngine::new(0.8);
        let result =
            engine.psychological_rehearsal(&cmd_no_tee("arm-1", vec![0.1])).await;
        assert!(matches!(result, Err(KineticError::AttestationMissing)));
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

    #[tokio::test]
    async fn pinned_measurement_matching_report_passes() {
        let mut ctx = TeeContext::new(TeeVendor::Software);
        ctx.extend_measurement(b"runtime:v1");
        let report = ctx.generate_report(b"nonce").unwrap();
        let measurement = {
            let mut c = MeasurementChain::new();
            c.extend(b"runtime:v1");
            c.digest()
        };

        let mut engine = ObjectiveDrivenEngine::new(0.8);
        engine.set_expected_measurement(measurement);

        let c = KineticCommand {
            actuator_id: "arm".to_string(),
            target_state: vec![0.1],
            priority: 1,
            tee_attestation: Some(report),
        };
        assert!(engine.psychological_rehearsal(&c).await.is_ok());
    }

    #[tokio::test]
    async fn pinned_measurement_mismatching_report_fails() {
        let mut ctx = TeeContext::new(TeeVendor::Software);
        ctx.extend_measurement(b"runtime:v1");
        let report = ctx.generate_report(b"nonce").unwrap();

        // Pin to a different measurement.
        let wrong_measurement = [0xFFu8; 32];
        let mut engine = ObjectiveDrivenEngine::new(0.8);
        engine.set_expected_measurement(wrong_measurement);

        let c = KineticCommand {
            actuator_id: "arm".to_string(),
            target_state: vec![0.1],
            priority: 1,
            tee_attestation: Some(report),
        };
        assert!(matches!(
            engine.psychological_rehearsal(&c).await,
            Err(KineticError::AttestationInvalid(_))
        ));
    }
}
