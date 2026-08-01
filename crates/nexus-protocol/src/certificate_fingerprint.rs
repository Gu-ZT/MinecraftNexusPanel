use sha2::Digest;
use sha2::Sha256;

const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

#[must_use]
pub fn certificate_sha256(certificate: &[u8]) -> String {
    let digest = Sha256::digest(certificate);
    let mut fingerprint = String::with_capacity(digest.len() * 2);
    for byte in digest {
        fingerprint.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
        fingerprint.push(char::from(HEX_DIGITS[usize::from(byte & 0x0f)]));
    }

    fingerprint
}
