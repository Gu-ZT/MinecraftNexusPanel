/// 待写入实例扩展安装记录。
///
/// `kind` 和 `path` 共同表达安装位置；混合端即使插件与模组物理目录相同，
/// 也必须保留两种独立的扩展种类记录。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewExtensionInstall {
    /// 安装记录标识。
    pub id: String,
    /// 所属 Core 标识。
    pub core_id: String,
    /// 所属实例标识。
    pub instance_id: String,
    /// 扩展种类，如 `PLUGIN` 或 `MOD`。
    pub kind: String,
    /// 相对于实例目录的安装路径。
    pub path: String,
    /// 安装文件 SHA-256 摘要。
    pub sha256: String,
    /// 安装来源分类。
    pub source: String,
    /// 可选的来源项目标识。
    pub project_id: Option<String>,
    /// 可选的来源版本标识。
    pub version: Option<String>,
    /// 安装发生时间。
    pub installed_at: String,
}
