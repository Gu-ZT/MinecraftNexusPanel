<div align="center">

<img src="assets/mcnp-icon.png" width="256" height="256" alt="Minecraft Nexus Panel icon">

# Minecraft Nexus Panel

**A multi-node Minecraft server management platform for web, desktop, and mobile operations.**

English | [简体中文](README.zh_CN.md)

</div>

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
