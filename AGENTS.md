# MCNP 工作区编码规范

## Git 纪律

1. 每次更改结束后都应提交 commit；提交信息遵循 Conventional Commits（`type(scope): 中文摘要` + `- ` 要点列表）。
2. 中大型更改应拆分为多个 commit：按子系统/改动性质划分边界（如领域层、Mock 层、组件库、页面各占一个提交），每个提交独立可验证、可回滚。
3. 提交前必须运行与改动匹配的验证（`pnpm typecheck` / `pnpm lint` / `pnpm build` / `cargo check`），通过后方可提交。
4. 禁止提交生成产物与依赖目录（`dist/`、`node_modules/`、`components.d.ts` 等）。
5. 推送远程需用户明确要求；获准推送前先 `git fetch`，远程有更新则先 rebase 再推送。

## 界面规范（前端）

1. 任何位置的图标都不要使用 emoji，统一使用 arco-design 的 icon（`@arco-design/web-vue/es/icon`）或 iconpark 官方图标库。
2. 所有面向用户的文本不应直接暴露计划/实现细节（如协议名、里程碑编号、HTTP 状态码、内部字段名），而要是便于用户理解的表述。
3. 各种弹窗需要限制最大高度，正文超出时滚动，避免撑满整个屏幕高度。
4. 表格统一遵守：
   - 除长文本（如描述类）外，列宽由内容撑开，单元格不折行；
   - 描述类长文本超长显示 `...`，鼠标悬停以 tooltip 展示完整内容（优先使用列组件的 `ellipsis tooltip`）；
   - 操作列冻结在最右侧（`fixed="right"`），其余列可横向滚动（`scroll.x` 取列宽总和）；
   - 禁止对 `td` 设置 `max-width: 0` 之类与 Arco 列宽机制（col 元素）冲突的覆盖。

## 代码质量

1. 禁止无 `TODO` 标注的占位实现；禁止空 `catch`；错误必须记录日志或反馈给用户，不得静默兜底。
2. 禁止刻意编写的兼容/兜底代码与防御不存在边界的过度设计；能两三行表达的逻辑不单独抽象。
3. TypeScript 开启 strict；禁止无谓的 `as any` 与 `@ts-ignore`；跨包共享类型一律放在 `@mcnp/api-client` 的 domain 层。
4. 包依赖方向固定：`app → {ui, api-client, platform}`、`ui → api-client`（仅类型）；禁止反向依赖；平台差异只能通过 `PlatformAdapter` 引入。
5. 权限模型：服务端按权限点过滤，前端 `PermissionGate`/按钮禁用仅作为配合；新增接口必须在 Mock 层同步实现权限校验与审计记录。
6. 文档与注释使用简体中文；公开类型、接口与复杂逻辑必须有注释说明动机与约束。
7. 中大型改动完成后，派发审查子代理核查变更，修复其发现的问题后再提交。
