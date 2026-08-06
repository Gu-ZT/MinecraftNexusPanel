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

/// Desktop 应用数据目录下的 sidecar 日志目录名称。
pub const LOG_DIRECTORY_NAME: &str = "logs";

const LOG_FILE_NAME: &str = "mcnp-sidecar.log";
const ROTATED_LOG_FILE_NAME: &str = "mcnp-sidecar.log.1";
const MAX_LOG_FILE_BYTES: u64 = 10 * 1024 * 1024;

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

    use super::LOG_DIRECTORY_NAME;
    use super::MAX_LOG_FILE_BYTES;
    use super::prepare_sidecar_log;

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
}
