use crate::NewCore;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredCore {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) address: String,
    pub(crate) secret_envelope: Vec<u8>,
    pub(crate) secret_updated_at: String,
    pub(crate) connect_timeout_seconds: u32,
    pub(crate) skip_certificate_verification: bool,
    pub(crate) tags_json: String,
    pub(crate) revision: u32,
}

impl StoredCore {
    #[must_use]
    pub fn from_new(core: &NewCore) -> Self {
        Self {
            id: core.id.clone(),
            name: core.name.clone(),
            address: core.address.clone(),
            secret_envelope: core.secret_envelope.clone(),
            secret_updated_at: core.secret_updated_at.clone(),
            connect_timeout_seconds: core.connect_timeout_seconds,
            skip_certificate_verification: core.skip_certificate_verification,
            tags_json: core.tags_json.clone(),
            revision: 1,
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn address(&self) -> &str {
        &self.address
    }

    #[must_use]
    pub fn secret_envelope(&self) -> &[u8] {
        &self.secret_envelope
    }

    #[must_use]
    pub fn secret_updated_at(&self) -> &str {
        &self.secret_updated_at
    }

    #[must_use]
    pub const fn connect_timeout_seconds(&self) -> u32 {
        self.connect_timeout_seconds
    }

    #[must_use]
    pub const fn skip_certificate_verification(&self) -> bool {
        self.skip_certificate_verification
    }

    #[must_use]
    pub fn tags_json(&self) -> &str {
        &self.tags_json
    }

    #[must_use]
    pub const fn revision(&self) -> u32 {
        self.revision
    }
}
