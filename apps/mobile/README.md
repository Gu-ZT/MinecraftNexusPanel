# apps/mobile —— Tauri Mobile 壳

依据 PLAN.md 第 1、2 节（M5）：Tauri Mobile 客户端只连接 Panel Web API，
不直连 Core、不接触 Core 密钥与节点拓扑；Refresh Token 由系统安全存储保护。
业务页面复用 `frontend/app` 的同一构建产物。

TODO(M5): 初始化 Tauri Mobile 工程、设备登录与生物识别保护。
