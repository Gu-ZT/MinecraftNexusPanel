# 目录结构决策说明

依据 PLAN.md 第 2、3 节，本仓库为 Rust + 前端 pnpm 混合 monorepo。本文记录各模块职责与依赖方向，作为 M0 结构冻结的说明文档。

## 顶层布局

```text
MinecraftNexusPanel/
├── apps/        # 可启动单元（CLI、Tauri 壳）
├── crates/      # Rust 库 crates（被 apps 组装）
├── frontend/    # 三端共享的 Vue 3 前端 workspace
├── docs/        # API/协议、架构决策、运维文档
├── examples/    # 示例配置与反向代理配置
└── tests/       # 跨进程端到端测试
```

## apps/

| 目录 | 形态 | 职责 |
|---|---|---|
| `nexus` | Cargo bin | core / panel / all 三种模式的统一 CLI 入口 |
| `desktop` | Tauri（M5） | 本地 GUI 壳 + sidecar 生命周期，不复制业务 API |
| `mobile` | Tauri Mobile（M5） | 仅连接 Panel 的移动客户端壳 |

## crates/ 依赖方向

依赖只允许单向流动，禁止环：

```text
nexus-config     nexus-domain
     │                │
     │        nexus-protocol
     │           │       │
     │      nexus-core  nexus-panel ──→ nexus-storage
     └────────┴────┴────────┘
                    │
                apps/nexus（组装）
```

- **nexus-domain**：纯领域类型，零 IO、零框架依赖（PLAN 第 6 节）。
- **nexus-protocol**：Core TCP 帧/握手/版本协商（PLAN 第 4.1 节），Panel 与 Core 共同引用。
- **nexus-core**：实例生命周期与节点侧全部能力（PLAN 第 4.5 节），状态机事件的唯一生产者。
- **nexus-panel**：Web API、鉴权、RBAC、审计、Core 连接池（PLAN 第 4.2~4.4 节）。
- **nexus-storage**：SQLite/PostgreSQL 仓储与迁移，密钥信封加密落库。
- **nexus-config**：配置加载、环境变量与校验，被 core/panel/CLI 共用。

`all` 与 `desktop` 模式不允许私有捷径：Panel 经 loopback TCP 连接内置 Core（PLAN 第 2 节）。

## frontend/ 依赖方向

```text
platform ──→（被注入）──→ app ──→ ui
                     │
                     └──→ api-client
```

- **platform**：Browser/Tauri 能力适配器（令牌存储、API/WS 地址、系统集成）。
- **api-client**：API 类型与接口、实时事件 SDK；当前为 Mock 实现，M1 起由 OpenAPI 生成真实客户端。
- **ui**：设计令牌 + Arco Design Vue 主题定制 + 领域共享组件。
- **app**：页面、路由、store；平台差异只能通过 platform 适配器访问。

## 关键边界（摘自 PLAN.md 第 2 节）

- Core 只信任持有节点 PSK 的 Panel，不处理终端用户身份。
- Panel 是用户、权限、审计与公开 Web API 的唯一权威来源。
- Mobile 永不直连 Core。
