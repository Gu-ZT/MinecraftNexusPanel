//! MCNP Core TCP 协议层（PLAN.md 第 4.1 节）。
//!
//! 职责：长度前缀帧编解码、Noise PSK 握手与流量加密、requestId/响应配对、
//! 事件 eventId 与单调序号、协议版本协商、分块传输。v1 载荷为 UTF-8 JSON。
//! 协议细节将冻结于 docs/api/core-tcp.md。
//!
//! 依赖 nexus-domain，不依赖 core/panel 任一实现。
//!
//! TODO(M0): 冻结 v1 帧格式与错误模型，编写编解码往返测试。
