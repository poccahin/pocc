//! AMI World Model Bridge (ami-world-model-bridge)
//!
//! JEPA-like (Joint Embedding Predictive Architecture) bridge for edge agents.
//! Provides short-horizon state prediction to allow the ObjectiveDrivenEngine
//! to rehearse kinetic commands against a probabilistic world model before
//! committing them to the physical LaneQueue.
//!
//! Architecture:
//! ```text
//!   KineticCommand
//!        │
//!        ▼
//!   WorldModelBridge ──► predict_next_state()
//!        │                       │
//!        │                  WorldState (predicted)
//!        │                       │
//!        └──► collision / safety check ──► Ok / Err
//! ```

use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorldModelError {
    #[error("Predicted collision probability {0:.2} exceeds safety threshold")]
    CollisionRisk(f64),
    #[error("World model has not been initialised with an environment snapshot")]
    UnitialisedModel,
    #[error("Embedding dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },
}

/// A compact latent-space snapshot of the physical environment.
/// In production this would be populated from sensor fusion (LiDAR, camera,
/// proprioception). Here it is represented as a dense float vector.
#[derive(Debug, Clone)]
pub struct WorldState {
    /// Latent embedding dimension – must be consistent across calls.
    pub embedding: Vec<f64>,
    /// Approximate collision probability ∈ [0, 1] predicted for the next step.
    pub collision_probability: f64,
    /// Energy cost estimate for the associated action (Joules).
    pub energy_cost_estimate: f64,
}

/// Prediction result produced by [`AmiWorldModelBridge::predict_next_state`].
#[derive(Debug)]
pub struct PredictionResult {
    pub predicted_state: WorldState,
    /// Confidence score ∈ [0, 1]; 1.0 = model is certain.
    pub confidence: f64,
}

/// JEPA-style world model bridge.
///
/// Wraps a lightweight, locally-executed predictive model that maps
/// (current_state, action_embedding) → predicted_next_state.
pub struct AmiWorldModelBridge {
    /// Latest ingested environment snapshot.
    current_state: Option<WorldState>,
    /// Collision probability threshold above which a command is rejected.
    safety_threshold: f64,
    /// Expected embedding dimension.
    embedding_dim: usize,
}

impl AmiWorldModelBridge {
    /// Create a new bridge with the given safety threshold and embedding size.
    ///
    /// # Arguments
    /// * `safety_threshold` – Reject commands whose predicted collision probability
    ///   exceeds this value (0.0 = perfectly safe, 1.0 = always allow).
    /// * `embedding_dim` – Dimensionality of the latent world-state vector.
    pub fn new(safety_threshold: f64, embedding_dim: usize) -> Self {
        Self {
            current_state: None,
            safety_threshold,
            embedding_dim,
        }
    }

    /// Ingest a fresh environment snapshot from sensors / perception stack.
    pub fn update_world_state(&mut self, state: WorldState) -> Result<(), WorldModelError> {
        if state.embedding.len() != self.embedding_dim {
            return Err(WorldModelError::DimensionMismatch {
                expected: self.embedding_dim,
                got: state.embedding.len(),
            });
        }
        self.current_state = Some(state);
        Ok(())
    }

    /// Predict the next world state given an action embedding.
    ///
    /// Implements a simplified JEPA-style forward pass:
    ///   predicted = current + Δ(action)
    /// where Δ is approximated as the element-wise scaled action contribution.
    pub fn predict_next_state(
        &self,
        action_embedding: &[f64],
    ) -> Result<PredictionResult, WorldModelError> {
        let current = self
            .current_state
            .as_ref()
            .ok_or(WorldModelError::UnitialisedModel)?;

        if action_embedding.len() != self.embedding_dim {
            return Err(WorldModelError::DimensionMismatch {
                expected: self.embedding_dim,
                got: action_embedding.len(),
            });
        }

        // Simplified prediction: linear superposition of current state and action.
        let predicted_embedding: Vec<f64> = current
            .embedding
            .iter()
            .zip(action_embedding.iter())
            .map(|(s, a)| (s + a * 0.1).clamp(-1.0, 1.0))
            .collect();

        // Collision probability is estimated as the L2-norm of the action scaled
        // by the current collision probability.  In production this would be a
        // trained neural-network head.
        let action_magnitude: f64 = action_embedding.iter().map(|x| x * x).sum::<f64>().sqrt();
        let predicted_collision =
            (current.collision_probability + action_magnitude * 0.05).clamp(0.0, 1.0);

        let predicted_energy = action_magnitude * 2.5; // rough Joule estimate

        let confidence = 1.0 - predicted_collision * 0.5;

        Ok(PredictionResult {
            predicted_state: WorldState {
                embedding: predicted_embedding,
                collision_probability: predicted_collision,
                energy_cost_estimate: predicted_energy,
            },
            confidence,
        })
    }

    /// Rehearse an action embedding and return `Err` if the predicted world
    /// state is unsafe (collision probability above threshold).
    pub fn safety_check(
        &self,
        action_embedding: &[f64],
    ) -> Result<PredictionResult, WorldModelError> {
        let result = self.predict_next_state(action_embedding)?;
        if result.predicted_state.collision_probability > self.safety_threshold {
            return Err(WorldModelError::CollisionRisk(
                result.predicted_state.collision_probability,
            ));
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state(collision: f64) -> WorldState {
        WorldState {
            embedding: vec![0.1, 0.2, 0.3, 0.0],
            collision_probability: collision,
            energy_cost_estimate: 1.0,
        }
    }

    #[test]
    fn predict_succeeds_with_low_risk_action() {
        let mut bridge = AmiWorldModelBridge::new(0.5, 4);
        bridge.update_world_state(make_state(0.1)).unwrap();
        let action = vec![0.05, 0.05, 0.05, 0.05];
        let result = bridge.safety_check(&action).unwrap();
        assert!(result.predicted_state.collision_probability < 0.5);
    }

    #[test]
    fn safety_check_rejects_high_risk_action() {
        let mut bridge = AmiWorldModelBridge::new(0.3, 4);
        bridge.update_world_state(make_state(0.25)).unwrap();
        // Large action will push collision probability over threshold.
        let action = vec![1.5, 1.5, 1.5, 1.5];
        assert!(matches!(
            bridge.safety_check(&action),
            Err(WorldModelError::CollisionRisk(_))
        ));
    }

    #[test]
    fn uninitialised_model_returns_error() {
        let bridge = AmiWorldModelBridge::new(0.5, 4);
        let action = vec![0.1, 0.1, 0.1, 0.1];
        assert!(matches!(
            bridge.predict_next_state(&action),
            Err(WorldModelError::UnitialisedModel)
        ));
    }

    #[test]
    fn dimension_mismatch_is_detected() {
        let mut bridge = AmiWorldModelBridge::new(0.5, 4);
        bridge.update_world_state(make_state(0.0)).unwrap();
        let bad_action = vec![0.1, 0.1]; // wrong dim
        assert!(matches!(
            bridge.predict_next_state(&bad_action),
            Err(WorldModelError::DimensionMismatch { .. })
        ));
    }
}
