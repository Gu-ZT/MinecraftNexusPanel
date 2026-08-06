//! Desktop sidecar 日志文件和系统目录入口。
//!
//! sidecar 的环境变量包含 Panel 主密钥和 Core PSK，因此本模块只接收子进程输出流，绝不
//! 记录启动命令、环境变量或桌面秘密。输出由独立线程消费，避免日志管道反向阻塞 sidecar。

use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use tauri::AppHandle;
use tauri::Manager;
use tauri::Runtime;
use tauri::plugin::Plugin;
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_opener::init;

use serde_json::Value;

/// Desktop 应用数据目录下的 sidecar 日志目录名称。
pub const LOG_DIRECTORY_NAME: &str = "logs";

const LOG_FILE_NAME: &str = "mcnp-sidecar.log";
const ROTATED_LOG_FILE_NAME: &str = "mcnp-sidecar.log.1";
const MAX_LOG_FILE_BYTES: u64 = 10 * 1024 * 1024;
const REDACTED_VALUE: &str = "[REDACTED]";

/// 创建官方 opener 插件，使设置页可以调用系统文件管理器打开日志目录。
pub fn plugin<R: Runtime>() -> impl Plugin<R> {
    init()
}

/// 为一次 sidecar 启动准备日志目录和追加写入文件。
///
/// 单个日志文件达到上限时只保留一个旧文件，避免 Desktop 长期驻留导致日志无限增长。
pub fn prepare_sidecar_log(app_data_directory: &Path) -> io::Result<(PathBuf, File)> {
    let log_directory = app_data_directory.join(LOG_DIRECTORY_NAME);
    fs::create_dir_all(&log_directory)?;

    let log_path = log_directory.join(LOG_FILE_NAME);
    if fs::metadata(&log_path)
        .map(|metadata| metadata.len() >= MAX_LOG_FILE_BYTES)
        .unwrap_or(false)
    {
        let rotated_path = log_directory.join(ROTATED_LOG_FILE_NAME);
        if rotated_path.exists() {
            fs::remove_file(&rotated_path)?;
        }
        fs::rename(&log_path, rotated_path)?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    writeln!(file, "\n--- MCNP sidecar started ---")?;
    Ok((log_path, file))
}

/// 递归遮盖结构化日志中的秘密字段；非 JSON 文本按原样保留。
///
/// Desktop sidecar 显式请求逐行 JSON，因此正常运行日志会进入该分支。无法解析的启动器或
/// panic 文本继续依赖上游不输出秘密的日志契约，避免用字符串替换误删文件路径和错误上下文。
pub fn redact_sensitive_fields(line: &str) -> String {
    let Ok(mut value) = serde_json::from_str::<Value>(line) else {
        return line.to_owned();
    };
    redact_value(&mut value);
    serde_json::to_string(&value).unwrap_or_else(|_| line.to_owned())
}

fn redact_value(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            for (key, field) in fields {
                if sensitive_key(key) {
                    *field = Value::String(REDACTED_VALUE.to_owned());
                } else {
                    redact_value(field);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(redact_value),
        _ => {}
    }
}

fn sensitive_key(key: &str) -> bool {
    let normalized = key
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect::<String>();
    matches!(
        normalized.as_str(),
        "password"
            | "token"
            | "tokenhash"
            | "accesstoken"
            | "refreshtoken"
            | "psk"
            | "corepsk"
            | "presharedkey"
            | "secret"
            | "authorization"
            | "cookie"
            | "setcookie"
            | "masterkey"
            | "panelmasterkey"
    )
}

/// 打开 Desktop 日志目录，具体文件管理器由操作系统决定。
#[tauri::command]
pub fn open_desktop_log_directory<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join(LOG_DIRECTORY_NAME);
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    app.opener()
        .open_path(directory.to_string_lossy().to_string(), None::<String>)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;

    use serde_json::Value;

    use super::LOG_DIRECTORY_NAME;
    use super::MAX_LOG_FILE_BYTES;
    use super::REDACTED_VALUE;
    use super::prepare_sidecar_log;
    use super::redact_sensitive_fields;

    #[test]
    fn rotates_a_large_sidecar_log_before_appending() {
        let temporary_directory = tempfile::tempdir().expect("temporary directory should exist");
        let log_directory = temporary_directory.path().join(LOG_DIRECTORY_NAME);
        fs::create_dir_all(&log_directory).expect("log directory should exist");
        let log_path = log_directory.join("mcnp-sidecar.log");
        let mut existing_log = fs::File::create(&log_path).expect("log file should be created");
        existing_log
            .write_all(&vec![b'x'; MAX_LOG_FILE_BYTES as usize])
            .expect("large log should be written");

        let (prepared_path, _) =
            prepare_sidecar_log(temporary_directory.path()).expect("log should be prepared");

        assert!(prepared_path.exists());
        assert!(log_directory.join("mcnp-sidecar.log.1").exists());
        assert!(
            fs::metadata(prepared_path)
                .expect("prepared log should be readable")
                .len()
                < MAX_LOG_FILE_BYTES
        );
    }

    #[test]
    fn redacts_nested_sensitive_json_fields() {
        let line = r#"{"message":"request","accessToken":"access","context":{"password":"password","requestId":"safe"},"items":[{"core_psk":"psk"}]}"#;

        let redacted = redact_sensitive_fields(line);
        let value = serde_json::from_str::<Value>(&redacted).expect("redacted log should be JSON");

        assert_eq!(value["accessToken"], REDACTED_VALUE);
        assert_eq!(value["context"]["password"], REDACTED_VALUE);
        assert_eq!(value["items"][0]["core_psk"], REDACTED_VALUE);
        assert_eq!(value["context"]["requestId"], "safe");
    }

    #[test]
    fn preserves_non_json_diagnostic_text() {
        assert_eq!(
            redact_sensitive_fields("sidecar exited with code 1"),
            "sidecar exited with code 1"
        );
    }
}
