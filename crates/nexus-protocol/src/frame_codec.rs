use crate::Frame;
use crate::FrameError;
use crate::MAX_CIPHERTEXT_FRAME_BYTES;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameCodec {
    maximum_frame_bytes: usize,
}

impl Default for FrameCodec {
    fn default() -> Self {
        Self::new(MAX_CIPHERTEXT_FRAME_BYTES)
    }
}

impl FrameCodec {
    #[must_use]
    pub const fn new(maximum_frame_bytes: usize) -> Self {
        Self {
            maximum_frame_bytes,
        }
    }

    pub fn decode<'a>(&self, input: &'a [u8]) -> Result<Option<Frame<'a>>, FrameError> {
        if input.len() < u32::BITS as usize / 8 {
            return Ok(None);
        }

        let declared_length = u32::from_be_bytes([input[0], input[1], input[2], input[3]]) as usize;
        self.validate_payload_length(declared_length)?;

        let consumed = u32::BITS as usize / 8 + declared_length;
        if input.len() < consumed {
            return Ok(None);
        }

        Ok(Some(Frame::new(&input[4..consumed], consumed)))
    }

    pub fn decode_complete<'a>(&self, input: &'a [u8]) -> Result<Frame<'a>, FrameError> {
        let Some(frame) = self.decode(input)? else {
            let expected = input
                .get(..4)
                .and_then(|header| header.try_into().ok())
                .map(u32::from_be_bytes)
                .map_or(4, |length| 4 + length as usize);

            return Err(FrameError::Truncated {
                actual: input.len(),
                expected,
            });
        };

        if frame.consumed() != input.len() {
            return Err(FrameError::TrailingBytes {
                actual: input.len() - frame.consumed(),
            });
        }

        Ok(frame)
    }

    pub fn encode(&self, payload: &[u8]) -> Result<Vec<u8>, FrameError> {
        self.validate_payload_length(payload.len())?;

        let payload_length =
            u32::try_from(payload.len()).map_err(|_| FrameError::FrameTooLarge {
                actual: payload.len(),
                maximum: self.maximum_frame_bytes,
            })?;
        let mut frame = Vec::with_capacity(4 + payload.len());
        frame.extend_from_slice(&payload_length.to_be_bytes());
        frame.extend_from_slice(payload);

        Ok(frame)
    }

    pub fn validate_payload_length(&self, length: usize) -> Result<(), FrameError> {
        if length == 0 {
            return Err(FrameError::EmptyFrame);
        }

        if length > self.maximum_frame_bytes {
            return Err(FrameError::FrameTooLarge {
                actual: length,
                maximum: self.maximum_frame_bytes,
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::FrameCodec;
    use crate::FrameError;

    #[test]
    fn round_trips_a_frame() {
        let codec = FrameCodec::default();
        let encoded = codec.encode(b"payload").expect("frame is valid");

        let decoded = codec.decode_complete(&encoded).expect("frame is complete");

        assert_eq!(decoded.payload(), b"payload");
    }

    #[test]
    fn waits_for_a_complete_frame() {
        let codec = FrameCodec::default();
        let encoded = codec.encode(b"payload").expect("frame is valid");

        assert_eq!(codec.decode(&encoded[..5]).expect("header is valid"), None);
    }

    #[test]
    fn rejects_an_empty_frame() {
        let codec = FrameCodec::default();

        assert_eq!(
            codec.decode_complete(&[0, 0, 0, 0]),
            Err(FrameError::EmptyFrame)
        );
    }

    #[test]
    fn rejects_a_truncated_frame() {
        let codec = FrameCodec::default();

        assert_eq!(
            codec.decode_complete(&[0, 0, 0, 4, 1, 2]),
            Err(FrameError::Truncated {
                expected: 8,
                actual: 6,
            })
        );
    }

    #[test]
    fn rejects_a_frame_over_the_limit() {
        let codec = FrameCodec::new(4);

        assert_eq!(
            codec.encode(b"oversized"),
            Err(FrameError::FrameTooLarge {
                actual: 9,
                maximum: 4,
            })
        );
    }
}
