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
- Panel 已支持按模板和 `PLUGIN`/`MOD` 类型扫描扩展：每个声明目录独立返回文件页，缺失目录返回空页；还可通过带可选 `If-Match` 的受限原子写入放置或更新已准备的本地产物、持久化本地安装元数据，并在显式确认、校验幂等键后异步删除单个文件。扩展目录还可通过 Modrinth 搜索 MOD/PLUGIN 项目，支持 Minecraft 版本、加载器和分页过滤，读取项目版本与依赖记录，并解析受限的 required 依赖计划。安装请求会重新解析计划，创建可查询的异步任务，仅下载 HTTPS Modrinth 归档，校验大小与 SHA-512，通过 Core `transfer-v1` 分片上传到用户选择的声明目录，并写入每个已提交文件的来源安装记录；共享 TypeScript Client 已暴露这些操作和任务查询。多目录安装需要显式选择，任务不跨 Panel 重启恢复，失败回滚、Core 侧统一任务、更多来源适配器和完整更新流程仍待完成。
- Velocity、Waterfall、BungeeCord、Lightfall 使用一对多后端拓扑；Geyser 使用一对一拓扑，并提供专门的基岩版管理能力。
- Bedrock Dedicated Server、PocketMine-MP、Nukkit、Cloudburst Nukkit 与 Geyser 使用专门画像管理 RakNet UDP、默认端口 `19132`、配置文件、扩展能力和声明的扩展目录。

### 当前文件管理能力

首批文件管理能力已经通过 Core `files` capability 和 Panel API 提供：

- 带分页的实例目录列表；
- 单次 32 KiB 的分块读取，返回完整文件 SHA-256 和 EOF 元数据；
- 最大 1 MiB 的原子写入，支持可选 `ETag`/`If-Match` 校验和幂等键；
- 递归创建目录和同一实例内移动，支持覆盖选项并保护非空目录；
- 文件和递归目录异步删除，要求显式 `DELETE` 确认并支持任务查询与路径安全校验；
- 支持 `MKDIR`、`MOVE`、`WRITE`、`DELETE` 的顺序批量文件任务，返回逐项进度和部分失败结果；
- 支持最多 128 个文件或目录的异步 ZIP 归档准备，覆盖空目录和实例根目录，按归档条目报告进度并原子写入输出；
- 支持会话化分块上传，固定 1 MiB 分片，校验分片和完整文件 SHA-256，按序 offset、重复分片重试、放弃和原子替换；
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
- 提供原文读写，并复用文件沙箱、ETag、If-Match 和幂等键保护。

provider-specific Schema 和跨文件校验仍属于 M3 后续工作。

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
