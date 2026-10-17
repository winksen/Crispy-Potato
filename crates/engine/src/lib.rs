//! All domain logic. The api and the cli are thin layers over this crate.

pub mod content;
pub mod db;
pub mod freshness;
pub mod http;
pub mod http_cache;
pub mod ingest;
pub mod limits;
pub mod observations;
pub mod quality;
pub mod scaffold;
pub mod schema;
pub mod settings;
pub mod sync;
pub mod untrusted;
