//! MCNP Core 与 Panel 之间的安全应用协议。
//!
//! 本 crate 负责线协议的版本、长度前缀帧、JSON 消息和安全传输适配；业务字段
//! 仍由 `nexus-domain` 定义。协议实现不应把来自网络的 JSON 直接当作已验证业务输入。

mod certificate_fingerprint;
mod frame;
mod frame_codec;
mod frame_error;
mod insecure_server_certificate_verifier;
mod message_codec;
mod message_error;
mod noise_transport;
mod preshared_key;
mod preshared_key_error;
mod protocol_version;
mod protocol_version_error;
mod session_error;
mod tls_client;
mod tls_error;
mod wire_error;
mod wire_message;

pub use certificate_fingerprint::certificate_sha256;
pub use frame::Frame;
pub use frame_codec::FrameCodec;
pub use frame_error::FrameError;
pub use message_codec::deserialize_message;
pub use message_codec::serialize_message;
pub use message_error::MessageError;
pub use noise_transport::NoiseTransport;
pub use preshared_key::PresharedKey;
pub use preshared_key_error::PresharedKeyError;
pub use protocol_version::ProtocolVersion;
pub use protocol_version_error::ProtocolVersionError;
pub use session_error::SessionError;
pub use tls_client::TlsClientStream;
pub use tls_client::connect_tls;
pub use tls_error::TlsError;
pub use wire_error::WireError;
pub use wire_message::WireMessage;

/// 当前协议版本值，用于握手协商。
pub const CURRENT_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::new(1, 0);
/// 单个密文帧允许的最大负载长度，单位为字节。
pub const MAX_CIPHERTEXT_FRAME_BYTES: usize = 65_535;
/// 单个明文 JSON 消息允许的最大长度，单位为字节。
pub const MAX_PLAINTEXT_JSON_BYTES: usize = 60 * 1024;
/// Noise 握手使用的固定协议前导字节串。
pub const NOISE_PROLOGUE: &[u8] = b"MCNP/1";
/// 对外展示的协议版本字符串。
pub const PROTOCOL_VERSION: &str = "1.0";
