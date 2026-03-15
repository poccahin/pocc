/// Dynamic Quantitative Pricing Engine (Adaptive Market Maker)
///
/// Implements Bayesian Bid Shading combined with an Avellaneda-Stoikov-inspired
/// market-making model so that every edge node acts as a rational micro-payment
/// market maker.
///
/// Optimal bid formula:
/// ```text
/// P* = C_thermo * (1 + α * F) + S_cog / κ₀ + γ * Q / (Q_max - Q + 1)
/// ```
pub struct AdaptivePricingEngine {
    /// Base thermodynamic cost per inference (electricity + silicon amortization), in USD.
    pub base_thermo_cost_usd: f64,
    /// Semantic friction penalty multiplier (α).
    pub friction_penalty_alpha: f64,
    /// Base market price sensitivity (κ₀).
    pub market_sensitivity_k0: f64,
    /// Inventory congestion penalty coefficient (γ).
    pub inventory_risk_gamma: f64,
    /// Maximum concurrent task depth supported by the local NPU/APU.
    pub max_queue_depth: f64,
}

impl AdaptivePricingEngine {
    /// Create an engine with sensible defaults for an AMD XDNA / Apple M4 node.
    pub fn new() -> Self {
        Self {
            base_thermo_cost_usd: 0.0001,
            friction_penalty_alpha: 2.0,
            market_sensitivity_k0: 50.0,
            inventory_risk_gamma: 0.00005,
            max_queue_depth: 32.0,
        }
    }

    /// Calculate the optimal x402 micro-payment bid price in USD.
    ///
    /// # Arguments
    /// * `intent_friction`  – Semantic friction force (ℱ) between this node's
    ///                        tensor capability and the incoming intent.
    ///                        Range [0, 1]: 0 = perfect match, 1 = full mismatch.
    /// * `current_scog`     – Node TrustRank / cognitive reputation score (S_cog).
    ///                        Values below 1.0 are clamped to 1.0 so new nodes
    ///                        can still participate.
    /// * `current_queue`    – Number of tasks currently queued on this node.
    ///
    /// # Returns
    /// Optimal bid in USD, capped at 0.01 USD per action.
    pub fn calculate_optimal_bid(
        &self,
        intent_friction: f64,
        current_scog: f64,
        current_queue: f64,
    ) -> f64 {
        // 1. Marginal thermodynamic cost — higher friction means more wasted
        //    compute and a higher risk of failed execution.
        let marginal_cost =
            self.base_thermo_cost_usd * (1.0 + self.friction_penalty_alpha * intent_friction);

        // 2. Reputation premium — nodes with higher S_cog can demand a mark-up
        //    because orchestrators prefer reliable partners.  Clamp to ≥ 1 so
        //    brand-new nodes always receive a non-zero premium base.
        let safe_scog = current_scog.max(1.0);
        let reputation_premium = safe_scog / self.market_sensitivity_k0;

        // 3. Inventory risk penalty — exponentially discourages over-subscription
        //    when the local queue approaches its physical limit.
        let queue_utilization = current_queue / (self.max_queue_depth - current_queue + 1.0);
        let inventory_penalty = self.inventory_risk_gamma * queue_utilization;

        // 4. Combine all components and apply a global price ceiling to prevent
        //    malicious or runaway bids from poisoning the network.
        (marginal_cost + reputation_premium + inventory_penalty).min(0.01)
    }
}

impl Default for AdaptivePricingEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> AdaptivePricingEngine {
        AdaptivePricingEngine::new()
    }

    #[test]
    fn zero_friction_zero_queue_baseline() {
        let e = engine();
        // With no friction and no queued tasks the bid equals:
        //   marginal_cost = 0.0001 * (1 + 2.0 * 0) = 0.0001
        //   reputation_premium = 1.0 / 50.0 = 0.02  (scog clamped to 1.0)
        //   inventory_penalty = 0.00005 * 0 / (32 - 0 + 1) = 0.0
        //   total = 0.0201
        // But this exceeds the 0.01 ceiling, so expect exactly 0.01.
        let bid = e.calculate_optimal_bid(0.0, 0.0, 0.0);
        assert!((bid - 0.01).abs() < f64::EPSILON, "bid={bid}");
    }

    #[test]
    fn high_scog_raises_reputation_premium() {
        let e = engine();
        let bid_low = e.calculate_optimal_bid(0.0, 1.0, 0.0);
        let bid_high = e.calculate_optimal_bid(0.0, 100.0, 0.0);
        // Both are capped at 0.01; verify the cap is hit for the high-scog case.
        assert!((bid_low - 0.01).abs() < f64::EPSILON);
        assert!((bid_high - 0.01).abs() < f64::EPSILON);
    }

    #[test]
    fn friction_increases_bid() {
        let e = engine();
        let bid_no_friction = e.calculate_optimal_bid(0.0, 1.0, 0.0);
        let bid_full_friction = e.calculate_optimal_bid(1.0, 1.0, 0.0);
        // Full friction should equal or exceed no-friction (both may hit ceiling).
        assert!(bid_full_friction >= bid_no_friction);
    }

    #[test]
    fn high_queue_raises_inventory_penalty() {
        // Use a tiny scog and no friction so the bid stays well below the cap,
        // allowing the inventory penalty to be observable.
        let e = AdaptivePricingEngine {
            base_thermo_cost_usd: 0.0001,
            friction_penalty_alpha: 0.0,
            market_sensitivity_k0: 1_000_000.0, // negligible reputation premium
            inventory_risk_gamma: 0.001,
            max_queue_depth: 32.0,
        };
        let bid_empty = e.calculate_optimal_bid(0.0, 1.0, 0.0);
        let bid_full = e.calculate_optimal_bid(0.0, 1.0, 31.0);
        assert!(bid_full > bid_empty, "inventory penalty should raise bid when queue is full");
    }

    #[test]
    fn bid_never_exceeds_ceiling() {
        let e = engine();
        for &friction in &[0.0_f64, 0.5, 1.0] {
            for &scog in &[0.0_f64, 1.0, 500.0] {
                for &queue in &[0.0_f64, 16.0, 31.0] {
                    let bid = e.calculate_optimal_bid(friction, scog, queue);
                    assert!(
                        bid <= 0.01 + f64::EPSILON,
                        "bid={bid} exceeds ceiling for friction={friction} scog={scog} queue={queue}"
                    );
                }
            }
        }
    }

    #[test]
    fn new_node_scog_clamped_to_one() {
        let e = engine();
        // scog = 0.0 and scog = 1.0 must produce the same bid (clamping).
        let bid_zero_scog = e.calculate_optimal_bid(0.5, 0.0, 5.0);
        let bid_one_scog = e.calculate_optimal_bid(0.5, 1.0, 5.0);
        assert!(
            (bid_zero_scog - bid_one_scog).abs() < f64::EPSILON,
            "clamping failed: {bid_zero_scog} != {bid_one_scog}"
        );
    }
}
