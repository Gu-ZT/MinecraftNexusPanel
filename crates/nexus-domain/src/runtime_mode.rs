use serde::Deserialize;
use serde::Serialize;

/// 描述实例进程运行在宿主机还是容器中。
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuntimeMode {
    /// 由 Core 直接在受管宿主机目录中启动进程。
    #[default]
    Host,
    /// 由容器运行时启动进程；当前 Core 只保存配置，尚未执行容器启动。
    Container,
}
