use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TrustProfile {
    pub agent_did: String,
    pub s_cog_score: f64,
}

pub struct DynamicTrustRouter {
    profiles: HashMap<String, TrustProfile>,
}

impl Default for DynamicTrustRouter {
    fn default() -> Self { Self::new() }
}

impl DynamicTrustRouter {
    pub fn new() -> Self { Self { profiles: HashMap::new() } }
    pub fn slash_agent(&mut self, did: &str, severity: f64) {
        let profile = self.profiles.entry(did.to_string()).or_insert(TrustProfile { agent_did: did.to_string(), s_cog_score: 50.0 });
        profile.s_cog_score = (profile.s_cog_score - severity * 20.0).max(0.0);
    }
    pub fn get_routing_weight(&self, did: &str) -> f64 {
        self.profiles.get(did).map(|p| p.s_cog_score).unwrap_or(0.0)
    }
}
