/// 待写入 Core 注册记录。
///
/// `secret_envelope` 必须已经由 Panel 加密，存储层不会替调用方加密或解密。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewCore {
    /// Core 稳定标识。
    pub id: String,
    /// Core 显示名称。
    pub name: String,
    /// Core TLS 或网络地址。
    pub address: String,
    /// 加密后的 Core 预共享秘密信封。
    pub secret_envelope: Vec<u8>,
    /// 秘密信封最后更新时间。
    pub secret_updated_at: String,
    /// 建立 Core 连接的超时时间，单位为秒。
    pub connect_timeout_seconds: u32,
    /// 是否跳过 TLS 证书信任校验。
    pub skip_certificate_verification: bool,
    /// JSON 编码的标签集合。
    pub tags_json: String,
    /// 注册记录创建时间。
    pub created_at: String,
}
