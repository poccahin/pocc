pub mod bandwidth_allocator;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RouterError {
    #[error("Score {0} is out of range [0, 100]")]
    InvalidScore(f64),
}

#[derive(Debug, Clone)]
pub struct TrustProfile {
    pub agent_did: String,
    pub s_cog_score: f64,
}

pub struct DynamicTrustRouter {
    profiles: HashMap<String, TrustProfile>,
}

impl Default for DynamicTrustRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl DynamicTrustRouter {
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
        }
    }

    /// Register a new agent with an initial S_cog score in [0, 100].
    ///
    /// Returns `Err(InvalidScore)` if `initial_score` is outside [0, 100].
    /// Returns `Ok(false)` if the agent is already registered (no change).
    /// Returns `Ok(true)` on first registration.
    pub fn register_agent(&mut self, did: &str, initial_score: f64) -> Result<bool, RouterError> {
        if !(0.0..=100.0).contains(&initial_score) {
            return Err(RouterError::InvalidScore(initial_score));
        }
        if self.profiles.contains_key(did) {
            return Ok(false);
        }
        self.profiles.insert(did.to_string(), TrustProfile {
            agent_did: did.to_string(),
            s_cog_score: initial_score,
        });
        Ok(true)
    }

    pub fn slash_agent(&mut self, did: &str, severity: f64) {
        let profile = self
            .profiles
            .entry(did.to_string())
            .or_insert(TrustProfile {
                agent_did: did.to_string(),
                s_cog_score: 50.0,
            });
        profile.s_cog_score = (profile.s_cog_score - severity * 20.0).max(0.0);
    }

    pub fn get_routing_weight(&self, did: &str) -> f64 {
        self.profiles.get(did).map(|p| p.s_cog_score).unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_agent_creates_profile() {
        let mut router = DynamicTrustRouter::new();
        let inserted = router.register_agent("did:node:alpha", 75.0).unwrap();
        assert!(inserted);
        assert!((router.get_routing_weight("did:node:alpha") - 75.0).abs() < f64::EPSILON);
    }

    #[test]
    fn duplicate_registration_is_idempotent() {
        let mut router = DynamicTrustRouter::new();
        router.register_agent("did:node:beta", 80.0).unwrap();
        let second = router.register_agent("did:node:beta", 10.0).unwrap();
        assert!(!second);
        // Score remains from the first registration
        assert!((router.get_routing_weight("did:node:beta") - 80.0).abs() < f64::EPSILON);
    }

    #[test]
    fn out_of_range_score_returns_error() {
        let mut router = DynamicTrustRouter::new();
        assert!(matches!(
            router.register_agent("did:node:bad", 150.0),
            Err(RouterError::InvalidScore(_))
        ));
        assert!(matches!(
            router.register_agent("did:node:bad", -1.0),
            Err(RouterError::InvalidScore(_))
        ));
    }

    #[test]
    fn slash_reduces_score_to_zero_minimum() {
        let mut router = DynamicTrustRouter::new();
        router.register_agent("did:node:delta", 10.0).unwrap();
        router.slash_agent("did:node:delta", 99999.0);
        assert_eq!(router.get_routing_weight("did:node:delta"), 0.0);
    }
}
