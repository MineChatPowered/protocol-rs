# MineChat Protocol

[![Rust](https://github.com/walker84837/minechat-protocol/actions/workflows/rust.yml/badge.svg)](https://github.com/walker84837/minechat-protocol/actions/workflows/rust.yml)

MineChat is a Rust library designed to facilitate communication with a Minecraft chat server. It provides an asynchronous, runtime-independent API to send and receive messages, handle authentication, and manage connections.

## Features

- Asynchronous message sending and receiving via the `MessageStream` trait.
- Runtime-agnostic core library with an optional `tokio` feature for a default implementation.
- Messages are serialized with CBOR and compressed with zstd for efficiency.
- Support for rich text chat messages using `kyori-component-json`.
- Detailed error types for robust error handling.
- UUID generation for client identification.

## Protocol Specification

The MineChat protocol uses a simple message framing structure on top of a reliable, ordered stream transport (like TCP or TLS).

### Message Framing

Each message sent over the stream is framed as follows:

1.  **Decompressed Size (4 bytes):** A 32-bit unsigned integer representing the size of the original, uncompressed message payload in bytes.
2.  **Compressed Size (4 bytes):** A 32-bit unsigned integer representing the size of the compressed message payload in bytes.
3.  **Compressed Payload:** The zstd-compressed, CBOR-serialized `MineChatMessage`.

### Message Types

The following message types are supported:

| Type         | Payload              | Description                                                                 |
|--------------|----------------------|-----------------------------------------------------------------------------|
| `AUTH`       | `AuthPayload`        | Sent by the client to authenticate with the server.                         |
| `AUTH_ACK`   | `AuthAckPayload`     | Sent by the server to acknowledge the authentication status.                |
| `CHAT`       | `ChatPayload`        | Sent by the client to send a chat message to the server.                    |
| `BROADCAST`  | `BroadcastPayload`   | Sent by the server to broadcast a chat message to all connected clients.    |
| `DISCONNECT` | `DisconnectPayload`  | Sent by either the client or the server to gracefully close the connection. |

All message payloads are defined as Rust structs and are serialized using CBOR.

## Usage

The core of the library is the `MessageStream` trait, which provides `send_message` and `receive_message` methods. You can implement this trait for any stream that provides `AsyncRead` and `AsyncWrite`.

A default implementation for `tokio` streams is provided under the `tokio` feature flag (enabled by default).

### Example with Tokio

```rust
use minechat_protocol::{protocol::*, TokioMessageStream, MessageStream};
use tokio::net::TcpStream;
use kyori_component_json::Component;

#[tokio::main]
async fn main() {
    let server_addr = "127.0.0.1:8080";
    let stream = TcpStream::connect(server_addr).await.unwrap();
    let mut message_stream = TokioMessageStream::new(stream);

    let message = MineChatMessage::Chat {
        payload: ChatPayload {
            message: Component::text("Hello, server!"),
        },
    };

    if let Err(e) = message_stream.send_message(&message).await {
        eprintln!("Failed to send message: {}", e);
    }

    match message_stream.receive_message().await {
        Ok(message) => println!("Received message: {:?}", message),
        Err(e) => eprintln!("Failed to receive message: {}", e),
    }
}
```

## License

This project is licensed under the **Mozilla Public License Version 2.0**. See the [LICENSE](LICENSE) file for more details.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## Contact

For any questions or support, please open an issue on the [GitHub repository](https://github.com/walker84837/minechat-protocol).