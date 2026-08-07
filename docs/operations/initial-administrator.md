# 首位管理员与登录会话

Panel 使用 SQLite 保存用户与会话。首次启动空数据库前，通过环境变量提供首位管理员凭据：

```powershell
$env:MCNP_INITIAL_ADMIN_USERNAME = "admin"
$env:MCNP_INITIAL_ADMIN_PASSWORD = "replace-with-a-strong-password"
cargo run -p mcnp -- panel
```

用户名长度为 1 到 64 个字符，首位管理员密码长度为 12 到 1024 字节。两项必须同时配置，否则进程拒绝启动。

初始化在 SQLite `IMMEDIATE` 事务中完成。只要用户表中已有记录，后续启动提供的初始凭据就不会创建或替换管理员。初始化成功后应从服务环境中移除 `MCNP_INITIAL_ADMIN_PASSWORD`，避免长期暴露引导凭据。

## Desktop 独立版

Windows x64 Desktop 不要求用户预先配置环境变量。Tauri 首次启动时会在
`%APPDATA%\dev.mcnp.desktop\desktop-secrets.json` 中生成并保存 `admin` 用户名、
随机首启密码、Panel 主密钥和 Core PSK，然后把这些凭据注入随安装包发布的 `mcnp all`
sidecar。首启密码只用于初始化空数据库中的管理员，既不会显示在登录页，也不会通过 Tauri IPC
下发给共享前端，因此不存在固定或暴露给 WebView 的默认密码。

Tauri 会使用独立随机设备秘密调用仅限 loopback 的会话引导接口，直接建立原生会话，不要求
用户填写登录表单。设备秘密只保存在 `desktop-secrets.json` 和 Desktop/sidecar 进程内，不会
下发给 WebView。首次会话建立后，Desktop 删除秘密文件中的首启密码；SQLite 数据库、Panel
主密钥、Core PSK、设备秘密和 Core TLS 身份仍保留。后续启动优先
轮换系统凭据库中的 refresh token，失效或缺失时通过设备秘密重新签发会话。若安全存储或清理
失败，应用会保留可重试状态并显示错误。不要手动删除整个
`desktop-secrets.json`，否则已有 Panel 数据可能无法解密，Core 也会失去原有身份关联。

Desktop 的本地运行数据位于 `%APPDATA%\dev.mcnp.desktop\data`。安装包升级不会覆盖该目录，
备份或迁移 Desktop 时必须将 `data` 与 `desktop-secrets.json` 作为一个整体保护；秘密文件
包含可解密本地节点凭据的密钥，文件权限应限制为当前用户。

密码使用 Argon2id 保存。原生客户端的 Access Token、Refresh Token，以及浏览器会话 Cookie 和 CSRF Token 都由操作系统随机源生成；数据库仅保存 SHA-256 摘要。Refresh Token 每次使用后轮换，重用旧 Token 会撤销对应会话。

浏览器 Cookie 固定使用 `HttpOnly; Secure; SameSite=Lax; Path=/`。非本机部署必须在受信任的反向代理或 Panel 前提供 HTTPS，不能为了兼容明文 HTTP 去掉 `Secure`。

登录失败同时按规范化用户名和 TCP 来源 IP 使用五分钟滑动窗口限流。当前版本不信任 `X-Forwarded-For` 等转发头；部署在反向代理后时，所有请求会被视为代理来源。后续支持代理来源 IP 前，必须先配置可信代理地址范围，不能直接接受客户端提供的转发头。
