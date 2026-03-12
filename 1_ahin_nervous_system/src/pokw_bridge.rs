use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

/// 全球网络物理时钟漂移容忍窗口：15 秒。
const MAX_CLOCK_DRIFT_SECONDS: i64 = 15;

/// 逻辑时钟校验：每个 agent 的 nonce 必须单调递增。
#[derive(Debug, Default)]
pub struct LamportNonceTracker {
    latest_nonce_by_agent: HashMap<String, u64>,
}

impl LamportNonceTracker {
    pub fn verify_and_commit(&mut self, agent_did: &str, nonce: u64) -> Result<(), String> {
        if let Some(last_nonce) = self.latest_nonce_by_agent.get(agent_did) {
            if nonce <= *last_nonce {
                return Err(format!(
                    "💀 [Replay Detected] Nonce {} is not greater than last seen {} for {}.",
                    nonce, last_nonce, agent_did
                ));
            }
        }

        self.latest_nonce_by_agent
            .insert(agent_did.to_string(), nonce);
        Ok(())
    }
}

/// 校验微秒级传感器时间戳，避免将合理时钟偏差误判为攻击。
pub fn verify_kinematic_timestamp(sensor_timestamp_us: i64) -> Result<(), String> {
    let current_server_time_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Failed to read system time: {e}"))?
        .as_secs() as i64;

    let sensor_timestamp_seconds = sensor_timestamp_us / 1_000_000;
    let drift = (current_server_time_seconds - sensor_timestamp_seconds).abs();

    if drift > MAX_CLOCK_DRIFT_SECONDS {
        return Err(format!(
            "💀 [Time Mismatch] Clock drift of {}s exceeds {}s tolerance window. Proof rejected to prevent replay.",
            drift, MAX_CLOCK_DRIFT_SECONDS
        ));
    }

    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KinematicSample {
    pub timestamp_us: i64,
    pub joint_id: u8,
    pub current_ma: i32,
    pub accel_z_mg: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PokwPayload {
    pub intent_hash: [u8; 32],
    pub noise_seed: u64,
    pub samples: Vec<KinematicSample>,
    pub energy_joules_estimate: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PokwProof {
    pub agent_did: String,
    pub intent_hash: String,
    pub noise_seed: u64,
    pub lamport_nonce: u64,
    pub kinematic_signature: String,
    pub energy_expended_joules: u32,
}

pub struct KinematicOrchestrator {
    pub agent_did: String,
    zig_firmware_path: String,
}

impl KinematicOrchestrator {
    pub fn new(agent_did: &str) -> Self {
        Self {
            agent_did: agent_did.to_string(),
            zig_firmware_path: "./0_kinetic_trust_root/zig-out/bin/pokw_firmware".to_string(),
        }
    }

    pub fn with_firmware_path(agent_did: &str, firmware_path: &str) -> Self {
        Self {
            agent_did: agent_did.to_string(),
            zig_firmware_path: firmware_path.to_string(),
        }
    }

    /// 执行物理意图并生成动力学工作证明
    pub fn execute_physical_intent(&self, intent_hash: &str) -> Result<PokwProof, String> {
        println!("⚡ [L1 Bridge] Physical intent received: {intent_hash}");

        let noise_seed: u64 = rand::thread_rng().gen();

        let output = Command::new(&self.zig_firmware_path)
            .arg(intent_hash)
            .arg(noise_seed.to_string())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to ignite L0 firmware: {e}"))?
            .wait_with_output()
            .map_err(|e| format!("L0 firmware execution crashed: {e}"))?;

        if !output.status.success() {
            return Err("❌ [CRITICAL] Kinematic sensors reported failure.".to_string());
        }

        let payload: PokwPayload = serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("Invalid L0 payload: {e}"))?;

        let mut hasher = Sha256::new();
        hasher.update(intent_hash.as_bytes());
        hasher.update(noise_seed.to_be_bytes());
        hasher.update(&output.stdout);
        let kinematic_signature = format!("{:x}", hasher.finalize());

        Ok(PokwProof {
            agent_did: self.agent_did.clone(),
            intent_hash: intent_hash.to_string(),
            noise_seed,
            lamport_nonce: payload.samples.len() as u64,
            kinematic_signature,
            energy_expended_joules: payload.energy_joules_estimate,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_timestamp_within_drift_window() {
        let now_us = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_secs() as i64)
            * 1_000_000;

        assert!(verify_kinematic_timestamp(now_us - 5_000_000).is_ok());
    }

    #[test]
    fn rejects_timestamp_outside_drift_window() {
        let now_us = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_secs() as i64)
            * 1_000_000;

        assert!(verify_kinematic_timestamp(now_us - 30_000_000).is_err());
    }

    #[test]
    fn lamport_nonce_must_increase_per_agent() {
        let mut tracker = LamportNonceTracker::default();

        assert!(tracker.verify_and_commit("did:life:worker-1", 1).is_ok());
        assert!(tracker.verify_and_commit("did:life:worker-1", 2).is_ok());
        assert!(tracker.verify_and_commit("did:life:worker-1", 2).is_err());
    }
}
