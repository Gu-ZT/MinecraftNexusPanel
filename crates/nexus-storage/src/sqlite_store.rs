use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;

use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::Result as SqliteResult;
use rusqlite::Row;
use rusqlite::TransactionBehavior;

use crate::NewCore;
use crate::NewExtensionInstall;
use crate::NewSession;
use crate::StorageError;
use crate::StoredCore;
use crate::StoredExtensionInstall;
use crate::StoredSession;
use crate::StoredUser;

const DATABASE_FILE_NAME: &str = "panel.db";

/// 线程安全共享的 Panel SQLite 数据库访问器。
///
/// 打开数据库时会创建数据目录并执行迁移；连接使用互斥锁串行化 rusqlite
/// 操作，写入方法在需要时使用立即事务保证初始化、轮换等操作的原子性。
#[derive(Clone)]
pub struct SqliteStore {
    connection: Arc<Mutex<Connection>>,
    database_path: PathBuf,
}

impl SqliteStore {
    /// 打开数据目录中的 Panel 数据库并执行 schema 迁移。
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

    /// 在数据库为空时原子创建首个管理员用户。
    ///
    /// 返回 `true` 表示插入成功；已有任意用户时返回 `false`，不会覆盖现有数据。
    pub fn create_initial_user(
        &self,
        user_id: &str,
        username: &str,
        display_name: &str,
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
                "INSERT INTO users (id, username, display_name, password_hash, is_admin, created_at) VALUES (?1, ?2, ?3, ?4, 1, ?5)",
                (user_id, username, display_name, password_hash, created_at),
            )
            .map_err(StorageError::Query)?;
        transaction.commit().map_err(StorageError::Query)?;

        Ok(true)
    }

    /// 返回持久化的 Panel 标识，不存在时使用候选值创建。
    pub fn get_or_create_panel_id(&self, candidate: &str) -> Result<String, StorageError> {
        let mut connection = self.lock_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(StorageError::Query)?;
        transaction
            .execute(
                "INSERT OR IGNORE INTO panel_metadata (key, value) VALUES ('panel_id', ?1)",
                [candidate],
            )
            .map_err(StorageError::Query)?;
        let panel_id = transaction
            .query_row(
                "SELECT value FROM panel_metadata WHERE key = 'panel_id'",
                [],
                |row| row.get(0),
            )
            .map_err(StorageError::Query)?;
        transaction.commit().map_err(StorageError::Query)?;

        Ok(panel_id)
    }

    /// 插入 Core 注册记录；相同标识已存在时返回 `false`。
    pub fn insert_core(&self, core: &NewCore) -> Result<bool, StorageError> {
        let connection = self.lock_connection()?;
        let inserted = connection
            .execute(
                "INSERT OR IGNORE INTO cores (
                    id, name, address, secret_envelope, secret_updated_at,
                    connect_timeout_seconds, skip_certificate_verification, tags_json,
                    revision, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, ?9)",
                (
                    core.id.as_str(),
                    core.name.as_str(),
                    core.address.as_str(),
                    core.secret_envelope.as_slice(),
                    core.secret_updated_at.as_str(),
                    core.connect_timeout_seconds,
                    core.skip_certificate_verification,
                    core.tags_json.as_str(),
                    core.created_at.as_str(),
                ),
            )
            .map_err(StorageError::Query)?;

        Ok(inserted == 1)
    }

    /// 按稳定标识顺序列出所有 Core 注册记录。
    pub fn list_cores(&self) -> Result<Vec<StoredCore>, StorageError> {
        let connection = self.lock_connection()?;
        let mut statement = connection
            .prepare(
                "SELECT id, name, address, secret_envelope, secret_updated_at,
                        connect_timeout_seconds, skip_certificate_verification, tags_json, revision
                 FROM cores ORDER BY id",
            )
            .map_err(StorageError::Query)?;
        let rows = statement
            .query_map([], map_core)
            .map_err(StorageError::Query)?;

        rows.collect::<SqliteResult<Vec<_>>>()
            .map_err(StorageError::Query)
    }

    /// 写入或更新一个实例扩展安装记录，并返回数据库中的最终值。
    ///
    /// 唯一键是 `(core_id, instance_id, path)`；调用方应通过 `kind` 保留插件和
    /// 模组的领域区分，而不是因为路径相同就合并记录。
    pub fn upsert_extension_install(
        &self,
        install: &NewExtensionInstall,
    ) -> Result<StoredExtensionInstall, StorageError> {
        let connection = self.lock_connection()?;
        connection
            .execute(
                "INSERT INTO extension_installs (
                    id, core_id, instance_id, kind, path, sha256, source,
                    project_id, version, installed_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(core_id, instance_id, path) DO UPDATE SET
                    kind = excluded.kind,
                    sha256 = excluded.sha256,
                    source = excluded.source,
                    project_id = excluded.project_id,
                    version = excluded.version,
                    installed_at = excluded.installed_at",
                (
                    install.id.as_str(),
                    install.core_id.as_str(),
                    install.instance_id.as_str(),
                    install.kind.as_str(),
                    install.path.as_str(),
                    install.sha256.as_str(),
                    install.source.as_str(),
                    install.project_id.as_deref(),
                    install.version.as_deref(),
                    install.installed_at.as_str(),
                ),
            )
            .map_err(StorageError::Query)?;

        connection
            .query_row(
                "SELECT id, core_id, instance_id, kind, path, sha256, source,
                        project_id, version, installed_at
                 FROM extension_installs
                 WHERE core_id = ?1 AND instance_id = ?2 AND path = ?3",
                (&install.core_id, &install.instance_id, &install.path),
                map_extension_install,
            )
            .map_err(StorageError::Query)
    }

    /// 按 Core、实例和扩展种类列出安装记录。
    pub fn list_extension_installs(
        &self,
        core_id: &str,
        instance_id: &str,
        kind: &str,
    ) -> Result<Vec<StoredExtensionInstall>, StorageError> {
        let connection = self.lock_connection()?;
        let mut statement = connection
            .prepare(
                "SELECT id, core_id, instance_id, kind, path, sha256, source,
                        project_id, version, installed_at
                 FROM extension_installs
                 WHERE core_id = ?1 AND instance_id = ?2 AND kind = ?3
                 ORDER BY path",
            )
            .map_err(StorageError::Query)?;
        let rows = statement
            .query_map((core_id, instance_id, kind), map_extension_install)
            .map_err(StorageError::Query)?;

        rows.collect::<SqliteResult<Vec<_>>>()
            .map_err(StorageError::Query)
    }

    /// 删除指定安装路径的扩展记录，返回是否删除了一行。
    pub fn delete_extension_install(
        &self,
        core_id: &str,
        instance_id: &str,
        path: &str,
    ) -> Result<bool, StorageError> {
        let connection = self.lock_connection()?;
        let deleted = connection
            .execute(
                "DELETE FROM extension_installs
                 WHERE core_id = ?1 AND instance_id = ?2 AND path = ?3",
                (core_id, instance_id, path),
            )
            .map_err(StorageError::Query)?;

        Ok(deleted == 1)
    }

    /// 创建一条登录会话记录。
    pub fn create_session(&self, session: &NewSession) -> Result<(), StorageError> {
        let connection = self.lock_connection()?;
        connection
            .execute(
                "INSERT INTO sessions (
                    id, user_id, client_type, access_token_hash, access_expires_at,
                    refresh_token_hash, refresh_expires_at, csrf_token_hash, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
                (
                    session.id.as_str(),
                    session.user_id.as_str(),
                    session.client_type.as_str(),
                    session.access_token_hash.as_deref(),
                    session.access_expires_at,
                    session.refresh_token_hash.as_str(),
                    session.refresh_expires_at,
                    session.csrf_token_hash.as_deref(),
                    session.created_at,
                ),
            )
            .map_err(StorageError::Query)?;

        Ok(())
    }

    /// 按用户名查找用户，用户名比较遵循数据库的大小写不敏感约束。
    pub fn find_user_by_username(
        &self,
        username: &str,
    ) -> Result<Option<StoredUser>, StorageError> {
        let connection = self.lock_connection()?;
        connection
            .query_row(
                "SELECT id, username, display_name, password_hash, is_admin
                 FROM users WHERE username = ?1",
                [username],
                map_user,
            )
            .optional()
            .map_err(StorageError::Query)
    }

    /// 查找未撤销且访问令牌未过期的会话。
    pub fn find_session_by_access_token(
        &self,
        token_hash: &str,
        now: i64,
    ) -> Result<Option<StoredSession>, StorageError> {
        self.find_session(
            "s.access_token_hash = ?1 AND s.access_expires_at > ?2",
            token_hash,
            now,
        )
    }

    /// 查找未撤销且刷新令牌未过期的会话。
    pub fn find_session_by_refresh_token(
        &self,
        token_hash: &str,
        now: i64,
    ) -> Result<Option<StoredSession>, StorageError> {
        self.find_session(
            "s.refresh_token_hash = ?1 AND s.refresh_expires_at > ?2",
            token_hash,
            now,
        )
    }

    /// 原子轮换刷新令牌并登记旧令牌，以支持重放撤销。
    ///
    /// 返回 `true` 表示旧令牌仍有效且会话已更新；并发或重复轮换返回 `false`。
    pub fn rotate_session(
        &self,
        session_id: &str,
        old_refresh_token_hash: &str,
        replacement: &NewSession,
    ) -> Result<bool, StorageError> {
        let mut connection = self.lock_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(StorageError::Query)?;
        transaction
            .execute(
                "DELETE FROM rotated_session_tokens WHERE expires_at <= ?1",
                [replacement.created_at],
            )
            .map_err(StorageError::Query)?;
        transaction
            .execute(
                "INSERT OR IGNORE INTO rotated_session_tokens (token_hash, session_id, expires_at)
                 SELECT refresh_token_hash, id, refresh_expires_at
                 FROM sessions
                 WHERE id = ?1 AND refresh_token_hash = ?2 AND revoked_at IS NULL",
                (session_id, old_refresh_token_hash),
            )
            .map_err(StorageError::Query)?;
        let updated = transaction
            .execute(
                "UPDATE sessions
                 SET access_token_hash = ?1,
                     access_expires_at = ?2,
                     refresh_token_hash = ?3,
                     refresh_expires_at = ?4,
                     csrf_token_hash = ?5,
                     updated_at = ?6
                 WHERE id = ?7 AND refresh_token_hash = ?8 AND revoked_at IS NULL",
                (
                    replacement.access_token_hash.as_deref(),
                    replacement.access_expires_at,
                    replacement.refresh_token_hash.as_str(),
                    replacement.refresh_expires_at,
                    replacement.csrf_token_hash.as_deref(),
                    replacement.created_at,
                    session_id,
                    old_refresh_token_hash,
                ),
            )
            .map_err(StorageError::Query)?;
        transaction.commit().map_err(StorageError::Query)?;

        Ok(updated == 1)
    }

    /// 撤销指定会话，返回是否实际更新了一行。
    pub fn revoke_session(&self, session_id: &str, now: i64) -> Result<bool, StorageError> {
        let connection = self.lock_connection()?;
        let updated = connection
            .execute(
                "UPDATE sessions SET revoked_at = ?1, updated_at = ?1
                 WHERE id = ?2 AND revoked_at IS NULL",
                (now, session_id),
            )
            .map_err(StorageError::Query)?;

        Ok(updated == 1)
    }

    /// 根据已轮换的旧刷新令牌撤销其关联会话。
    pub fn revoke_session_for_rotated_token(
        &self,
        token_hash: &str,
        now: i64,
    ) -> Result<bool, StorageError> {
        let connection = self.lock_connection()?;
        let updated = connection
            .execute(
                "UPDATE sessions SET revoked_at = ?1, updated_at = ?1
                 WHERE id = (
                    SELECT session_id FROM rotated_session_tokens
                    WHERE token_hash = ?2 AND expires_at > ?1
                 ) AND revoked_at IS NULL",
                (now, token_hash),
            )
            .map_err(StorageError::Query)?;

        Ok(updated == 1)
    }

    /// 返回当前数据库文件路径。
    #[must_use]
    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    /// 判断数据库中是否已经存在用户。
    pub fn has_users(&self) -> Result<bool, StorageError> {
        let connection = self.lock_connection()?;

        connection
            .query_row("SELECT EXISTS(SELECT 1 FROM users)", [], |row| row.get(0))
            .map_err(StorageError::Query)
    }

    fn find_session(
        &self,
        token_predicate: &str,
        token_hash: &str,
        now: i64,
    ) -> Result<Option<StoredSession>, StorageError> {
        let connection = self.lock_connection()?;
        let query = format!(
            "SELECT s.id, s.client_type, s.csrf_token_hash,
                    u.id, u.username, u.display_name, u.password_hash, u.is_admin
             FROM sessions s
             INNER JOIN users u ON u.id = s.user_id
             WHERE {token_predicate} AND s.revoked_at IS NULL"
        );

        connection
            .query_row(&query, (token_hash, now), map_session)
            .optional()
            .map_err(StorageError::Query)
    }

    fn lock_connection(&self) -> Result<MutexGuard<'_, Connection>, StorageError> {
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
                display_name TEXT NOT NULL,
                password_hash TEXT NOT NULL,
                is_admin INTEGER NOT NULL CHECK (is_admin IN (0, 1)),
                created_at TEXT NOT NULL
            );
            ",
        )
        .map_err(StorageError::Migrate)?;

    if !column_exists(connection, "users", "display_name")? {
        connection
            .execute(
                "ALTER TABLE users ADD COLUMN display_name TEXT NOT NULL DEFAULT 'Administrator'",
                [],
            )
            .map_err(StorageError::Migrate)?;
    }

    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY NOT NULL,
                user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                client_type TEXT NOT NULL CHECK (client_type IN ('BROWSER', 'NATIVE')),
                access_token_hash TEXT UNIQUE,
                access_expires_at INTEGER,
                refresh_token_hash TEXT NOT NULL UNIQUE,
                refresh_expires_at INTEGER NOT NULL,
                csrf_token_hash TEXT,
                revoked_at INTEGER,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS rotated_session_tokens (
                token_hash TEXT PRIMARY KEY NOT NULL,
                session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                expires_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS sessions_user_id_idx ON sessions(user_id);
            CREATE INDEX IF NOT EXISTS rotated_session_tokens_session_id_idx
                ON rotated_session_tokens(session_id);

            CREATE TABLE IF NOT EXISTS panel_metadata (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS cores (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                address TEXT NOT NULL,
                secret_envelope BLOB NOT NULL,
                secret_updated_at TEXT NOT NULL,
                connect_timeout_seconds INTEGER NOT NULL CHECK (
                    connect_timeout_seconds BETWEEN 1 AND 60
                ),
                skip_certificate_verification INTEGER NOT NULL CHECK (
                    skip_certificate_verification IN (0, 1)
                ),
                tags_json TEXT NOT NULL,
                revision INTEGER NOT NULL CHECK (revision >= 1),
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS extension_installs (
                id TEXT PRIMARY KEY NOT NULL,
                core_id TEXT NOT NULL,
                instance_id TEXT NOT NULL,
                kind TEXT NOT NULL CHECK (kind IN ('PLUGIN', 'MOD')),
                path TEXT NOT NULL,
                sha256 TEXT NOT NULL,
                source TEXT NOT NULL,
                project_id TEXT,
                version TEXT,
                installed_at TEXT NOT NULL,
                UNIQUE (core_id, instance_id, path)
            );

            CREATE INDEX IF NOT EXISTS extension_installs_instance_kind_idx
                ON extension_installs(core_id, instance_id, kind);

            PRAGMA user_version = 3;
            ",
        )
        .map_err(StorageError::Migrate)
}

fn column_exists(connection: &Connection, table: &str, column: &str) -> Result<bool, StorageError> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(StorageError::Migrate)?;
    let mut rows = statement.query([]).map_err(StorageError::Migrate)?;
    while let Some(row) = rows.next().map_err(StorageError::Migrate)? {
        let name: String = row.get(1).map_err(StorageError::Migrate)?;
        if name == column {
            return Ok(true);
        }
    }

    Ok(false)
}

fn map_user(row: &Row<'_>) -> SqliteResult<StoredUser> {
    Ok(StoredUser::new(
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
    ))
}

fn map_core(row: &Row<'_>) -> SqliteResult<StoredCore> {
    Ok(StoredCore {
        id: row.get(0)?,
        name: row.get(1)?,
        address: row.get(2)?,
        secret_envelope: row.get(3)?,
        secret_updated_at: row.get(4)?,
        connect_timeout_seconds: row.get(5)?,
        skip_certificate_verification: row.get(6)?,
        tags_json: row.get(7)?,
        revision: row.get(8)?,
    })
}

fn map_extension_install(row: &Row<'_>) -> SqliteResult<StoredExtensionInstall> {
    Ok(StoredExtensionInstall {
        id: row.get(0)?,
        core_id: row.get(1)?,
        instance_id: row.get(2)?,
        kind: row.get(3)?,
        path: row.get(4)?,
        sha256: row.get(5)?,
        source: row.get(6)?,
        project_id: row.get(7)?,
        version: row.get(8)?,
        installed_at: row.get(9)?,
    })
}

fn map_session(row: &Row<'_>) -> SqliteResult<StoredSession> {
    Ok(StoredSession::new(
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        StoredUser::new(
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
            row.get(7)?,
        ),
    ))
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::DATABASE_FILE_NAME;
    use super::SqliteStore;
    use crate::NewCore;
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
                    "Administrator",
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
                    "Another User",
                    "password-hash",
                    "2026-07-30T10:15:31Z",
                )
                .expect("second initialization is rejected")
        );
        assert!(store.database_path().exists());
    }

    #[test]
    fn migrates_the_initial_user_schema() {
        let data_directory = tempdir().expect("temporary Panel data directory is created");
        let connection = Connection::open(data_directory.path().join(DATABASE_FILE_NAME))
            .expect("legacy Panel database opens");
        connection
            .execute_batch(
                "CREATE TABLE users (
                    id TEXT PRIMARY KEY NOT NULL,
                    username TEXT NOT NULL COLLATE NOCASE UNIQUE,
                    password_hash TEXT NOT NULL,
                    is_admin INTEGER NOT NULL,
                    created_at TEXT NOT NULL
                 );
                 INSERT INTO users VALUES (
                    'user-1', 'admin', 'password-hash', 1, '2026-07-30T10:15:31Z'
                 );",
            )
            .expect("legacy user schema is created");
        drop(connection);

        let store = SqliteStore::open(data_directory.path()).expect("Panel database migrates");
        let user = store
            .find_user_by_username("admin")
            .expect("migrated user query succeeds")
            .expect("migrated user exists");

        assert_eq!(user.display_name(), "Administrator");
    }

    #[test]
    fn persists_core_registrations_and_a_stable_panel_id() {
        let data_directory = tempdir().expect("temporary Panel data directory is created");
        let store = SqliteStore::open(data_directory.path()).expect("Panel database opens");
        let core = NewCore {
            id: "0198f8a8-c684-7361-b36a-43c7831c84c0".to_owned(),
            name: "Game Node".to_owned(),
            address: "tls://core.example.com:25580".to_owned(),
            secret_envelope: vec![1, 2, 3, 4],
            secret_updated_at: "2026-08-01T10:00:00Z".to_owned(),
            connect_timeout_seconds: 10,
            skip_certificate_verification: false,
            tags_json: "[\"production\"]".to_owned(),
            created_at: "2026-08-01T10:00:00Z".to_owned(),
        };

        assert!(store.insert_core(&core).expect("Core is inserted"));
        assert!(!store.insert_core(&core).expect("duplicate Core is ignored"));
        let cores = store.list_cores().expect("Core registrations are listed");
        let stored = cores.first().expect("stored Core is returned");

        assert_eq!(cores.len(), 1);
        assert_eq!(stored.name(), "Game Node");
        assert_eq!(stored.secret_envelope(), [1, 2, 3, 4]);
        assert_eq!(
            store
                .get_or_create_panel_id("panel-1")
                .expect("Panel ID is created"),
            "panel-1"
        );
        assert_eq!(
            store
                .get_or_create_panel_id("panel-2")
                .expect("Panel ID is reused"),
            "panel-1"
        );
    }

    #[test]
    fn upserts_lists_and_deletes_extension_install_records() {
        let data_directory = tempdir().expect("temporary Panel data directory is created");
        let store = SqliteStore::open(data_directory.path()).expect("Panel database opens");
        let install = crate::NewExtensionInstall {
            id: "install-1".to_owned(),
            core_id: "core-1".to_owned(),
            instance_id: "survival".to_owned(),
            kind: "PLUGIN".to_owned(),
            path: "plugins/example.jar".to_owned(),
            sha256: "a".repeat(64),
            source: "LOCAL".to_owned(),
            project_id: None,
            version: None,
            installed_at: "2026-08-04T00:00:00Z".to_owned(),
        };

        let stored = store
            .upsert_extension_install(&install)
            .expect("extension install is persisted");
        assert_eq!(stored.id(), "install-1");
        assert_eq!(
            store
                .list_extension_installs("core-1", "survival", "PLUGIN")
                .expect("extension installs are listed")
                .len(),
            1
        );
        assert!(
            store
                .delete_extension_install("core-1", "survival", "plugins/example.jar")
                .expect("extension install is deleted")
        );
        assert!(
            store
                .list_extension_installs("core-1", "survival", "PLUGIN")
                .expect("extension installs are listed after deletion")
                .is_empty()
        );
    }
}
