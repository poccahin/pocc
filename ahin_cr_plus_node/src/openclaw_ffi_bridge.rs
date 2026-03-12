use std::os::raw::{c_float, c_int};
use tokio::task;

#[repr(C)]
#[derive(Debug, Clone)]
pub struct SensorFrameC {
    pub timestamp_ns: u64,
    pub torque_nm: c_float,
    pub angular_vel_rads: c_float,
    pub accel_x: c_float,
    pub accel_y: c_float,
    pub accel_z: c_float,
    pub thermal_noise: u32,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct PokwProofC {
    pub agent_pubkey: [u8; 32],
    pub challenge_nonce: u64,
    pub total_joules: c_float,
    pub signature_hash: [u8; 32],
}

#[cfg(feature = "native-ffi")]
#[link(name = "openclaw_kinetic", kind = "static")]
unsafe extern "C" {
    fn pokw_generate_hash_c(
        pubkey: *const u8,
        nonce: u64,
        frames_ptr: *const SensorFrameC,
        frames_len: usize,
        out_proof: *mut PokwProofC,
    ) -> c_int;
}

#[cfg(feature = "native-ffi")]
#[link(name = "openclaw_tensor", kind = "static")]
unsafe extern "C" {
    fn tensor_evaluate_friction_c(
        intent_ptr: *const c_float,
        intent_dim: usize,
        capability_ptr: *const c_float,
        capability_dim: usize,
        out_friction: *mut c_float,
    ) -> c_int;
}

pub struct OpenClawRuntime;

impl OpenClawRuntime {
    pub async fn extract_physical_truth(
        pubkey: [u8; 32],
        nonce: u64,
        sensor_data: Vec<SensorFrameC>,
    ) -> Result<PokwProofC, String> {
        task::spawn_blocking(move || {
            let mut proof_out = PokwProofC {
                agent_pubkey: [0; 32],
                challenge_nonce: 0,
                total_joules: 0.0,
                signature_hash: [0; 32],
            };

            // SAFETY: pointers passed to the C ABI are derived from valid in-scope Rust values
            // that remain alive during this call. The C side must honor provided lengths and
            // write only to `out_proof`.
            let result_code = unsafe {
                pokw_generate_hash(
                    pubkey.as_ptr(),
                    nonce,
                    sensor_data.as_ptr(),
                    sensor_data.len(),
                    &mut proof_out as *mut PokwProofC,
                )
            };

            if result_code == 0 {
                Ok(proof_out)
            } else {
                Err(format!(
                    "💀 [L0 FATAL] Zig hardware driver failed with code: {}",
                    result_code
                ))
            }
        })
        .await
        .unwrap_or_else(|_| {
            Err("💥 [L1 FATAL] Tokio blocking task panicked during PoKW extraction".to_string())
        })
    }

    pub async fn collapse_cognitive_canxian(
        intent_vector: Vec<f32>,
        capability_vector: Vec<f32>,
    ) -> Result<f32, String> {
        if intent_vector.len() != capability_vector.len() {
            return Err(
                "📐 [L2.5 ERROR] Dimensionality mismatch between Intent and Capability tensors."
                    .to_string(),
            );
        }

        task::spawn_blocking(move || {
            let mut friction_out: c_float = 0.0;

            // SAFETY: pointers passed to the C ABI are borrowed from vectors moved into this
            // closure and thus alive for the duration of the call. `out_friction` points to a
            // valid mutable scalar.
            let result_code = unsafe {
                tensor_evaluate_friction(
                    intent_vector.as_ptr(),
                    intent_vector.len(),
                    capability_vector.as_ptr(),
                    capability_vector.len(),
                    &mut friction_out as *mut c_float,
                )
            };

            if result_code == 0 {
                Ok(friction_out as f32)
            } else {
                Err(format!(
                    "🧠 [L2.5 FATAL] C++ Tensor engine (MLX/XDNA) failed with code: {}",
                    result_code
                ))
            }
        })
        .await
        .unwrap_or_else(|_| {
            Err("💥 [L1 FATAL] Tokio blocking task panicked during Tensor evaluation".to_string())
        })
    }
}

#[cfg(feature = "native-ffi")]
unsafe fn pokw_generate_hash(
    pubkey: *const u8,
    nonce: u64,
    frames_ptr: *const SensorFrameC,
    frames_len: usize,
    out_proof: *mut PokwProofC,
) -> c_int {
    // SAFETY: forwarded to FFI boundary with original contract.
    unsafe { pokw_generate_hash_c(pubkey, nonce, frames_ptr, frames_len, out_proof) }
}

#[cfg(not(feature = "native-ffi"))]
unsafe fn pokw_generate_hash(
    pubkey: *const u8,
    nonce: u64,
    _frames_ptr: *const SensorFrameC,
    _frames_len: usize,
    out_proof: *mut PokwProofC,
) -> c_int {
    // SAFETY: output pointer comes from caller and is expected to be non-null and writable.
    unsafe {
        (*out_proof)
            .agent_pubkey
            .copy_from_slice(std::slice::from_raw_parts(pubkey, 32));
        (*out_proof).challenge_nonce = nonce;
        (*out_proof).total_joules = 42.0;
        (*out_proof).signature_hash = [7; 32];
    }
    0
}

#[cfg(feature = "native-ffi")]
unsafe fn tensor_evaluate_friction(
    intent_ptr: *const c_float,
    intent_dim: usize,
    capability_ptr: *const c_float,
    capability_dim: usize,
    out_friction: *mut c_float,
) -> c_int {
    // SAFETY: forwarded to FFI boundary with original contract.
    unsafe {
        tensor_evaluate_friction_c(
            intent_ptr,
            intent_dim,
            capability_ptr,
            capability_dim,
            out_friction,
        )
    }
}

#[cfg(not(feature = "native-ffi"))]
unsafe fn tensor_evaluate_friction(
    intent_ptr: *const c_float,
    intent_dim: usize,
    capability_ptr: *const c_float,
    capability_dim: usize,
    out_friction: *mut c_float,
) -> c_int {
    if intent_dim != capability_dim {
        return -2;
    }

    // SAFETY: caller provides valid pointers with `*_dim` elements.
    unsafe {
        let intent = std::slice::from_raw_parts(intent_ptr, intent_dim);
        let capability = std::slice::from_raw_parts(capability_ptr, capability_dim);
        let value = intent
            .iter()
            .zip(capability.iter())
            .map(|(i, c)| (i - c).abs())
            .sum::<f32>();
        *out_friction = value;
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rejects_tensor_dimension_mismatch() {
        let err = OpenClawRuntime::collapse_cognitive_canxian(vec![1.0], vec![1.0, 2.0])
            .await
            .expect_err("must reject mismatched dimensions");

        assert!(err.contains("Dimensionality mismatch"));
    }

    #[tokio::test]
    async fn mock_tensor_path_returns_friction_value() {
        let out = OpenClawRuntime::collapse_cognitive_canxian(vec![1.0, 4.0], vec![0.0, 2.0])
            .await
            .expect("mock ffi should succeed");

        assert!((out - 3.0).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn mock_pokw_path_returns_marshaled_proof() {
        let pubkey = [9u8; 32];
        let proof = OpenClawRuntime::extract_physical_truth(pubkey, 11, vec![])
            .await
            .expect("mock ffi should succeed");

        assert_eq!(proof.agent_pubkey, [9u8; 32]);
        assert_eq!(proof.challenge_nonce, 11);
    }
}
