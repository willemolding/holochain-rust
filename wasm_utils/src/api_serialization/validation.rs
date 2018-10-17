use holochain_core_types::{chain_header::ChainHeader, hash::HashString};

extern crate serde_json;

#[derive(Clone, Serialize, Deserialize)]
pub struct ValidationData {
    pub chain_header: Option<ChainHeader>,
    pub sources: Vec<HashString>,
    pub source_chain_entries: Option<Vec<serde_json::Value>>,
    pub source_chain_headers: Option<Vec<ChainHeader>>,
    pub custom: Option<serde_json::Value>,
    pub lifecycle: EntryLifecycle,
    pub action: EntryAction,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum EntryLifecycle {
    Chain,
    Dht,
    Meta,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum EntryAction {
    Commit,
    Modify,
    Delete,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum LinkAction {
    Commit,
    Delete,
}
