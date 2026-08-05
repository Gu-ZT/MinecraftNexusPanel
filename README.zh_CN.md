<div align="center">

<img src="assets/mcnp-icon.png" width="256" height="256" alt="Minecraft Nexus Panel 图标">

# Minecraft Nexus Panel

**面向 Web、桌面端与移动端运维的多节点 Minecraft 服务器管理平台。**

[English](README.md) | 简体中文

</div>

## 当前 M2 能力范围

当前 M2 领域模型和模板目录已覆盖 29 类服务端、代理端与面向基岩版的运行画像：

| 分类 | 类型 |
|------|------|
| Java 原版端 | Vanilla |
| Java 模组端 | NeoForge、Forge、Fabric |
| Java 插件端 | Bukkit、Spigot、Paper、Purpur、Pufferfish、Folia、Leaf |
| Java 混合端 | Mohist、Magma、Sponge、Arclight、Youer、AsyncYouer、Silkard、CatServer、Lingshu |
| 反向代理端 | Velocity、Waterfall、BungeeCord、Lightfall、Geyser |
| 基岩版服务端 | Bedrock Dedicated Server、PocketMine-MP、Nukkit、Cloudburst Nukkit |

模板目录与已验证安装器是两个层次：当前已为 Vanilla、Paper、Velocity、Fabric、NeoForge、Forge、Bukkit、Spigot、Purpur、Pufferfish、Folia、Leaf、Mohist、Youer、Silkard、Magma、Sponge、Arclight、CatServer、Waterfall、BungeeCord、Lightfall、Geyser、Bedrock Dedicated Server、PocketMine-MP、Nukkit、Cloudburst Nukkit 接入版本元数据提供方。NeoForge 使用官方 Maven XML 目录，Pufferfish 聚合五个官方 Jenkins job，Bukkit 和 Spigot 使用官方 Jenkins RSS Atom feed，Mohist 和 Youer 使用 MohistMC 官方 project API，Silkard 使用官方 GitHub branches API，GitHub provider 要求存在 JAR/PHAR 资产，BDS 使用 Mojang 官方下载链接 API 解析 Windows/Linux 稳定版和 Preview ZIP，Nukkit 变体使用 OpenCollab Maven 元数据；Sponge 当前对应官方历史 SpongeVanilla Releases，Magma 主要提供开发构建。RSS、归档、构建、project 和 branch 元数据本身不代表服务端产物已验证可直接安装。归档结构、启动配方和安装验证仍需按类型与版本逐项补充。

- 混合端的插件与模组分别管理，扩展目录由模板和版本声明，不能使用全局固定路径；领域目录可按扩展类型分别展开这些目录。
- 基岩插件安装前校验现在会解析 PocketMine-MP PHAR/TAR 和 Nukkit/Cloudburst Nukkit JAR/ZIP 根 `plugin.yml`；调用方可传入精确的目标 Bedrock API 版本，自动 API 发现仍待完成。
- Panel 已支持按模板和 `PLUGIN`/`MOD` 类型扫描扩展：每个声明目录独立返回文件页，缺失目录返回空页；还可通过带可选 `If-Match` 的受限原子写入放置或更新已准备的本地产物、持久化本地安装元数据，并在显式确认、校验幂等键后异步删除单个文件。对应安装记录仅在 Core 删除任务成功后清理；删除失败或超时会保留记录，且旧删除任务不会误删同路径的新安装记录。扩展目录还可通过 Modrinth 搜索 MOD/PLUGIN 项目，支持 Minecraft 版本、加载器和分页过滤，读取项目版本与依赖记录，并解析受限的 required 依赖计划。安装请求会重新解析计划，创建可查询的异步任务，仅下载 HTTPS Modrinth 归档，校验大小与 SHA-512，通过 Core `transfer-v1` 分片上传到用户选择的声明目录，并写入每个已提交文件的来源安装记录。新的多文件安装会在写入前拒绝已存在目标，失败后仅在文件哈希和安装记录仍匹配时补偿删除，并通过 `rollbackState` 返回回滚结果。已持久化的 Modrinth 扩展现在可以启动来源更新任务，重新解析目标版本，只替换根文件，并通过 Core 上传会话的目标摘要保护记录的 SHA-256；共享 TypeScript Client 已暴露这些操作和任务查询。同一 Core、实例、扩展类型和操作重复使用 `Idempotency-Key` 会复用原任务，不会重复下载或写入。多目录安装需要显式选择，任务不跨 Panel 重启恢复，Core 侧统一任务、更多来源适配器和批量更新仍待完成。
- Velocity、Waterfall、BungeeCord、Lightfall 使用一对多后端拓扑；Geyser 使用一对一拓扑，并提供专门的基岩版管理能力。Panel 可要求登记的 Core 节点对每个后端关系执行有界 TCP 连通性和 Minecraft Java Status 协议检查，并分别返回网络状态与协议状态。
- 代理动作可由登记的 Core 编排启用后端实例：启动先处理后端、停止先处理代理，重复目标只执行一次，并支持 `includeBackends`、停止超时和部分失败结果；被后端失败阻断的步骤不会被标记为成功。
- Bedrock Dedicated Server、PocketMine-MP、Nukkit、Cloudburst Nukkit 与 Geyser 使用专门画像管理 RakNet UDP、默认绑定地址、配置文件、扩展能力、声明的扩展目录、配置格式和扩展兼容性策略。Core 会在支持时读取 BDS/PocketMine/Nukkit 的 `server-ip`/`server-port` 或 Geyser 的 `bedrock.address`/`bedrock.port`，只接受 IP 字面量，地址和端口配置分别在不可用或非法时回退到 `0.0.0.0` 与 `19132`，并报告地址/端口来源以及 UDP 绑定可用、已占用或不可用。画像策略仍为声明性元数据；manifest 结构校验和可选目标 API 匹配已经前置到安装阶段，自动发现和完整版本矩阵仍待完成。
- Core 还提供专用基岩健康检查，使用 RakNet Unconnected Ping/Pong 区分已响应、超时、无效响应和探测不可用，并在响应有效时返回服务端身份；绑定地址为未指定地址时使用本机回环探测。该检查独立于 UDP 端口可绑定检查和 Java Status 健康检查。
- 实例启动配置将 HOST/CONTAINER 运行模式与 DIRECT/MCDR 监督模式分开。HOST 支持直接命令和显式的 MCDR `{server}`/`{serverArgs}` 包装模板；容器后端完成前，CONTAINER 只保存配置并由 Core 拒绝启动。
- Core 会将实例定义和运行时快照持久化到 `instances.json`。Core 重启后，原来处于 `STARTING`、`RUNNING` 或 `STOPPING` 的快照会恢复为明确的 `UNKNOWN`；管理员必须使用 `confirmation=RESET` 显式复位后才能再次启动，Core 不会声称已经重新接管旧 PID。
- Core 会记录实例启动成功/失败、停止和强制终止请求，以及受管进程的异常退出；MCDR 包装器退出会记录专用失败原因，并可通过 `instance.audit.list`、Panel REST 和共享 TypeScript Client 查询最新记录。Core 会在 `instance-audit.json` 中通过原子替换持久化最近 2048 条记录，文件损坏时拒绝启动；这些 Core 事实不能替代仍需由 Panel 持久化的用户级审计日志。

### 当前文件管理能力

首批文件管理能力已经通过 Core `files` capability 和 Panel API 提供：

- 带分页的实例目录列表；
- 单次 32 KiB 的分块读取，返回完整文件 SHA-256 和 EOF 元数据；
- 最大 1 MiB 的原子写入，支持可选 `ETag`/`If-Match` 校验和幂等键；
- 递归创建目录和同一实例内移动，支持覆盖选项并保护非空目录；
- 文件和递归目录异步删除，要求显式 `DELETE` 确认并支持任务查询与路径安全校验；
- 支持 `MKDIR`、`MOVE`、`WRITE`、`DELETE` 的顺序批量文件任务，返回逐项进度和部分失败结果；
- 支持最多 128 个文件或目录的异步 ZIP 归档准备，覆盖空目录和实例根目录，按归档条目报告进度并原子写入输出；
- 支持会话化分块上传，固定 1 MiB 分片，校验分片和完整文件 SHA-256，支持目标文件 `ETag`/`If-Match`，按序 offset、重复分片重试、放弃和原子替换；
- 支持会话化分块下载，固定 1 MiB 分片，返回完整文件和分片 SHA-256、顺序 offset、已读分片重试及完成校验；
- Panel 二进制响应以及对应的 TypeScript Client 方法。

ZIP 归档生成和会话化大文件下载已经完成；跨 Core 重启续传、快照、差异比较和统一任务中心进度仍属于 M3 后续工作。

### 当前配置管理能力

首个配置提供者已经通过 Core TCP 和 Panel API 提供：

- 递归发现最大 1 MiB 的 UTF-8 `.properties` 配置文件；
- 返回 JSON Schema、UI Schema、稳定的路径派生文档 ID 和 SHA-256 revision；
- 支持顶层标量 JSON Merge Patch，并保留注释、键顺序和换行风格；
- 支持 JSON 对象扫描、类型化 JSON Schema/UI Schema 和嵌套顶层 Merge Patch；规范化写入必须显式设置 `allowLossy=true`；
- 支持 YAML/YML 和 TOML 对象扫描、类型化 JSON Schema/UI Schema 和规范化顶层 Merge Patch；写入必须显式设置 `allowLossy=true`；
- 为 Minecraft `server.properties` 的常见布尔、整数、难度/模式枚举提供专用元数据，并将 `rcon.password` 标记为敏感密码控件；未知键仍按字符串处理；
- 提供原文读写，并复用文件沙箱、ETag、If-Match 和幂等键保护。
- JSON、YAML、TOML 的嵌套对象和数组现在返回递归 Schema/UI Schema 元数据，客户端可以按字段层级渲染 group、array、number 和 checkbox 控件，但不会把样本值伪装成完整的版本 Schema。
- 提供实例级 `config-documents:validate` 诊断，校验 Java 端口、Query/RCON 设置、`server-ip`、`eula.txt`、Geyser 端点和重复监听端口。

前端完整复杂表单、版本专用 Schema、异构数组编辑和更多跨文件规则仍属于 M3 后续工作。

### 当前 CPU 拓扑能力

Core 现在会在启动时缓存保守的宿主机 CPU 拓扑快照，并通过 `cpu.topology` 与
`GET /api/v1/cores/{coreId}/cpu-topology` 提供查询：

- 返回架构、可见逻辑 CPU，以及平台能够报告时的物理核心数量；
- 性能核/能效核信息不可用时明确返回 `UNKNOWN`；
- 返回探测来源和置信度，不根据 CPU 编号猜测性能类别。

- Linux Core 现在读取 sysfs 拓扑、进程 cpuset、ARM `cpu_capacity`、NUMA、online/offline
  和隔离信息；平台未提供的字段保持明确未知值。
- Core 和 Panel 现在提供带严格/降级结果的 CPU policy 只读预览，以及 Core 内存态
  `cpu-reservations` 登记、列表、冲突检查和释放，并要求实例配置 revision 匹配。
  预留记录只表示已选择不重叠的 CPU ID，不代表宿主机 affinity 或 Docker cpuset 已应用，
  也不会在 Core 重启后恢复。
Windows EfficiencyClass、实际 CPU affinity/cpuset 执行器、持久化预留和实例 policy 持久化仍属于 M4 后续工作。

## 工程布局

```text
apps/
  nexus/              core、panel、all 统一命令行入口
  desktop/src-tauri/  Tauri Desktop 壳
  mobile/src-tauri/   Tauri Mobile 壳
crates/
  nexus-domain/       共享领域类型
  nexus-protocol/     Core TCP 协议
  nexus-core/         节点和实例执行能力
  nexus-panel/        HTTP、鉴权与节点连接池
  nexus-storage/      SQLite/PostgreSQL 存储实现
  nexus-config/       配置加载与运行模式
frontend/
  app/                Web、Desktop、Mobile 统一 Vue 3 应用
  api-client/         OpenAPI 生成客户端的承载包
  ui/                 共享组件和设计令牌
  platform/           Browser/Tauri 平台适配器
```

## 本地命令

```powershell
cargo test --workspace
cargo run -p mcnp -- all

pnpm install
pnpm typecheck
pnpm build
pnpm dev
```

协议设计和产品范围参见 [PLAN.md](PLAN.md) 与 [API 文档](docs/api/README.md)。
