//! CaaS MCP service registration helper.
//! This is an AP2-side adapter that can later be bound to real ERC-8004 contracts.

use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct McpServiceRegistration {
    pub service_id: String,
    pub mcp_endpoint: String,
    pub metadata_cid: String,
    pub registrant_agent_id: String,
}

#[derive(Clone, Default)]
pub struct CaaSRegistry {
    registrations: Arc<Mutex<Vec<McpServiceRegistration>>>,
}

impl CaaSRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_service(
        &self,
        registration: McpServiceRegistration,
    ) -> Result<String, &'static str> {
        if !registration.mcp_endpoint.starts_with("mcp://") {
            return Err("MCP_REGISTRY_REJECTED: endpoint must start with mcp://");
        }

        let tx_hash = format!("0x{}", &hash_registration(&registration)[..32]);
        self.registrations
            .lock()
            .expect("registry lock poisoned")
            .push(registration);
        Ok(tx_hash)
    }

    pub fn count(&self) -> usize {
        self.registrations
            .lock()
            .expect("registry lock poisoned")
            .len()
    }
}

fn hash_registration(reg: &McpServiceRegistration) -> String {
    use sha2::{Digest, Sha256};

    let payload = format!(
        "{}|{}|{}|{}",
        reg.service_id, reg.mcp_endpoint, reg.metadata_cid, reg.registrant_agent_id
    );
    hex::encode(Sha256::digest(payload.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_mcp_registration() {
        let registry = CaaSRegistry::new();
        let tx_hash = registry
            .register_service(McpServiceRegistration {
                service_id: "lifepp-caas".to_string(),
                mcp_endpoint: "mcp://caas.lifepp.local".to_string(),
                metadata_cid: "bafy-demo".to_string(),
                registrant_agent_id: "cai-1".to_string(),
            })
            .expect("should register");

        assert!(tx_hash.starts_with("0x"));
        assert_eq!(registry.count(), 1);
    }
}
