use serde::Deserialize;

/// 注册远程 Core 的请求输入。
///
/// `secret` 只在建立连接和加密保存时使用，不应回显到 API 响应或日志；其余字段
/// 会在注册前执行长度、地址、超时和标签校验。
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
    /// 返回去除首尾空白的 Core 名称。
    #[must_use]
    pub fn name(&self) -> &str {
        self.name.trim()
    }

    /// 返回去除首尾空白的 Core 地址。
    #[must_use]
    pub fn address(&self) -> &str {
        self.address.trim()
    }

    /// 返回预共享密钥文本；调用方不得记录该值。
    #[must_use]
    pub fn secret(&self) -> &str {
        &self.secret
    }

    /// 返回连接超时秒数。
    #[must_use]
    pub const fn connect_timeout_seconds(&self) -> u32 {
        self.connect_timeout_seconds
    }

    /// 返回是否跳过系统证书校验。
    #[must_use]
    pub const fn skip_certificate_verification(&self) -> bool {
        self.skip_certificate_verification
    }

    /// 返回第一个无效字段名。
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

    /// 返回排序、去空白并去重后的标签。
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
