pub mod actor;
pub mod entry;
pub mod header;
pub mod memory;
pub mod pair;
pub mod pair_meta;
pub mod status;
pub mod sys_entry;
pub mod links_entry;
pub mod deletion_entry;

// use agent::keys::Keys;
use error::HolochainError;
use hash_table::{pair_meta::Meta,
                 links_entry::Link, links_entry::LinkListEntry,
    entry::Entry,
                 //header::Header,
};
use nucleus::ribosome::api::get_links::GetLinksArgs;

pub type HashString = String;

/// Trait of the data structure storing the source chain
/// source chain is stored as a hash table of Pairs.
/// Pair is a pair holding an Entry and its Header
pub trait HashTable: Send + Sync + Clone + 'static {
    // internal state management
    // @TODO does this make sense at the trait level?
    // @see https://github.com/holochain/holochain-rust/issues/262
    fn setup(&mut self) -> Result<(), HolochainError>;
    fn teardown(&mut self) -> Result<(), HolochainError>;

    // crud
    /// add a Pair to the HashTable, analogous to chain.push() but ordering is not enforced
//    fn commit(&mut self, pair: &Pair) -> Result<(), HolochainError>;

    fn put(&mut self, entry: &Entry) -> Result<(), HolochainError>;

    /// lookup an Entry from the HashTable
    fn entry(&self, key: &str) -> Result<Option<Entry>, HolochainError>;

//    /// add a new Pair to the HashTable as per commit and status link an old Pair as MODIFIED
//    fn modify(
//        &mut self,
//        keys: &Keys,
//        old_pair: &Pair,
//        new_pair: &Pair,
//    ) -> Result<(), HolochainError>;

    // /// set the status of a Pair to DELETED
    // fn retract(&mut self, keys: &Keys, pair: &Pair) -> Result<(), HolochainError>;

    /// Add a link to an Entry has Metadata
    fn add_link(&mut self, link: &Link) -> Result<(), HolochainError>;
    fn remove_link(&mut self, link: &Link) -> Result<(), HolochainError>;
    fn links(&mut self, links_request: &GetLinksArgs) -> Result<Option<LinkListEntry>, HolochainError>;

    // Meta
    /// assert a given PairMeta in the HashTable
    fn assert_meta(&mut self, meta: &Meta) -> Result<(), HolochainError>;
    /// lookup a PairMeta from the HashTable by key
    fn get_meta(&mut self, key: &str) -> Result<Option<Meta>, HolochainError>;

    /// lookup all PairMeta for a given Pair
    // fn get_pair_meta(&mut self, pair: &Pair) -> Result<Vec<Meta>, HolochainError>;

    /// lookup all PairMeta for a given Entry
    fn get_entry_meta(&mut self, entry: &Entry) -> Result<Vec<Meta>, HolochainError>;

    // ;)
    fn get_meta_for(&mut self, entry_hash: HashString, attribute_name: &str) -> Result<Option<Meta>, HolochainError>;

    // query
    // @TODO how should we handle queries?
    // @see https://github.com/holochain/holochain-rust/issues/141
    // fn query (&self, query: &Query) -> Result<std::collections::HashSet, HolochainError>;
}
