#![deny(clippy::unwrap_used)]

#[macro_use]
extern crate log;

pub use aries_vcx;
pub use aries_vcx::aries_vcx_wallet::wallet::askar::askar_wallet_config::AskarWalletConfig;
pub use url::Url;

pub mod connection_service;
pub mod error;
pub mod framework;
pub mod invitation_service;
pub mod messaging_service;
mod transports;
