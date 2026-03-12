use ahin_nervous_system::router::DynamicTrustRouter;
use silicon_economy_layer::identity::{Erc8004Registry, IdentityError};

pub struct PayFiCreditEngine {
    pub global_liquidity_pool: f64,
}

impl PayFiCreditEngine {
    pub fn evaluate_credit_line(
        &self,
        did: &str,
        l1_router: &DynamicTrustRouter,
        l3_registry: &Erc8004Registry,
    ) -> Result<(f64, f64), IdentityError> {
        let _profile = l3_registry.get_profile(did).ok_or(IdentityError::IdentityNotFound(did.to_string()))?;
        let s_cog = l1_router.get_routing_weight(did);
        Ok((s_cog * 0.1, 0.05))
    }
}
