use std::str::FromStr;

/// MCNP 进程要启动的服务组合。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunMode {
    /// 只启动 Core 服务。
    Core,
    /// 只启动 Panel 服务。
    Panel,
    /// 同时启动 Core 和 Panel。
    All,
}

impl FromStr for RunMode {
    type Err = String;

    /// 从命令行模式名称解析运行模式。
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "core" => Ok(Self::Core),
            "panel" => Ok(Self::Panel),
            "all" => Ok(Self::All),
            _ => Err(format!(
                "unsupported mode '{value}'; expected core, panel, or all"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RunMode;

    #[test]
    fn parses_supported_modes() {
        assert_eq!("core".parse(), Ok(RunMode::Core));
        assert_eq!("panel".parse(), Ok(RunMode::Panel));
        assert_eq!("all".parse(), Ok(RunMode::All));
    }
}
