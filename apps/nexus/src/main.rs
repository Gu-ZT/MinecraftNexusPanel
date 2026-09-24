//! MCNP 命令行入口。
//!
//! 依据 PLAN.md 第 1 节，同一二进制支持 `core` / `panel` / `all` 三种运行模式：
//! - `core`：仅启动 Core 服务，管理本机实例并暴露加密 TCP 接口；
//! - `panel`：仅启动 Panel 服务，提供鉴权、RBAC、Web API/WebUI；
//! - `all`：单进程同时运行两者，Panel 仍经 loopback TCP 连接内置 Core。
//!
//! TODO(M1): 实现模式解析（clap）、配置加载（nexus-config）与服务装配。

fn main() {
    // TODO(M1): 解析运行模式并装配 Core/Panel 服务。
    eprintln!("nexus: not implemented yet (see PLAN.md milestone M1)");
    std::process::exit(2);
}
