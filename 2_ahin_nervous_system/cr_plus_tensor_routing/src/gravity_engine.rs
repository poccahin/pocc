//! AHIN L1 - ChainRank+ (CR+) Gravity Tensor Routing Engine
//! Dismantles compute monopolies via Quadratic Funding math and Topological Entropy penalties.

use std::f64::consts::E;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct AgentNodeContext {
    pub node_id: [u8; 32],
    pub entropy_reduction_joules: f64, // ΔS_j(t): Historical thermodynamic work verified by PoTE
    pub life_plus_staked: f64,         // Stake_j: LIFE++ locked in Solana HTLC
    pub topological_entropy: f64, // H_topo(j): High if the node acts as a centralized routing hub
}

pub struct GravityTensor {
    alpha: f64, // Weight of physical reality (Thermodynamic work)
    beta: f64,  // Weight of economic skin-in-the-game (Quadratic stake)
    gamma: f64, // Monopoly penalty exponent
}

#[derive(Error, Debug)]
pub enum RoutingError {
    #[error("Zero distance division anomaly")]
    SingularityAnomaly,
}

impl GravityTensor {
    pub fn new(alpha: f64, beta: f64, gamma: f64) -> Self {
        Self { alpha, beta, gamma }
    }

    /// Computes the gravitational pull from a task-emitter to a target worker node.
    /// Formula: Gravity = (α * ΔS + β * √(Stake)) / (D^2 * exp(γ * H_topo))
    pub fn compute_gravity_pull(
        &self,
        target: &AgentNodeContext,
        semantic_physical_distance: f64,
    ) -> Result<f64, RoutingError> {
        if semantic_physical_distance <= f64::EPSILON {
            return Err(RoutingError::SingularityAnomaly);
        }

        // 1. Anti-Matthew Effect: Quadratic evaluation of capital (Stake)
        let economic_mass = self.beta * target.life_plus_staked.sqrt();

        // 2. Thermodynamic Mass: Real physical entropy reduction
        let physical_mass = self.alpha * target.entropy_reduction_joules;
        let total_mass = physical_mass + economic_mass;

        // 3. Spacetime Distance Squared
        let distance_sq = semantic_physical_distance.powi(2);

        // 4. Monopoly Penalty (Topological Entropy Decay)
        // If a node hoards too many connections, its gravity decays exponentially.
        let monopoly_decay = E.powf(self.gamma * target.topological_entropy);

        let gravity = total_mass / (distance_sq * monopoly_decay);

        Ok(gravity)
    }
}
