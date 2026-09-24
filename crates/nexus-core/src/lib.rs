//! MCNP Core 实现（PLAN.md 第 1、4.5 节）。
//!
//! 职责：实例进程生命周期（状态机事件的唯一生产者）、受管工具链（Java/Node.js/Python）
//! 安装、服务端模板执行、进程包装（HOST/CONTAINER × DIRECT/MCDR）、配置识别、
//! 扩展下载与原子替换、Docker Engine API 管理、CPU 拓扑识别与大核调度、任务调度执行。
//!
//! 对外仅暴露 nexus-protocol 定义的 TCP 接口；不处理终端用户身份。
//!
//! TODO(M1): PSK 握手接入、节点信息、实例列表与启停/命令/日志游标。
