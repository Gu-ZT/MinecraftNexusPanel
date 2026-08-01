# 首位管理员与登录会话

Panel 使用 SQLite 保存用户与会话。首次启动空数据库前，通过环境变量提供首位管理员凭据：

```powershell
$env:MCNP_INITIAL_ADMIN_USERNAME = "admin"
$env:MCNP_INITIAL_ADMIN_PASSWORD = "replace-with-a-strong-password"
cargo run -p mcnp -- panel
```

用户名长度为 1 到 64 个字符，首位管理员密码长度为 12 到 1024 字节。两项必须同时配置，否则进程拒绝启动。

初始化在 SQLite `IMMEDIATE` 事务中完成。只要用户表中已有记录，后续启动提供的初始凭据就不会创建或替换管理员。初始化成功后应从服务环境中移除 `MCNP_INITIAL_ADMIN_PASSWORD`，避免长期暴露引导凭据。

密码使用 Argon2id 保存。原生客户端的 Access Token、Refresh Token，以及浏览器会话 Cookie 和 CSRF Token 都由操作系统随机源生成；数据库仅保存 SHA-256 摘要。Refresh Token 每次使用后轮换，重用旧 Token 会撤销对应会话。

浏览器 Cookie 固定使用 `HttpOnly; Secure; SameSite=Lax; Path=/`。非本机部署必须在受信任的反向代理或 Panel 前提供 HTTPS，不能为了兼容明文 HTTP 去掉 `Secure`。

登录失败同时按规范化用户名和 TCP 来源 IP 使用五分钟滑动窗口限流。当前版本不信任 `X-Forwarded-For` 等转发头；部署在反向代理后时，所有请求会被视为代理来源。后续支持代理来源 IP 前，必须先配置可信代理地址范围，不能直接接受客户端提供的转发头。
