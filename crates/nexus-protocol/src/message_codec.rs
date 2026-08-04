use crate::MAX_PLAINTEXT_JSON_BYTES;
use crate::MessageError;
use crate::WireMessage;

/// 将明文 JSON 负载反序列化为线协议消息。
///
/// 函数只负责 JSON 语法和消息外形解析；调用方仍需在业务层校验方法名、参数
/// 和权限，并且输入长度会先经过协议上限检查。
pub fn deserialize_message(payload: &[u8]) -> Result<WireMessage, MessageError> {
    validate_message_length(payload.len())?;

    serde_json::from_slice(payload).map_err(MessageError::InvalidJson)
}

/// 将线协议消息序列化为受长度限制的 JSON 负载。
pub fn serialize_message(message: &WireMessage) -> Result<Vec<u8>, MessageError> {
    let payload = serde_json::to_vec(message).map_err(MessageError::InvalidJson)?;
    validate_message_length(payload.len())?;

    Ok(payload)
}

fn validate_message_length(length: usize) -> Result<(), MessageError> {
    if length > MAX_PLAINTEXT_JSON_BYTES {
        return Err(MessageError::MessageTooLarge {
            actual: length,
            maximum: MAX_PLAINTEXT_JSON_BYTES,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use nexus_domain::RequestId;
    use serde_json::json;

    use super::deserialize_message;
    use super::serialize_message;
    use crate::FrameCodec;
    use crate::WireMessage;

    #[test]
    fn round_trips_a_request_through_json_and_frame_encoding() {
        let message = WireMessage::Request {
            request_id: RequestId::new(),
            method: "system.ping".to_owned(),
            params: json!({ "sentAt": "2026-07-30T10:15:30Z" }),
            deadline: None,
            idempotency_key: None,
        };
        let codec = FrameCodec::default();

        let message_payload = serialize_message(&message).expect("message serializes");
        let encoded_frame = codec.encode(&message_payload).expect("frame encodes");
        let decoded_frame = codec
            .decode_complete(&encoded_frame)
            .expect("frame decodes");
        let decoded_message =
            deserialize_message(decoded_frame.payload()).expect("message decodes");

        assert_eq!(decoded_message, message);
    }
}
