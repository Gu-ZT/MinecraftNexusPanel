# 错误模型

## 1. HTTP 错误体

所有非 `2xx` Web API 响应使用统一结构：

```json
{
  "error": {
    "code": "INSTANCE_STATE_CONFLICT",
    "message": "Instance is already running",
    "requestId": "0198...",
    "retryable": false,
    "details": {
      "currentState": "RUNNING"
    }
  }
}
```

- `code`：稳定、可供程序判断的 ASCII 标识。
- `message`：面向开发者的英文摘要，不作为前端稳定翻译键。
- `requestId`：用于日志与审计关联。
- `retryable`：在请求内容不变时是否可能通过稍后重试成功。
- `details`：可选结构化信息；不得包含密码、Token、PSK、完整控制台命令或堆栈。

字段校验错误：

```json
{
  "error": {
    "code": "VALIDATION_FAILED",
    "message": "Request validation failed",
    "requestId": "0198...",
    "retryable": false,
    "details": {
      "violations": [
        {
          "path": "launch.stopTimeoutSeconds",
          "rule": "range",
          "message": "Must be between 1 and 300"
        }
      ]
    }
  }
}
```

## 2. HTTP 状态映射

| 状态 | 使用场景                                 |
|-----:|------------------------------------------|
|  400 | JSON/查询参数非法、业务参数校验失败      |
|  401 | 未登录、Access Token 无效或过期          |
|  403 | 已登录但无资源权限、CSRF/Origin 校验失败 |
|  404 | 资源不存在，或为防止枚举而隐藏不可见资源 |
|  409 | 状态冲突、重复 ID、幂等键与请求体冲突    |
|  412 | `If-Match` revision 不一致               |
|  413 | 请求体或上传分块超过限制                 |
|  422 | 语法正确但无法执行的领域操作             |
|  428 | 写操作缺少 `If-Match` 等前置条件         |
|  429 | 触发限流或并发配额                       |
|  502 | Core 返回无效响应或协议失败              |
|  503 | Core 离线、Panel 未就绪或服务正在关闭    |
|  504 | Core 请求超时，最终结果可能未知          |

`504` 后客户端不能盲目重复危险操作；应使用原 `Idempotency-Key` 重试或查询任务/实例状态。

## 3. 通用错误码

| Code                     | HTTP | 可重试 | 含义                              |
|--------------------------|-----:|--------|-----------------------------------|
| `BAD_REQUEST`            |  400 | 否     | 请求格式不合法                    |
| `VALIDATION_FAILED`      |  400 | 否     | 字段校验失败                      |
| `UNAUTHENTICATED`        |  401 | 否     | 缺少有效会话                      |
| `TOKEN_EXPIRED`          |  401 | 否     | Access Token 过期，可尝试 refresh |
| `FORBIDDEN`              |  403 | 否     | 权限不足                          |
| `CSRF_REJECTED`          |  403 | 否     | CSRF Token 或 Origin 无效         |
| `NOT_FOUND`              |  404 | 否     | 资源不存在或不可见                |
| `REVISION_MISMATCH`      |  412 | 否     | 乐观锁冲突                        |
| `PRECONDITION_REQUIRED`  |  428 | 否     | 缺少必须的前置条件                |
| `IDEMPOTENCY_KEY_REUSED` |  409 | 否     | 同一 key 用于不同请求体           |
| `RATE_LIMITED`           |  429 | 是     | 触发限流                          |
| `INTERNAL_ERROR`         |  500 | 可能   | 未分类服务端错误                  |
| `SERVICE_UNAVAILABLE`    |  503 | 是     | 服务暂不可用                      |

## 4. 鉴权错误码

| Code                            | HTTP | 含义                                                 |
|---------------------------------|-----:|------------------------------------------------------|
| `AUTH_INVALID_CREDENTIALS`      |  401 | 用户名、密码或 MFA 无效，统一返回                    |
| `AUTH_MFA_REQUIRED`             |  401 | 需要 MFA，details 带短期 challengeId                 |
| `AUTH_SESSION_REVOKED`          |  401 | 会话被用户或管理员撤销                               |
| `AUTH_REFRESH_REUSED`           |  401 | 检测到已轮换 Refresh Token 重用，Token Family 已撤销 |
| `AUTH_ACCOUNT_DISABLED`         |  403 | 账号已禁用                                           |
| `AUTH_PASSWORD_CHANGE_REQUIRED` |  403 | 必须先修改密码                                       |

## 5. Core 与实例错误码

| Code                         | HTTP | Core retryable | 含义                      |
|------------------------------|-----:|----------------|---------------------------|
| `CORE_OFFLINE`               |  503 | 是             | Panel 未连接到 Core       |
| `CORE_CONNECT_TIMEOUT`       |  504 | 是             | 建立连接超时              |
| `CORE_AUTH_FAILED`           |  502 | 否             | PSK 不匹配                |
| `CORE_PROTOCOL_INCOMPATIBLE` |  502 | 否             | 协议主版本不兼容          |
| `CORE_PROTOCOL_ERROR`        |  502 | 可能           | Core 返回非法帧或响应     |
| `CORE_REQUEST_TIMEOUT`       |  504 | 可能           | 最终执行结果可能未知      |
| `METHOD_NOT_SUPPORTED`       |  422 | 否             | 未协商该能力/方法         |
| `INSTANCE_NOT_FOUND`         |  404 | 否             | 实例不存在                |
| `INSTANCE_ALREADY_EXISTS`    |  409 | 否             | 实例 ID 已存在            |
| `INSTANCE_STATE_CONFLICT`    |  409 | 否             | 当前状态不能执行该操作    |
| `INSTANCE_PROCESS_FAILED`    |  422 | 可能           | 进程创建或控制失败        |
| `INSTANCE_COMMAND_REJECTED`  |  422 | 否             | 实例未运行或 stdin 不可用 |
| `INSTANCE_REVISION_MISMATCH` |  412 | 否             | 实例配置 revision 已变化  |
| `INSTANCE_MUST_BE_STOPPED`   |  409 | 否             | 此设置只能在停服时修改    |
| `INSTANCE_RESTART_REQUIRED`  |  409 | 否             | 修改已保存但需要重启生效  |

## 6. 文件与任务错误码

| Code                     | HTTP | 含义                                   |
|--------------------------|-----:|----------------------------------------|
| `FILE_NOT_FOUND`         |  404 | 文件或目录不存在                       |
| `FILE_ALREADY_EXISTS`    |  409 | 目标存在且不允许覆盖                   |
| `FILE_NOT_DIRECTORY`     |  409 | 路径不是目录                           |
| `FILE_NOT_REGULAR`       |  409 | 路径不是普通文件                       |
| `FILE_DIRECTORY_NOT_EMPTY` | 409 | 非递归删除拒绝非空目录                 |
| `FILE_TASK_NOT_FOUND`    |  404 | 文件删除任务不存在                     |
| `FILE_PATH_FORBIDDEN`    |  403 | 路径或符号链接逃逸实例根目录           |
| `FILE_REVISION_MISMATCH` |  412 | 文件 `If-Match` 摘要不一致       |
| `FILE_OPERATION_FAILED`  |  503 | Core 文件操作失败                      |
| `FILE_PATH_INVALID`      |  400 | 路径格式不合法                         |
| `FILE_PATH_ESCAPE`       |  403 | 路径或符号链接逃逸实例根目录           |
| `FILE_TOO_LARGE`         |  413 | 超过端点或配额限制                     |
| `FILE_HASH_MISMATCH`     |  422 | 上传内容摘要不一致                     |
| `FILE_IO_ERROR`          |  422 | Core 文件系统操作失败                  |
| `STORAGE_QUOTA_EXCEEDED` |  422 | 实例/Core 配额不足                     |
| `UPLOAD_NOT_FOUND`       |  404 | 上传不存在或已过期                     |
| `UPLOAD_PART_INVALID`    |  422 | 分块编号、偏移或摘要非法               |
| `TASK_NOT_FOUND`         |  404 | 任务不存在或不可见                     |
| `TASK_NOT_CANCELLABLE`   |  409 | 任务已结束或进入不可取消阶段           |
| `TASK_FAILED`            |  422 | 任务执行失败，details 可含安全的子错误 |

## 7. 环境、扩展、Docker 与调度错误码

| Code                           | HTTP | 含义                           |
|--------------------------------|-----:|--------------------------------|
| `RUNTIME_NOT_FOUND`            |  404 | 受管环境不存在                 |
| `RUNTIME_IN_USE`               |  409 | 环境仍被实例引用               |
| `RUNTIME_UNSUPPORTED`          |  422 | 平台、架构或版本不受支持       |
| `DOWNLOAD_HASH_MISMATCH`       |  422 | 下载摘要不匹配                 |
| `TEMPLATE_UNTRUSTED`           |  403 | 模板签名或来源不可信           |
| `PROVISION_PLAN_EXPIRED`       |  409 | Catalog 变化，需要重新 resolve |
| `CONFIG_NOT_RECOGNIZED`        |  404 | 没有适用的配置提供者           |
| `CONFIG_LOSSY_WRITE_REQUIRED`  |  409 | 写入可能丢失格式，需要显式确认 |
| `EXTENSION_INCOMPATIBLE`       |  422 | Minecraft/加载器/依赖不兼容    |
| `EXTENSION_SOURCE_DENIED`      |  403 | 来源或下载许可不允许           |
| `DOCKER_UNAVAILABLE`           |  503 | Core 未安装或无法连接 Docker   |
| `CONTAINER_POLICY_DENIED`      |  403 | 参数违反 Core 容器安全策略     |
| `IMAGE_IN_USE`                 |  409 | 镜像仍被实例/容器引用          |
| `IMAGE_BUILD_FAILED`           |  422 | 镜像构建失败                   |
| `SCHEDULE_INVALID`             |  400 | Cron、时区、事件或动作非法     |
| `SCHEDULE_EXECUTION_DUPLICATE` |  409 | 相同触发已执行或正在执行       |

## 8. Provider 错误码

| Code                            | HTTP | 含义                        |
|---------------------------------|-----:|-----------------------------|
| `TENANT_SUSPENDED`              |  403 | 租户已暂停                  |
| `PLAN_NOT_AVAILABLE`            |  422 | 套餐在目标区域/节点池不可用 |
| `QUOTA_EXCEEDED`                |  422 | 租户或套餐配额不足          |
| `CAPACITY_UNAVAILABLE`          |  503 | 没有满足硬约束的节点容量    |
| `PLACEMENT_RESERVATION_EXPIRED` |  409 | 放置预留已过期              |
| `SUBSCRIPTION_STATE_CONFLICT`   |  409 | 订阅状态不允许该操作        |
| `PROVISION_FAILED`              |  422 | 自动供应失败                |
| `WEBHOOK_DELIVERY_FAILED`       |  422 | Webhook 达到自动重试上限    |

CPU 调度错误补充：

| Code                       |    HTTP | 含义                                          |
|----------------------------|--------:|-----------------------------------------------|
| `CPU_TOPOLOGY_UNAVAILABLE` |     503 | Core 无法读取 CPU 拓扑                        |
| `CPU_POLICY_UNSUPPORTED`   |     422 | 平台不支持所请求的性能类别或绑定方式          |
| `CPU_CAPACITY_UNAVAILABLE` |     409 | 没有足够的性能核、NUMA 或独占容量             |
| `CPU_AFFINITY_DENIED`      |     403 | Core 权限或容器策略拒绝设置 affinity          |
| `CPU_POLICY_DEGRADED`      | 200/202 | 已应用但未满足全部偏好，详情见 degradedReason |
| `CPU_RESERVATION_CONFLICT` |     409 | 与其他实例独占预留冲突                        |

## 9. Core 协议错误

Core TCP 错误对象不含 HTTP 状态，但复用相同 `code`、`message`、`retryable` 和 `details` 字段。协议层补充：

| Code                           | 行为                           |
|--------------------------------|--------------------------------|
| `PROTOCOL_VERSION_UNSUPPORTED` | 返回错误后关闭连接             |
| `SESSION_NOT_INITIALIZED`      | `session.hello` 前发送业务请求 |
| `DUPLICATE_REQUEST_ID`         | 当前连接复用了 requestId       |
| `REQUEST_DEADLINE_EXCEEDED`    | 请求到达时 deadline 已过       |
| `REQUEST_CANCELLED`            | 请求在执行前或执行中被取消     |
| `FRAME_TOO_LARGE`              | 尽可能记录安全摘要并关闭连接   |
| `INVALID_MESSAGE`              | 非法 JSON 或消息结构，关闭连接 |

握手/解密失败时直接关闭 TCP，不向未认证对端返回结构化错误。

## 10. 未知错误与展示

- 客户端必须能够处理未知 `code`，使用 HTTP 状态和 `retryable` 决定行为。
- WebUI 以本地化文案解释常见错误，并显示短 requestId 供排障；不能直接把内部 message 当作唯一用户提示。
- 任何未知 `5xx` 都不得无限自动重试。查询最多退避重试 3 次，写操作只在具有幂等键时重试。
- 服务端记录完整内部错误链，但 API 只返回安全摘要；生产环境永不返回 Rust backtrace。
