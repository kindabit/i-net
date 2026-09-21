# case_011 导出与 KeePass 导入（修订版）

> 本文件是 `case_011_导出与KeePass导入.md` 的修订版，记录并固化了与实测一致的计划内容；
> 修订点见文末「修订记录」。正式脚本：`e2e\script\case_011\case_011.js`。

## 测试主题

覆盖数据库导出的三种模式（不包含字段 / 包含字段但不包含字段值 / 包含字段且包含字段值）与
KeePass 2.0 数据导入的两条路线（导入单个画布、按分组生成多个画布），以及导入的各失败路径
（未选文件、未输密码、错误密码、非 .kdbx 文件被过滤、伪 .kdbx 文件无效）。
本用例验证「明文导出 → 内容分级 → KeePass 解密导入 → 新画布生成」链路。

## 前置条件

- 复制 fixture `base` 到 `<case>\output\data` 并以该目录启动应用
- 解锁 `zz-e2e-base` / `e2e-password`，位于根画布（fixture 的 registry `lastScene` 指向根画布）
- 界面语言：中文
- 准备步骤：在「账号 A」编辑对话框中添加字段 `口令`（类型「单行文本」）值 `secret-export-123`；
  在「备注 B」添加字段 `备注字段`（类型「单行文本」）值 `note-export-value`
- 脚本资产（位于 `e2e\script\case_011\`）：
  - `e2e-test.kdbx`：KDBX 4.0 / Argon2id（M=8 MiB），Master Password `e2e-kdbx-pass`。
    由 `make-kdbx.mjs` 生成（Argon2 注入方式与 `src\migration\keepass2.ts` 一致）。内容：
    根分组 `KeePass 根分组`（entry `KeePass 条目`：密码 `kp-secret`、访问链接
    `https://keepass.example.com/`、备注 `kp-note`）、子分组 `子分组 A`（entry `子条目 A`：密码
    `kp-sub-secret`）、子分组 `子分组 B`（entry `子条目 B`：备注 `kp-b-note`）；KDBX 自带回收站
    分组由解析器按 `recycleBinUuid` 跳过；
  - `invalid-file.kdbx`：内容为普通文本的伪 .kdbx（覆盖「无效的 KeePass 2.0 数据库文件」）；
  - `not-a-database.txt`：普通文本文件（覆盖文件选择器扩展名过滤的 F4 变体）。

## 覆盖范围

前端功能点：
- 右下角「工具」菜单（`database.migration.menu-button`）与两个菜单项「KeePass 2.0数据导入」「导出数据库」
- 导出对话框：明文警告、三种模式单选与说明文案、默认选中「包含字段但不包含字段值」、
  确认后进入文件选择器（标题「导出数据库」、默认文件名「用户数据库.md」）
- 导出成功 snackbar「数据库已导出」
- KeePass 导入对话框：只读文件路径框 + 「选择文件」按钮（FilePicker open、扩展名 kdbx）、
  Master Password 输入、导入方式两个单选（默认「导入单个画布」）、确认按钮
- 导入成功 snackbar「KeePass2 数据导入完成」与跳转新画布
- 导入失败提示：未选文件/未输密码的内联提示；密码错误、文件无效的 snackbar

后端命令（经由 UI 触发）：
`user_database_export_export`、`user_database_migration_read_file`、
`user_database_migration_import_keepass2`、`user_database_migration_import_keepass2_canvases`、
`user_database_node_field_set`、`user_database_node_list` / `user_database_canvas_list`、
`file_system_list_directory` / `file_system_roots` / `file_system_path_exists`。

## UI 操作流程与预期结果

| # | 操作 | 预期结果 |
|---|------|----------|
| 1 | 点击右下角「工具」菜单按钮，断言菜单项「KeePass 2.0数据导入」「导出数据库」 | 菜单展开；截图留档 |
| 2 | 点击「导出数据库」 | 打开对话框「导出数据库」；警告文案「导出的文件为明文 Markdown 文件，任何获得该文件的人都可以直接阅读其中的内容，请妥善保管。」；「导出模式」含三个单选与说明；默认选中「包含字段但不包含字段值」；截图留档 |
| 3 | 选择「不包含字段」，点击「确认」 | 打开文件选择器（标题「导出数据库」、默认文件名「用户数据库.md」，剪贴板验证） |
| 4 | 路径框输入 `<case>\output` 回车，文件名输入 `export-exclude.md`，点击「确认」 | snackbar「数据库已导出」；脚本断言文件存在且非空 |
| 5 | 重复步骤 2–4，选择「包含字段但不包含字段值」，导出为 `export-mask.md` | snackbar「数据库已导出」；文件存在 |
| 6 | 重复步骤 2–4，选择「包含字段且包含字段值」，导出为 `export-include.md` | snackbar「数据库已导出」；文件存在 |
| 7 | 脚本读取三个导出文件并比对 | 三个文件均含 `## 画布：root`（画布名取数据库内名称 `root`，非面包屑显示名「根画布」）、`### 节点：账号 A`、`### 节点：备注 B`、`#### 关系` 与边行 `- 账号 A --[]--> 备注 B`；`export-exclude.md` 不含字段表格与「口令」；`export-mask.md` 含「口令」「备注字段」但不含明文值，且以保留首尾字符的掩码出现；`export-include.md` 含 `secret-export-123` 与 `note-export-value` |
| 8 | 打开「工具」菜单，点击「KeePass 2.0数据导入」 | 打开对话框「KeePass2数据导入」；路径框 label「KeePass 2.0 数据库文件」；「选择文件」按钮；Master Password 输入框；「导入方式」两个单选「导入单个画布」（默认）「按分组生成多个画布」；截图留档 |
| 9 | 不选文件直接点击「确认」 | 路径框下方出现「请选择 KeePass 2.0 数据库文件」（失败路径 F1），对话框保持打开 |
| 10 | 点击「选择文件」 | 打开文件选择器（标题「选择文件」、扩展名过滤 kdbx、含「显示全部文件」开关） |
| 10a | （F4 变体）路径导航到 `<case>`，打开「显示全部文件」，单击/双击 `not-a-database.txt` 行 | 非 kdbx 行置灰且不可选中：确认按钮保持禁用、双击不关闭选择器；计划预期的「文件扩展名无效」提示在 UI 上不可达，按计录偏差记录；截图留档 |
| 11 | 重新打开选择文件，路径框输入 `<case>` 资产目录回车，双击 `e2e-test.kdbx` | 文件选择器关闭；路径框显示该文件绝对路径（剪贴板验证） |
| 12 | Master Password 留空，点击「确认」 | 输入框下方出现「请输入 Master Password」（失败路径 F2），对话框保持打开 |
| 13 | 输入 `wrong-password`，点击「确认」 | snackbar 出现「Master Password 不正确，无法解密所选数据库」；对话框保持打开可重试（失败路径 F3）；截图留档 |
| 13a | （附加）选择 `invalid-file.kdbx`、输入正确密码并确认 | snackbar 出现「无效的 KeePass 2.0 数据库文件」；对话框保持打开（失败路径 F6） |
| 14 | 重新选择 `e2e-test.kdbx`，输入 `e2e-kdbx-pass`，保持「导入单个画布」，点击「确认」 | snackbar「KeePass2 数据导入完成」；对话框关闭；路由跳转到新画布（面包屑出现 `e2e-test`）；截图留档 |
| 15 | 在新画布中断言导入内容 | 出现根节点 `KeePass 根分组`、entry 节点 `KeePass 条目`、分组节点 `子分组 A` 与其 entry `子条目 A`；双击 `KeePass 条目` 后字段区含「密码」（类型「密码」，值经复制按钮 + 剪贴板验证为 `kp-secret`）、「访问链接」（类型「网址」）、「备注」（类型「多行文本」）；截图留档 |
| 16 | 再次打开「KeePass 2.0数据导入」，选择同一文件并输入正确密码，选择「按分组生成多个画布」，点击「确认」 | snackbar「KeePass2 数据导入完成」；跳转到顶层新画布 `KeePass 根分组` |
| 17 | 断言多画布导入结果 | 顶层画布出现数据节点 `KeePass 条目` 与画布数据节点 `子分组 A`、`子分组 B`（子分组内容不在顶层画布）；点击面包屑「画布宇宙」后出现 `根画布`、`e2e-test`、`KeePass 根分组`、`子分组 A`、`子分组 B`；双击 `子分组 A` 进入后可见其 entry 节点 `子条目 A`；截图留档 |

## 失败路径

| # | 失败场景 | 对应步骤 | 预期结果 |
|---|----------|----------|----------|
| F1 | 未选择 KeePass 文件提交 | 9 | 内联提示「请选择 KeePass 2.0 数据库文件」，不发起导入 |
| F2 | 未输入 Master Password 提交 | 12 | 内联提示「请输入 Master Password」，不发起导入 |
| F3 | Master Password 错误 | 13 | snackbar「Master Password 不正确，无法解密所选数据库」；对话框保持打开可重试 |
| F4 | 选择非 `.kdbx` 文件 | 10a | 文件选择器将非 kdbx 文件置灰且不可选中（确认按钮保持禁用、双击不关闭）；「文件扩展名无效」提示在 UI 上不可达，记录偏差并跳过（后端 `InvalidKdbxFileExtension` 由后端单测覆盖） |
| F5 | 导入的 KeePass 文件结构不含分组 | — | 多画布模式下后端可能返回 `EmptyImportedCanvasList`；本用例资产保证含分组，该分支不构造 |
| F6（附加） | 伪 `.kdbx` 文件（扩展名合法、内容非法） | 13a | snackbar「无效的 KeePass 2.0 数据库文件」；对话框保持打开 |

## 脚本资产需求

- `e2e-test.kdbx` / `invalid-file.kdbx` / `not-a-database.txt`：位于 `e2e\script\case_011\`，
  由 `node e2e\script\case_011\make-kdbx.mjs` 生成（可重复执行）
- 三个导出文件 `export-exclude.md`、`export-mask.md`、`export-include.md` 运行期输出到 `output\`
- 关键步骤截图（工具菜单、导出对话框、三种模式结果各一张、导入对话框、F1/F2/F4/F3/F6 提示、
  单画布导入结果、导入节点字段、多画布导入结果、画布宇宙、子画布）存入 `output\`

## 备注与风险

- 导出目标路径不得位于应用数据目录内（后端 `InvalidExportTargetPath` 校验），脚本输出到
  `<case>\output` 即可；默认文件名为「用户数据库.md」，脚本在文件名框内替换。
- 导出文件的固定文案语言取当前界面语言（前端传 locale）；画布/节点名称取数据库内的名称
  （根画布在导出文件中显示为 `画布：root`，而非面包屑显示名「根画布」）。
- KeePass 导入解密与解析在前端完成（`kdbxweb`），后端只读字节与聚合落库；密码错误与文件无效
  的文案来自前端，文件扩展名错误来自后端 `InvalidKdbxFileExtension`。
- 单画布导入在根画布创建画布数据节点与新画布（名称取文件 stem）后跳转；多画布导入创建整棵
  画布子树（顶层画布名取根 group 名）并跳转顶层画布。
- 导入对话框确认按钮在导入期间显示「导入中…」并禁用；该状态持续时间取决于 Argon2 解密耗时，
  脚本不做瞬时断言（避免抖动）。
- 导出文件内容断言基于 Markdown 文本，字段值打码形式只断言「含/不含明文」与掩码模式
  （保留首尾字符），不精确断言星号个数。

## 修订记录

1. **资产名称**：`keepass.kdbx` → `e2e-test.kdbx`（单画布导入的新画布名取文件 stem，
   便于断言面包屑出现确定的画布名 `e2e-test`）。
2. **资产生成**：固化只用 `kdbxweb` + `@noble/hashes` 现场生成 KDBX 4.0 / Argon2id（M=8 MiB）
   一条路线；同时生成 `invalid-file.kdbx` 与 `not-a-database.txt` 两个辅助资产。
3. **资产内容**：明确根分组名 `KeePass 根分组`、两个子分组（`子分组 A`/`子分组 B`）及其 entry
   的标题与字段值，供单画布与多画布两条路线的断言使用。
4. **步骤 7**：修正断言口径——导出文件中的画布名是数据库内名称 `root`，不是面包屑显示名
   「根画布」；三个文件均断言画布/节点/关系行，另断言掩码模式保留首尾字符。
5. **步骤 10/11**：明确步骤 10 结构断言与步骤 10a（F4 变体），步骤 11 改为「重新打开选择文件」
   以衔接 F4 的取消动作。
6. **F4**：原预期「提示文件扩展名无效」不可达（文件选择器置灰不可选中），按原计划授权记录偏差
   并跳过；改为断言文件选择器的阻止行为（确认禁用、双击不关闭）。
7. **新增 F6**：追加伪 `.kdbx`（内容非法）失败路径，覆盖原「覆盖范围」中提及的
   「文件无效」snackbar。
8. **步骤 13 的「导入中…」**：因属 loading 瞬态，不纳入断言；其余预期不变。
9. **步骤 15**：补充断言根节点名 `KeePass 根分组` 与分组节点 `子分组 A`/`子条目 A`，
   密码值经复制按钮 + 剪贴板验证。
10. **步骤 17**：补充「点击面包屑画布宇宙」的具体路径（从顶层画布返回宇宙）与
    「顶层画布不含子分组内容（子条目 A 未出现）」的分割断言。
