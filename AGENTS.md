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

## Tool Preference For Other Work

For tasks other than searching or reading project files, prefer IDEA MCP tools and the `workspace-agent-bridge` skill
when they provide the relevant operation, such as diagnostics, formatting, refactoring, or other IDE-aware actions.
