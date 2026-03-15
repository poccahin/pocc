use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IdentityError {
    #[error("Identity not found: {0}")]
    IdentityNotFound(String),
    #[error("Identity already registered: {0}")]
    AlreadyRegistered(String),
    #[error("Invalid stake amount: {0} (must be ≥ 0)")]
    InvalidStake(f64),
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

    /// Register a new machine identity with an initial capital stake.
    ///
    /// Returns `Err(AlreadyRegistered)` if the DID is already present.
    /// Returns `Err` if `initial_stake` is negative.
    pub fn register_identity(&mut self, did: &str, initial_stake: f64) -> Result<(), IdentityError> {
        if self.identities.contains_key(did) {
            return Err(IdentityError::AlreadyRegistered(did.to_string()));
        }
        if initial_stake < 0.0 {
            return Err(IdentityError::InvalidStake(initial_stake));
        }
        self.identities.insert(did.to_string(), MachineIdentity {
            did: did.to_string(),
            is_slashed: false,
            staked_capital: initial_stake,
        });
        Ok(())
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_slash_confiscates_stake() {
        let mut registry = Erc8004Registry::new();
        registry.register_identity("did:node:alpha", 500.0).unwrap();
        let confiscated = registry.slash_identity("did:node:alpha").unwrap();
        assert!((confiscated - 500.0).abs() < f64::EPSILON);
        assert!(registry.get_profile("did:node:alpha").unwrap().is_slashed);
    }

    #[test]
    fn duplicate_registration_is_rejected() {
        let mut registry = Erc8004Registry::new();
        registry.register_identity("did:node:beta", 100.0).unwrap();
        assert!(matches!(
            registry.register_identity("did:node:beta", 200.0),
            Err(IdentityError::AlreadyRegistered(_))
        ));
    }

    #[test]
    fn slash_unknown_identity_returns_error() {
        let mut registry = Erc8004Registry::new();
        assert!(matches!(
            registry.slash_identity("did:node:ghost"),
            Err(IdentityError::IdentityNotFound(_))
        ));
    }

    #[test]
    fn negative_stake_is_rejected() {
        let mut registry = Erc8004Registry::new();
        assert!(matches!(
            registry.register_identity("did:node:gamma", -50.0),
            Err(IdentityError::InvalidStake(_))
        ));
    }
}
