# MCNP Desktop

MCNP Desktop 是带本地 Core/Panel 的独立 Windows x64 桌面版。Tauri 负责系统集成和
sidecar 生命周期，业务页面仍来自共享的 `frontend/app`，不得在本目录复制业务页面。

## 构建安装包

在仓库根目录执行：

```powershell
pnpm install
pnpm desktop:build
```

构建脚本会依次构建 Vue 前端、构建 release `mcnp` 二进制，并将它复制到 Tauri
资源目录。默认产物是：

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

登录页会直接显示首启用户名和密码，不存在固定的默认密码。首位管理员成功登录后，
引导密码会从秘密文件删除；Panel 数据库和长期密钥仍会保留。完整说明见
[`docs/operations/initial-administrator.md`](../../docs/operations/initial-administrator.md)。

关闭主窗口时应用会隐藏到系统托盘，本地 Core/Panel 继续运行。托盘悬浮提示显示当前动态
Panel 地址；双击托盘图标或选择 `Open MCNP` 可恢复主窗口，选择 `Quit MCNP` 才会退出应用并停止本地 sidecar。当前版本已交付
Windows x64 独立 sidecar 安装包。重复启动会转交给已运行的进程并恢复主窗口，不会再次启动
sidecar。sidecar 的 stdout/stderr 会收集到 `%APPDATA%\dev.mcnp.desktop\logs`，日志文件达到
10 MiB 时保留一个轮转副本；Desktop 请求逐行 JSON，并在写盘前递归遮盖已知秘密字段。
设置页可直接打开日志目录，也可以启用当前用户登录时启动；
由登录项启动时不会弹出主窗口，而是直接驻留托盘。其他平台系统密钥环、Windows 签名、自动更新及
Linux/macOS 安装包仍属于后续发布工作。

Windows Desktop 会把原生 refresh token 保存到 Windows Credential Manager，macOS Desktop
使用系统 Keychain；应用重启时通过 Panel 刷新接口换取短期 access token，登出时删除该凭据。
Desktop 还会在 access token 到期前 60 秒轮换 refresh token，系统休眠恢复后会尽快补刷新；
Linux Desktop 使用 keyutils 与 Secret Service 持久组合后端，采用 Rust 加密实现并静态构建
DBus 依赖；凭据服务不可用时拒绝操作，不回退 mock 或明文文件。

Windows x64 发布工作流会生成明确标记为 unsigned 的 NSIS 安装包和 `SHA256SUMS.txt`；在
Authenticode 代码签名完成前不得将其描述为已签名版本。详见
[`docs/operations/desktop-release.md`](../../docs/operations/desktop-release.md)。
