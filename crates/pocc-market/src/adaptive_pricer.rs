//! Adaptive Pricing Engine for POCC edge nodes.
//!
//! Implements an Avellaneda-Stoikov-inspired market-making model that lets
//! each edge daemon calculate an optimal micro-payment bid (in USD) for
//! every incoming cognitive intent.
//!
//! # Model
//!
//! ```text
//! bid_usd = friction_adjusted_cost × (1 + reputation_premium) + inventory_penalty
//!
//! where:
//!   friction_adjusted_cost = base_thermo_cost_usd / (1 − friction)
//!   reputation_premium     = reputation_coefficient × scog
//!   inventory_penalty      = γ × Q_current / (Q_max − Q_current + 1)
//! ```
//!
//! * **`friction`** – Semantic friction of the intent [0, 1).  Tasks with
//!   `friction ≥ 0.20` are typically rejected before calling this engine.
//! * **`scog`** – Node cognitive score / TrustRank [0, 1].  Higher score
//!   commands a reputation premium.
//! * **`queue_depth`** – Current number of pending tasks in this node's
//!   queue.  Acts as the inventory variable from the A-S model: a full
//!   queue raises the bid (inventory risk), an empty queue lowers it
//!   (bid-shading to attract flow).

/// Adaptive pricing engine for a single edge node.
///
/// Create once with [`AdaptivePricingEngine::new`] and call
/// [`AdaptivePricingEngine::calculate_optimal_bid`] for every intent the
/// daemon considers bidding on.
#[derive(Debug, Clone)]
pub struct AdaptivePricingEngine {
    /// Hard floor cost per task in USD (electricity + silicon depreciation).
    base_thermo_cost_usd: f64,
    /// Avellaneda-Stoikov inventory risk coefficient (γ).
    ///
    /// Typical range: 0.05 (low sensitivity) – 0.5 (high sensitivity).
    gamma: f64,
    /// Rated capacity of this node's task queue.
    queue_max: f64,
    /// Scales the reputation premium for high-S_cog nodes.
    ///
    /// A value of `0.5` means a node with `scog = 1.0` (maximum TrustRank) can
    /// charge up to 50 % above the thermodynamic floor.  Must be in `[0, 1]`.
    reputation_coefficient: f64,
}

impl Default for AdaptivePricingEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AdaptivePricingEngine {
    /// Create a new engine with production-grade default parameters.
    ///
    /// | Parameter              | Default  | Rationale                             |
    /// |------------------------|----------|---------------------------------------|
    /// | `base_thermo_cost_usd` | 0.001    | ~$0.001 per task at median load       |
    /// | `gamma`                | 0.1      | Moderate inventory-risk sensitivity   |
    /// | `queue_max`            | 100.0    | Typical edge-node task capacity       |
    /// | `reputation_coefficient`| 0.5    | Up to 50 % premium for top-rank nodes |
    pub fn new() -> Self {
        Self {
            base_thermo_cost_usd: 0.001,
            reputation_coefficient: 0.5,
            gamma: 0.1,
            queue_max: 100.0,
        }
    }

    /// Create an engine with explicit parameters for testing or custom deployments.
    pub fn with_params(
        base_thermo_cost_usd: f64,
        gamma: f64,
        queue_max: f64,
        reputation_coefficient: f64,
    ) -> Self {
        assert!(base_thermo_cost_usd > 0.0, "base cost must be positive");
        assert!(gamma >= 0.0, "gamma must be non-negative");
        assert!(queue_max > 0.0, "queue_max must be positive");
        assert!(
            (0.0..=1.0).contains(&reputation_coefficient),
            "reputation_coefficient must be in [0, 1]"
        );
        Self {
            base_thermo_cost_usd,
            gamma,
            queue_max,
            reputation_coefficient,
        }
    }

    /// Calculate the optimal bid for an incoming intent, denominated in USD.
    ///
    /// # Arguments
    ///
    /// * `friction`     – Semantic friction [0, 1).  Must be < 1; values ≥ 0.20
    ///                    are normally filtered by the daemon before calling here.
    /// * `scog`         – Node cognitive score / TrustRank [0, 1].
    /// * `queue_depth`  – Current number of tasks queued on this node (≥ 0).
    ///
    /// # Returns
    ///
    /// Optimal bid in USD (always ≥ `base_thermo_cost_usd`).
    pub fn calculate_optimal_bid(&self, friction: f64, scog: f64, queue_depth: f64) -> f64 {
        // Clamp inputs to safe ranges.
        let friction = friction.clamp(0.0, 0.999);
        let scog = scog.clamp(0.0, 1.0);
        let queue_depth = queue_depth.clamp(0.0, self.queue_max);

        // 1. Friction-adjusted thermodynamic floor cost.
        //    As friction → 1, computation cost diverges, reflecting a task that
        //    is almost incompatible with this node's capabilities.
        let friction_adjusted_cost = self.base_thermo_cost_usd / (1.0 - friction);

        // 2. Reputation premium: high-S_cog nodes charge more for their quality.
        let reputation_premium = self.reputation_coefficient * scog;

        // 3. Avellaneda-Stoikov inventory penalty.
        //    γ × Q / (Q_max − Q + 1)
        //    Rises steeply as queue fills, falls to 0 when queue is empty.
        let inventory_penalty =
            self.gamma * queue_depth / (self.queue_max - queue_depth + 1.0);

        friction_adjusted_cost * (1.0 + reputation_premium) + inventory_penalty
    }

    /// Convenience: convert a USD bid to token units given an oracle spot price.
    ///
    /// Returns `None` if `token_price_usd` is non-positive to guard against a
    /// zero-price oracle fault (division by zero in production would cause an
    /// infinite token bid, draining the buyer).
    pub fn usd_to_tokens(&self, bid_usd: f64, token_price_usd: f64) -> Option<f64> {
        if token_price_usd <= 0.0 {
            return None;
        }
        Some(bid_usd / token_price_usd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> AdaptivePricingEngine {
        AdaptivePricingEngine::with_params(0.001, 0.1, 100.0, 0.5)
    }

    // ── Basic sanity ─────────────────────────────────────────────────────────

    #[test]
    fn bid_is_at_least_base_cost_with_zero_friction_and_no_queue() {
        let e = engine();
        let bid = e.calculate_optimal_bid(0.0, 0.0, 0.0);
        // friction=0, scog=0, queue=0 → bid = base_cost / 1 * (1 + 0) + 0
        assert!((bid - 0.001).abs() < 1e-9, "expected 0.001, got {bid}");
    }

    #[test]
    fn reputation_premium_scales_with_scog() {
        let e = engine();
        let low = e.calculate_optimal_bid(0.0, 0.0, 0.0);
        let high = e.calculate_optimal_bid(0.0, 1.0, 0.0);
        // high-scog node should bid more (reputation premium = 0.5 × 1.0 = 0.5)
        assert!(high > low, "high-scog bid should exceed low-scog bid");
        // bid = 0.001 * (1 + 0.5) = 0.0015
        assert!((high - 0.0015).abs() < 1e-9, "expected 0.0015, got {high}");
    }

    #[test]
    fn friction_raises_bid() {
        let e = engine();
        let low_friction = e.calculate_optimal_bid(0.05, 0.5, 0.0);
        let high_friction = e.calculate_optimal_bid(0.19, 0.5, 0.0);
        assert!(high_friction > low_friction, "higher friction should raise bid");
    }

    #[test]
    fn full_queue_raises_bid_via_inventory_penalty() {
        let e = engine();
        let empty = e.calculate_optimal_bid(0.1, 0.5, 0.0);
        let full = e.calculate_optimal_bid(0.1, 0.5, 100.0);
        assert!(full > empty, "full queue should raise bid (inventory risk)");
    }

    // ── Flash-crash invariants ────────────────────────────────────────────────

    /// T+000ms: Token price drops 90%; USD bid is unchanged; token bid × 10.
    #[test]
    fn token_bid_scales_inversely_with_token_price() {
        let e = engine();
        let bid_usd = e.calculate_optimal_bid(0.1, 0.5, 10.0);

        let normal_price = 0.50_f64;
        let crash_price = 0.05_f64; // 90 % drop

        let tokens_normal = e.usd_to_tokens(bid_usd, normal_price).unwrap();
        let tokens_crash = e.usd_to_tokens(bid_usd, crash_price).unwrap();

        let ratio = tokens_crash / tokens_normal;
        assert!(
            (ratio - 10.0).abs() < 1e-6,
            "token bid should be 10× after 90 % price drop, got ratio {ratio}"
        );
    }

    /// T+150ms: Queue drains → inventory penalty falls → bid decreases (bid-shading).
    #[test]
    fn bid_decreases_as_queue_drains() {
        let e = engine();
        let friction = 0.1;
        let scog = 0.5;

        let bid_full = e.calculate_optimal_bid(friction, scog, 80.0);
        let bid_half = e.calculate_optimal_bid(friction, scog, 40.0);
        let bid_empty = e.calculate_optimal_bid(friction, scog, 0.0);

        assert!(bid_full > bid_half, "full queue bid should exceed half-full");
        assert!(bid_half > bid_empty, "half-full bid should exceed empty");
    }

    /// T+500ms: Elite nodes (high S_cog) maintain higher bids than rookies.
    #[test]
    fn elite_nodes_bid_higher_than_rookies() {
        let e = engine();
        let friction = 0.05;
        let sparse_queue = 2.0;

        let rookie = e.calculate_optimal_bid(friction, 0.1, sparse_queue);
        let elite = e.calculate_optimal_bid(friction, 0.9, sparse_queue);

        assert!(elite > rookie, "elite nodes must out-bid rookies");
    }

    // ── Oracle guard ─────────────────────────────────────────────────────────

    #[test]
    fn zero_token_price_returns_none() {
        let e = engine();
        assert!(e.usd_to_tokens(0.005, 0.0).is_none());
    }

    #[test]
    fn negative_token_price_returns_none() {
        let e = engine();
        assert!(e.usd_to_tokens(0.005, -1.0).is_none());
    }

    #[test]
    fn valid_token_conversion_is_correct() {
        let e = engine();
        let tokens = e.usd_to_tokens(0.10, 0.50).unwrap();
        assert!((tokens - 0.20).abs() < 1e-9, "expected 0.20 tokens, got {tokens}");
    }
}
