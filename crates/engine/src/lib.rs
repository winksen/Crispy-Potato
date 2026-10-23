//! All domain logic. The api and the cli are thin layers over this crate.

pub mod content;
pub mod db;
pub mod diagnostics;
mod egress;
pub mod freshness;
mod hashing;
pub mod http;
pub mod http_cache;
pub mod ingest;
pub mod limits;
pub mod links;
pub mod observations;
pub mod quality;
mod run_guard;
pub mod scaffold;
pub mod schema;
pub mod settings;
pub mod sync;
pub mod untrusted;
pub mod verify_ids;

#[cfg(test)]
extern crate self as engine;

#[cfg(test)]
mod cache_tests;
#[cfg(test)]
mod http_tests;
#[cfg(test)]
mod link_tests;
