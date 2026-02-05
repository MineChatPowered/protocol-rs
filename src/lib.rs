#![warn(missing_docs)]
#![forbid(unsafe_code)]
/// Contains client-side protocol operations.
pub mod client;
/// Contains packet type definitions with proper serialization.
pub mod packets;
/// Contains core protocol definitions, including packet types, payloads, and the `MessageStream` trait.
pub mod protocol;
/// Contains stream implementations for different I/O backends.
pub mod stream;
/// Contains TLS implementations for secure communication.
pub mod tls;
/// Contains new type definitions for the protocol.
pub mod types;

// Re-export commonly used items
#[cfg(feature = "tls-rustls")]
pub use stream::RustlsTlsMessageStream;
#[cfg(feature = "tls-native")]
pub use stream::TlsMessageStream;
#[cfg(feature = "tokio")]
pub use stream::{
    TokioMessageStream, link_with_server, send_capabilities, send_chat_message, send_disconnect,
    send_pong,
};

// Legacy protocol exports for compatibility
pub use protocol::{
    AuthOkPayload, CapabilitiesPayload, ChatMessagePayload, DisconnectPayload, LinkOkPayload,
    LinkPayload, MessageStream, MineChatError, MineChatPacket, ModerationPayload, Payload,
    PingPayload, PongPayload, chat_format, moderation_action, moderation_scope, packet_types,
};

// New type exports
pub use types::MessageContent;

// New packet exports
pub use packets::{
    ClientUuid, LinkCode, MineChatPacket as Packet, MinecraftUuid, ModerationAction,
    ModerationScope,
};

// Re-export kyori-component-json for component support
pub use kyori_component_json;
