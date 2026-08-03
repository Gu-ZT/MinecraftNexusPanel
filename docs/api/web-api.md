# Panel Web API v1

## 1. 基础约定

- 基础路径：`/api/v1`
- 生产环境只允许 HTTPS；Panel 位于反向代理后时必须配置可信代理列表。
- 请求和响应使用 `application/json; charset=utf-8`，文件流端点除外。
- JSON 字段使用 `camelCase`，时间使用 UTC RFC 3339。
- 每个响应包含 `X-Request-Id`；客户端可在请求中提供合法的 UUID 作为该值。
- 客户端必须发送 `X-Client-Version`，格式为语义版本。

成功的单资源响应直接返回资源，不额外包裹 `data`。列表使用统一分页结构：

```json
{
  "items": [],
  "nextCursor": "opaque-or-null"
}
```

游标是不透明字符串，客户端不得保存超过 24 小时或解析其内容。默认 `limit=50`，最大 `limit=200`。

## 2. 鉴权

### 2.1 浏览器会话

1. `POST /auth/login` 校验账号、密码和可选 MFA。
2. Panel 设置 `mcnp_session` Cookie：`HttpOnly; Secure; SameSite=Lax; Path=/`。
3. 响应体返回 CSRF Token；浏览器内存保存，并在所有非安全方法中发送 `X-CSRF-Token`。
4. `POST /auth/refresh` 轮换会话，`POST /auth/logout` 撤销会话。

浏览器 API 不把长期 Token 暴露给 JavaScript。跨站部署 WebUI 时必须使用明确 Origin allowlist，不能配置 `*` 与凭据同时使用。

### 2.2 Desktop/Mobile 会话

原生客户端在登录请求中设置 `clientType=NATIVE`，获得：

- Access Token：不透明短期 Token，建议 15 分钟。
- Refresh Token：一次性轮换 Token，建议最长 30 天。
- `sessionId` 与过期时间。

Access Token 使用 `Authorization: Bearer <token>`。Refresh Token 仅发送到 `POST /auth/refresh`
，必须存入操作系统安全存储；移动端可增加生物识别访问控制。

Refresh Token 每次使用后立即失效。再次使用旧 Token 视为泄漏，撤销同一 Token Family 的所有会话。

### 2.3 登录请求

```http
POST /api/v1/auth/login
Content-Type: application/json
```

```json
{
  "username": "admin",
  "password": "correct horse battery staple",
  "clientType": "NATIVE",
  "device": {
    "name": "Pixel 10",
    "platform": "ANDROID",
    "deviceId": "installation-scoped-random-id"
  },
  "mfaCode": null
}
```

```json
{
  "user": {
    "id": "0198...",
    "username": "admin",
    "displayName": "Administrator",
    "permissions": [
      "core.read",
      "instance.control"
    ]
  },
  "session": {
    "id": "0198...",
    "accessToken": "native-only-token",
    "accessExpiresAt": "2026-07-30T10:30:00Z",
    "refreshToken": "native-only-refresh-token",
    "refreshExpiresAt": "2026-08-29T10:15:00Z",
    "csrfToken": null
  }
}
```

登录失败统一返回 `AUTH_INVALID_CREDENTIALS`，不泄漏用户名是否存在。按账号和来源 IP 双重限流。

## 3. 幂等与并发控制

- `POST` 状态操作和创建操作接受 `Idempotency-Key`，值为客户端生成的 UUID。
- 同一用户、端点和 key 在 24 小时内返回首次请求的状态码与结果。
- key 相同但请求体不同返回 `409 IDEMPOTENCY_KEY_REUSED`。
- 资源响应包含整数 `revision` 和 `ETag: "<revision>"`。
- 更新、删除必须发送 `If-Match`；revision 不一致返回 `412 REVISION_MISMATCH`。
- 不提供 `If-Match` 的危险删除返回 `428 PRECONDITION_REQUIRED`。

## 4. 异步任务

安装、备份、恢复、批量删除等耗时操作返回：

```http
HTTP/1.1 202 Accepted
Location: /api/v1/tasks/0198...
Retry-After: 2
```

```json
{
  "taskId": "0198...",
  "state": "QUEUED",
  "acceptedAt": "2026-07-30T10:15:31Z"
}
```

客户端通过 `GET /tasks/{taskId}` 查询，或订阅 WebSocket 的 `task/{taskId}`。HTTP 请求超时不能推断任务失败。

## 5. 端点总览

环境、一键搭建、配置、扩展、Docker、计划任务和用户组的完整端点见 [`management-api.md`](management-api.md)
。商业服务商接口使用独立契约 [`provider-api.md`](provider-api.md)。

### 5.1 系统与鉴权

| 方法   | 路径                         | 权限           | 说明                         |
|--------|------------------------------|----------------|------------------------------|
| GET    | `/health/live`               | 公开           | 进程存活，不检查 Core/数据库 |
| GET    | `/health/ready`              | 公开           | Panel 是否能接收业务流量     |
| GET    | `/system/info`               | 已登录         | Panel 版本、API 版本和能力   |
| POST   | `/auth/login`                | 公开           | 创建浏览器或原生会话         |
| POST   | `/auth/refresh`              | Refresh/Cookie | 轮换会话凭据                 |
| POST   | `/auth/logout`               | 已登录         | 撤销当前会话                 |
| GET    | `/auth/me`                   | 已登录         | 当前用户、权限与资源范围     |
| GET    | `/auth/sessions`             | 已登录         | 当前用户的设备会话           |
| DELETE | `/auth/sessions/{sessionId}` | 已登录         | 撤销指定设备会话             |
| POST   | `/ws/tickets`                | 已登录         | 创建一次性 WebSocket 票据    |

### 5.2 Core

| 方法   | 路径                                | 权限          | 说明                          |
|--------|-------------------------------------|---------------|-------------------------------|
| GET    | `/cores`                            | `core.read`   | 可访问 Core 列表              |
| POST   | `/cores`                            | `core.manage` | 注册 Core，PSK 只在请求中出现 |
| GET    | `/cores/{coreId}`                   | `core.read`   | Core 配置、状态与能力         |
| PATCH  | `/cores/{coreId}`                   | `core.manage` | 更新名称、地址、标签或密钥    |
| DELETE | `/cores/{coreId}`                   | `core.manage` | 从 Panel 移除，不删除远端实例 |
| POST   | `/cores/{coreId}/actions/test`      | `core.manage` | 测试地址、PSK 和协议兼容性    |
| POST   | `/cores/{coreId}/actions/reconnect` | `core.manage` | 主动重建连接                  |
| GET    | `/cores/{coreId}/metrics`           | `core.read`   | Core 资源指标                 |

创建请求：

```json
{
  "name": "Game Node 01",
  "address": "tls://core-01.example.com:25580",
  "secret": "one-time-plaintext-psk",
  "skipCertificateVerification": false,
  "connectTimeoutSeconds": 10,
  "tags": [
    "cn-east",
    "production"
  ]
}
```

`address` 支持 IP/localhost 的 `host:port`，以及 `tls://`、`mcnp://`、`https://` URL。域名 URL 默认验证 TLS 证书；IP 和
localhost 自动跳过证书链验证。`skipCertificateVerification` 仅用于管理员明确接受自签名或私有证书的场景。

Core 响应永远不返回 `secret`，只返回 `secretConfigured`、`secretUpdatedAt`、实际 TLS 证书 SHA-256 指纹和验证结果。

Panel 使用 `MCNP_PANEL_MASTER_KEY` 对 Core PSK 执行 AES-256-GCM 信封加密，密文使用随机 nonce，并通过 Core ID 关联数据绑定到对应注册记录。Panel 启动时恢复注册表并自动连接；心跳失败后以 1 秒起步、最长 30 秒的指数退避重连。主动 reconnect 会立即丢弃现有连接并进入 `UNKNOWN`，后续连接结果更新为 `ONLINE`、`OFFLINE`、`AUTH_FAILED` 或 `INCOMPATIBLE`。

### 5.3 实例

实例路径总是包含 `coreId`，防止不同 Core 上相同实例 ID 产生歧义。

| 方法   | 路径                                     | 权限                     | 说明                       |
|--------|------------------------------------------|--------------------------|----------------------------|
| GET    | `/cores/{coreId}/instances`              | `instance.read`          | 实例列表                   |
| POST   | `/cores/{coreId}/instances`              | `instance.create`        | 创建实例                   |
| GET    | `/cores/{coreId}/instances/{instanceId}` | `instance.read`          | 实例配置和运行状态         |
| PATCH  | `/cores/{coreId}/instances/{instanceId}` | 按修改字段               | 部分更新实例               |
| DELETE | `/cores/{coreId}/instances/{instanceId}` | `instance.delete`        | 删除定义，可选异步删除文件 |
| POST   | `.../{instanceId}/actions/start`         | `instance.control`       | 启动                       |
| POST   | `.../{instanceId}/actions/stop`          | `instance.control`       | 优雅停止                   |
| POST   | `.../{instanceId}/actions/restart`       | `instance.control`       | 优雅重启                   |
| POST   | `.../{instanceId}/actions/kill`          | `instance.control`       | 强制终止，需要确认字段     |
| POST   | `.../{instanceId}/commands`              | `instance.console.write` | 发送一条控制台命令         |
| GET    | `.../{instanceId}/logs`                  | `instance.console.read`  | 游标分页读取日志           |
| GET    | `.../{instanceId}/metrics`               | `instance.read`          | 实例资源指标               |

创建实例：

```json
{
  "id": "survival",
  "name": "Survival",
  "kind": "PAPER",
  "directory": "instances/survival",
  "launch": {
    "executable": "java",
    "args": [
      "-Xms1G",
      "-Xmx4G",
      "-jar",
      "paper.jar",
      "nogui"
    ],
    "environment": {},
    "stopCommand": "stop",
    "stopTimeoutSeconds": 30
  }
}
```

状态操作的正常响应是 `202` 任务引用。若操作已达到目标状态，例如对 RUNNING 实例再次 start，使用同一幂等键时返回原结果；新请求返回
`409 INSTANCE_STATE_CONFLICT`。

发送命令：

```json
{
  "command": "say Maintenance starts in 5 minutes"
}
```

命令不得出现在访问日志中。API 响应只返回接收时间，不回显命令。

### 5.4 文件

| 方法   | 路径                                                   | 权限         | 说明           |
|--------|--------------------------------------------------------|--------------|----------------|
| GET    | `.../{instanceId}/files?path=`                         | `file.read`  | 列出目录       |
| GET    | `.../{instanceId}/file-content?path=server.properties&offset=0&length=32768` | `file.read`  | 分块读取文件   |
| PUT    | `.../{instanceId}/file-content?path=server.properties` | `file.write` | 小文件整体写入 |
| POST   | `.../{instanceId}/directories`                         | `file.write` | 创建目录       |
| POST   | `.../{instanceId}/file-actions/move`                   | `file.write` | 移动或重命名   |
| POST   | `.../{instanceId}/file-actions/batch`                  | `file.write` | 顺序执行批量文件操作 |
| POST   | `.../{instanceId}/archives`                             | `file.write` | 异步创建 ZIP 下载归档 |
| DELETE | `.../{instanceId}/files?path=logs/old&confirmation=DELETE` | `file.write` | 异步删除文件/目录 |
| GET    | `.../cores/{coreId}/file-tasks/{taskId}`              | 已登录       | 查询文件操作任务 |
| POST   | `.../{instanceId}/uploads`                             | `file.write` | 初始化分块上传 |
| PUT    | `.../cores/{coreId}/uploads/{transferId}/parts/{partNumber}` | `file.write` | 上传分块       |
| POST   | `.../cores/{coreId}/uploads/{transferId}/complete`     | `file.write` | 校验并提交     |
| DELETE | `.../cores/{coreId}/uploads/{transferId}`              | `file.write` | 放弃上传       |
| POST   | `.../{instanceId}/downloads`                          | `file.read`  | 初始化分块下载 |
| GET    | `.../cores/{coreId}/downloads/{transferId}/parts/{partNumber}` | `file.read` | 下载分块       |
| POST   | `.../cores/{coreId}/downloads/{transferId}/complete`   | `file.write` | 校验并关闭下载 |
| DELETE | `.../cores/{coreId}/downloads/{transferId}`            | `file.write` | 放弃下载       |

路径必须使用 UTF-8 和 `/`，以实例根目录为 `/`。Panel 与 Core 都必须拒绝绝对宿主机路径、NUL、`..` 段和逃逸实例根目录的符号链接。

当前文件接口已提供目录分页、单次最多 32 KiB 的分块读取、最多 1 MiB 的原子写入、目录创建、移动和异步删除。读取响应使用完整文件
SHA-256 的 `ETag`，并通过 `X-MCNP-File-Eof: true|false` 表示是否到达末尾；写入、移动、目录创建和删除使用 `Idempotency-Key`，写入可选
`If-Match` 传入带引号的当前文件 SHA-256。

删除必须携带查询参数 `confirmation=DELETE`。默认只能删除文件或空目录，`recursive=true` 才能删除非空目录；删除接口返回 `202 Accepted`
和 `taskId`，客户端通过 `/cores/{coreId}/file-tasks/{taskId}` 轮询 `RUNNING`、`SUCCEEDED` 或 `FAILED` 状态。Core 和 Panel 都拒绝符号链接、
绝对路径、`NUL`、`.`/`..` 段以及逃逸实例根目录的路径。

批量接口请求体为 `{ "operations": [...] }`，最多 64 项，支持 `MKDIR`、`MOVE`、`WRITE` 和 `DELETE`；每个删除项必须携带
`confirmation: "DELETE"`。接口返回 `202 Accepted` 和 `taskId`，任务按数组顺序执行，`FILE_BATCH` 的 `progress` 返回已完成数和总数，
`results` 返回逐项状态；某项失败时任务为 `FAILED`，保留已执行项和 `failedIndex`，不回滚之前的文件变更。

归档接口请求体为 `{ "paths": ["config", "server.properties"], "outputPath": "downloads/backup.zip" }`，一次最多 128 个源路径，
递归结果最多 16,384 个 ZIP 条目且未压缩源数据最多 4 GiB；`paths` 可以选择文件、目录或实例根目录；输出路径必须位于实例目录内且父目录已存在。接口返回 `202 Accepted` 和 `taskId`，
`FILE_ARCHIVE_CREATE` 任务按 ZIP 条目报告 `completed`/`total` 进度，成功时在 `archive` 字段返回生成的 `FileEntry`。Core 会拒绝
绝对路径、父目录段、符号链接和实例目录外路径，并使用同目录临时文件原子落盘；归档生成已实现，归档文件可以通过下述分块下载会话读取。

大文件上传使用会话化分块协议。初始化请求体为
`{ "path": "world.zip", "sizeBytes": 123456, "sha256": "..." }`，响应状态为 `201 Created`，并返回
`transferId`、固定 `chunkSize: 1048576`、`nextOffset: 0` 和 `sizeBytes`。分片路径中的 `partNumber` 从 0 开始，服务端按
`partNumber * chunkSize` 计算 offset；每个 part 使用 `Content-Type: application/octet-stream`、必需的
`Content-SHA256` 和 `Idempotency-Key`。相同分片可安全重试，失序分片会返回冲突。

单个文件最大 4 GiB，单个 Core 最多 16 个活动上传会话。完成接口会再次校验完整文件大小和 SHA-256，并在同一文件系统内原子替换目标；放弃接口删除临时文件。
上传状态只保存在 Core 内存中，Core 重启会清理未完成会话。下载初始化请求体为 `{ "path": "world.zip" }`，响应会返回
`transferId`、固定 `chunkSize`、当前 `nextOffset`、完整文件 `sizeBytes` 和 `sha256`。客户端使用
`GET /cores/{coreId}/downloads/{transferId}/parts/{partNumber}` 读取二进制分片；响应通过 `Content-SHA256`、完整文件
`ETag`、`X-MCNP-File-Transfer-Offset`、`X-MCNP-File-Transfer-Next-Offset`、`X-MCNP-File-Transfer-Size` 和
`X-MCNP-File-Eof` 传递校验和游标。已读分片可以重试，跳过当前游标的分片会被拒绝；完成时 Core 会重新校验源文件摘要，
放弃会释放会话。上传和下载会话状态目前都只保存在 Core 内存中，跨 Core 重启续传、快照和差异比较仍未实现。

### 5.5 任务、用户和审计

| 方法             | 路径                             | 权限               | 说明               |
|------------------|----------------------------------|--------------------|--------------------|
| GET              | `/tasks`                         | 已登录             | 按可见资源过滤任务 |
| GET              | `/tasks/{taskId}`                | 已登录             | 任务状态           |
| POST             | `/tasks/{taskId}/actions/cancel` | 对应写权限         | 尽力取消           |
| GET              | `/users`                         | `user.read`        | 用户列表           |
| POST             | `/users`                         | `user.manage`      | 创建用户           |
| GET/PATCH/DELETE | `/users/{userId}`                | 对应用户权限       | 用户管理           |
| GET              | `/roles`                         | `user.read`        | 角色列表           |
| POST             | `/roles`                         | `user.manage`      | 创建角色           |
| GET/PATCH/DELETE | `/roles/{roleId}`                | 对应用户权限       | 角色管理           |
| GET/POST         | `/groups`                        | `user.read/manage` | 用户组列表或创建   |
| GET/PATCH/DELETE | `/groups/{groupId}`              | `user.read/manage` | 用户组管理         |
| PUT              | `/groups/{groupId}/members`      | `user.manage`      | 设置用户组成员     |
| PUT              | `/groups/{groupId}/grants`       | `user.manage`      | 设置权限和实例范围 |
| GET              | `/audit-events`                  | `audit.read`       | 只读审计事件       |

审计事件不允许通过 API 修改或删除。筛选支持 `actorId`、`action`、`coreId`、`instanceId`、`from`、`to`。

## 6. 资源示例

### Core

```json
{
  "id": "0198...",
  "name": "Game Node 01",
  "address": "10.0.0.12:25580",
  "status": "ONLINE",
  "latencyMs": 12,
  "lastSeenAt": "2026-07-30T10:15:32Z",
  "version": "0.1.0",
  "protocolVersion": "1.0",
  "capabilities": [
    "events",
    "files",
    "metrics",
    "transfer-v1"
  ],
  "secretConfigured": true,
  "secretUpdatedAt": "2026-07-20T03:10:00Z",
  "tags": [
    "cn-east",
    "production"
  ],
  "revision": 4
}
```

Core 状态为 `ONLINE | DEGRADED | OFFLINE | INCOMPATIBLE | AUTH_FAILED | UNKNOWN`。

### Instance

```json
{
  "id": "survival",
  "coreId": "0198...",
  "name": "Survival",
  "kind": "PAPER",
  "revision": 7,
  "runtime": {
    "state": "RUNNING",
    "pid": 14273,
    "startedAt": "2026-07-30T10:12:12Z",
    "exitCode": null,
    "players": {
      "online": 3,
      "max": 20
    }
  }
}
```

实例状态为 `CREATED | STARTING | RUNNING | STOPPING | STOPPED | FAILED | UNKNOWN`。

## 7. 限流

- 登录：每 IP 每分钟 20 次，每账号每 15 分钟 10 次失败尝试。
- 普通 API：每会话每分钟 600 次，突发 100 次。
- 控制台命令：每实例每用户每秒 10 条。
- WebSocket ticket：每会话每分钟 10 个。
- 上传：按用户、实例和 Core 同时限制并发数与字节速率。

限流返回 `429`、`Retry-After` 和标准错误体。反向代理限流不能替代应用层按用户限流。

## 8. CORS 与 CSRF

- 默认同源，不返回 CORS 头。
- 配置跨域时只允许明确的 HTTPS Origin。
- 任何 Cookie 鉴权的非安全方法都验证 `Origin` 和 `X-CSRF-Token`。
- Bearer Token 请求不要求 CSRF，但仍受 CORS 和权限检查。
- WebSocket Upgrade 同样验证 Origin；原生客户端使用一次性 ticket。

## 9. API 演进

- 同一主版本可以增加端点、可选字段、错误码和枚举值。
- 删除/重命名字段、改变单位或默认行为需要 `/api/v2`。
- 被弃用端点响应 `Deprecation: true` 和带日期的 `Sunset`。
- Panel 的 `GET /system/info` 返回支持的 API 主版本、功能能力和最小客户端版本。
