//! MCNP 持久化层（PLAN.md 第 4.3 节）。
//!
//! 职责：默认 SQLite、可切换 PostgreSQL；用户密码 Argon2id；Core PSK 信封加密落库；
//! Refresh Token 仅存摘要并支持 Token Family 撤销；审计事件不可变存储与敏感字段脱敏；
//! 商业版所有业务表强制 tenantId 过滤。
//!
//! TODO(M1): 首批迁移（用户、会话、Core 注册信息）与仓储接口。
