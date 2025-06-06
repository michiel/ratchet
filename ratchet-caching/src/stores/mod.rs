//! Cache store implementations

pub mod inmemory;

#[cfg(feature = "lru")]
pub mod lru;

#[cfg(feature = "ttl")]
pub mod ttl;

#[cfg(feature = "moka")]
pub mod moka;

pub use inmemory::InMemoryCache;

#[cfg(feature = "lru")]
pub use lru::LruCache;

#[cfg(feature = "ttl")]
pub use ttl::TtlCache;

#[cfg(feature = "moka")]
pub use moka::MokaCache;