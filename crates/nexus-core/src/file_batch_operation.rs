use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::Value;

use crate::file_manager::MAXIMUM_FILE_WRITE_BYTES;

#[derive(Clone, Debug)]
pub(crate) enum FileBatchOperation {
    CreateDirectory {
        path: String,
        recursive: bool,
    },
    Move {
        from: String,
        to: String,
        overwrite: bool,
    },
    Write {
        path: String,
        content: Vec<u8>,
        expected_sha256: Option<String>,
    },
    Delete {
        path: String,
        recursive: bool,
    },
}

impl FileBatchOperation {
    pub(crate) fn from_value(value: &Value) -> Result<Self, ()> {
        let Some(kind) = value.get("kind").and_then(Value::as_str) else {
            return Err(());
        };

        match kind {
            "MKDIR" => Ok(Self::CreateDirectory {
                path: required_string(value, "path")?,
                recursive: optional_bool(value, "recursive")?,
            }),
            "MOVE" => Ok(Self::Move {
                from: required_string(value, "from")?,
                to: required_string(value, "to")?,
                overwrite: optional_bool(value, "overwrite")?,
            }),
            "WRITE" => {
                let data_base64 = required_string(value, "dataBase64")?;
                let content = STANDARD.decode(data_base64).map_err(|_| ())?;
                if content.len() > MAXIMUM_FILE_WRITE_BYTES {
                    return Err(());
                }
                Ok(Self::Write {
                    path: required_string(value, "path")?,
                    content,
                    expected_sha256: optional_string(value, "expectedSha256")?,
                })
            }
            "DELETE" => {
                if value.get("confirmation").and_then(Value::as_str) != Some("DELETE") {
                    return Err(());
                }
                Ok(Self::Delete {
                    path: required_string(value, "path")?,
                    recursive: optional_bool(value, "recursive")?,
                })
            }
            _ => Err(()),
        }
    }
}

fn required_string(value: &Value, name: &str) -> Result<String, ()> {
    value
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or(())
}

fn optional_string(value: &Value, name: &str) -> Result<Option<String>, ()> {
    match value.get(name) {
        None => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(()),
    }
}

fn optional_bool(value: &Value, name: &str) -> Result<bool, ()> {
    match value.get(name) {
        None => Ok(false),
        Some(Value::Bool(value)) => Ok(*value),
        Some(_) => Err(()),
    }
}
