use sha2::Digest;
use sha2::Sha256;

const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// 计算 DER 编码证书的 SHA-256 指纹并返回小写十六进制文本。
///
/// 指纹用于向上层展示或记录实际收到的证书；本函数不执行证书信任判断，
/// 也不应被用来替代 TLS 验证策略。
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
