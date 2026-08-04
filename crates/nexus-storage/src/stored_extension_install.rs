/// 从数据库读取的扩展安装记录。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredExtensionInstall {
    pub(crate) id: String,
    pub(crate) core_id: String,
    pub(crate) instance_id: String,
    pub(crate) kind: String,
    pub(crate) path: String,
    pub(crate) sha256: String,
    pub(crate) source: String,
    pub(crate) project_id: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) installed_at: String,
}

impl StoredExtensionInstall {
    /// 返回安装记录标识。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 返回所属 Core 标识。
    #[must_use]
    pub fn core_id(&self) -> &str {
        &self.core_id
    }

    /// 返回所属实例标识。
    #[must_use]
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    /// 返回扩展种类。
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// 返回相对于实例目录的安装路径。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// 返回安装文件 SHA-256 摘要。
    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// 返回安装来源分类。
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// 返回来源项目标识。
    #[must_use]
    pub fn project_id(&self) -> Option<&str> {
        self.project_id.as_deref()
    }

    /// 返回来源版本标识。
    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// 返回安装时间。
    #[must_use]
    pub fn installed_at(&self) -> &str {
        &self.installed_at
    }
}
