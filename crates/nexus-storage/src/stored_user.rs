#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredUser {
    id: String,
    username: String,
    display_name: String,
    password_hash: String,
    is_admin: bool,
}

impl StoredUser {
    pub(crate) fn new(
        id: String,
        username: String,
        display_name: String,
        password_hash: String,
        is_admin: bool,
    ) -> Self {
        Self {
            id,
            username,
            display_name,
            password_hash,
            is_admin,
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn username(&self) -> &str {
        &self.username
    }

    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    #[must_use]
    pub fn password_hash(&self) -> &str {
        &self.password_hash
    }

    #[must_use]
    pub const fn is_admin(&self) -> bool {
        self.is_admin
    }
}
