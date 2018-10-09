//! CAS Implementations
//!
//! (CAS == Content Addressable Storage)
//!
//! This crate contains implementations for the CAS and EAV traits
//! which are defined but not implemented in the core_types crate.

extern crate futures;
extern crate holochain_core_types;
extern crate riker;
extern crate riker_default;
extern crate riker_patterns;
#[macro_use]
extern crate unwrap_to;
extern crate snowflake;
extern crate tempfile;

pub mod cas;
pub mod eav;