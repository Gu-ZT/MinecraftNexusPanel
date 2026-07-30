mod core_id;
mod event_id;
mod request_id;

pub use core_id::CoreId;
pub use event_id::EventId;
pub use request_id::RequestId;

pub const API_VERSION: &str = "v1";
pub const PRODUCT_NAME: &str = "MCNP";
pub const PRODUCT_VERSION: &str = env!("CARGO_PKG_VERSION");
