#[derive(Debug)]
pub struct NewSession {
    pub id: String,
    pub user_id: String,
    pub client_type: String,
    pub access_token_hash: Option<String>,
    pub access_expires_at: Option<i64>,
    pub refresh_token_hash: String,
    pub refresh_expires_at: i64,
    pub csrf_token_hash: Option<String>,
    pub created_at: i64,
}
