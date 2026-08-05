use serde::Deserialize;
use serde::Serialize;

/// 描述一个显式接收实例命令的 MCDR 包装器。
///
/// `args` 必须包含且只能依赖两个精确占位符：`{server}` 会展开为实例可执行文件，
/// `{serverArgs}` 会展开为实例参数列表。Core 不猜测具体 MCDR 发行版的命令行，
/// 由模板或管理员明确提供包装器命令和占位符位置。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McdrConfig {
    executable: String,
    args: Vec<String>,
}

impl McdrConfig {
    /// 创建 MCDR 包装器配置。
    #[must_use]
    pub fn new(executable: String, args: Vec<String>) -> Self {
        Self { executable, args }
    }

    /// 返回 MCDR 包装器可执行文件。
    #[must_use]
    pub fn executable(&self) -> &str {
        &self.executable
    }

    /// 返回包装器参数模板。
    #[must_use]
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// 判断配置是否满足 Core 的边界和占位符约束。
    pub(crate) fn is_valid(&self) -> bool {
        !self.executable.trim().is_empty()
            && self.executable.len() <= 4096
            && !self.executable.contains('\0')
            && self.args.len() <= 256
            && self
                .args
                .iter()
                .all(|argument| argument.len() <= 8192 && !argument.contains('\0'))
            && self
                .args
                .iter()
                .filter(|argument| *argument == "{server}")
                .count()
                == 1
            && self
                .args
                .iter()
                .filter(|argument| *argument == "{serverArgs}")
                .count()
                == 1
    }

    /// 将实例命令展开为包装器命令。
    pub(crate) fn wrap_command(
        &self,
        server_executable: &str,
        server_args: &[String],
    ) -> Option<(String, Vec<String>)> {
        if !self.is_valid() {
            return None;
        }

        let mut args = Vec::new();
        for argument in &self.args {
            match argument.as_str() {
                "{server}" => args.push(server_executable.to_owned()),
                "{serverArgs}" => args.extend(server_args.iter().cloned()),
                _ => args.push(argument.clone()),
            }
        }

        Some((self.executable.clone(), args))
    }
}
