#![deny(unsafe_code)]
#![deny(clippy::dbg_macro, clippy::todo, clippy::unwrap_used)]
//! Windows CPU 拓扑系统调用的最小安全适配层。
//!
//! workspace 业务 crate 禁止 `unsafe`。本 crate 将 Windows 可变长 FFI 缓冲区
//! 立即复制为拥有所有权的安全记录，避免裸指针和平台结构泄漏到 Core。

#[cfg(windows)]
mod processor_core;

#[cfg(windows)]
pub use processor_core::ProcessorCore;
#[cfg(windows)]
pub use processor_core::processor_cores;
