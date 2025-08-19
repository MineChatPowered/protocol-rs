//! minechat-protocol: A Rust library for Minecraft chat server communication.
//!
//! This crate provides an asynchronous, runtime-independent API for interacting with a Minecraft
//! chat server. It handles message serialization (CBOR), compression (zstd), and framing,
//! allowing you to focus on the application logic.
//!
//! The core of the library is the [`MessageStream`] trait, which defines the interface for
//! sending and receiving [`MineChatMessage`]s. A default implementation for Tokio streams,
//! [`TokioMessageStream`], is provided under the `tokio` feature flag.
//!
//! ## Features
//!
//! - **Asynchronous**: Built for non-blocking I/O.
//! - **Runtime-Independent**: The core [`MessageStream`] trait can be implemented for any
//!   asynchronous runtime.
//! - **Efficient**: Messages are serialized with CBOR and compressed with zstd.
//! - **Rich Text Support**: Integrates with `kyori-component-json` for handling Minecraft's
//!   rich text components.
//! - **Authentication**: Provides mechanisms for client authentication with the server.
//!
//! ## Getting Started
//!
//! Add `minechat-protocol` to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! minechat-protocol = "0.4" # Use the latest version
//! tokio = { version = "1", features = ["full"] } # If using Tokio runtime
//! kyori-component-json = "0.2" # For rich text components
//! ```
//!
//! ## Examples
//!
//! ### Sending a Chat Message (with Tokio)
//!
//! ```no_run
//! use minechat_protocol::{protocol::*, TokioMessageStream, MessageStream};
//! use tokio::net::TcpStream;
//! use kyori_component_json::Component;
//!
//! #[tokio::test]
//! async fn example_send_chat_message() -> Result<(), MineChatError> {
//!     let server_addr = "127.0.0.1:8080";
//!     let stream = TcpStream::connect(server_addr).await?;
//!     let mut message_stream = TokioMessageStream::new(stream);
//!
//!     let chat_message = MineChatMessage::Chat {
//!         payload: ChatPayload {
//!             message: Component::text("Hello, server! From MineChat Rust client."),
//!         },
//!     };
//!
//!     message_stream.send_message(&chat_message).await?;
//!     println!("Sent chat message.");
//!     Ok(())
//! }
//! ```
//!
//! ### Receiving a Broadcast Message (with Tokio)
//!
//! ```no_run
//! use minechat_protocol::{protocol::*, TokioMessageStream, MessageStream};
//! use tokio::net::TcpStream;
//!
//! #[tokio::test]
//! async fn example_receive_broadcast_message() -> Result<(), MineChatError> {
//!     let server_addr = "127.0.0.1:8080";
//!     let stream = TcpStream::connect(server_addr).await?;
//!     let mut message_stream = TokioMessageStream::new(stream);
//!
//!     println!("Waiting for messages from server...");
//!     match message_stream.receive_message().await? {
//!         MineChatMessage::Broadcast { payload } => {
//!             println!("Received broadcast from {}: {:?}", payload.from, payload.message);
//!         },
//!         other_message => {
//!             println!("Received other message type: {:?}", other_message);
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ### Authenticating with the Server
//!
//! ```no_run
//! use minechat_protocol::{protocol::*, packets::link_with_server, TokioMessageStream, MessageStream};
//! use tokio::net::TcpStream;
//!
//! #[tokio::test]
//! async fn example_authenticate_with_server() -> Result<(), MineChatError> {
//!     let server_addr = "127.0.0.1:8080";
//!     let stream = TcpStream::connect(server_addr).await?;
//!     let mut message_stream = TokioMessageStream::new(stream);
//!
//!     let link_code = "YOUR_LINK_CODE_HERE"; // Replace with your actual link code
//!     match link_with_server(&mut message_stream, link_code).await {
//!         Ok(client_uuid) => {
//!             println!("Successfully linked with server. Client UUID: {}", client_uuid);
//!         },
//!         Err(e) => {
//!             eprintln!("Failed to link with server: {}", e);
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Protocol Details
//!
//! Messages are framed with 4 bytes for decompressed size, 4 bytes for compressed size,
//! followed by the zstd-compressed, CBOR-serialized [`MineChatMessage`] payload.
//!
//! For more detailed information on message structures and error types, refer to the
//! [`protocol`] module documentation.
//!
//! [`MessageStream`]: crate::protocol::MessageStream
//! [`MineChatMessage`]: crate::protocol::MineChatMessage
//! [`TokioMessageStream`]: crate::packets::TokioMessageStream
//! [`protocol`]: crate::protocol
#![allow(dead_code)]
#![warn(missing_docs)]
#![forbid(unsafe_code)]
/// Contains the implementation of the `TokioMessageStream` and the `link_with_server` function.
pub mod packets;
/// Contains the core protocol definitions, including message types, payloads, and the `MessageStream` trait.
pub mod protocol;
pub use protocol::MessageStream;
#[cfg(feature = "tokio")]
pub use packets::TokioMessageStream;