//! Panel 的 SQLite 持久化适配层。
//!
//! 本 crate 负责数据库文件、迁移、事务和存储映射；领域校验仍由上层完成。
//! 访问令牌只保存哈希，Core 预共享秘密保存为加密信封，查询结果不会还原明文秘密。

mod new_core;
mod new_extension_install;
mod new_session;
mod sqlite_store;
mod storage_error;
mod stored_core;
mod stored_extension_install;
mod stored_session;
mod stored_user;

pub use new_core::NewCore;
pub use new_extension_install::NewExtensionInstall;
pub use new_session::NewSession;
pub use sqlite_store::SqliteStore;
pub use storage_error::StorageError;
pub use stored_core::StoredCore;
pub use stored_extension_install::StoredExtensionInstall;
pub use stored_session::StoredSession;
pub use stored_user::StoredUser;

/// 默认 Panel SQLite 数据库文件名。
pub const DEFAULT_DATABASE_FILE_NAME: &str = "panel.db";
