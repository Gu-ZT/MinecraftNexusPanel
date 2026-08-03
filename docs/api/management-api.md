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

## 3. 一键搭建

### 3.1 安装目录

| 方法 | 路径                                              | 权限              | 说明                                 |
|------|---------------------------------------------------|-------------------|--------------------------------------|
| GET  | `/server-catalog/templates`                       | `instance.read`   | Vanilla/Paper/Velocity/Fabric 等模板 |
| GET  | `/server-catalog/templates/{templateId}/versions` | `instance.read`   | Minecraft、加载器与构建版本          |
| POST | `/cores/{coreId}/provision-plans:resolve`         | `instance.create` | 解析依赖、下载量和最终设置           |
| POST | `/cores/{coreId}/instance-provisions`             | `instance.create` | 执行一键搭建                         |
| GET  | `/instance-provisions/{provisionId}`              | 资源可见          | 查询供应状态                         |

执行前先 resolve：

```json
{
  "templateId": "paper",
  "minecraftVersion": "1.21.8",
  "build": "latest",
  "instance": {
    "id": "survival",
    "name": "Survival",
    "workingDirectory": "instances/survival",
    "expiresAt": null
  },
  "runtime": {
    "runtimeId": null,
    "installIfMissing": {
      "kind": "JAVA",
      "distribution": "TEMURIN",
      "majorVersion": 21
    }
  },
  "execution": {
    "runtimeMode": "HOST",
    "supervisorMode": "MCDR"
  }
}
```

`resolve` 返回精确版本、所需空间、下载项、哈希、将要安装的环境、默认启动/更新命令和警告。客户端确认后使用相同 `planHash` 创建
provision；Catalog 变化导致 hash 失效时必须重新确认。

### 3.2 代理子服务器

代理实例的子服务器关系由 Core 保存，Panel 负责鉴权、授权和审计。Velocity、Waterfall、BungeeCord、Lightfall
支持一对多目标；Geyser 只支持一个 Java 后端目标。

| 方法   | 路径                                                               | 说明                         |
|--------|--------------------------------------------------------------------|------------------------------|
| GET    | `/cores/{coreId}/instances/{proxyInstanceId}/proxy-subservers`     | 查询代理后端                 |
| POST   | `/cores/{coreId}/instances/{proxyInstanceId}/proxy-subservers`     | 创建或替换一个后端关系       |
| DELETE | `/cores/{coreId}/instances/{proxyInstanceId}/proxy-subservers/{subserverId}` | 删除后端关系                 |

后端记录包含 `targetInstanceId`、监听目标地址、端口和启用状态。目标实例必须已存在且不是代理实例；Geyser 的第二个目标返回
`PROXY_SUBSERVER_LIMIT_REACHED`。

### 3.3 模板安全

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
| GET   | `.../{instanceId}/config-documents/{documentId}`        | `config.read`  | Schema、UI Schema 与值   |
| PATCH | `.../{instanceId}/config-documents/{documentId}/values` | `config.write` | 按 JSON Merge Patch 修改 |
| GET   | `.../{instanceId}/config-documents/{documentId}/raw`    | `file.read`    | 原始文本                 |
| PUT   | `.../{instanceId}/config-documents/{documentId}/raw`    | `file.write`   | 原始文本编辑             |

结构化写入必须尽量保留注释、顺序、换行和未知字段；无法无损修改时 resolve 响应返回 `lossy=true`，要求用户显式确认。

## 6. 模组与插件

统一类型为 `MOD | PLUGIN | MODPACK | DATAPACK`，来源适配器可支持 Modrinth、CurseForge、Hangar 等。每个适配器必须遵守来源
API、授权和下载限制，禁止绕过需要授权的下载流程。

| 方法   | 路径                                                       | 权限               | 说明             |
|--------|------------------------------------------------------------|--------------------|------------------|
| GET    | `/extension-catalog/search`                                | `extension.read`   | 聚合搜索         |
| GET    | `/extension-catalog/projects/{source}/{projectId}`         | `extension.read`   | 项目详情         |
| GET    | `.../{instanceId}/extensions`                              | `extension.read`   | 已安装清单       |
| POST   | `.../{instanceId}/extension-plans:resolve`                 | `extension.manage` | 依赖与兼容性解析 |
| POST   | `.../{instanceId}/extension-installations`                 | `extension.manage` | 安装解析后的计划 |
| POST   | `.../{instanceId}/extensions/{extensionId}/actions/update` | `extension.manage` | 更新             |
| DELETE | `.../{instanceId}/extensions/{extensionId}`                | `extension.manage` | 删除             |

搜索参数至少包括 `query`、`type`、`source`、`minecraftVersion`、`loader` 和分页。安装记录保存来源、项目 ID、文件
ID、版本、SHA-256、依赖和本地相对路径。

更新前生成 plan，标记 Minecraft/加载器不兼容、依赖缺失、冲突和需要停服的变更。批量更新是单个可回滚任务，替换前保留文件备份。

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
| POST | `.../{instanceId}/archives:create`      | `file.read`  | 创建下载归档      |
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

Core 启动时识别并缓存拓扑。Linux 优先读取 sysfs topology、`cpu_capacity`/arch scale capacity 和 cgroup cpuset；Windows 使用
Processor Relationship/EfficiencyClass；ARM big.LITTLE 使用平台暴露的 capacity。无法可靠识别性能类别时，`PERFORMANCE`
自动模式必须返回 `UNSUPPORTED`，不得按 CPU 编号猜测。

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
