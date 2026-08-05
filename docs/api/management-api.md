# 管理功能 API v1

本文扩展 [`web-api.md`](web-api.md)，定义环境、一键搭建、配置识别、模组/插件、终端、Docker、计划任务和细粒度用户组
API。通用鉴权、分页、错误、幂等及乐观锁规则沿用 Web API。

## 1. 执行边界

- Panel 负责用户鉴权、资源授权、聚合内容源、保存期望配置和审计。
- Core 负责本机工具链、文件、进程、Docker Engine 和下载缓存。
- 所有安装、更新、镜像和批量操作返回 `202 + taskId`。
- 终端是实例 stdin/stdout/stderr，不是 Core 宿主机 Shell。
- API 只接受结构化参数；不得允许客户端拼接任意 Docker CLI 或安装 Shell。

## 2. 环境管理

受管环境类型为 `JAVA | NODEJS | PYTHON`。每个安装由 `runtimeId` 唯一标识，包含版本、发行版、平台、架构、路径、哈希、状态和被哪些实例引用。

| 方法   | 路径                                                  | 权限                 | 说明                   |
|--------|-------------------------------------------------------|----------------------|------------------------|
| GET    | `/runtime-catalog`                                    | `environment.read`   | 查询可安装版本与发行版 |
| GET    | `/cores/{coreId}/runtimes`                            | `environment.read`   | Core 已安装环境        |
| POST   | `/cores/{coreId}/runtime-installations`               | `environment.manage` | 安装指定环境           |
| GET    | `/cores/{coreId}/runtime-installations/{taskId}`      | `environment.read`   | 安装任务               |
| POST   | `/cores/{coreId}/runtimes/{runtimeId}/actions/verify` | `environment.manage` | 校验文件与版本         |
| DELETE | `/cores/{coreId}/runtimes/{runtimeId}`                | `environment.manage` | 删除未被使用的环境     |

安装请求：

```json
{
  "kind": "JAVA",
  "distribution": "TEMURIN",
  "version": "21.0.8+9",
  "architecture": "X86_64",
  "sourceId": "official-adoptium",
  "setAsCoreDefault": false
}
```

- Core 默认安装到自己的 data directory，不修改系统 PATH 或系统包管理器。
- Catalog 条目必须包含 SHA-256；可选签名验证失败时任务必须失败。
- 删除被实例引用的 runtime 返回 `RUNTIME_IN_USE`，除非先迁移引用。
- 实例通过 `runtimeId` 引用环境，不能保存易漂移的“java”字符串作为唯一选择。
- 运行时安装清单必须包含受支持平台/架构、压缩格式和相对可执行文件路径；Core 会在本地再次校验并复用 SHA-256 缓存。
- `runtime.install`、`runtime.verify` 和 `runtime.delete` 均要求 `Idempotency-Key`，安装目录使用临时目录完成原子切换。

`runtime.install` 返回 `taskId` 后由 `/runtime-installations/{taskId}` 查询；任务状态为 `RUNNING | SUCCEEDED | FAILED`，失败原因不会执行目录切换。

## 3. 一键搭建

### 3.1 安装目录

| 方法 | 路径                                              | 权限              | 说明                                 |
|------|---------------------------------------------------|-------------------|--------------------------------------|
| GET  | `/server-catalog/templates`                       | `instance.read`   | Java 原版/模组/插件/混合端、代理端和基岩端模板目录 |
| GET  | `/server-catalog/templates/{templateId}/versions` | `instance.read`   | Minecraft、加载器与构建版本          |
| POST | `/cores/{coreId}/provision-plans:resolve`         | `instance.create` | 解析依赖、下载量和最终设置           |
| POST | `/cores/{coreId}/instance-provisions`             | `instance.create` | 执行一键搭建                         |
| GET  | `/cores/{coreId}/instance-provisions/{taskId}`   | 资源可见          | 查询供应状态                         |

执行前先 resolve：

```json
{
  "templateId": "paper",
  "minecraftVersion": "1.21.8",
  "build": "latest",
  "instanceId": "survival",
  "instanceName": "Survival",
  "instanceKind": "PAPER",
  "instanceDirectory": "instances/survival",
  "expiresAt": null,
  "requiredRuntime": "JAVA",
  "runtimeId": null,
  "archive": {
    "url": "https://downloads.example.invalid/paper.zip",
    "sizeBytes": 1024,
    "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
    "platform": "WINDOWS",
    "architecture": "X86_64"
  },
  "archiveFormat": "ZIP",
  "executablePath": "server.jar",
  "launchArguments": ["-jar", "{server}"],
  "stopCommand": "stop",
  "stopTimeoutSeconds": 30
}
```

`resolve` 返回精确版本、所需空间、下载项、哈希、将要安装的环境、默认启动/更新命令和警告。客户端确认后使用相同 `planHash` 创建
provision；Catalog 变化导致 hash 失效时必须重新确认。

当前执行计划要求显式提供经来源校验的 `archive` 下载清单、压缩格式、归档内可执行文件相对路径和启动参数。Core 会再次校验
SHA-256、平台/架构、实例目录、归档条目和受管运行时；下载失败、实例目录已存在或计划 hash 变化时不会留下半成品目录。

### 3.2 代理子服务器

代理实例的子服务器关系由 Core 保存，Panel 负责鉴权、授权和审计。Velocity、Waterfall、BungeeCord、Lightfall
支持一对多目标；Geyser 只支持一个 Java 后端目标。

| 方法   | 路径                                                               | 说明                         |
|--------|--------------------------------------------------------------------|------------------------------|
| GET    | `/cores/{coreId}/instances/{proxyInstanceId}/proxy-subservers`     | 查询代理后端                 |
| POST   | `/cores/{coreId}/instances/{proxyInstanceId}/proxy-subservers`     | 创建或替换一个后端关系       |
| DELETE | `/cores/{coreId}/instances/{proxyInstanceId}/proxy-subservers/{subserverId}` | 删除后端关系                 |
| POST   | `/cores/{coreId}/instances/{proxyInstanceId}/proxy-subservers/{subserverId}/actions/check` | 从 Core 检查后端连通性 |
| POST   | `/cores/{coreId}/instances/{proxyInstanceId}/proxy-subservers/actions/start` | 按后端再代理顺序启动         |
| POST   | `/cores/{coreId}/instances/{proxyInstanceId}/proxy-subservers/actions/stop`  | 按代理再后端顺序停止         |

后端记录包含 `targetInstanceId`、监听目标地址、端口和启用状态。目标实例必须已存在且不是代理实例；Geyser 的第二个目标返回
`PROXY_SUBSERVER_LIMIT_REACHED`。
连通性检查从登记 Core 节点执行最多 3 秒的 TCP 探测，返回 `DISABLED`、`REACHABLE` 或 `UNREACHABLE`、延迟和受限错误分类；后端不可达不会被转换为 Core 注册错误。

启动和停止动作由 Core 节点执行并返回逐实例 `steps`。请求体可选，`includeBackends` 默认为 `true`；启动时先处理启用的后端，全部成功或已运行后才启动代理，任一后端失败会将代理标记为 `BLOCKED_BACKEND_FAILURE`。停止时先停止代理，再处理启用的后端；后端已停止会被视为成功。停止可设置 `timeoutSeconds`，范围为 `1..=300`，超出范围被拒绝。重复目标只执行一次，未启用关系不参与动作。结果的 `state` 为 `SUCCEEDED` 或 `PARTIAL`，不会伪造未执行步骤的成功；写操作需要 `Idempotency-Key`。

### 3.3 基岩端运维画像

| 方法 | 路径                                                           | 说明                         |
|------|----------------------------------------------------------------|------------------------------|
| GET  | `/cores/{coreId}/instances/{instanceId}/bedrock-profile`       | 查询基岩传输和配置管理能力   |
| POST | `/cores/{coreId}/instances/{instanceId}/bedrock-profile/actions/check-port` | 从 Core 检查配置/默认 RakNet UDP 端口 |
| POST | `/cores/{coreId}/instances/{instanceId}/bedrock-profile/actions/check-health` | 从 Core 执行 RakNet 健康检查 |

画像返回 RakNet UDP、默认端口 `19132`、配置文件列表、插件管理类型和扩展目录。BDS 不声明插件目录，PocketMine-MP、Nukkit 与
Cloudburst Nukkit 声明 `PLUGIN` 和 `plugins/`，Geyser 使用 `config.yml`、不声明插件目录，并通过代理子服务器关系管理唯一 Java 后端。
端口检查从登记 Core 节点优先探测配置端口：BDS、PocketMine-MP、Nukkit/Cloudburst Nukkit 读取 `server.properties` 的 `server-port`，Geyser 读取 `config.yml` 的 `bedrock.port`；配置不可读或无效时回退到画像默认端口 `19132`，并通过 `portSource=CONFIGURED|DEFAULT` 说明来源。结果返回 `AVAILABLE`、`IN_USE` 或 `UNAVAILABLE`，不复用 Java TCP 健康检查语义。

健康检查从登记 Core 节点读取同一配置端点并发送 RakNet Unconnected Ping，等待最多 3 秒的 Pong。未指定绑定地址会使用 `127.0.0.1` 或 `::1` 作为探测地址；响应必须通过 RakNet 魔数和长度校验。结果返回 `RESPONDED`、`UNREACHABLE`、`INVALID_RESPONSE` 或 `UNAVAILABLE`、延迟和服务端身份，并通过 `probeAddress` 区分绑定地址与实际探测目标。它不表示实例进程一定已由 MCNP 启动，也不替代端口绑定检查。

### 3.4 模板安全

- 内置模板随 Panel 版本签名；远程模板源必须配置公钥或显式标记为不可信。
- 模板步骤使用受限 DSL：download、extract、copy、writeConfig、installRuntime，不默认执行任意 Shell。
- 自定义 Shell 步骤只允许管理员创建，UI 明确展示命令并写入审计日志。
- 下载使用临时文件、摘要校验和原子移动；压缩包解压必须防 Zip Slip 和符号链接逃逸。

## 4. 实例设置

实例设置拆分为独立权限域，避免“能改名称”自动获得“能改挂载目录”：

```json
{
  "name": "Survival",
  "type": "PAPER",
  "expiresAt": "2027-01-01T00:00:00Z",
  "workingDirectory": "instances/survival",
  "runtimeId": "java-temurin-21.0.8-x64",
  "launchCommand": {
    "executable": "{runtime.java}",
    "args": [
      "-Xms1G",
      "-Xmx4G",
      "-jar",
      "paper.jar",
      "nogui"
    ],
    "environment": {}
  },
  "updateCommand": {
    "templateAction": "paper.update"
  },
  "execution": {
    "runtimeMode": "HOST",
    "supervisorMode": "DIRECT",
    "mcdr": null,
    "container": null,
    "cpuPolicy": {
      "mode": "PERFORMANCE",
      "minCpus": 2,
      "maxCpus": 2,
      "shareMode": "EXCLUSIVE",
      "strict": true
    }
  }
}
```

| 字段组                                                        | 必需权限                      |
|---------------------------------------------------------------|-------------------------------|
| name、type、expiresAt                                         | `instance.settings.basic`     |
| runtimeId、launchCommand、updateCommand、supervisorMode、mcdr | `instance.settings.launch`    |
| workingDirectory                                              | `instance.settings.path`      |
| runtimeMode、container                                        | `instance.settings.container` |

`PATCH /cores/{coreId}/instances/{instanceId}` 必须按实际修改字段逐组鉴权，并使用 `If-Match`。运行中不可热变更的设置返回
`INSTANCE_RESTART_REQUIRED` 或 `INSTANCE_MUST_BE_STOPPED`。

MCDR 包装器的 `args` 使用精确的 `{server}` 和 `{serverArgs}` 占位符，分别展开实例可执行文件和参数列表；Core 不猜测
具体 MCDR 发行版的命令行。`runtimeMode=CONTAINER` 当前只保存配置，启动时会返回不支持错误，不能伪装成宿主机执行。

## 5. 配置识别

配置提供者识别文件后返回：

- `format`：PROPERTIES、YAML、JSON、TOML 或 provider-specific。
- `schema`：JSON Schema 2020-12，定义类型、枚举、范围和说明。
- `uiSchema`：分组、控件建议、敏感字段和重启要求。
- `values`：结构化值。
- `revision` 与 `contentHash`：并发控制。
- `unmapped`：未识别但必须保留的键或文本区域。

| 方法  | 路径                                                    | 权限           | 说明                     |
|-------|---------------------------------------------------------|----------------|--------------------------|
| GET   | `.../{instanceId}/config-documents`                     | `config.read`  | 已识别配置清单           |
| POST  | `.../{instanceId}/config-documents:scan`                | `config.read`  | 重新扫描                 |
| POST  | `.../{instanceId}/config-documents:validate`            | `config.read`  | 校验跨文件关系和端口/EULA 诊断 |
| GET   | `.../{instanceId}/config-documents/{documentId}`        | `config.read`  | Schema、UI Schema 与值   |
| PATCH | `.../{instanceId}/config-documents/{documentId}/values` | `config.write` | 按 JSON Merge Patch 修改 |
| GET   | `.../{instanceId}/config-documents/{documentId}/raw`    | `file.read`    | 原始文本                 |
| PUT   | `.../{instanceId}/config-documents/{documentId}/raw`    | `file.write`   | 原始文本编辑             |

结构化写入必须尽量保留注释、顺序、换行和未知字段；无法无损修改时 resolve 响应返回 `lossy=true`，要求用户显式确认。

当前可验证 provider 包括 `PROPERTIES`、`JSON`、`YAML` 和 `TOML`：Core 扫描最多 1 MiB 的 UTF-8 `.properties`、`.json`、`.yaml`/`.yml`/`.toml` 文件，使用相对路径 SHA-256 生成 `documentId`，并将内容 SHA-256
同时作为 `revision` 和 `contentHash`。properties 结构化补丁仅处理顶层标量和删除键且始终 `lossy=false`；`server.properties` provider 会为常见布尔、整数和难度/模式枚举提供类型化 Schema/UI Schema，并将 `rcon.password` 标记为密码敏感字段，未知键仍按字符串处理。JSON/YAML/TOML provider 返回类型化 Schema/UI Schema，支持嵌套顶层 Merge Patch，但规范化写入必须显式设置 `allowLossy=true`。raw 读写用于保留 provider 尚未映射的文本区域。
`config-documents:validate` 已提供实例级配置诊断：检查 Java `server.properties` 端口范围、Query/RCON 启用条件、`server-ip` 和 `eula.txt`，并检查 Geyser YAML 的 Bedrock/Java 端点及重复监听端口。未知版本字段保持不变，复杂结构化控件仍属于后续 TODO。

## 6. 模组与插件

统一类型为 `MOD | PLUGIN | MODPACK | DATAPACK`，来源适配器可支持 Modrinth、CurseForge、Hangar 等。每个适配器必须遵守来源
API、授权和下载限制，禁止绕过需要授权的下载流程。

| 方法   | 路径                                                       | 权限               | 说明             |
|--------|------------------------------------------------------------|--------------------|------------------|
| GET    | `/extension-catalog/search`                                | `extension.read`   | 聚合搜索         |
| GET    | `/extension-catalog/projects/{source}/{projectId}`         | `extension.read`   | 项目详情         |
| GET    | `/cores/{coreId}/instances/{instanceId}/extensions`        | `extension.read`   | 按模板声明目录扫描已安装清单 |
| PUT    | `/cores/{coreId}/instances/{instanceId}/extensions`        | `extension.manage` | 在模板目录边界内写入已准备产物 |
| POST   | `/cores/{coreId}/instances/{instanceId}/extension-plans:resolve` | `extension.manage` | 解析根项目及 required 依赖的受限安装前计划 |
| POST   | `/cores/{coreId}/instances/{instanceId}/extensions`        | `extension.manage` | 重新解析并安装 Modrinth 计划 |
| GET    | `/cores/{coreId}/extension-tasks/{taskId}`                 | `extension.read`   | 查询扩展安装任务进度与已提交记录 |
| POST   | `.../{instanceId}/extensions/{extensionId}/actions/update` | `extension.manage` | 更新             |
| DELETE | `/cores/{coreId}/instances/{instanceId}/extensions`        | `extension.manage` | 在模板目录边界内异步删除 |

搜索参数至少包括 `query`、`type`、`source`、`minecraftVersion`、`loader` 和分页。安装记录保存来源、项目 ID、文件
ID、版本、SHA-256、依赖和本地相对路径。

 当前 Panel 已提供来源搜索、项目版本详情、受限依赖计划解析、异步计划安装、安装任务查询和实例扫描接口。`GET /extension-catalog/search` 当前接入 Modrinth，要求 `query`、`type=PLUGIN|MOD`，可选 `source=modrinth`、`minecraftVersion`、`loader`、`limit` 和 `offset`；返回项目支持版本、加载器及基于请求过滤条件的来源兼容性提示。`GET /extension-catalog/projects/{source}/{projectId}` 当前读取 Modrinth 版本、依赖记录和带 SHA-512 的 HTTPS 归档摘要；`POST /cores/{coreId}/instances/{instanceId}/extension-plans:resolve` 会校验模板/实例/扩展类型，递归解析最多 64 个 required 项目并检测版本冲突、缺失项目、循环和无归档，返回安装前计划；这些只读操作不下载或安装文件。`POST /cores/{coreId}/instances/{instanceId}/extensions` 接收同一计划字段、可选 `directory` 和可选 `bedrockApiVersions`，再次解析计划后创建内存安装任务并返回 `202`/`taskId`；后台只下载 HTTPS Modrinth 归档，校验来源声明的大小与 SHA-512，PocketMine-MP PHAR/TAR 和 Nukkit/Cloudburst Nukkit JAR/ZIP 还必须有合法根 `plugin.yml`，提供 `bedrockApiVersions` 时要求与 manifest 的 `api` 有精确交集，然后通过 Core `transfer-v1` 以 1 MiB 分片上传，原子提交后写入对应的来源安装记录。新多文件安装会在写入前拒绝已有目标，失败后仅在文件哈希和安装记录仍匹配时执行补偿删除，任务以 `rollbackState` 报告回滚结果。同一 Core、实例和扩展类型重复使用 `Idempotency-Key` 会复用原任务，不会重复下载或写入。`POST .../{instanceId}/extensions/{extensionId}/actions/update` 会定位已持久化的 Modrinth 来源记录，校验项目、扩展类型、模板目录和可选 `If-Match`，重新解析目标版本后只更新根文件，并以 `EXTENSION_UPDATE` 任务返回进度。`GET /cores/{coreId}/extension-tasks/{taskId}` 返回进度、回滚状态、已提交记录和失败状态；任务不跨 Panel 重启恢复。模板为同一扩展类型声明多个目录时必须显式选择目录，插件和模组始终使用独立的 `kind`。自动 Bedrock API 发现、Core 侧统一任务和批量更新动作仍未提供。
实例扫描调用时必须传入 `templateId` 和 `kind=PLUGIN|MOD`；Panel 校验模板与实例类型一致后，按
`InstallTemplateExtensionLayout` 声明的目录分别读取 Core 文件页。混合端的插件和模组不会合并，模板声明多个目录时也会分别返回；不存在的目录返回空页。
同一路径的 `DELETE` 操作要求额外传入 `path`、`confirmation=DELETE` 和合法的 `Idempotency-Key`，只允许删除所选模板和扩展类型声明目录下的单个文件，并返回 Core 异步文件任务。
同一路径的 `PUT` 操作接收不超过 1 MiB 的 `application/octet-stream`，要求传入 `path` 和合法的 `Idempotency-Key`，可用 `If-Match` 校验已有文件 SHA-256，通过 Core 原子写入把已准备产物放到声明目录内，并持久化 `LOCAL` 来源、SHA-256、路径和安装时间。删除操作仅在 Core 删除任务成功后清理对应记录；任务失败、超时或记录已被新的同路径安装替换时保留记录，避免把未删除或新安装的文件误记为已删除。Core 统一安装任务、批量更新、完整更新流程和安装级兼容性校验仍未提供。

更新前生成 plan，标记 Minecraft/加载器不兼容、依赖缺失、冲突和需要停服的变更。当前单个来源扩展更新只替换根文件并使用记录摘要保护并发；批量更新仍应实现为单个可回滚任务，替换前保留文件备份。

## 7. 终端

- 历史：`GET .../{instanceId}/logs?after=<cursor>`。
- 输入：`POST .../{instanceId}/commands`。
- 实时：WebSocket topic `instance/{coreId}/{instanceId}/console`。
- 输入长度最大 8 KiB；不得写入访问日志或普通审计详情。
- 控制台输出作为不可信纯文本渲染，ANSI 解析器必须过滤 OSC、链接和控制序列。
- 多人同时打开终端时广播输入者的用户 ID 和时间，但不广播敏感命令正文到审计事件。

打开终端时，客户端先通过 REST 获取历史页和 `eventCursor`，完成历史渲染后再用该游标订阅 WebSocket 控制台 topic。
`nextCursor` 只用于历史分页。WS/WSS 只发送订阅后的增量输出，不重复发送完整历史；发生游标过期或缺口时停止拼接实时流，重新通过
REST 建立快照和游标基线。Panel 由 HTTPS 提供时使用 WSS，否则使用 WS。浏览器和移动端不直接连接 Core TCP。

## 8. 文件管理

基础文件端点见 [`web-api.md`](web-api.md)。补充操作：

| 方法 | 路径                                    | 权限         | 说明              |
|------|-----------------------------------------|--------------|-------------------|
| POST | `.../{instanceId}/archives:extract`     | `file.write` | 安全解压          |
| POST | `.../{instanceId}/archives`             | `file.write` | 异步创建 ZIP 下载归档 |
| POST | `.../{instanceId}/file-actions/copy`    | `file.write` | Core 内复制       |
| GET  | `.../{instanceId}/file-content:preview` | `file.read`  | 限长文本/图片预览 |

文件 API 默认限制在实例工作目录。即使用户有 `file.write`，也不能修改 Core 配置、其他实例、受管 runtime、Docker socket 或
Panel 数据。

## 9. Docker 镜像与容器

### 9.1 镜像

| 方法            | 路径                                     | 权限           | 说明                       |
|-----------------|------------------------------------------|----------------|----------------------------|
| GET             | `/cores/{coreId}/images`                 | `image.read`   | 本地镜像                   |
| POST            | `/cores/{coreId}/image-pulls`            | `image.manage` | 按 tag/digest 拉取或更新   |
| DELETE          | `/cores/{coreId}/images/{imageId}`       | `image.manage` | 删除未使用镜像             |
| POST            | `/cores/{coreId}/image-builds`           | `image.build`  | 由上传上下文或实例目录构建 |
| GET             | `/cores/{coreId}/image-builds/{buildId}` | `image.read`   | 构建状态和结果             |
| GET/POST/DELETE | `/registry-credentials[/{id}]`           | `image.manage` | 私有仓库凭据               |

API 响应不回传 registry 密码/token。镜像优先保存不可变 digest；“更新”是重新解析 tag 并拉取，不能静默改变运行中容器。

### 9.2 容器设置

```json
{
  "image": "registry.example.com/minecraft/java21@sha256:...",
  "command": null,
  "environment": {
    "EULA": "TRUE"
  },
  "ports": [
    {
      "containerPort": 25565,
      "hostIp": "0.0.0.0",
      "hostPort": 25565,
      "protocol": "TCP"
    }
  ],
  "mounts": [
    {
      "source": "instances/survival",
      "target": "/data",
      "readOnly": false
    }
  ],
  "resources": {
    "cpuCores": 2,
    "memoryBytes": 4294967296,
    "pidsLimit": 512
  },
  "networkMode": "BRIDGE",
  "restartPolicy": "NO",
  "readOnlyRootFilesystem": false
}
```

- 禁止 `privileged`、host PID/IPC、挂载 Docker socket、挂载任意绝对路径和未授权 host network。
- Core 管理实例生命周期，Docker restart policy 默认必须为 `NO`，避免双重控制。
- 容器名、标签和卷带 MCNP instance ID；Core 重启后通过标签重新关联。
- 容器模式下的 MCDR 必须包含在镜像/模板中，Core 不跨容器注入宿主机进程。

## 10. 大核调度与 CPU 亲和

### 10.1 拓扑识别

| 方法 | 路径                                        | 权限                    | 说明                                      |
|------|---------------------------------------------|-------------------------|-------------------------------------------|
| GET  | `/cores/{coreId}/cpu-topology`              | `core.read`             | CPU 包、物理核、逻辑 CPU、NUMA 和性能类别 |
| POST | `/cores/{coreId}/cpu-policies:resolve`      | `core.read`             | 预览策略可绑定的 CPU 与冲突               |
| GET  | `/cores/{coreId}/cpu-reservations`          | `core.read`             | 当前独占/预留情况                         |
| POST | `/cores/{coreId}/cpu-reservations:release`  | `core.manage`           | 释放失效预留                              |
| GET  | `.../{instanceId}/cpu-policy`               | `instance.read`         | 实例请求、应用和降级状态                  |
| PUT  | `.../{instanceId}/cpu-policy`               | `instance.settings.cpu` | 修改实例 CPU policy                       |
| POST | `.../{instanceId}/actions/apply-cpu-policy` | `instance.settings.cpu` | 停机后重新应用 affinity                   |

Core 启动时识别并缓存拓扑，并通过上表的 `GET /cores/{coreId}/cpu-topology` 返回架构、逻辑 CPU、物理核心数量、可用集合和探测置信度。
Linux 实现读取 sysfs 的 possible/online、进程 `Cpus_allowed_list`、物理核心、NUMA、隔离集合和 `core_type`/`cpu_capacity`；
性能类别、NUMA、隔离状态在未被平台可靠报告时返回未知，不能按 CPU 编号猜测。Windows Processor Relationship/EfficiencyClass、
其他平台的等价能力、host affinity 和独占预留仍需逐平台实现。`POST /cores/{coreId}/cpu-policies:resolve` 只返回
候选/建议集合、冲突和降级原因，不表示资源已经应用。

拓扑示例：

```json
{
  "architecture": "x86_64",
  "logicalCpus": [
    {
      "id": 0,
      "physicalCoreId": "0",
      "performanceClass": "PERFORMANCE",
      "online": true,
      "isolated": false,
      "numaNode": 0
    },
    {
      "id": 1,
      "physicalCoreId": "0",
      "performanceClass": "PERFORMANCE",
      "online": true,
      "isolated": false,
      "numaNode": 0
    },
    {
      "id": 8,
      "physicalCoreId": "8",
      "performanceClass": "EFFICIENCY",
      "online": true,
      "isolated": false,
      "numaNode": 0
    }
  ],
  "available": {
    "performanceCpuIds": [
      0,
      1
    ],
    "efficiencyCpuIds": [
      8
    ]
  },
  "detection": {
    "source": "LINUX_CPU_CAPACITY",
    "confidence": "HIGH"
  }
}
```

`isolated: null` 表示平台没有报告可验证的隔离集合；它不等价于 `false`，严格 CPU policy 必须显式处理该未知状态。

### 10.2 Policy

```json
{
  "mode": "PERFORMANCE",
  "requestedCpuIds": [],
  "minCpus": 2,
  "maxCpus": 2,
  "preferPhysicalCores": true,
  "numaNode": null,
  "shareMode": "EXCLUSIVE",
  "strict": true
}
```

- `mode`：`AUTO | PERFORMANCE | EFFICIENCY | CUSTOM`。
- `PERFORMANCE` 表示优先大核/性能核，不等于固定 CPU 编号。
- `CUSTOM` 才使用 `requestedCpuIds`；Core 必须确认 CPU online、在允许 cpuset 内且没有越界。
- `shareMode=EXCLUSIVE` 创建 CpuReservation，不能与其他独占实例重叠；`SHARED` 只设置 affinity，不阻止其他实例使用。
- `strict=true` 时无法满足最小核数、性能类别、NUMA 或独占条件返回 `CPU_CAPACITY_UNAVAILABLE`；`strict=false` 可使用较低等级并返回
  `DEGRADED`。
- 实例运行中修改 affinity 默认需要重启；允许在线 apply 的平台必须在响应中标注是否已应用。

响应状态：

```json
{
  "requested": {
    "mode": "PERFORMANCE",
    "minCpus": 2,
    "strict": true
  },
  "status": "APPLIED",
  "appliedCpuIds": [
    0,
    1
  ],
  "performanceClass": "PERFORMANCE",
  "reservationId": "0198...",
  "degradedReason": null,
  "appliedAt": "2026-07-30T10:15:31Z"
}
```

状态为 `PENDING | APPLIED | DEGRADED | UNSUPPORTED | FAILED`。Panel/WebUI 必须同时展示 requested 与 applied，不能只显示“已绑定大核”。

### 10.3 执行与容器

- 宿主机模式使用平台进程 affinity；Core 必须在子进程启动前设置，避免启动初期跑到错误 CPU。
- Docker 模式映射为 `cpuset-cpus`，NUMA 可用时同时设置 `cpuset-mems`；容器 Engine 不支持时按 strict 规则失败或降级。
- Windows Docker Linux containers、macOS 虚拟化和部分云主机可能无法把宿主机 P/E 类别传入容器，必须标记 `UNKNOWN/DEGRADED`。
- Core 重启后根据 reservation ID 和容器标签恢复关联；不能只依赖 PID。
- CPU reservation 与商业套餐配额、NodePool 容量共同校验，不能通过普通实例 API 绕过。

## 11. 计划任务

任务类型：

- `CRON`：Cron 表达式 + IANA 时区。
- `EVENT`：实例 started/stopped/crashed、玩家数阈值、备份完成、到期临近等事件。

动作包括 start、stop、restart、command、backup、update、webhook 和组合的有序动作；危险动作受创建者权限快照与执行时权限/策略双重校验。

| 方法             | 路径                                                  | 权限                   | 说明         |
|------------------|-------------------------------------------------------|------------------------|--------------|
| GET/POST         | `.../{instanceId}/schedules`                          | `schedule.read/manage` | 列表或创建   |
| GET/PATCH/DELETE | `.../{instanceId}/schedules/{scheduleId}`             | `schedule.read/manage` | 单计划管理   |
| POST             | `.../{instanceId}/schedules/{scheduleId}/actions/run` | `schedule.manage`      | 手动测试执行 |
| GET              | `.../{instanceId}/schedule-executions`                | `schedule.read`        | 执行历史     |

Cron 必须定义 DST 策略。事件触发使用 `eventId + scheduleId` 去重；服务停机期间的错过策略为
`SKIP | RUN_ONCE | CATCH_UP_LIMITED`。

## 12. 用户组与实例范围

| 方法             | 路径                        | 权限               | 说明                              |
|------------------|-----------------------------|--------------------|-----------------------------------|
| GET/POST         | `/groups`                   | `user.read/manage` | 用户组列表或创建                  |
| GET/PATCH/DELETE | `/groups/{groupId}`         | `user.read/manage` | 用户组管理                        |
| PUT              | `/groups/{groupId}/members` | `user.manage`      | 设置成员                          |
| PUT              | `/groups/{groupId}/grants`  | `user.manage`      | 设置权限和资源范围                |
| GET              | `/permissions`              | `user.read`        | 当前版本权限目录                  |
| POST             | `/authorization:check`      | 已登录             | UI 批量检查动作，不替代服务端鉴权 |

授权示例：

```json
{
  "permissions": [
    "instance.read",
    "instance.control",
    "instance.console.read",
    "instance.console.write",
    "config.read"
  ],
  "instanceScopes": [
    {
      "kind": "INSTANCE",
      "coreId": "core-a",
      "instanceId": "survival"
    },
    {
      "kind": "TAG",
      "coreTags": [
        "customer-a"
      ],
      "instanceTags": [
        "shared"
      ]
    }
  ]
}
```

该用户组能查看、启停和操作指定实例终端，但没有 `file.read/write`，也没有任何 `instance.settings.*`
，因此不能访问文件或修改启动命令、工作目录和容器设置。

## 13. OpenAPI 冻结策略

本文覆盖 M2-M4 的完整资源边界。端点进入对应里程碑实现前，按批次合入 `openapi.yaml` 并生成 Vue 3 TypeScript Client；尚未合入
OpenAPI 的端点属于设计草案，不承诺字段稳定。
