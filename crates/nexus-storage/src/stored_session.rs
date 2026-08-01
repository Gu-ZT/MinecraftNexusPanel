use crate::StoredUser;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredSession {
    id: String,
    client_type: String,
    csrf_token_hash: Option<String>,
    user: StoredUser,
}

impl StoredSession {
    pub(crate) fn new(
        id: String,
        client_type: String,
        csrf_token_hash: Option<String>,
        user: StoredUser,
    ) -> Self {
        Self {
            id,
            client_type,
            csrf_token_hash,
            user,
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn client_type(&self) -> &str {
        &self.client_type
    }

    #[must_use]
    pub fn csrf_token_hash(&self) -> Option<&str> {
        self.csrf_token_hash.as_deref()
    }

    #[must_use]
    pub const fn user(&self) -> &StoredUser {
        &self.user
    }
}
