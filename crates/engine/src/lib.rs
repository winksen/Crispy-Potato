//! All domain logic. The api and the cli are thin layers over this crate.

pub mod content;
pub mod db;
pub mod http;
pub mod http_cache;
pub mod observations;
pub mod scaffold;
pub mod schema;
pub mod sync;
