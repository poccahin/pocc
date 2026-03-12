use crate::kinetic::{KineticCommand, KineticError};
use std::collections::HashSet;

pub struct ObjectiveDrivenEngine {
    safety_threshold: f64,
    hardware_blacklist: HashSet<String>,
}

impl ObjectiveDrivenEngine {
    pub fn new(safety_threshold: f64) -> Self {
        Self { safety_threshold, hardware_blacklist: HashSet::new() }
    }
    pub async fn psychological_rehearsal(&self, _cmd: &KineticCommand) -> Result<(), KineticError> {
        Ok(())
    }
    pub fn inject_hardware_blacklist(&mut self, did: &str) {
        self.hardware_blacklist.insert(did.to_string());
    }
}
