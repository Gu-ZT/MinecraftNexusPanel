use std::str::FromStr;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunMode {
    Core,
    Panel,
    All,
}

impl FromStr for RunMode {
    type Err = String;

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
