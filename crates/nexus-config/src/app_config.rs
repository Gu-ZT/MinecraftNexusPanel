use crate::RunMode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppConfig {
    mode: RunMode,
}

impl AppConfig {
    pub fn from_args(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mode = args
            .next()
            .map_or(Ok(RunMode::All), |value| value.parse())?;

        if let Some(argument) = args.next() {
            return Err(format!("unexpected argument: {argument}"));
        }

        Ok(Self { mode })
    }

    #[must_use]
    pub const fn mode(self) -> RunMode {
        self.mode
    }
}

#[cfg(test)]
mod tests {
    use super::AppConfig;
    use crate::RunMode;

    #[test]
    fn defaults_to_all_mode() {
        let config = AppConfig::from_args(std::iter::empty()).expect("default mode is valid");

        assert_eq!(config.mode(), RunMode::All);
    }

    #[test]
    fn rejects_extra_arguments() {
        let args = ["core".to_owned(), "unexpected".to_owned()].into_iter();

        assert!(AppConfig::from_args(args).is_err());
    }
}
