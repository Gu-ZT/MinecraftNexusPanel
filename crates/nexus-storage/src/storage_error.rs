use std::io;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("failed to create the Panel data directory {path}")]
    CreateDataDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("Panel database lock is poisoned")]
    LockPoisoned,
    #[error("failed to migrate the Panel database")]
    Migrate(#[source] rusqlite::Error),
    #[error("failed to open the Panel database {path}")]
    Open {
        path: PathBuf,
        #[source]
        source: rusqlite::Error,
    },
    #[error("Panel database operation failed")]
    Query(#[source] rusqlite::Error),
}
