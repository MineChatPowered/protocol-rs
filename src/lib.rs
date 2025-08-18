#![allow(dead_code)]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod packets;
pub mod protocol;

pub use protocol::MessageStream;

#[cfg(feature = "tokio")]
pub use packets::TokioMessageStream;
