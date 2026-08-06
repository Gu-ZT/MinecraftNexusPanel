//! Desktop 原生会话凭据的安全存储。
//!
//! Windows 安装包使用 Windows Credential Manager，macOS 使用系统 Keychain 保存原生
//! refresh token；访问令牌只在 WebView 会话内存和 `sessionStorage` 中短期存在。尚未发布
//! 的其他 Desktop 构建不使用 keyring 的 mock 后端，避免把“看似成功”的令牌写入不安全的
//! 非持久存储。

#[cfg(any(windows, target_os = "macos"))]
use keyring::Entry;
#[cfg(any(windows, target_os = "macos"))]
use keyring::Error as KeyringError;

#[cfg(any(windows, target_os = "macos"))]
const KEYRING_SERVICE: &str = "dev.mcnp.desktop";
#[cfg(any(windows, target_os = "macos"))]
const KEYRING_USER: &str = "native-refresh-token";
#[cfg(not(any(windows, target_os = "macos")))]
const UNSUPPORTED_MESSAGE: &str =
    "secure Desktop credential storage is unavailable on this platform";

/// 读取当前用户保存的原生 refresh token；没有令牌时返回 `None`。
#[tauri::command]
pub fn get_desktop_refresh_token() -> Result<Option<String>, String> {
    read_refresh_token()
}

/// 将原生 refresh token 写入当前用户的操作系统凭据存储。
#[tauri::command]
pub fn set_desktop_refresh_token(refresh_token: String) -> Result<(), String> {
    if refresh_token.is_empty() {
        return Err("refresh token cannot be empty".to_owned());
    }
    write_refresh_token(&refresh_token)
}

/// 删除当前用户保存的原生 refresh token。
#[tauri::command]
pub fn clear_desktop_refresh_token() -> Result<(), String> {
    delete_refresh_token()
}

#[cfg(any(windows, target_os = "macos"))]
fn credential_entry() -> Result<Entry, String> {
    Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|error| error.to_string())
}

#[cfg(any(windows, target_os = "macos"))]
fn read_refresh_token() -> Result<Option<String>, String> {
    match credential_entry()?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
fn read_refresh_token() -> Result<Option<String>, String> {
    Err(UNSUPPORTED_MESSAGE.to_owned())
}

#[cfg(any(windows, target_os = "macos"))]
fn write_refresh_token(refresh_token: &str) -> Result<(), String> {
    credential_entry()?
        .set_password(refresh_token)
        .map_err(|error| error.to_string())
}

#[cfg(not(any(windows, target_os = "macos")))]
fn write_refresh_token(_refresh_token: &str) -> Result<(), String> {
    Err(UNSUPPORTED_MESSAGE.to_owned())
}

#[cfg(any(windows, target_os = "macos"))]
fn delete_refresh_token() -> Result<(), String> {
    match credential_entry()?.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
fn delete_refresh_token() -> Result<(), String> {
    Err(UNSUPPORTED_MESSAGE.to_owned())
}

#[cfg(test)]
mod tests {
    use super::set_desktop_refresh_token;

    #[test]
    fn rejects_an_empty_refresh_token_before_accessing_the_keyring() {
        assert_eq!(
            set_desktop_refresh_token(String::new()),
            Err("refresh token cannot be empty".to_owned())
        );
    }
}
