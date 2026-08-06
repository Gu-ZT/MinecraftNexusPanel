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

应用退出时会停止本地 sidecar。当前版本已交付 Windows x64 独立 sidecar 安装包；托盘、
开机启动、系统密钥环、Windows 签名、自动更新及 Linux/macOS 安装包仍属于后续发布工作。
