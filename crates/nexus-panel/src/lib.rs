//! MCNP Panel 实现（PLAN.md 第 1、4.2、4.3、4.4 节）。
//!
//! 职责：/api/v1 REST 与 WebSocket、登录鉴权（Cookie/CSRF 与 Token 双制式）、
//! RBAC 与实例资源范围过滤、审计日志、Core 注册信息管理与连接池
//! （心跳、指数退避重连）、Vue WebUI 构建产物托管。
//!
//! Panel 是用户、权限、审计与公开 Web API 的唯一权威来源。
//!
//! TODO(M1): 管理员初始化、登录、Core 增删与连通性测试、实例代理 API。
