use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::process::{Command, Stdio};

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
            kinematic_signature,
            energy_expended_joules: payload.energy_joules_estimate,
        })
    }
}
