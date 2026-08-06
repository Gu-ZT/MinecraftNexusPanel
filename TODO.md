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
- [x] Core：实例配置仓储持久化到数据目录，支持 `instance.create`、`instance.list`、`instance.get`、输入校验和分页读取。
  - [x] Core 重启时恢复 `instances.json`；无法重新确认旧进程的 `STARTING`/`RUNNING`/`STOPPING` 快照会恢复为 `UNKNOWN`。
  - [x] Core/Panel：`FAILED`/`UNKNOWN` 实例必须携带幂等键和 `confirmation=RESET` 显式复位为 `STOPPED`，避免误接管旧进程。
- [x] Core：安全测试进程启动、stdin 优雅停止、强制终止、异常退出检测与 `instance.state` 事件。
- [x] Panel：Core 加密连接客户端、Panel HTTP 存活/就绪探针、请求 ID 中间件与 SQLite 初始化基础。
- [x] `all`：预先校验 Core/Panel 监听器并并发运行，不绕过 Core TCP 接口。

- [x] Core：TLS 自动/自定义证书、Panel 地址验证策略、Noise PSK 握手与 `session.hello` / `session.welcome`。
- [x] Core：节点信息、实例配置仓储、实例创建、列表和详情读取。
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
  - [x] 提供基岩端 RakNet UDP、默认端口、配置文件、插件能力和扩展目录画像。
  - [x] 解析 Vanilla、Paper、Velocity、Fabric、NeoForge、Forge、Bukkit、Spigot、Purpur、Pufferfish、Folia、Leaf、Mohist、Youer、Silkard、Magma、Sponge、Arclight、CatServer、Waterfall、BungeeCord、Lightfall、Geyser、Bedrock Dedicated Server、PocketMine-MP、Nukkit 和 Cloudburst Nukkit 二十七种模板的官方版本元数据并通过 Panel API 提供统一版本目录。
    - [x] NeoForge 使用官方 Maven XML 版本目录；Pufferfish 使用五个官方 Jenkins job；Leaf 使用官方 GitHub Releases，并过滤无 JAR、草稿、预发布或失败构建。
    - [x] Magma、SpongeVanilla、Arclight、CatServer 和 Lightfall 使用官方 GitHub Releases；BDS 使用 Mojang 官方下载链接 API 解析 Windows/Linux 稳定版和 Preview ZIP；PocketMine-MP 使用 PHAR 资产；Nukkit 和 Cloudburst Nukkit 使用官方 OpenCollab Maven 版本 API。
    - [x] Bukkit 和 Spigot 使用官方 Jenkins RSS Atom feed，解析稳定构建编号和构建页链接；RSS 元数据不代表构建产物已验证可直接安装。
    - [x] Mohist 和 Youer 使用 MohistMC 官方 project API 读取公开版本目录；构建下载与 SHA-256 字段仍需接入版本化安装配方。
    - [x] Silkard 使用官方 GitHub branches API 读取数字开头的版本分支并过滤开发分支；分支元数据不代表已存在可直接安装的发布归档。
  - [x] 执行模板安装。
  - [x] AsyncYouer 和 Lingshu 已确认是非公开服务端；仅保留类型目录和混合端扩展布局建模，不纳入公开版本 provider、归档验证、安装配方或二十七种公开模板统计。
  - [ ] 为已接入版本 provider 的二十七种模板及后续类型补齐按版本验证的归档结构、可执行文件、默认配置、启动参数和更新策略。
  - [ ] 不能把仅有模板目录或版本元数据 provider 误认为完整安装支持。

### M2 服务端类型矩阵

下列类型已经进入 `InstanceKind` 与内置模板目录；“已建模”不等于每个版本都已完成官方元数据和安装验证。

| 分类 | 类型 | 管理约束 |
|------|------|----------|
| Java 原版端 | Vanilla | Java 运行时；无默认模组/插件扩展布局。 |
| Java 模组端 | NeoForge、Forge、Fabric | 模组独立管理；当前通用布局为 `mods/`，但最终目录由模板/版本决定。 |
| Java 插件端 | Bukkit、Spigot、Paper、Purpur、Pufferfish、Folia、Leaf | 插件独立管理；当前通用布局为 `plugins/`，但最终目录由模板/版本决定。 |
| Java 混合端 | Mohist、Magma、Sponge、Arclight、Youer、AsyncYouer、Silkard、CatServer、Lingshu | 插件与模组分别管理；每种端可声明不同目录，不能共用单一默认路径；AsyncYouer/Lingshu 仅为非公开目录画像。 |
| 反向代理端 | Velocity、Waterfall、BungeeCord、Lightfall | 一对多代理；使用独立的子服务器关系，可关联多个非代理实例。 |
| 基岩版反向代理端 | Geyser | 一对一代理；面向 Bedrock 使用 RakNet UDP，并且只关联一个 Java 后端实例。 |
| 基岩版服务端 | Bedrock Dedicated Server、PocketMine-MP、Nukkit、Cloudburst Nukkit | 使用专门的基岩端运维画像；默认端口 `19132`，配置与扩展能力按端区分。 |

### M2 专门管理边界

- 混合端的插件和模组必须在扫描、安装、更新、删除、兼容性提示和审计记录中保持独立的 `ExtensionKind`；目录解析必须消费模板声明，不能在 Panel 中写死全局路径。
- Velocity、Waterfall、BungeeCord、Lightfall 使用一对多子服务器拓扑，Geyser 使用一对一拓扑；子服务器关系需要独立的列表、创建/替换和删除操作，不能伪装成普通实例字段。
- 基岩版端需要独立处理 RakNet UDP 监听、端口占用、`server.properties`/`config.yml` 等配置、插件能力、扩展目录、启动健康检查、备份恢复和版本升级；不能复用只适用于 Java 服务端的探针和配置假设。
- 当前 `BedrockManagementProfile` 已提供传输、默认端口、配置文件和扩展能力画像，Core 已补充专用 Unconnected Ping/Pong 健康检查；完整的基岩端配置编辑、扩展生命周期、监听绑定运维、备份恢复和版本升级仍属于后续 TODO。
- [ ] Direct 与 MCDR 进程包装配置及审计任务。
  - [x] HOST 下支持 DIRECT 和带显式占位符的 MCDR 包装；CONTAINER 配置会被 Core 明确拒绝而不会回退为宿主机执行。
  - [x] Core 记录启动成功/失败、停止和强制终止请求，以及受管进程异常退出；MCDR 包装器异常退出使用专门原因码，并通过 `instance.audit.list`、Panel REST 和 TypeScript Client 查询。
  - [x] Core 将审计记录写入数据目录下的 JSON 文件，启动时恢复并限制最近 2048 条；追加使用同目录临时文件原子替换，损坏文件会拒绝 Core 启动。
  - [x] Panel HTTP 中间件持久化用户 ID、请求 ID、来源 IP、权限结果、方法、路径和状态码；请求体、查询参数、Cookie、Token 与密码不进入审计库，管理员可按最新优先读取最近 10,000 条。
  - [ ] 将 Panel 审计权限从当前管理员门禁扩展为 `audit.read` RBAC、资源范围筛选、归档导出和保留策略配置。
- [x] 实例名称、类型、到期、工作目录、启动命令和更新命令设置。

## M3：日常运维

- [ ] properties、YAML、JSON、TOML 配置识别与无损补丁。
  - [x] Core/Panel `PROPERTIES` 提供者：递归扫描、JSON Schema/UI Schema、SHA-256 revision 和原文读写。
  - [x] `server.properties` 顶层标量 Merge Patch：保留注释、顺序、换行和未修改文本，并使用原子写入与并发校验。
  - [x] JSON provider：递归扫描、类型化 JSON Schema/UI Schema 和顶层 Merge Patch；规范化写入必须显式确认 `allowLossy=true`。
  - [x] YAML/TOML provider：递归扫描、类型化 JSON Schema/UI Schema 和顶层 Merge Patch；规范化写入必须显式确认 `allowLossy=true`。
  - [x] Minecraft `server.properties` provider-specific Schema：常见布尔、整数和难度/模式枚举，以及 `rcon.password` 敏感字段和密码控件。
  - [x] Core/Panel `config.validate`：返回带路径、字段、严重级别和关联位置的实例级诊断。
  - [x] 校验 Java `server.properties` 端口范围、启用 Query/RCON 条件、RCON 密码、`server-ip` 和 `eula.txt`。
  - [x] 校验 Geyser `config.yml` 的 Bedrock/Java 端点，并报告重复监听端口；未知版本字段不误报。
  - [x] JSON/YAML/TOML provider 为嵌套对象和数组生成递归 JSON Schema/UI Schema，前端可按字段层级选择 group、array、number 和 checkbox 控件。
  - [x] WebUI 提供配置文档列表、重新扫描、实例级校验、revision 保存和有损写回确认；按 Schema/UI Schema 渲染递归对象、布尔、数字、枚举、敏感文本和 Schema 明确的同构数组。
  - [ ] 异构数组编辑、版本专用 Schema 和更多跨文件规则；元组、`oneOf`/`anyOf` 或未声明数组项 Schema 当前保持只读。
- [ ] 实例文件浏览、上传、下载、移动、删除与路径逃逸防护。
  - [x] Core 文件沙箱：目录列表、分页游标、32 KiB 分块读取、SHA-256 和 1 MiB 内原子写入。
  - [x] Panel REST 与 TypeScript Client：二进制读取、ETag/If-Match、幂等写入和路径错误映射。
  - [x] 目录创建和同一实例内移动，包含递归目录、覆盖选项和非空目录保护。
  - [x] 文件和递归目录删除：Core 后台任务、`DELETE` 确认、非空目录保护和任务查询。
  - [x] 批量文件任务：支持 `MKDIR`、`MOVE`、`WRITE`、`DELETE`，返回逐项结果、进度和失败索引。
  - [x] Core/Panel 会话化分块上传：临时文件、固定 1 MiB 分片、顺序 offset、重复分片重试、摘要校验、创建会话时的目标 SHA-256 并发校验、4 GiB 单文件上限、16 会话配额、原子提交和放弃。
  - [x] Core/Panel 会话化分块下载：固定 1 MiB 分片、完整文件/分片 SHA-256、顺序 offset、已读分片重试、完成校验、放弃和二进制 HTTP 响应。
  - [x] Core/Panel 异步 ZIP 下载归档准备：最多 128 个源路径、16,384 个递归条目和 4 GiB 未压缩源数据，覆盖文件、目录、空目录和实例根目录，按条目报告进度并原子生成归档。
  - [ ] 跨 Core 重启续传、快照、差异比较和统一任务中心进度。
- [ ] 模组/插件搜索、解析、安装、更新、删除与兼容性提示。
  - [x] `InstallTemplate` 可按独立的 `ExtensionKind` 展开一个或多个声明目录，并保留插件/模组共用目录的类型边界。
  - [x] Panel 按模板声明的扩展目录分别扫描混合端插件与模组，支持多目录、缺失目录空页和模板/实例类型校验。
  - [x] Panel 在模板声明目录边界内通过 Core 原子写入已准备的单个扩展文件，限制 1 MiB 并要求幂等键。
  - [x] Panel 在模板声明目录边界内委托 Core 异步删除单个扩展文件，并要求类型、路径、DELETE 确认和幂等键校验。
  - [x] 共享 TypeScript Client 已暴露扩展扫描、写入、删除和 Bedrock 扩展目录字段；全量 OpenAPI 生成仍属于 M5。
  - [x] 为本地产物持久化扩展安装记录，记录类型、路径、SHA-256、来源和安装时间；更新支持 If-Match 并发保护，删除任务成功后清理记录，失败/超时或同路径已被新安装替换时保留记录。
  - [x] 通过 Modrinth 公共 API 搜索 MOD/PLUGIN 项目，支持 Minecraft 版本、加载器和分页过滤，并返回来源元数据兼容性提示。
  - [x] 读取 Modrinth 项目版本、依赖记录、HTTPS 归档 URL 和 SHA-512 元数据，并按请求版本/加载器返回版本兼容性提示。
  - [x] 解析根项目及 required 依赖的 Modrinth 版本，检测循环/冲突/缺失归档，并通过 `extension-plans:resolve` 返回受限安装前计划。
  - [x] 安装请求重新解析 Modrinth 计划，仅接受 HTTPS 归档，校验声明大小与 SHA-512，通过 Core `transfer-v1` 分片上传到模板声明目录，并持久化每个已提交文件的来源安装记录。
  - [x] 将计划安装改为 Panel 内存异步任务，返回 `202`/`taskId`，提供进度、已提交记录和失败状态查询；任务不跨 Panel 重启恢复。
  - [x] 重复使用同一 Core、实例和扩展类型作用域内的 `Idempotency-Key` 时复用原安装任务，避免重复下载和写入。
  - [x] 为已持久化的 Modrinth 来源扩展增加单个更新动作：重新解析目标版本，只更新根文件，并通过记录 SHA-256 和 Core 上传会话摘要保护并发。
  - [x] Panel 新多文件安装任务在写入前拒绝目标冲突，失败后按文件哈希和安装记录执行补偿删除，并以 `rollbackState` 报告成功或部分回滚。
  - [ ] 补齐 Core 侧统一安装任务、批量更新和更多来源适配器；更新任务失败时仍不伪造整文件恢复。
  - [x] `BedrockManagementProfile` 为 PocketMine-MP、Nukkit 和 Cloudburst Nukkit 暴露 `plugins/`，BDS/Geyser 保持无插件目录。
  - [x] 基岩画像声明 `PROPERTIES`/`YAML` 配置格式和 `UNSUPPORTED`/`PLUGIN_MANIFEST` 扩展兼容性策略。
  - [x] Panel 在写入 Core 前解析 PocketMine-MP PHAR/TAR 和 Nukkit/Cloudburst Nukkit JAR/ZIP 的根 `plugin.yml`，校验 `name`、`main`、`version`、`api`，并支持调用方提供目标 API 列表做精确匹配。
  - [ ] 为不同基岩端补齐目标 API 自动发现、版本 provider 绑定、更多归档格式和升级兼容性矩阵。
- [ ] 代理端与基岩端专门运维：子服务器连通性、Bedrock/RakNet 监听、端口冲突、健康检查、升级和备份恢复。
  - [x] 从实际 Core 节点对已登记代理后端执行受限 TCP 连通性和 Minecraft Java Status 协议检查，分别返回网络状态、协议状态、延迟和错误分类。
  - [x] 从 Core 节点优先读取 BDS/PocketMine/Nukkit 的 `server.properties:server-port` 或 Geyser 的 `config.yml:bedrock.port`，探测配置端口并在失败时回退 `19132`，区分端口可用、已占用和绑定失败。
  - [x] 从 Core 节点读取 BDS/PocketMine/Nukkit 的 `server.properties:server-ip` 或 Geyser 的 `config.yml:bedrock.address`，仅接受 IP 字面量，缺失/非法时回退 `0.0.0.0` 并报告绑定地址来源。
  - [x] 通过 Core 编排代理启停：启动按“后端 -> 代理”、停止按“代理 -> 后端”执行，按目标实例去重并返回逐步状态。
  - [x] 代理动作支持 `includeBackends`、停止超时、后端失败阻断代理启动和部分失败结果；Panel 已暴露对应动作接口。
  - [x] 基岩端专用 RakNet Unconnected Ping/Pong 健康检查：读取配置端点，处理 `0.0.0.0`/`::` 回环探测、超时、无效响应和服务端身份。
  - [ ] 基岩端监听绑定地址、配置/扩展兼容性、升级和备份恢复。
- [ ] Cron/事件计划任务、去重、执行记录和任务中心。
- [ ] RBAC、用户组、实例 scope 与审计日志。

## M4：Docker 与资源治理

- [ ] Docker Engine 能力检测、镜像列表、拉取、删除与构建日志。
- [ ] 容器化实例启动、端口、网络、挂载、环境变量和资源限制。
- [ ] 挂载逃逸、特权容器、Docker socket 与 host network 安全策略。
- [ ] CPU 拓扑识别：物理核、逻辑 CPU、NUMA、性能/能效类别。
  - [x] Core 启动时缓存架构、可见逻辑 CPU、物理核心数量和明确的未知值，并通过 `cpu.topology` 与 Panel `/cpu-topology` 只读暴露。
  - [x] Linux sysfs、进程 cpuset、ARM capacity、NUMA、online/offline 和隔离 CPU 的可信探测；缺失字段保持未知。
  - [ ] Windows Processor Relationship/EfficiencyClass 和其他平台等价探测。
- [ ] CPU policy：AUTO、PERFORMANCE、EFFICIENCY、CUSTOM、严格/降级语义。
  - [x] Domain 校验、Core `cpu.policy.resolve` 和 Panel `cpu-policies:resolve` 只读候选解析。
  - [x] Core/Panel `cpu-reservations`：校验实例 revision，原子检查不重叠 CPU 集合，并提供列表、登记、释放和稳定错误映射。
  - [x] CPU 预留写入 Core 数据目录 `cpu-reservations.json`，启动时恢复并限制为合法 JSON；追加、替换和释放使用同目录临时文件原子替换，持久化失败会回滚内存状态。
  - [x] `CpuPolicy` 纳入实例创建、部分更新和 `instances.json` 持久化；旧实例存档缺少该字段时使用默认 AUTO/SHARED policy。
  - [ ] 实际 host affinity、Docker cpuset 和跨 Core 调度锁。
- [ ] 宿主机 affinity、Docker cpuset、独占预留和冲突检测。
  - [x] Core 内存态独占预留冲突检测和释放。
  - [ ] 宿主机 affinity、Docker cpuset 执行器、跨 Core 调度锁和审计记录。

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
