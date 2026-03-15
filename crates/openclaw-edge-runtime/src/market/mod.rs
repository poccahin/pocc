//! Edge Daemon with adaptive market-making integration.
//!
//! [`EdgeDaemon`] wraps the core runtime with an auction loop that uses
//! [`AdaptivePricingEngine`] to compute per-intent micro-payment bids.
//! Every intent received from the network is evaluated against the node's
//! current thermodynamic state before a signed bid is submitted.
//!
//! # Auction loop summary
//!
//! ```text
//! for each incoming intent:
//!   1. Measure semantic friction + queue depth + local trust rank (S_cog).
//!   2. Reject intents with friction ≥ 0.20 (incompatible with this node).
//!   3. Calculate optimal USD bid via AdaptivePricingEngine.
//!   4. Convert USD → LIFE++ tokens using the oracle spot price.
//!   5. Submit a sealed bid to the network.
//! ```

use std::sync::Arc;
use tokio::sync::Mutex;
use pocc_market::AdaptivePricingEngine;

// ─────────────────────────────────────────────────────────────────────────────
// Domain types
// ─────────────────────────────────────────────────────────────────────────────

/// A network intent that a buyer has broadcast for auction.
#[derive(Debug, Clone)]
pub struct NetworkIntent {
    pub id: String,
    pub payload: String,
}

/// A sealed bid submitted by an edge daemon.
#[derive(Debug, Clone)]
pub struct SealedBid {
    pub intent_id: String,
    pub node_did: String,
    pub bid_tokens: f64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Hardware host abstraction (injected for testability)
// ─────────────────────────────────────────────────────────────────────────────

/// Abstraction over hardware telemetry.  Implementations may read real CPU
/// sensors, mock values for tests, or stream data from a K8s metrics endpoint.
pub trait HardwareHost: Send + Sync {
    /// Compute the semantic friction for the given intent [0, 1].
    fn compute_semantic_friction(&self, intent: &NetworkIntent) -> f64;
    /// Return the current task queue depth (number of queued items).
    fn get_current_queue_depth(&self) -> f64;
}

// ─────────────────────────────────────────────────────────────────────────────
// Oracle client abstraction
// ─────────────────────────────────────────────────────────────────────────────

/// Abstraction over a price oracle.
pub trait OracleClient: Send + Sync {
    /// Return the latest LIFE++ token price in USD.
    fn get_latest_price(&self) -> f64;
}

// ─────────────────────────────────────────────────────────────────────────────
// EdgeDaemon
// ─────────────────────────────────────────────────────────────────────────────

/// An edge daemon with adaptive market-making capability.
///
/// Create via [`EdgeDaemon::new`], then call
/// [`EdgeDaemon::enter_market_making_loop`] to start processing intents.
pub struct EdgeDaemon<H: HardwareHost, O: OracleClient> {
    pub node_did: String,
    hardware_host: Arc<H>,
    oracle_client: Arc<O>,
    pricer: AdaptivePricingEngine,
    /// Accumulated bids from the current session (used in tests / telemetry).
    submitted_bids: Arc<Mutex<Vec<SealedBid>>>,
    /// Maximum semantic friction accepted (default 0.20).
    friction_threshold: f64,
    /// Node cognitive score / TrustRank [0, 1].
    local_trust_rank: f64,
}

impl<H: HardwareHost, O: OracleClient> EdgeDaemon<H, O> {
    /// Create a new edge daemon.
    ///
    /// # Arguments
    /// * `node_did`         – Decentralised identity of this edge node.
    /// * `hardware_host`    – Hardware telemetry provider.
    /// * `oracle_client`    – LIFE++ token price oracle.
    /// * `local_trust_rank` – Node S_cog score [0, 1].
    pub fn new(
        node_did: impl Into<String>,
        hardware_host: Arc<H>,
        oracle_client: Arc<O>,
        local_trust_rank: f64,
    ) -> Arc<Self> {
        Arc::new(Self {
            node_did: node_did.into(),
            hardware_host,
            oracle_client,
            pricer: AdaptivePricingEngine::new(),
            submitted_bids: Arc::new(Mutex::new(Vec::new())),
            friction_threshold: 0.20,
            local_trust_rank: local_trust_rank.clamp(0.0, 1.0),
        })
    }

    /// Process a single network intent and optionally produce a sealed bid.
    ///
    /// Returns `Some(SealedBid)` if the node participates in the auction,
    /// `None` if the intent is rejected (friction too high or oracle fault).
    pub async fn evaluate_intent(self: &Arc<Self>, intent: &NetworkIntent) -> Option<SealedBid> {
        // 1. Measure physical state.
        let friction = self.hardware_host.compute_semantic_friction(intent);
        let current_queue = self.hardware_host.get_current_queue_depth();
        let scog = self.local_trust_rank;

        // 2. Reject high-friction intents.
        if friction >= self.friction_threshold {
            return None;
        }

        // 3. Calculate optimal USD bid.
        let optimal_bid_usd = self.pricer.calculate_optimal_bid(friction, scog, current_queue);

        // 4. Convert to LIFE++ tokens via oracle.
        let token_price = self.oracle_client.get_latest_price();
        let bid_tokens = self.pricer.usd_to_tokens(optimal_bid_usd, token_price)?;

        println!(
            "💱 [AUCTION] Intent {} | Friction: {:.3} | Bid: {:.6} LIFE++",
            intent.id, friction, bid_tokens
        );

        let bid = SealedBid {
            intent_id: intent.id.clone(),
            node_did: self.node_did.clone(),
            bid_tokens,
        };

        self.submitted_bids.lock().await.push(bid.clone());
        Some(bid)
    }

    /// Run the market-making loop over a pre-populated stream of intents.
    ///
    /// In production this would listen on a Kafka topic or p2p gossip channel.
    /// The iterator variant is provided for deterministic integration testing.
    pub async fn enter_market_making_loop(
        self: Arc<Self>,
        intents: impl IntoIterator<Item = NetworkIntent>,
    ) -> Vec<SealedBid> {
        println!(
            "📈 [DAEMON] Market-Making loop active. Node {} ready for auction.",
            self.node_did
        );

        let mut bids = Vec::new();
        for intent in intents {
            if let Some(bid) = self.evaluate_intent(&intent).await {
                bids.push(bid);
            }
        }
        bids
    }

    /// Return all bids submitted during the session.
    pub async fn submitted_bids(&self) -> Vec<SealedBid> {
        self.submitted_bids.lock().await.clone()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Stub implementations ─────────────────────────────────────────────────

    struct StubHardware {
        friction: f64,
        queue: f64,
    }

    impl HardwareHost for StubHardware {
        fn compute_semantic_friction(&self, _: &NetworkIntent) -> f64 {
            self.friction
        }
        fn get_current_queue_depth(&self) -> f64 {
            self.queue
        }
    }

    struct StubOracle {
        price: f64,
    }

    impl OracleClient for StubOracle {
        fn get_latest_price(&self) -> f64 {
            self.price
        }
    }

    fn daemon(friction: f64, queue: f64, price: f64, scog: f64) -> Arc<EdgeDaemon<StubHardware, StubOracle>> {
        EdgeDaemon::new(
            "did:node:test",
            Arc::new(StubHardware { friction, queue }),
            Arc::new(StubOracle { price }),
            scog,
        )
    }

    fn intent(id: &str) -> NetworkIntent {
        NetworkIntent { id: id.to_string(), payload: "test payload".to_string() }
    }

    // ── Unit tests ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn low_friction_intent_produces_bid() {
        let d = daemon(0.10, 5.0, 0.50, 0.5);
        let bid = d.evaluate_intent(&intent("i-001")).await;
        assert!(bid.is_some(), "should produce a bid for low-friction intent");
    }

    #[tokio::test]
    async fn high_friction_intent_is_rejected() {
        let d = daemon(0.25, 5.0, 0.50, 0.5);
        let bid = d.evaluate_intent(&intent("i-002")).await;
        assert!(bid.is_none(), "friction ≥ 0.20 should be rejected");
    }

    #[tokio::test]
    async fn zero_oracle_price_is_rejected() {
        let d = daemon(0.10, 5.0, 0.0, 0.5);
        let bid = d.evaluate_intent(&intent("i-003")).await;
        assert!(bid.is_none(), "zero oracle price should return None");
    }

    /// Flash-crash: after a 90 % price drop, the token bid should be ~10× larger.
    #[tokio::test]
    async fn flash_crash_raises_token_bid_tenfold() {
        let friction = 0.10;
        let queue = 10.0;
        let scog = 0.5;

        let d_normal = daemon(friction, queue, 0.50, scog);
        let d_crash = daemon(friction, queue, 0.05, scog);

        let bid_normal = d_normal.evaluate_intent(&intent("flash-1")).await.unwrap();
        let bid_crash = d_crash.evaluate_intent(&intent("flash-1")).await.unwrap();

        let ratio = bid_crash.bid_tokens / bid_normal.bid_tokens;
        assert!(
            (ratio - 10.0).abs() < 1e-6,
            "token bid should be 10× after 90 % price drop, got ratio {ratio}"
        );
    }

    /// Bid-shading: empty queue → lower bid than full queue.
    #[tokio::test]
    async fn empty_queue_produces_lower_bid_than_full_queue() {
        let price = 0.50;
        let scog = 0.5;
        let friction = 0.10;

        let d_empty = daemon(friction, 0.0, price, scog);
        let d_full = daemon(friction, 80.0, price, scog);

        let bid_empty = d_empty.evaluate_intent(&intent("q-test")).await.unwrap();
        let bid_full = d_full.evaluate_intent(&intent("q-test")).await.unwrap();

        assert!(
            bid_full.bid_tokens > bid_empty.bid_tokens,
            "full-queue bid should exceed empty-queue bid"
        );
    }

    /// Loop: submitted bids are recorded.
    #[tokio::test]
    async fn market_making_loop_records_submitted_bids() {
        let d = daemon(0.10, 5.0, 0.50, 0.7);
        let intents = vec![intent("a"), intent("b"), intent("c")];
        let bids = Arc::clone(&d)
            .enter_market_making_loop(intents)
            .await;
        assert_eq!(bids.len(), 3);
        let recorded = d.submitted_bids().await;
        assert_eq!(recorded.len(), 3);
    }
}
