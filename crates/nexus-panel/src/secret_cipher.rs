use aes_gcm::Aes256Gcm;
use aes_gcm::Nonce;
use aes_gcm::aead::Aead;
use aes_gcm::aead::KeyInit;
use aes_gcm::aead::Payload;
use getrandom::fill;
use nexus_config::PanelMasterKey;
use nexus_domain::CoreId;

use crate::SecretCipherError;

const ENVELOPE_VERSION: u8 = 1;
const NONCE_BYTES: usize = 12;
const TAG_BYTES: usize = 16;

#[derive(Clone)]
pub struct SecretCipher {
    cipher: Aes256Gcm,
}

impl SecretCipher {
    #[must_use]
    pub fn new(master_key: &PanelMasterKey) -> Self {
        Self {
            cipher: Aes256Gcm::new(master_key.as_bytes().into()),
        }
    }

    pub fn encrypt(&self, core_id: CoreId, plaintext: &[u8]) -> Result<Vec<u8>, SecretCipherError> {
        let mut nonce_bytes = [0_u8; NONCE_BYTES];
        fill(&mut nonce_bytes)?;
        let ciphertext = self
            .cipher
            .encrypt(
                Nonce::from_slice(&nonce_bytes),
                Payload {
                    msg: plaintext,
                    aad: &associated_data(core_id),
                },
            )
            .map_err(|_| SecretCipherError::Authentication)?;
        let mut envelope = Vec::with_capacity(1 + NONCE_BYTES + ciphertext.len());
        envelope.push(ENVELOPE_VERSION);
        envelope.extend_from_slice(&nonce_bytes);
        envelope.extend_from_slice(&ciphertext);

        Ok(envelope)
    }

    pub fn decrypt(&self, core_id: CoreId, envelope: &[u8]) -> Result<Vec<u8>, SecretCipherError> {
        if envelope.len() < 1 + NONCE_BYTES + TAG_BYTES || envelope[0] != ENVELOPE_VERSION {
            return Err(SecretCipherError::InvalidEnvelope);
        }
        let nonce = Nonce::from_slice(&envelope[1..1 + NONCE_BYTES]);

        self.cipher
            .decrypt(
                nonce,
                Payload {
                    msg: &envelope[1 + NONCE_BYTES..],
                    aad: &associated_data(core_id),
                },
            )
            .map_err(|_| SecretCipherError::Authentication)
    }
}

fn associated_data(core_id: CoreId) -> Vec<u8> {
    format!("mcnp-core-secret-v1:{core_id}").into_bytes()
}

#[cfg(test)]
mod tests {
    use nexus_config::PanelMasterKey;
    use nexus_domain::CoreId;

    use super::SecretCipher;

    #[test]
    fn encrypts_with_random_nonces_and_binds_the_core_id() {
        let cipher = SecretCipher::new(&PanelMasterKey::from_bytes([7_u8; 32]));
        let other_cipher = SecretCipher::new(&PanelMasterKey::from_bytes([8_u8; 32]));
        let core_id = CoreId::new();
        let other_core_id = CoreId::new();
        let first = cipher
            .encrypt(core_id, b"core-secret")
            .expect("secret encrypts");
        let second = cipher
            .encrypt(core_id, b"core-secret")
            .expect("secret encrypts with another nonce");

        assert_ne!(first, second);
        assert_eq!(
            cipher.decrypt(core_id, &first).expect("secret decrypts"),
            b"core-secret"
        );
        assert!(cipher.decrypt(other_core_id, &first).is_err());
        assert!(other_cipher.decrypt(core_id, &first).is_err());
    }
}
