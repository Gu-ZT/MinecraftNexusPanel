use crate::NewCore;

/// 从数据库读取的 Core 注册记录。
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
    /// 从待写入记录创建初始修订号为 1 的存储值。
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

    /// 返回 Core 标识。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 返回 Core 显示名称。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 返回 Core 地址。
    #[must_use]
    pub fn address(&self) -> &str {
        &self.address
    }

    /// 返回加密秘密信封字节。
    #[must_use]
    pub fn secret_envelope(&self) -> &[u8] {
        &self.secret_envelope
    }

    /// 返回秘密信封更新时间。
    #[must_use]
    pub fn secret_updated_at(&self) -> &str {
        &self.secret_updated_at
    }

    /// 返回连接超时时间，单位为秒。
    #[must_use]
    pub const fn connect_timeout_seconds(&self) -> u32 {
        self.connect_timeout_seconds
    }

    /// 返回是否跳过 TLS 证书校验。
    #[must_use]
    pub const fn skip_certificate_verification(&self) -> bool {
        self.skip_certificate_verification
    }

    /// 返回 JSON 编码的标签集合。
    #[must_use]
    pub fn tags_json(&self) -> &str {
        &self.tags_json
    }

    /// 返回数据库修订号。
    #[must_use]
    pub const fn revision(&self) -> u32 {
        self.revision
    }
}
