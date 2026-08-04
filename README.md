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

The catalog distinguishes a server profile from a verified installer. Version metadata currently covers Vanilla, Paper, Velocity, Fabric, NeoForge, Forge, Bukkit, Spigot, Purpur, Pufferfish, Folia, Leaf, Mohist, Youer, Silkard, Magma, Sponge, Arclight, CatServer, Waterfall, BungeeCord, Lightfall, Geyser, Bedrock Dedicated Server, PocketMine-MP, Nukkit, and Cloudburst Nukkit providers. NeoForge uses the official Maven XML catalog, Pufferfish aggregates five official Jenkins jobs, Bukkit and Spigot use official Jenkins RSS Atom feeds, Mohist and Youer use MohistMC's public project API, Silkard uses the official GitHub branches API, GitHub release providers require JAR/PHAR assets, BDS uses Mojang's official download-links API for Windows/Linux stable and Preview ZIPs, and Nukkit variants use OpenCollab Maven metadata; Sponge currently reflects legacy official SpongeVanilla releases and Magma primarily exposes development builds. RSS, archive, build, project, and branch metadata do not by themselves verify a directly installable server artifact. Archive layouts, launch recipes, and installation validation remain incremental for every profile.

- Hybrid servers manage plugins and mods separately, with extension directories declared by the template and version rather than assumed globally; the domain catalog resolves those directories per extension kind.
- Panel can now perform a read-only extension scan by template and `PLUGIN`/`MOD` kind, returning each declared directory as a separate file page and treating missing directories as empty pages. Installation, update, deletion, compatibility resolution, and source search remain planned.
- Velocity, Waterfall, BungeeCord, and Lightfall use one-to-many backend topology; Geyser uses one-to-one topology and has dedicated Bedrock-facing management.
- Bedrock Dedicated Server, PocketMine-MP, Nukkit, Cloudburst Nukkit, and Geyser expose dedicated management profiles for RakNet UDP, default port `19132`, configuration files, extension capabilities, and declared extension directories.

### File Management Slice

The first file-management slice is now available through the Core `files` capability and Panel API:

- sandboxed directory listing with pagination;
- 32 KiB chunked reads with full-file SHA-256 and EOF metadata;
- atomic writes up to 1 MiB with optional `ETag`/`If-Match` protection and idempotency keys;
- recursive directory creation and same-instance moves with overwrite and non-empty-directory guards;
- asynchronous file and recursive-directory deletion with explicit `DELETE` confirmation, task polling, and path-safety guards;
- ordered batch file tasks for `MKDIR`, `MOVE`, `WRITE`, and `DELETE`, with per-item progress and partial-failure results;
- asynchronous ZIP archive preparation for up to 128 files or directories, including empty directories and the instance root, with entry progress and atomic output;
- session-based chunked uploads with fixed 1 MiB parts, per-part and full-file SHA-256 checks, ordered offsets, retries, cancellation, and atomic replacement;
- session-based chunked downloads with fixed 1 MiB parts, full-file and per-part SHA-256 metadata, ordered offsets, retryable completed parts, and completion verification;
- binary Panel responses and TypeScript Client methods for the same contract.

Archive creation and session-based large-file downloads are available. Cross-restart transfer resume, snapshots, difference comparison, and unified task-center progress remain planned M3 work.

### Configuration Management Slice

The first configuration provider is now available through Core TCP and the Panel API:

- recursive discovery of UTF-8 `.properties` documents up to 1 MiB;
- JSON Schema/UI Schema values with stable path-derived document IDs and SHA-256 revisions;
- lossless top-level scalar Merge Patch updates for properties files that preserve comments, ordering, and line endings;
- JSON object discovery with typed JSON Schema/UI Schema and nested top-level Merge Patch updates; normalized writes require explicit `allowLossy=true`;
- YAML/YML and TOML object discovery with typed JSON Schema/UI Schema and normalized top-level Merge Patch updates; writes require explicit `allowLossy=true`;
- raw text reads and writes with the same sandbox, ETag, If-Match, and idempotency protections as file management.

Provider-specific schemas and cross-file validation remain planned M3 work.

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
