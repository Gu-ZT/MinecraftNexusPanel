# Minecraft Nexus Panel（MCNP）

面向 Minecraft 服主和运维人员的多节点服务器管理工具。产品与里程碑定义见 [PLAN.md](PLAN.md)。

## 仓库结构

| 目录 | 说明 |
|---|---|
| `apps/nexus` | core / panel / all 三模式 CLI 入口（Rust） |
| `apps/desktop` `apps/mobile` | Tauri 壳（M5 接入） |
| `crates/*` | 领域、协议、Core、Panel、存储、配置六个 Rust crate（空壳，M1 起实现） |
| `frontend/app` | 统一 Vue 3 + TypeScript 应用（当前为全页面 Mock 原型） |
| `frontend/api-client` | API 接口定义 + Mock 实现 + 实时事件 SDK |
| `frontend/ui` | 设计令牌 + Arco Design Vue 主题 + 领域共享组件 |
| `frontend/platform` | Browser/Tauri 能力适配器 |
| `docs/architecture/structure.md` | 结构决策与依赖方向 |

## 快速开始（前端原型）

```bash
pnpm install
pnpm dev        # http://127.0.0.1:5173
```

Mock 账户（密码均为「用户名 + 123」）：

| 账户 | 用户组 | 权限特征 |
|---|---|---|
| `admin` | 管理员 | 全部权限、全部实例 |
| `operator` | 运维组 | 实例控制/终端/文件/配置等，限上海节点 |
| `viewer` | 只读观察 | 只读，仅可见创造测试服 |

```bash
cargo check --workspace     # Rust 空壳编译验证
pnpm typecheck && pnpm lint # 前端静态检查
pnpm build                  # 前端生产构建
```

## 当前状态

- ✅ M0 目录骨架与工程配置
- ✅ 前端全页面原型（Mock 数据层驱动，实例状态机 / 任务进度 / 实时日志均有模拟行为）
- ⏳ 后端实现：M1 起（见 PLAN.md 分阶段交付），Mock 层将被 OpenAPI 生成的真实客户端替换
