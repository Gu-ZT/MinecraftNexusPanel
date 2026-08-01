#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewCore {
    pub id: String,
    pub name: String,
    pub address: String,
    pub secret_envelope: Vec<u8>,
    pub secret_updated_at: String,
    pub connect_timeout_seconds: u32,
    pub skip_certificate_verification: bool,
    pub tags_json: String,
    pub created_at: String,
}
