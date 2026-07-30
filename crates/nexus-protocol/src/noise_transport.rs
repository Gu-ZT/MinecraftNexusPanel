use snow::Builder;
use snow::HandshakeState;
use snow::TransportState;
use snow::params::NoiseParams;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;

use crate::FrameCodec;
use crate::MAX_CIPHERTEXT_FRAME_BYTES;
use crate::NOISE_PROLOGUE;
use crate::PresharedKey;
use crate::SessionError;
use crate::WireMessage;
use crate::deserialize_message;
use crate::serialize_message;

const NOISE_PATTERN: &str = "Noise_NNpsk0_25519_ChaChaPoly_BLAKE2s";

pub struct NoiseTransport<S> {
    stream: S,
    transport: TransportState,
}

impl<S> NoiseTransport<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn accept(stream: S, pre_shared_key: &PresharedKey) -> Result<Self, SessionError> {
        let mut handshake = build_responder(pre_shared_key)?;
        let mut buffer = vec![0_u8; MAX_CIPHERTEXT_FRAME_BYTES];
        let mut stream = stream;

        let request = read_raw_frame(&mut stream).await?;
        let payload_length = handshake.read_message(&request, &mut buffer)?;
        if payload_length != 0 {
            return Err(SessionError::UnexpectedHandshakePayload);
        }

        let response_length = handshake.write_message(&[], &mut buffer)?;
        write_raw_frame(&mut stream, &buffer[..response_length]).await?;

        Ok(Self {
            stream,
            transport: handshake.into_transport_mode()?,
        })
    }

    pub async fn connect(stream: S, pre_shared_key: &PresharedKey) -> Result<Self, SessionError> {
        let mut handshake = build_initiator(pre_shared_key)?;
        let mut buffer = vec![0_u8; MAX_CIPHERTEXT_FRAME_BYTES];
        let mut stream = stream;

        let request_length = handshake.write_message(&[], &mut buffer)?;
        write_raw_frame(&mut stream, &buffer[..request_length]).await?;

        let response = read_raw_frame(&mut stream).await?;
        let payload_length = handshake.read_message(&response, &mut buffer)?;
        if payload_length != 0 {
            return Err(SessionError::UnexpectedHandshakePayload);
        }

        Ok(Self {
            stream,
            transport: handshake.into_transport_mode()?,
        })
    }

    pub async fn read_message(&mut self) -> Result<WireMessage, SessionError> {
        let ciphertext = read_raw_frame(&mut self.stream).await?;
        let mut plaintext = vec![0_u8; ciphertext.len()];
        let plaintext_length = self.transport.read_message(&ciphertext, &mut plaintext)?;

        deserialize_message(&plaintext[..plaintext_length]).map_err(SessionError::from)
    }

    pub async fn write_message(&mut self, message: &WireMessage) -> Result<(), SessionError> {
        let plaintext = serialize_message(message)?;
        let mut ciphertext = vec![0_u8; MAX_CIPHERTEXT_FRAME_BYTES];
        let ciphertext_length = self.transport.write_message(&plaintext, &mut ciphertext)?;

        write_raw_frame(&mut self.stream, &ciphertext[..ciphertext_length]).await
    }
}

fn build_initiator(pre_shared_key: &PresharedKey) -> Result<HandshakeState, SessionError> {
    build_handshake(pre_shared_key)?
        .build_initiator()
        .map_err(SessionError::from)
}

fn build_responder(pre_shared_key: &PresharedKey) -> Result<HandshakeState, SessionError> {
    build_handshake(pre_shared_key)?
        .build_responder()
        .map_err(SessionError::from)
}

fn build_handshake(pre_shared_key: &PresharedKey) -> Result<Builder<'_>, SessionError> {
    let parameters: NoiseParams = NOISE_PATTERN.parse()?;

    Builder::new(parameters)
        .prologue(NOISE_PROLOGUE)?
        .psk(0, pre_shared_key.as_bytes())
        .map_err(SessionError::from)
}

async fn read_raw_frame<S>(stream: &mut S) -> Result<Vec<u8>, SessionError>
where
    S: AsyncRead + Unpin,
{
    let mut header = [0_u8; 4];
    stream.read_exact(&mut header).await?;

    let payload_length = u32::from_be_bytes(header) as usize;
    FrameCodec::default().validate_payload_length(payload_length)?;

    let mut payload = vec![0_u8; payload_length];
    stream.read_exact(&mut payload).await?;

    Ok(payload)
}

async fn write_raw_frame<S>(stream: &mut S, payload: &[u8]) -> Result<(), SessionError>
where
    S: AsyncWrite + Unpin,
{
    let frame = FrameCodec::default().encode(payload)?;
    stream.write_all(&frame).await?;
    stream.flush().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use nexus_domain::RequestId;
    use serde_json::json;
    use tokio::io::duplex;

    use super::NoiseTransport;
    use crate::PresharedKey;
    use crate::WireMessage;

    #[tokio::test]
    async fn round_trips_an_encrypted_message() {
        let pre_shared_key = PresharedKey::from_secret(b"0123456789abcdef0123456789abcdef")
            .expect("test secret is valid");
        let (client_stream, server_stream) = duplex(128 * 1024);
        let server_key = pre_shared_key.clone();
        let server = tokio::spawn(async move {
            let mut transport = NoiseTransport::accept(server_stream, &server_key)
                .await
                .expect("responder handshake succeeds");
            let message = transport
                .read_message()
                .await
                .expect("encrypted request is readable");
            transport
                .write_message(&message)
                .await
                .expect("encrypted response is writable");
        });
        let mut client = NoiseTransport::connect(client_stream, &pre_shared_key)
            .await
            .expect("initiator handshake succeeds");
        let request = WireMessage::Request {
            request_id: RequestId::new(),
            method: "system.ping".to_owned(),
            params: json!({ "sentAt": "2026-07-30T10:15:30Z" }),
            deadline: None,
            idempotency_key: None,
        };

        client
            .write_message(&request)
            .await
            .expect("encrypted request is writable");

        assert_eq!(
            client
                .read_message()
                .await
                .expect("encrypted response is readable"),
            request
        );
        server.await.expect("responder task completes");
    }
}
