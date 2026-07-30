# MCNP 开发待办

本清单以 [PLAN.md](PLAN.md) 为产品与架构依据，按可交付、可验证的工作项维护。勾选仅表示代码、测试和文档已达到当前阶段的验收标准，不表示后续阶段无需回归验证。

## 当前重点

- [x] 完成 M0：工程契约、配置、日志、请求 ID、协议编解码与 CI。
- [ ] 启动 M1：Core/Panel 最小纵向链路。

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

- [x] Core TCP：Noise NNpsk0 PSK 握手、加密帧、`session.hello`/`session.welcome`、`system.ping`、`system.info` 与持久化 `coreId`。
- [x] Panel：Core 加密连接客户端、Panel HTTP 存活/就绪探针、请求 ID 中间件与 SQLite 初始化基础。
- [x] `all`：预先校验 Core/Panel 监听器并并发运行，不绕过 Core TCP 接口。

- [ ] Core：TCP 监听、Noise PSK 握手与 `session.hello` / `session.welcome`。
- [ ] Core：节点信息、实例内存仓储、实例列表和详情读取。
- [ ] Core：安全测试进程启动、停止、终止与状态事件。
- [ ] Core：实例 stdin 命令、stdout/stderr 游标日志与基础指标。
- [ ] Panel：SQLite 初始化、首位管理员初始化和登录会话。
- [ ] Panel：Core 添加、加密密钥保存、连通性检测和重连状态。
- [ ] Panel：实例代理 REST API、幂等键和统一错误响应。
- [ ] Panel：WebSocket 日志、任务进度和 Core 状态推送。
- [ ] WebUI：登录、Core 切换、实例列表、状态控制和控制台。
- [ ] `all`：单命令同时启动 Panel 与 loopback Core，仍暴露 Core TCP 接口。
- [ ] 集成测试：空数据目录创建实例、运行测试进程、读取日志并安全停止。

## M2：环境与一键搭建

- [ ] Java、Node.js、Python 受管环境发现、安装、校验、删除与缓存。
- [ ] 可信下载清单、SHA-256 校验、平台/架构校验和取消机制。
- [ ] Vanilla、Paper、Velocity、Fabric 安装模板与版本元数据提供方。
- [ ] Direct 与 MCDR 进程包装配置及审计任务。
- [ ] 实例名称、类型、到期、工作目录、启动命令和更新命令设置。

## M3：日常运维

- [ ] properties、YAML、JSON、TOML 配置识别与无损补丁。
- [ ] 实例文件浏览、上传、下载、移动、删除与路径逃逸防护。
- [ ] 模组/插件搜索、解析、安装、更新、删除与兼容性提示。
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
