use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct LifetimeSig {
    pub module: String,
    pub name: String,
    pub lifetime_params: Vec<String>,
    pub param_lifetimes: Vec<Option<String>>,
    pub ret_lifetime: Option<String>,
    pub elided: bool,
}

#[derive(Debug, Clone, Default)]
pub struct RegionResult {
    pub lifetimes: BTreeMap<String, LifetimeSig>,
}

impl RegionResult {
    pub fn get(&self, module: &str, name: &str) -> Option<&LifetimeSig> {
        self.lifetimes.get(&format!("{module}::{name}"))
    }
}
