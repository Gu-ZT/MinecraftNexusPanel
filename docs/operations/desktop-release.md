# Desktop 发布流程

当前已验证并对外产出的桌面包仅包含 Windows x64 NSIS。Linux、macOS、Windows ARM64、
代码签名和自动更新仍未交付，不应在下载页标记为受支持。

## 本地校验和

完成安装包构建后，在仓库根目录执行：

```powershell
pnpm desktop:build
pnpm desktop:checksum
```

`desktop:checksum` 会读取 `target/release/bundle/nsis` 中的发布文件，并生成按文件名排序的
`SHA256SUMS.txt`。重复执行不会把旧清单自身加入哈希。

## GitHub Actions

`.github/workflows/desktop-package.yml` 支持手动运行，也会在推送 `v*` 标签时运行。工作流会：

1. 使用结构化 Cargo metadata 和 Tauri JSON 校验版本一致。
2. 对标签发布要求标签严格等于 `v<Desktop 版本>`。
3. 运行 checksum 脚本测试、Desktop rustfmt、严格 Clippy 和 Rust 测试。
4. 构建 Windows x64 NSIS，并将文件名明确标记为 `unsigned`。
5. 上传安装包与 `SHA256SUMS.txt` 为 Actions artifact。
6. 标签触发时幂等创建或更新同名 GitHub Release，全部资产上传后才退出 draft。

当前产物命名为：

```text
MCNP-Desktop_<version>_windows-x64-unsigned-setup.exe
SHA256SUMS.txt
```

未配置 Authenticode 代码签名之前，不得移除文件名中的 `unsigned`。自动更新同样必须等待
稳定更新端点、签名公钥和已签名更新清单确定后再启用。
