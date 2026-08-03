<div align="center">

<img src="assets/mcnp-icon.png" width="256" height="256" alt="Minecraft Nexus Panel icon">

# Minecraft Nexus Panel

**A multi-node Minecraft server management platform for web, desktop, and mobile operations.**

English | [简体中文](README.zh_CN.md)

</div>

## Current M2 Scope

The current M2 domain and template catalog model 29 server, proxy, and Bedrock-facing profiles:

| Profile | Types |
|---------|-------|
| Java vanilla | Vanilla |
| Java modded | NeoForge, Forge, Fabric |
| Java plugins | Bukkit, Spigot, Paper, Purpur, Pufferfish, Folia, Leaf |
| Java hybrid | Mohist, Magma, Sponge, Arclight, Youer, AsyncYouer, Silkard, CatServer, Lingshu |
| Proxies | Velocity, Waterfall, BungeeCord, Lightfall, Geyser |
| Bedrock servers | Bedrock Dedicated Server, PocketMine-MP, Nukkit, Cloudburst Nukkit |

The catalog distinguishes a server profile from a verified installer. Version metadata and installation validation currently cover the initial Vanilla, Paper, Velocity, and Fabric providers; the remaining profiles are modeled for incremental provider and recipe support.

- Hybrid servers manage plugins and mods separately, with extension directories declared by the template and version rather than assumed globally.
- Velocity, Waterfall, BungeeCord, and Lightfall use one-to-many backend topology; Geyser uses one-to-one topology and has dedicated Bedrock-facing management.
- Bedrock Dedicated Server, PocketMine-MP, Nukkit, Cloudburst Nukkit, and Geyser expose dedicated management profiles for RakNet UDP, default port `19132`, configuration files, and extension capabilities.

### File Management Slice

The first file-management slice is now available through the Core `files` capability and Panel API:

- sandboxed directory listing with pagination;
- 32 KiB chunked reads with full-file SHA-256 and EOF metadata;
- atomic writes up to 1 MiB with optional `ETag`/`If-Match` protection and idempotency keys;
- recursive directory creation and same-instance moves with overwrite and non-empty-directory guards;
- asynchronous file and recursive-directory deletion with explicit `DELETE` confirmation, task polling, and path-safety guards;
- ordered batch file tasks for `MKDIR`, `MOVE`, `WRITE`, and `DELETE`, with per-item progress and partial-failure results;
- binary Panel responses and TypeScript Client methods for the same contract.

Task-based large-file transfer, snapshots, and difference comparison remain planned M3 work.

## Project Layout

```text
apps/
  nexus/              Unified CLI entry point for core, panel, and all modes
  desktop/src-tauri/  Tauri Desktop shell
  mobile/src-tauri/   Tauri Mobile shell
crates/
  nexus-domain/       Shared domain types
  nexus-protocol/     Core TCP protocol
  nexus-core/         Node and instance execution capabilities
  nexus-panel/        HTTP, authentication, and Core connection pool
  nexus-storage/      SQLite/PostgreSQL storage implementations
  nexus-config/       Configuration loading and runtime modes
frontend/
  app/                Shared Vue 3 application for Web, Desktop, and Mobile
  api-client/         Package for the generated OpenAPI client
  ui/                 Shared components and design tokens
  platform/           Browser/Tauri platform adapters
```

## Local Commands

```powershell
cargo test --workspace
cargo run -p mcnp -- all

pnpm install
pnpm typecheck
pnpm build
pnpm dev
```

See [PLAN.md](PLAN.md) and the [API documentation](docs/api/README.md) for the protocol design and product scope.
