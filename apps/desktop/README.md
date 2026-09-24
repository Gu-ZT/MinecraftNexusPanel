# apps/desktop —— Tauri Desktop 壳

依据 PLAN.md 第 1、7 节（M5）：Tauri Desktop 提供本地 GUI 外壳、Core/Panel sidecar
生命周期管理、托盘与开机启动。业务页面复用 `frontend/app` 的同一构建产物，
Tauri 仅做系统集成，不复制业务 API。

TODO(M5): 初始化 Tauri 工程与 sidecar 生命周期管理。
