use std::io;
use std::path::PathBuf;

use thiserror::Error;

/// Panel SQLite 存储层错误。
#[derive(Debug, Error)]
pub enum StorageError {
    /// 无法创建数据目录。
    #[error("failed to create the Panel data directory {path}")]
    CreateDataDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    /// 数据库连接锁发生中毒，不能继续安全访问。
    #[error("Panel database lock is poisoned")]
    LockPoisoned,
    /// 数据库 schema 迁移失败。
    #[error("failed to migrate the Panel database")]
    Migrate(#[source] rusqlite::Error),
    /// 无法打开数据库文件。
    #[error("failed to open the Panel database {path}")]
    Open {
        path: PathBuf,
        #[source]
        source: rusqlite::Error,
    },
    /// SQL 查询、写入或事务提交失败。
    #[error("Panel database operation failed")]
    Query(#[source] rusqlite::Error),
}
