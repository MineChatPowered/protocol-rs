// Re-export commonly used types for backwards compatibility
pub use crate::client::{
    link_with_server, send_capabilities, send_chat_message, send_disconnect, send_pong,
};
#[cfg(feature = "tokio")]
pub use crate::stream::TokioMessageStream;
#[cfg(feature = "tls-rustls")]
pub use crate::tls::RustlsTlsMessageStream;
#[cfg(feature = "tls-native")]
pub use crate::tls::TlsMessageStream;
