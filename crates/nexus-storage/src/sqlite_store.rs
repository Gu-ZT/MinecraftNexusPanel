use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use rusqlite::Connection;
use rusqlite::TransactionBehavior;

use crate::StorageError;

const DATABASE_FILE_NAME: &str = "panel.db";

#[derive(Clone)]
pub struct SqliteStore {
    connection: Arc<Mutex<Connection>>,
    database_path: PathBuf,
}

impl SqliteStore {
    pub fn open(data_directory: &Path) -> Result<Self, StorageError> {
        fs::create_dir_all(data_directory).map_err(|source| StorageError::CreateDataDirectory {
            path: data_directory.to_path_buf(),
            source,
        })?;

        let database_path = data_directory.join(DATABASE_FILE_NAME);
        let mut connection =
            Connection::open(&database_path).map_err(|source| StorageError::Open {
                path: database_path.clone(),
                source,
            })?;

        migrate(&mut connection)?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            database_path,
        })
    }

    pub fn create_initial_user(
        &self,
        user_id: &str,
        username: &str,
        password_hash: &str,
        created_at: &str,
    ) -> Result<bool, StorageError> {
        let mut connection = self.lock_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(StorageError::Query)?;
        let user_exists: bool = transaction
            .query_row("SELECT EXISTS(SELECT 1 FROM users)", [], |row| row.get(0))
            .map_err(StorageError::Query)?;

        if user_exists {
            transaction.commit().map_err(StorageError::Query)?;
            return Ok(false);
        }

        transaction
            .execute(
                "INSERT INTO users (id, username, password_hash, is_admin, created_at) VALUES (?1, ?2, ?3, 1, ?4)",
                (user_id, username, password_hash, created_at),
            )
            .map_err(StorageError::Query)?;
        transaction.commit().map_err(StorageError::Query)?;

        Ok(true)
    }

    #[must_use]
    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    pub fn has_users(&self) -> Result<bool, StorageError> {
        let connection = self.lock_connection()?;

        connection
            .query_row("SELECT EXISTS(SELECT 1 FROM users)", [], |row| row.get(0))
            .map_err(StorageError::Query)
    }

    fn lock_connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>, StorageError> {
        self.connection
            .lock()
            .map_err(|_| StorageError::LockPoisoned)
    }
}

fn migrate(connection: &mut Connection) -> Result<(), StorageError> {
    connection
        .execute_batch(
            "
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;

            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY NOT NULL,
                username TEXT NOT NULL COLLATE NOCASE UNIQUE,
                password_hash TEXT NOT NULL,
                is_admin INTEGER NOT NULL CHECK (is_admin IN (0, 1)),
                created_at TEXT NOT NULL
            );
            ",
        )
        .map_err(StorageError::Migrate)
}

#[cfg(test)]
mod tests {
    use super::SqliteStore;
    use tempfile::tempdir;

    #[test]
    fn creates_only_one_initial_user() {
        let data_directory = tempdir().expect("temporary Panel data directory is created");
        let store = SqliteStore::open(data_directory.path()).expect("Panel database opens");

        assert!(
            store
                .create_initial_user(
                    "user-1",
                    "administrator",
                    "password-hash",
                    "2026-07-30T10:15:31Z",
                )
                .expect("first user initializes")
        );
        assert!(store.has_users().expect("user existence is queried"));
        assert!(
            !store
                .create_initial_user(
                    "user-2",
                    "another-user",
                    "password-hash",
                    "2026-07-30T10:15:31Z",
                )
                .expect("second initialization is rejected")
        );
        assert!(store.database_path().exists());
    }
}
