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

模板目录与已验证安装器是两个层次：当前已为 Vanilla、Paper、Velocity、Fabric 接入首批版本元数据提供方；其余类型已完成领域建模，后续逐项补充官方元数据和安装配方。

- 混合端的插件与模组分别管理，扩展目录由模板和版本声明，不能使用全局固定路径。
- Velocity、Waterfall、BungeeCord、Lightfall 使用一对多后端拓扑；Geyser 使用一对一拓扑，并提供专门的基岩版管理能力。
- Bedrock Dedicated Server、PocketMine-MP、Nukkit、Cloudburst Nukkit 与 Geyser 使用专门画像管理 RakNet UDP、默认端口 `19132`、配置文件和扩展能力。

### 当前文件管理能力

首批文件管理能力已经通过 Core `files` capability 和 Panel API 提供：

- 带分页的实例目录列表；
- 单次 32 KiB 的分块读取，返回完整文件 SHA-256 和 EOF 元数据；
- 最大 1 MiB 的原子写入，支持可选 `ETag`/`If-Match` 校验和幂等键；
- Panel 二进制响应以及对应的 TypeScript Client 方法。

目录创建、移动、删除和任务化大文件传输仍属于 M3 后续工作。

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
