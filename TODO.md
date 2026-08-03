# MCNP 开发待办

本清单以 [PLAN.md](PLAN.md) 为产品与架构依据，按可交付、可验证的工作项维护。勾选仅表示代码、测试和文档已达到当前阶段的验收标准，不表示后续阶段无需回归验证。

## 当前重点

- [x] 完成 M0：工程契约、配置、日志、请求 ID、协议编解码与 CI。
- [x] 启动 M1：Core/Panel 最小纵向链路。
- [x] 完成 M2 受管运行时、可信下载缓存和模板计划执行的基础链路。

## M0：设计冻结与工程骨架

- [x] 建立 Rust workspace、pnpm workspace 与统一 Vue 3 应用结构。
- [x] 建立 Desktop/Mobile Tauri 壳和基础跨平台图标资源。
- [x] 冻结 Core TCP、Web API、管理 API、WebSocket 与错误模型文档。
- [x] 建立运行配置：模式、监听地址、数据目录和日志过滤级别。
- [x] 建立 UUIDv7 `requestId` 领域类型，并在 Core TCP 请求/响应与 Panel HTTP 响应头中传播。
- [x] 建立版本信息与协议版本协商的领域类型。
- [x] 实现长度前缀 Core 帧编解码，并覆盖空帧、超限和截断场景。
- [x] 实现 Core 请求、响应、事件 JSON 消息模型及往返测试。
- [x] 接入结构化日志，确保日志不输出密码、Token 或 PSK。
- [x] 使用 Redocly 校验 `docs/api/openapi.yaml`。
- [x] 添加 GitHub Actions 质量检查：Windows、Linux、macOS 的 Rust/Tauri 与 pnpm 构建。
- [x] 补充平台/架构支持矩阵及当前发布限制说明。

## M1：最小可用纵向链路

### 当前进展

- [x] Core TCP：TLS 证书身份、Noise NNpsk0 PSK 握手、加密帧、`session.hello`/`session.welcome` 与持久化 `coreId`。
- [x] Core：内存实例仓储，支持 `instance.create`、`instance.list`、`instance.get`、输入校验和分页读取。
- [x] Core：安全测试进程启动、stdin 优雅停止、强制终止、异常退出检测与 `instance.state` 事件。
- [x] Panel：Core 加密连接客户端、Panel HTTP 存活/就绪探针、请求 ID 中间件与 SQLite 初始化基础。
- [x] `all`：预先校验 Core/Panel 监听器并并发运行，不绕过 Core TCP 接口。

- [x] Core：TLS 自动/自定义证书、Panel 地址验证策略、Noise PSK 握手与 `session.hello` / `session.welcome`。
- [x] Core：节点信息、实例内存仓储、实例创建、列表和详情读取。
- [x] Core：安全测试进程启动、停止、终止与状态事件。
- [x] Core：实例 stdin 命令、stdout/stderr 游标日志与基础指标。
- [x] Panel：SQLite 初始化、首位管理员初始化和登录会话。
- [x] Panel：Core 添加、加密密钥保存、连通性检测和重连状态。
- [x] Panel：实例代理 REST API、幂等键和统一错误响应。
- [x] Panel：WebSocket 日志、任务进度和 Core 状态推送。
- [x] WebUI：登录、Core 切换、实例列表、状态控制和控制台。
- [x] `all`：单命令同时启动 Panel 与 loopback Core，仍暴露 Core TCP 接口。
- [x] 集成测试：空数据目录创建实例、运行测试进程、读取日志并安全停止。

## M2：环境与一键搭建

- [x] Java、Node.js、Python 受管环境管理。
  - [x] 发现系统与受管目录中的 Java、Node.js、Python，并校验可执行文件和版本。
  - [x] 受管环境安装、删除与缓存。
- [x] 可信下载清单、SHA-256 校验、平台/架构校验和取消机制。
  - [x] 定义带 SHA-256、平台和架构约束的下载清单，并实现 Core 本地校验下载缓存。
  - [x] 下载过程支持取消，并在失败或取消时清理临时文件。
- [ ] Java、代理端、混合端和基岩版服务端安装模板、版本元数据与按类型安装配方。
  - [x] 提供 Java 服务端、插件端、混合端、代理端和基岩端的内置模板目录。
  - [x] 记录混合端的插件/模组目录，以及一对多和一对一代理拓扑。
  - [x] 提供代理子服务器关系管理，并约束一对多与一对一拓扑。
  - [x] 提供基岩端 RakNet UDP、默认端口、配置文件和插件能力画像。
  - [x] 解析 Vanilla、Paper、Velocity、Fabric、NeoForge、Forge、Bukkit、Spigot、Purpur、Pufferfish、Folia、Leaf、Magma、Sponge、Arclight、CatServer、Waterfall、BungeeCord、Lightfall、Geyser、Bedrock Dedicated Server、PocketMine-MP、Nukkit 和 Cloudburst Nukkit 二十四种模板的官方版本元数据并通过 Panel API 提供统一版本目录。
    - [x] NeoForge 使用官方 Maven XML 版本目录；Pufferfish 使用五个官方 Jenkins job；Leaf 使用官方 GitHub Releases，并过滤无 JAR、草稿、预发布或失败构建。
    - [x] Magma、SpongeVanilla、Arclight、CatServer 和 Lightfall 使用官方 GitHub Releases；BDS 使用 Mojang 官方下载链接 API 解析 Windows/Linux 稳定版和 Preview ZIP；PocketMine-MP 使用 PHAR 资产；Nukkit 和 Cloudburst Nukkit 使用官方 OpenCollab Maven 版本 API。
    - [x] Bukkit 和 Spigot 使用官方 Jenkins RSS Atom feed，解析稳定构建编号和构建页链接；RSS 元数据不代表构建产物已验证可直接安装。
  - [x] 执行模板安装。
  - [ ] 为 Mohist、Youer、AsyncYouer、Silkard 和 Lingshu 补齐可验证的官方元数据与安装配方。
  - [ ] 为已接入版本 provider 的二十四种模板及后续类型补齐按版本验证的归档结构、可执行文件、默认配置、启动参数和更新策略。
  - [ ] 不能把仅有模板目录或版本元数据 provider 误认为完整安装支持。

### M2 服务端类型矩阵

下列类型已经进入 `InstanceKind` 与内置模板目录；“已建模”不等于每个版本都已完成官方元数据和安装验证。

| 分类 | 类型 | 管理约束 |
|------|------|----------|
| Java 原版端 | Vanilla | Java 运行时；无默认模组/插件扩展布局。 |
| Java 模组端 | NeoForge、Forge、Fabric | 模组独立管理；当前通用布局为 `mods/`，但最终目录由模板/版本决定。 |
| Java 插件端 | Bukkit、Spigot、Paper、Purpur、Pufferfish、Folia、Leaf | 插件独立管理；当前通用布局为 `plugins/`，但最终目录由模板/版本决定。 |
| Java 混合端 | Mohist、Magma、Sponge、Arclight、Youer、AsyncYouer、Silkard、CatServer、Lingshu | 插件与模组分别管理；每种端可声明不同目录，不能共用单一默认路径。 |
| 反向代理端 | Velocity、Waterfall、BungeeCord、Lightfall | 一对多代理；使用独立的子服务器关系，可关联多个非代理实例。 |
| 基岩版反向代理端 | Geyser | 一对一代理；面向 Bedrock 使用 RakNet UDP，并且只关联一个 Java 后端实例。 |
| 基岩版服务端 | Bedrock Dedicated Server、PocketMine-MP、Nukkit、Cloudburst Nukkit | 使用专门的基岩端运维画像；默认端口 `19132`，配置与扩展能力按端区分。 |

### M2 专门管理边界

- 混合端的插件和模组必须在扫描、安装、更新、删除、兼容性提示和审计记录中保持独立的 `ExtensionKind`；目录解析必须消费模板声明，不能在 Panel 中写死全局路径。
- Velocity、Waterfall、BungeeCord、Lightfall 使用一对多子服务器拓扑，Geyser 使用一对一拓扑；子服务器关系需要独立的列表、创建/替换和删除操作，不能伪装成普通实例字段。
- 基岩版端需要独立处理 RakNet UDP 监听、端口占用、`server.properties`/`config.yml` 等配置、插件能力、扩展目录、启动健康检查、备份恢复和版本升级；不能复用只适用于 Java 服务端的探针和配置假设。
- 当前 `BedrockManagementProfile` 已提供传输、默认端口、配置文件和扩展能力画像；完整的基岩端配置编辑、扩展生命周期和专门运维流程仍属于后续 TODO。
- [ ] Direct 与 MCDR 进程包装配置及审计任务。
- [x] 实例名称、类型、到期、工作目录、启动命令和更新命令设置。

## M3：日常运维

- [ ] properties、YAML、JSON、TOML 配置识别与无损补丁。
  - [x] Core/Panel `PROPERTIES` 提供者：递归扫描、JSON Schema/UI Schema、SHA-256 revision 和原文读写。
  - [x] `server.properties` 顶层标量 Merge Patch：保留注释、顺序、换行和未修改文本，并使用原子写入与并发校验。
  - [x] JSON provider：递归扫描、类型化 JSON Schema/UI Schema 和顶层 Merge Patch；规范化写入必须显式确认 `allowLossy=true`。
  - [x] YAML/TOML provider：递归扫描、类型化 JSON Schema/UI Schema 和顶层 Merge Patch；规范化写入必须显式确认 `allowLossy=true`。
  - [ ] provider-specific Schema 提供者。
  - [ ] 跨文件校验、敏感字段标记和复杂结构化控件。
- [ ] 实例文件浏览、上传、下载、移动、删除与路径逃逸防护。
  - [x] Core 文件沙箱：目录列表、分页游标、32 KiB 分块读取、SHA-256 和 1 MiB 内原子写入。
  - [x] Panel REST 与 TypeScript Client：二进制读取、ETag/If-Match、幂等写入和路径错误映射。
  - [x] 目录创建和同一实例内移动，包含递归目录、覆盖选项和非空目录保护。
  - [x] 文件和递归目录删除：Core 后台任务、`DELETE` 确认、非空目录保护和任务查询。
  - [x] 批量文件任务：支持 `MKDIR`、`MOVE`、`WRITE`、`DELETE`，返回逐项结果、进度和失败索引。
  - [x] Core/Panel 会话化分块上传：临时文件、固定 1 MiB 分片、顺序 offset、重复分片重试、摘要校验、4 GiB 单文件上限、16 会话配额、原子提交和放弃。
  - [x] Core/Panel 会话化分块下载：固定 1 MiB 分片、完整文件/分片 SHA-256、顺序 offset、已读分片重试、完成校验、放弃和二进制 HTTP 响应。
  - [x] Core/Panel 异步 ZIP 下载归档准备：最多 128 个源路径、16,384 个递归条目和 4 GiB 未压缩源数据，覆盖文件、目录、空目录和实例根目录，按条目报告进度并原子生成归档。
  - [ ] 跨 Core 重启续传、快照、差异比较和统一任务中心进度。
- [ ] 模组/插件搜索、解析、安装、更新、删除与兼容性提示。
  - [ ] 按模板声明的扩展目录分别扫描和管理混合端插件与模组。
  - [ ] 为不同基岩端提供插件/扩展目录、配置和版本兼容性策略。
- [ ] 代理端与基岩端专门运维：子服务器连通性、Bedrock/RakNet 监听、端口冲突、健康检查、升级和备份恢复。
- [ ] Cron/事件计划任务、去重、执行记录和任务中心。
- [ ] RBAC、用户组、实例 scope 与审计日志。

## M4：Docker 与资源治理

- [ ] Docker Engine 能力检测、镜像列表、拉取、删除与构建日志。
- [ ] 容器化实例启动、端口、网络、挂载、环境变量和资源限制。
- [ ] 挂载逃逸、特权容器、Docker socket 与 host network 安全策略。
- [ ] CPU 拓扑识别：物理核、逻辑 CPU、NUMA、性能/能效类别。
- [ ] CPU policy：AUTO、PERFORMANCE、EFFICIENCY、CUSTOM、严格/降级语义。
- [ ] 宿主机 affinity、Docker cpuset、独占预留和冲突检测。

## M5：统一客户端

- [ ] 由 OpenAPI 生成共享 TypeScript API Client。
- [ ] 完成 WebUI 全部管理页面和权限驱动交互。
- [ ] Desktop sidecar、托盘、开机启动和安全 WebUI 暴露。
- [ ] Mobile 设备登录、安全存储、生物识别保护和移动控制台。
- [ ] Browser/Tauri 平台适配器与共享状态、表单、实时事件 SDK。

## M6：商业服务商版本

- [ ] Tenant、Plan、Subscription、NodePool、Allocation 与 UsageRecord 数据模型。
- [ ] 多租户隔离、套餐配额、到期策略与可审计资源预留。
- [ ] Provider API、API Key、幂等供应、签名 Webhook 和用量导出。
- [ ] 自动节点放置、性能核预留、容量评分与故障域约束。
- [ ] PostgreSQL、高可用 Panel、对象存储备份和 SLA 仪表盘。

## M7：发布与生态

- [ ] Windows、Linux、macOS 安装包、校验和、签名和自动更新。
- [ ] Android/iOS 构建、签名、商店发布与设备兼容性验证。
- [ ] Docker 镜像、多架构清单和部署示例。
- [ ] Paper、Velocity、Fabric 模板市场与扩展元数据生态。
- [ ] 导入/导出、迁移、灾难恢复和兼容性矩阵。

## 发布前确认

- [ ] 首发支持系统范围与架构。
- [ ] 产品展示名、二进制名、默认端口和域名策略。
- [ ] 社区版与 Provider 模块的授权与交付边界。
- [ ] 开源许可证或专有授权方式。
- [ ] HTTPS 终止、密钥管理和备份存储的默认部署方案。
