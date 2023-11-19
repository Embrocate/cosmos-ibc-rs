//! Exports data structures and implementations of different IBC core (TAO) components.
#![no_std]
#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::disallowed_methods, clippy::disallowed_types,))]
#![deny(
    warnings,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications,
    rust_2018_idioms
)]

pub mod entrypoint {
    #[doc(inline)]
    pub use ibc_core_handler::entrypoint::*;
}

pub mod channel {
    #[doc(inline)]
    pub use ibc_core_channel::*;
}

pub mod client {
    #[doc(inline)]
    pub use ibc_core_client::*;
}

pub mod commitment_types {
    #[doc(inline)]
    pub use ibc_core_commitment_types::*;
}

pub mod connection {
    #[doc(inline)]
    pub use ibc_core_connection::*;
}

pub mod host {
    #[doc(inline)]
    pub use ibc_core_host::*;
}

pub mod handler {
    #[doc(inline)]
    pub use ibc_core_handler::*;
}

pub mod router {
    #[doc(inline)]
    pub use ibc_core_router::*;
}

pub mod primitives {
    #[doc(inline)]
    pub use ibc_primitives::*;
}
