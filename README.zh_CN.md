<div align="center">

<img src="assets/mcnp-icon.png" width="256" height="256" alt="Minecraft Nexus Panel 图标">

# Minecraft Nexus Panel

**面向 Web、桌面端与移动端运维的多节点 Minecraft 服务器管理平台。**

[English](README.md) | 简体中文

</div>

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
