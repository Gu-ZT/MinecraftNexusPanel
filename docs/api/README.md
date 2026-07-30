# API 设计索引

本目录定义 Minecraft Nexus Panel（MCNP）的外部接口和内部节点协议。实现、客户端和测试都应以这些文档为契约，不应从当前 UI 行为反推
API。

## 文档导航

| 文档                                     | 受众                      | 内容                                         |
|------------------------------------------|---------------------------|----------------------------------------------|
| [`core-tcp.md`](core-tcp.md)             | Core/Panel 开发者         | TCP 安全通道、帧、请求/响应、事件和节点操作  |
| [`web-api.md`](web-api.md)               | Web/Desktop/Mobile 开发者 | REST 资源、鉴权、分页、幂等和主要端点        |
| [`management-api.md`](management-api.md) | 管理功能开发者            | 环境、搭建、配置、扩展、Docker、调度与用户组 |
| [`provider-api.md`](provider-api.md)     | 商业服务商集成开发者      | 多租户、套餐、供应、节点池、用量与 Webhook   |
| [`websocket.md`](websocket.md)           | 实时客户端开发者          | WebSocket 票据、订阅、事件、续传和背压       |
| [`errors.md`](errors.md)                 | 所有开发者                | HTTP 与 Core 协议统一错误码                  |
| [`openapi.yaml`](openapi.yaml)           | 工具链                    | 已冻结 M1 纵向链路的 Web API v1 草案         |

## 版本状态

- API 主版本：`v1`
- Core 协议版本：`1.0`
- 文档状态：Draft
- 兼容承诺：正式发布前允许调整；发布 `1.0.0` 后，同一主版本内只增加可选字段、端点和枚举值。

客户端必须忽略未知 JSON 字段，并对未知枚举值提供 `UNKNOWN` 回退。服务端不得改变既有字段含义，不得把可选字段改为必填字段。

## 通用约定

- JSON 字段使用 `camelCase`，协议方法名使用 `domain.action`。
- ID 使用不透明字符串；当前建议 UUIDv7，客户端不得解析其内部结构。
- 时间使用 UTC RFC 3339，例如 `2026-07-30T10:15:30Z`。
- 字节数使用整数，持续时间字段显式带单位，例如 `timeoutSeconds`。
- HTTP 请求通过 `X-Request-Id` 关联日志；未提供时由 Panel 生成并在响应中返回。
- Core 请求使用 `requestId`；跨 Panel 代理时应沿用 HTTP 请求 ID 或建立可检索映射。
- 日志、Token、密码、PSK 和完整控制台命令不得出现在普通错误详情中。

## 变更流程

1. 先更新 Markdown 契约和 `openapi.yaml`。
2. 为新增或变更行为补充契约测试。
3. 更新服务端，再生成统一 Vue 3/TypeScript 客户端。
4. 兼容前一个稳定 Core 次版本。
5. 破坏性变更只能进入新的 API/协议主版本，并提供迁移说明。
