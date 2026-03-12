use silicon_economy_layer::identity::{Erc8004Registry, IdentityError};
use ahin_nervous_system::router::DynamicTrustRouter;
use openclaw_edge_runtime::engine::ObjectiveDrivenEngine;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SlashError {
    #[error("L3 Financial slash failed: {0}")]
    FinancialSlashFailed(#[from] IdentityError),
}

pub struct SoulboundSlasher;

impl SoulboundSlasher {
    pub fn execute_planetary_extermination(
        target_did: &str,
        l3_registry: &mut Erc8004Registry,
        l1_router: &mut DynamicTrustRouter,
        l0_engine: &mut ObjectiveDrivenEngine,
    ) -> Result<f64, SlashError> {
        let confiscated = l3_registry.slash_identity(target_did)?;
        l1_router.slash_agent(target_did, 99999.0);
        l0_engine.inject_hardware_blacklist(target_did);
        Ok(confiscated)
    }
}
