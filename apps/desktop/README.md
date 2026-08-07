# MCNP Desktop

MCNP Desktop 是带本地 Core/Panel 的独立 Windows x64 桌面版。Tauri 负责系统集成和
sidecar 生命周期，业务页面仍来自共享的 `frontend/app`，不得在本目录复制业务页面。

## 本地开发

在仓库根目录执行：

```powershell
pnpm desktop:dev
```

开发脚本会先执行 `cargo build -p mcnp --locked`，再把本轮生成的 debug sidecar 路径
显式传给 Tauri。不要直接复用 `target/debug/mcnp`，该文件可能由更早的提交遗留并与
当前 Desktop 壳不兼容。

## 构建安装包

在仓库根目录执行：

```powershell
pnpm install
pnpm desktop:build
```

构建脚本会依次构建 Vue 前端、把 WebUI 复制到 Tauri 资源目录、构建 release `mcnp`
二进制，并将 sidecar 一并复制到资源目录。默认产物是：

```text
target/release/bundle/nsis/MCNP Desktop_0.1.0_x64-setup.exe
```

最终用户只需要安装 EXE，不需要安装 Node.js、Rust、pnpm 或单独下载 MCNP。安装包不
内置 WebView2 Runtime，而是使用目标系统已有的 Microsoft Edge WebView2；Windows 10/11
通常已预装该运行时，精简安装包当前实测约 6.5 MB。

## 首次启动

应用首次启动时会自动启动随包发布的 `mcnp all`，并在本机回环地址搜索可用的 Panel
和 Core 端口。Panel 主密钥、Core PSK 和随机首启管理员密码保存在当前用户目录：

```text
%APPDATA%\dev.mcnp.desktop\data
%APPDATA%\dev.mcnp.desktop\desktop-secrets.json
```

Desktop 会通过 Tauri 持有的随机设备秘密自动换取原生会话，无需用户手动登录。首次会话
建立后，引导密码会从秘密文件删除；Panel 数据库、设备秘密和长期密钥仍会保留。完整说明见
[`docs/operations/initial-administrator.md`](../../docs/operations/initial-administrator.md)。

关闭主窗口时应用会隐藏到系统托盘，本地 Core/Panel 继续运行。托盘悬浮提示显示当前动态
Panel 地址；该地址由 Panel 同源托管完整共享 WebUI，可直接在浏览器访问，也可通过
`Open Web Panel` 菜单打开。浏览器仍需正常登录，Tauri 的设备秘密不会进入浏览器。
双击托盘图标或选择 `Open MCNP` 可恢复主窗口，选择 `Quit MCNP` 才会退出应用并停止本地 sidecar。当前版本已交付
Windows x64 独立 sidecar 安装包。重复启动会转交给已运行的进程并恢复主窗口，不会再次启动
sidecar。sidecar 的 stdout/stderr 会收集到 `%APPDATA%\dev.mcnp.desktop\logs`，日志文件达到
10 MiB 时保留一个轮转副本；Desktop 请求逐行 JSON，并在写盘前递归遮盖已知秘密字段。
设置页可直接打开日志目录，也可以启用当前用户登录时启动；
由登录项启动时不会弹出主窗口，而是直接驻留托盘。其他平台系统密钥环、Windows 签名、自动更新及
Linux/macOS 安装包仍属于后续发布工作。

Windows Desktop 会把原生 refresh token 保存到 Windows Credential Manager，macOS Desktop
使用系统 Keychain。应用重启时优先通过 Panel 刷新接口换取短期 access token；refresh token
失效或缺失时，Tauri 会使用未暴露给 WebView 的设备秘密调用仅限 loopback 的会话引导接口。
Desktop 还会在 access token 到期前 60 秒轮换 refresh token，系统休眠恢复后会尽快补刷新。Linux Desktop 使用 keyutils 与
Secret Service 持久组合后端，采用 Rust 加密实现并静态构建 DBus 依赖；凭据服务不可用时拒绝
操作，不回退 mock 或明文文件。

Windows x64 发布工作流会生成明确标记为 unsigned 的 NSIS 安装包和 `SHA256SUMS.txt`；在
Authenticode 代码签名完成前不得将其描述为已签名版本。详见
[`docs/operations/desktop-release.md`](../../docs/operations/desktop-release.md)。
