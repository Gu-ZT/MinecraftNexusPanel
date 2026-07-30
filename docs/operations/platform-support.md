# 平台与架构支持

本文区分“当前在原生 CI 上验证”与“计划中的发布支持”。只有同时完成打包、架构检查和适当的运行验证的平台，才能在发行版中标记为受支持。

## 当前 CI 验证

| 平台 | 架构 | GitHub Actions runner | 验证范围 | 状态 |
| --- | --- | --- | --- | --- |
| Windows | x86_64 | `windows-latest` | Rust workspace（含 Tauri 壳）格式、lint、测试；Vue 类型检查和构建；OpenAPI 校验 | 已验证 |
| Linux | x86_64 | `ubuntu-24.04` | 同上，另安装 WebKit/GTK Tauri 构建依赖 | 已验证 |
| macOS | ARM64 | `macos-14` | Rust workspace（含 Tauri 壳）格式、lint、测试；Vue 类型检查和构建；OpenAPI 校验 | 已验证 |

M0 只建立质量检查，不生成安装包、Docker 镜像或发布资产。Desktop 与 Mobile 的原生壳会被 Rust 工作区编译；Mobile 的 Android/iOS SDK 打包不在此阶段执行。

## 未声明发布支持的平台

| 平台 | 架构 | 当前状态 | 在 M7 前需要完成的工作 |
| --- | --- | --- | --- |
| Windows | ARM64 | 尚未在 CI 中编译、打包或运行验证 | 明确 MSVC ARM64 交叉工具链，生成安装包并在 ARM64 设备上进行运行测试 |
| Linux | ARM64 | 尚未在 CI 中编译、打包或运行验证 | 建立原生 ARM64 runner 或可复现交叉打包链路，验证 WebKit/GTK 运行时与产物架构 |
| macOS | x86_64 | 尚未在 CI 中编译、打包或运行验证 | 使用 Intel macOS runner 生成 DMG/App，完成签名、公证和原生冒烟测试 |
| Android | ARM64 | Tauri Mobile 壳已存在，但未配置 Android CI 与签名 | 固定 Android SDK/NDK、生成 APK/AAB、接入签名与设备测试 |
| iOS | ARM64 | Tauri Mobile 壳已存在，但未配置 iOS CI 与签名 | 固定 Xcode/iOS 构建链、配置签名与真机/模拟器测试 |

没有稳定的跨架构 Tauri 打包、签名和产物检查流程之前，不能把“Rust 可以交叉编译”视为用户可用的目标平台支持。

## 本地验证

```powershell
pnpm install --frozen-lockfile
pnpm api:lint
pnpm typecheck
pnpm build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```
