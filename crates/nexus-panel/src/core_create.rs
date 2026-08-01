use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreCreate {
    name: String,
    address: String,
    secret: String,
    #[serde(default = "default_connect_timeout_seconds")]
    connect_timeout_seconds: u32,
    #[serde(default)]
    skip_certificate_verification: bool,
    #[serde(default)]
    tags: Vec<String>,
}

impl CoreCreate {
    #[must_use]
    pub fn name(&self) -> &str {
        self.name.trim()
    }

    #[must_use]
    pub fn address(&self) -> &str {
        self.address.trim()
    }

    #[must_use]
    pub fn secret(&self) -> &str {
        &self.secret
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
    pub fn invalid_field(&self) -> Option<&'static str> {
        if self.name().is_empty() || self.name().chars().count() > 128 || self.name.contains('\0') {
            return Some("name");
        }
        if self.address().is_empty() || self.address.contains('\0') {
            return Some("address");
        }
        if !(1..=60).contains(&self.connect_timeout_seconds) {
            return Some("connectTimeoutSeconds");
        }
        if self.tags.len() > 32
            || self.tags.iter().any(|tag| {
                let tag = tag.trim();
                tag.is_empty() || tag.chars().count() > 64 || tag.contains('\0')
            })
        {
            return Some("tags");
        }

        None
    }

    #[must_use]
    pub fn normalized_tags(&self) -> Vec<String> {
        let mut tags: Vec<_> = self.tags.iter().map(|tag| tag.trim().to_owned()).collect();
        tags.sort();
        tags.dedup();
        tags
    }
}

const fn default_connect_timeout_seconds() -> u32 {
    10
}
