# 本地运行

## 前置条件

- Rust `1.85` 或更高版本。
- Node.js `22.12` 或更高版本。
- pnpm `11.17.0`。

## 生成 Core 连接密钥

Core 只从环境变量 `MCNP_CORE_PSK` 读取连接密钥，不接受命令行密钥，避免其出现在 shell 历史或进程列表中。密钥必须是至少 32 个随机字节的无填充 Base64URL。

Core 默认在数据目录的 `tls/` 下生成并复用自签名证书。需要使用自有证书时，同时设置 `MCNP_CORE_TLS_CERT` 和
`MCNP_CORE_TLS_KEY` 为 PEM 文件路径。域名连接默认要求受信任证书；本地开发使用 IP/localhost 时自动跳过证书链验证。

```powershell
node -e "console.log(require('node:crypto').randomBytes(32).toString('base64url'))"
```

将输出设置到当前 PowerShell 会话：

```powershell
$env:MCNP_CORE_PSK = 'replace-with-the-generated-secret'
```

`all` 和 `core` 模式需要该变量；`panel` 模式不需要。可从根目录的 [`.env.example`](../../.env.example) 查看其他可选运行变量。实际 `.env` 文件被 Git 忽略。

## 启动模式

```powershell
cargo run -p mcnp -- core
cargo run -p mcnp -- panel
cargo run -p mcnp -- all
```

默认 Core TCP 地址为 `0.0.0.0:25580`，Panel HTTP 地址为 `127.0.0.1:8080`。启动后可访问 `http://127.0.0.1:8080/api/v1/health/live` 或 `/api/v1/health/ready` 验证 Panel 存活状态。

## 本地质量检查

```powershell
pnpm install --frozen-lockfile
pnpm api:lint
pnpm typecheck
pnpm build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```
