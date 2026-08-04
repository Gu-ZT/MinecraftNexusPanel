use std::collections::HashMap;
use std::collections::VecDeque;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::Mutex;

use argon2::Argon2;
use argon2::PasswordHash;
use argon2::PasswordHasher;
use argon2::PasswordVerifier;
use argon2::password_hash::SaltString;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use nexus_config::InitialAdminConfig;
use nexus_storage::NewSession;
use nexus_storage::SqliteStore;
use nexus_storage::StoredSession;
use nexus_storage::StoredUser;
use sha2::Digest;
use sha2::Sha256;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

use crate::AuthError;
use crate::ClientType;
use crate::IssuedSession;
use crate::LoginRequest;

const ACCESS_TOKEN_LIFETIME_SECONDS: i64 = 15 * 60;
const ACCOUNT_LOGIN_ATTEMPT_LIMIT: usize = 5;
const IP_LOGIN_ATTEMPT_LIMIT: usize = 20;
const LOGIN_ATTEMPT_WINDOW_SECONDS: i64 = 5 * 60;
const MAX_LOGIN_ATTEMPT_KEYS: usize = 10_000;
const SESSION_LIFETIME_SECONDS: i64 = 30 * 24 * 60 * 60;
const RANDOM_CREDENTIAL_BYTES: usize = 32;
const PASSWORD_SALT_BYTES: usize = 16;

/// Panel 身份认证、会话轮换和登录限流服务。
///
/// 密码使用 Argon2 哈希，令牌只以 SHA-256 哈希进入存储；浏览器会话额外要求
/// CSRF 校验，重复使用已轮换刷新令牌会撤销关联会话。
#[derive(Clone)]
pub struct AuthService {
    store: SqliteStore,
    login_attempts: Arc<Mutex<HashMap<String, VecDeque<i64>>>>,
}

impl AuthService {
    /// 创建认证服务。
    #[must_use]
    pub fn new(store: SqliteStore) -> Self {
        Self {
            store,
            login_attempts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 在数据库为空时初始化首个管理员账户。
    pub fn initialize_admin(&self, config: &InitialAdminConfig) -> Result<bool, AuthError> {
        if self.store.has_users()? {
            return Ok(false);
        }

        let password_hash = hash_password(config.password())?;
        let created_at = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned());

        self.store
            .create_initial_user(
                &Uuid::now_v7().to_string(),
                config.username(),
                "Administrator",
                &password_hash,
                &created_at,
            )
            .map_err(AuthError::from)
    }

    /// 判断数据库是否已经存在用户。
    pub fn has_users(&self) -> Result<bool, AuthError> {
        self.store.has_users().map_err(AuthError::from)
    }

    /// 校验登录请求并创建浏览器或原生客户端会话。
    pub fn login(
        &self,
        request: &LoginRequest,
        source_ip: IpAddr,
    ) -> Result<IssuedSession, AuthError> {
        self.enforce_login_rate_limit(request.username(), source_ip)?;
        let user = self.store.find_user_by_username(request.username())?;
        let Some(user) = user else {
            let _ = hash_password(request.password())?;
            self.record_login_failure(request.username(), source_ip)?;
            return Err(AuthError::InvalidCredentials);
        };
        if !password_matches(request.password(), user.password_hash()) {
            self.record_login_failure(request.username(), source_ip)?;
            return Err(AuthError::InvalidCredentials);
        }
        self.clear_account_login_failures(request.username())?;

        let (issued, record) = build_session(user, request.client_type(), None)?;
        self.store.create_session(&record)?;

        Ok(issued)
    }

    /// 轮换原生客户端刷新令牌。
    pub fn refresh_native(&self, refresh_token: &str) -> Result<IssuedSession, AuthError> {
        let token_hash = hash_token(refresh_token);
        let now = current_unix_timestamp();
        let Some(session) = self.store.find_session_by_refresh_token(&token_hash, now)? else {
            return self.handle_missing_refresh_token(&token_hash, now);
        };
        if session.client_type() != ClientType::Native.as_str() {
            return Err(AuthError::InvalidSession);
        }

        self.rotate_session(&session, ClientType::Native, &token_hash)
    }

    /// 校验浏览器 Cookie 和 CSRF 令牌并轮换会话。
    pub fn refresh_browser(
        &self,
        session_cookie: &str,
        csrf_token: &str,
    ) -> Result<IssuedSession, AuthError> {
        let token_hash = hash_token(session_cookie);
        let now = current_unix_timestamp();
        let Some(session) = self.store.find_session_by_refresh_token(&token_hash, now)? else {
            return self.handle_missing_refresh_token(&token_hash, now);
        };
        if session.client_type() != ClientType::Browser.as_str() {
            return Err(AuthError::InvalidSession);
        }
        verify_csrf_token(&session, csrf_token)?;

        self.rotate_session(&session, ClientType::Browser, &token_hash)
    }

    /// 验证原生客户端访问令牌并返回会话用户。
    pub fn authenticate_access_token(
        &self,
        access_token: &str,
    ) -> Result<StoredSession, AuthError> {
        let session = self
            .store
            .find_session_by_access_token(&hash_token(access_token), current_unix_timestamp())?
            .ok_or(AuthError::InvalidSession)?;
        if session.client_type() != ClientType::Native.as_str() {
            return Err(AuthError::InvalidSession);
        }

        Ok(session)
    }

    /// 验证浏览器会话 Cookie 并返回会话用户。
    pub fn authenticate_browser_session(
        &self,
        session_cookie: &str,
    ) -> Result<StoredSession, AuthError> {
        let session = self
            .store
            .find_session_by_refresh_token(&hash_token(session_cookie), current_unix_timestamp())?
            .ok_or(AuthError::InvalidSession)?;
        if session.client_type() != ClientType::Browser.as_str() {
            return Err(AuthError::InvalidSession);
        }

        Ok(session)
    }

    /// 验证会话对应的 CSRF 令牌。
    pub fn verify_csrf(&self, session: &StoredSession, csrf_token: &str) -> Result<(), AuthError> {
        verify_csrf_token(session, csrf_token)
    }

    /// 撤销指定会话。
    pub fn logout(&self, session_id: &str) -> Result<(), AuthError> {
        if self
            .store
            .revoke_session(session_id, current_unix_timestamp())?
        {
            Ok(())
        } else {
            Err(AuthError::InvalidSession)
        }
    }

    fn rotate_session(
        &self,
        session: &StoredSession,
        client_type: ClientType,
        old_refresh_token_hash: &str,
    ) -> Result<IssuedSession, AuthError> {
        let (issued, replacement) = build_session(
            session.user().clone(),
            client_type,
            Some(session.id().to_owned()),
        )?;
        if self
            .store
            .rotate_session(session.id(), old_refresh_token_hash, &replacement)?
        {
            Ok(issued)
        } else {
            self.handle_missing_refresh_token(old_refresh_token_hash, current_unix_timestamp())
        }
    }

    fn handle_missing_refresh_token(
        &self,
        token_hash: &str,
        now: i64,
    ) -> Result<IssuedSession, AuthError> {
        if self
            .store
            .revoke_session_for_rotated_token(token_hash, now)?
        {
            Err(AuthError::RefreshReused)
        } else {
            Err(AuthError::InvalidSession)
        }
    }

    fn enforce_login_rate_limit(&self, username: &str, source_ip: IpAddr) -> Result<(), AuthError> {
        let now = current_unix_timestamp();
        let cutoff = now - LOGIN_ATTEMPT_WINDOW_SECONDS;
        let mut attempts = self
            .login_attempts
            .lock()
            .map_err(|_| AuthError::RateLimitLock)?;
        prune_login_attempts(&mut attempts, cutoff);

        let account_key = account_attempt_key(username);
        let ip_key = ip_attempt_key(source_ip);
        let retry_after = [
            (account_key.as_str(), ACCOUNT_LOGIN_ATTEMPT_LIMIT),
            (ip_key.as_str(), IP_LOGIN_ATTEMPT_LIMIT),
        ]
        .into_iter()
        .filter_map(|(key, limit)| {
            attempts
                .get(key)
                .filter(|timestamps| timestamps.len() >= limit)
                .and_then(|timestamps| timestamps.front())
                .map(|first_attempt| first_attempt + LOGIN_ATTEMPT_WINDOW_SECONDS - now)
        })
        .max();

        match retry_after {
            Some(seconds) => Err(AuthError::RateLimited {
                retry_after_seconds: seconds.max(1) as u64,
            }),
            None => Ok(()),
        }
    }

    fn record_login_failure(&self, username: &str, source_ip: IpAddr) -> Result<(), AuthError> {
        let now = current_unix_timestamp();
        let cutoff = now - LOGIN_ATTEMPT_WINDOW_SECONDS;
        let mut attempts = self
            .login_attempts
            .lock()
            .map_err(|_| AuthError::RateLimitLock)?;
        prune_login_attempts(&mut attempts, cutoff);

        for key in [account_attempt_key(username), ip_attempt_key(source_ip)] {
            if attempts.contains_key(&key) || attempts.len() < MAX_LOGIN_ATTEMPT_KEYS {
                attempts.entry(key).or_default().push_back(now);
            }
        }

        Ok(())
    }

    fn clear_account_login_failures(&self, username: &str) -> Result<(), AuthError> {
        self.login_attempts
            .lock()
            .map_err(|_| AuthError::RateLimitLock)?
            .remove(&account_attempt_key(username));

        Ok(())
    }
}

fn prune_login_attempts(attempts: &mut HashMap<String, VecDeque<i64>>, cutoff: i64) {
    attempts.retain(|_, timestamps| {
        while timestamps
            .front()
            .is_some_and(|timestamp| *timestamp <= cutoff)
        {
            timestamps.pop_front();
        }
        !timestamps.is_empty()
    });
}

fn account_attempt_key(username: &str) -> String {
    format!("account:{}", username.trim().to_lowercase())
}

fn ip_attempt_key(source_ip: IpAddr) -> String {
    format!("ip:{source_ip}")
}

fn build_session(
    user: StoredUser,
    client_type: ClientType,
    session_id: Option<String>,
) -> Result<(IssuedSession, NewSession), AuthError> {
    let now = current_unix_timestamp();
    let session_id = session_id.unwrap_or_else(|| Uuid::now_v7().to_string());
    let refresh_expires_at = now + SESSION_LIFETIME_SECONDS;
    let refresh_credential = generate_credential()?;
    let (access_token, access_expires_at, csrf_token, browser_cookie) = match client_type {
        ClientType::Native => (
            Some(generate_credential()?),
            now + ACCESS_TOKEN_LIFETIME_SECONDS,
            None,
            None,
        ),
        ClientType::Browser => (
            None,
            refresh_expires_at,
            Some(generate_credential()?),
            Some(refresh_credential.clone()),
        ),
    };
    let record = NewSession {
        id: session_id.clone(),
        user_id: user.id().to_owned(),
        client_type: client_type.as_str().to_owned(),
        access_token_hash: access_token.as_deref().map(hash_token),
        access_expires_at: access_token.as_ref().map(|_| access_expires_at),
        refresh_token_hash: hash_token(&refresh_credential),
        refresh_expires_at,
        csrf_token_hash: csrf_token.as_deref().map(hash_token),
        created_at: now,
    };
    let issued = IssuedSession {
        session_id,
        user,
        access_token,
        access_expires_at,
        refresh_token: (client_type == ClientType::Native).then_some(refresh_credential),
        refresh_expires_at: (client_type == ClientType::Native).then_some(refresh_expires_at),
        csrf_token,
        browser_cookie,
    };

    Ok((issued, record))
}

fn hash_password(password: &str) -> Result<String, AuthError> {
    let mut salt_bytes = [0_u8; PASSWORD_SALT_BYTES];
    getrandom::fill(&mut salt_bytes)?;
    let salt = SaltString::encode_b64(&salt_bytes)?;

    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)?
        .to_string())
}

fn password_matches(password: &str, encoded_hash: &str) -> bool {
    PasswordHash::new(encoded_hash).ok().is_some_and(|hash| {
        Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok()
    })
}

fn generate_credential() -> Result<String, AuthError> {
    let mut bytes = [0_u8; RANDOM_CREDENTIAL_BYTES];
    getrandom::fill(&mut bytes)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn hash_token(token: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(token.as_bytes()))
}

fn verify_csrf_token(session: &StoredSession, csrf_token: &str) -> Result<(), AuthError> {
    match session.csrf_token_hash() {
        Some(expected_hash) if expected_hash == hash_token(csrf_token) => Ok(()),
        _ => Err(AuthError::InvalidCsrfToken),
    }
}

fn current_unix_timestamp() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}
