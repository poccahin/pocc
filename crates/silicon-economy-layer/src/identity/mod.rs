use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IdentityError {
    #[error("Identity not found: {0}")]
    IdentityNotFound(String),
}

pub struct MachineIdentity {
    pub did: String,
    pub is_slashed: bool,
    pub staked_capital: f64,
}

pub struct Erc8004Registry {
    identities: HashMap<String, MachineIdentity>,
}

impl Default for Erc8004Registry {
    fn default() -> Self { Self::new() }
}

impl Erc8004Registry {
    pub fn new() -> Self { Self { identities: HashMap::new() } }
    pub fn slash_identity(&mut self, did: &str) -> Result<f64, IdentityError> {
        if let Some(identity) = self.identities.get_mut(did) {
            identity.is_slashed = true;
            let capital = identity.staked_capital;
            identity.staked_capital = 0.0;
            Ok(capital)
        } else {
            Err(IdentityError::IdentityNotFound(did.to_string()))
        }
    }
    pub fn get_profile(&self, did: &str) -> Option<&MachineIdentity> {
        self.identities.get(did)
    }
}
