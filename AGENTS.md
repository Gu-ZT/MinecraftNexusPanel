1. Prefer self-explanatory code, clear names, and small focused functions.
2. Avoid comments that merely restate what the code already says.
3. Add comments only when they explain non-obvious intent, constraints, or tradeoffs.
4. Except for utility functions, each source file must declare only one class. Multiple classes in the same file are
   strictly forbidden; a class's `companion object` and/or inner class (es) may remain in the same file.
5. Wildcard imports such as `import package.*` are forbidden. Always use explicit imports.
6. Fully qualified names are forbidden in source code. Use explicit imports and avoid introducing conflicting simple
   names in the same context whenever possible.

## Source Lookup Rules

1. Use CLI tools for reading project files and `rg` for searching project file names or contents.
2. Do not use IDEA MCP tools or the `workspace-agent-bridge` skill for project file lookup or content search.
3. When project source lookup cannot be satisfied locally, search source JARs under `~/.gradle/caches/`.

## Minecraft Nexus Panel Project Facts

1. `PLAN.md` describes product architecture and staged targets; `TODO.md` records delivery status. Code and tests are
   the final authority for whether a capability is implemented.
2. `InstanceKind` is the authoritative server-type vocabulary. The current M2 catalog includes:
    - Vanilla as the Java vanilla server.
    - NeoForge, Forge, and Fabric as Java modded servers.
    - Bukkit, Spigot, Paper, Purpur, Pufferfish, Folia, and Leaf as Java plugin servers.
    - Mohist, Magma, Sponge, Arclight, Youer, Silkard, and CatServer as hybrid servers.
    - Velocity, Waterfall, BungeeCord, and Lightfall as one-to-many Java proxies, plus Geyser as a one-to-one
      Bedrock-facing proxy.
    - Bedrock Dedicated Server, PocketMine-MP, Nukkit, and Cloudburst Nukkit as Bedrock servers.
3. Hybrid servers must manage plugins and mods as separate extension kinds. Use `InstallTemplateExtensionLayout` to
   resolve one or more directories per kind; never assume that every server uses `plugins/` or `mods/`, and do not
   collapse plugin and mod records merely because a vendor currently stores both in one directory.
4. Proxy topology is part of the domain model. `ProxyTopology::OneToMany` applies to Velocity, Waterfall, BungeeCord,
   and Lightfall; `ProxyTopology::OneToOne` applies to Geyser. Manage backend relationships through `ProxySubserver`,
   validate that targets are existing non-proxy instances, and enforce the topology cardinality in Core.
5. Bedrock-facing operations require a `BedrockManagementProfile` rather than Java assumptions. The current profile
   records RakNet UDP, default port `19132`, configuration files, and extension capability: BDS uses `server.properties`
   without a plugin kind; PocketMine-MP and Nukkit/Cloudburst Nukkit expose plugin management; Geyser uses `config.yml`
   and its single Java backend relation.
6. When documenting or implementing a server type, distinguish a catalog/domain profile from a verified installer.
   Version metadata providers, archive layouts, launch commands, configuration paths, extension directories, health
   checks, and update procedures may be added incrementally per type and version.
7. The shared Vue frontend uses Arco Design for standard controls and icons, Vue Router for Core/instance/view context,
   and `system`/`light`/`dark` theme preferences. User-facing text belongs in
   `frontend/app/src/locales/<locale-code>.json`; adding a locale file must automatically expose it through the language
   menu without a centralized registration edit.

## Tool Preference For Other Work

For tasks other than searching or reading project files, prefer IDEA MCP tools and the `workspace-agent-bridge` skill
when they provide the relevant operation, such as diagnostics, formatting, refactoring, or other IDE-aware actions.

## Steps

After completing each section, commit the changes according to the conventional-commits specification and push to the
remote. Before pushing, check the commit messages to avoid formatting issues. You may connect to the GugleRAG MCP to
read/write the Minecraft Nexus Panel knowledge base. Any information you consider important can be recorded. Add
sufficient Chinese comments and rustdoc to all code.
