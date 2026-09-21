# case_008 附件管理

## 测试主题

在节点附件对话框中完成附件全生命周期管理：文本附件创建（扩展名校验）、导入附件、重命名、拖拽排序、预览与文本编辑保存（含未保存关闭确认）、导出附件（含覆盖确认）、逻辑删除/恢复/永久删除，以及孤儿文件的上报与删除。本用例验证「附件创建/导入 → 阅读编辑 → 导出 → 回收站 → 孤儿清理」链路。

## 前置条件

- 复制 fixture `base` 到 `<case>\output\data` 并以该目录启动应用
- 解锁 `zz-e2e-base` / `e2e-password`，位于根画布（若停在画布宇宙则双击「根画布」进入）
- 界面语言：中文
- 脚本资产 `sample.txt`（内容已知，如 `hello e2e attachment`）存放在用例目录，供导入附件与内容比对使用

## 覆盖范围

前端功能点：
- 节点 hover 按钮 `title="管理附件"` 打开附件对话框（标题「节点"账号 A"的附件」）
- 创建文本附件：文件名校验（无扩展名自动补 `.txt`）、非法扩展名 snackbar、确认/取消图标按钮
- 导入附件经 FilePickerDialog（open 模式、无扩展名过滤）
- 重命名：内联编辑、空名拦截（snackbar + 标红）、Enter 提交/Esc/失焦取消
- 拖拽排序：`title="拖拽调整顺序"` 手柄拖放后交换顺序并调用排序接口
- 预览：文本类走文本查看器（工具栏语言名/未保存标记/保存按钮）；编辑保存与未保存关闭确认
- 导出附件：FilePickerDialog（save 模式、默认文件名）、目标已存在的覆盖确认
- 逻辑删除/回收站分区/恢复/永久删除（各确认框）
- 孤儿文件：对话框打开时自动检测、警告区展示与删除流程

后端命令（经由 UI 触发）：
- `user_database_attachment_create`、`user_database_attachment_import`、`user_database_attachment_list`（正常与 `deleted=true`）
- `user_database_attachment_load`（预览时解密读取，二进制 IPC 响应）
- `user_database_attachment_update_file`（文本保存）
- `user_database_attachment_rename`、`user_database_attachment_export`
- `user_database_attachment_logical_delete`、`user_database_attachment_restore`、`user_database_attachment_physical_delete`
- `user_database_attachment_swap_sort_order`（拖拽排序）
- `user_database_attachment_list_orphan_files`、`user_database_attachment_remove_orphan_file`
- `file_system_list_directory` / `file_system_roots` / `file_system_path_exists`（FilePickerDialog 内部）

## UI 操作流程与预期结果

| # | 操作 | 预期结果 |
|---|------|----------|
| 1 | hover「账号 A」，点击 `title="管理附件"` | 打开附件对话框；空态显示「暂无附件，点击下方按钮导入」；无孤儿文件警告 |
| 2 | 点击「创建文本附件」，在 placeholder「文件名（如 note.txt）」的输入框输入 `note`，点击 `title="确认创建"` | snackbar「附件已创建」；列表出现 `note.txt`（自动补全扩展名）；截图留档 |
| 3 | 点击「创建文本附件」，输入 `bad.exe`，点击 `title="确认创建"` | snackbar「文件名必须使用文本类型的扩展名（如 .txt、.md）」；输入框标红并保持输入态（失败路径 F1） |
| 4 | 将输入改为 `bad.md`，点击 `title="取消创建"` | 错误态解除；未创建附件 |
| 5 | 点击「导入附件」 | 打开文件选择器，标题「导入附件」；路径框（label「路径」）可输入 |
| 6 | 在路径框输入 `<case>` 资产目录绝对路径，按 Enter；等待列表加载后双击 `sample.txt` 行 | snackbar「附件已导入」；列表出现 `sample.txt`；截图留档 |
| 7 | 再创建两个文本附件 `a.txt`、`b.txt` | 列表共 4 个附件：`note.txt`、`sample.txt`、`a.txt`、`b.txt` |
| 8 | 点击 `note.txt` 行的 `title="重命名附件"` 按钮 | 文件名变为内联输入框（原值 `note.txt`） |
| 9 | 全选输入框内容并清空，按 Enter | snackbar「文件名不能为空」；输入框标红（失败路径 F2） |
| 10 | 输入 `note-renamed.md`，按 Enter | snackbar「附件已重命名」；列表显示 `note-renamed.md` |
| 11 | 用 `title="拖拽调整顺序"` 手柄把第一项拖到第三项位置后松开 | 列表顺序变化（`note-renamed.md` 移到原第三项之后）；截图留档；刷新对话框后顺序保持（排序已落库） |
| 12 | 点击 `note-renamed.md` 行的 `title="预览附件"` | 打开预览对话框（标题 `note-renamed.md`）；文本查看器工具栏显示语言名与「保存」按钮（禁用） |
| 13 | 点击编辑区（可编辑区域），`Ctrl+A` 后输入 `e2e attachment edited` | 工具栏出现「未保存」标记；「保存」按钮可用 |
| 14 | 点击「保存」 | snackbar「附件已保存」；「未保存」标记消失；截图留档 |
| 15 | 再次在编辑区输入 `pending`，点击「关闭」 | 弹出确认框：标题「未保存的修改」、正文「当前附件有未保存的修改，关闭后将丢失，确定要关闭吗？」 |
| 16 | 点击确认框「取消」，再次点击「关闭」并在确认框中点击「确认」 | 取消时留在预览对话框；确认后对话框关闭（修改被放弃） |
| 17 | 点击 `sample.txt` 行的 `title="导出附件"` | 打开文件选择器（save 模式、标题「导出附件」、默认文件名 `sample.txt`） |
| 18 | 路径框输入 `<case>\output`，文件名输入 `sample-export.txt`，点击「确认」 | snackbar「附件已导出」；脚本断言 `<case>\output\sample-export.txt` 存在且内容等于资产 `sample.txt` |
| 19 | 再次导出 `sample.txt` 到同一路径 | 文件选择器弹出覆盖确认：标题「文件已存在」、正文「"sample-export.txt" 已存在，是否覆盖？」；点击「确认」后导出成功 |
| 20 | 点击 `a.txt` 行的 `title="删除附件"` | 确认框：标题「删除附件」、正文「确定要删除附件"a.txt"吗？它将移入回收站。」 |
| 21 | 点击「确认」 | snackbar「附件已删除」；正常列表无 `a.txt`；出现分区「回收站（1）」 |
| 22 | 点击回收站分区中 `title="恢复附件"` | snackbar「附件已恢复」；`a.txt` 回到正常列表；回收站分区消失 |
| 23 | 再次删除 `a.txt`，在回收站分区点击 `title="永久删除附件"` | 确认框：标题「永久删除附件」、正文「确定要永久删除附件"a.txt"吗？附件文件将一并删除，此操作不可恢复。」；确认后 snackbar 与分区消失 |
| 24 | 关闭附件对话框；脚本在 `<data>\user_database_set\<uuid>\attachment\` 下写入文件 `11111111-2222-3333-4444-555555555555.bin`；重新打开「管理附件」 | 出现警告「发现 1 个孤儿文件」与说明「以下附件文件没有对应的元数据记录，可能是异常中断留下的残留，确认无用后可删除。」；下方列出该文件 id；截图留档 |
| 25 | 点击该条目的 `title="删除孤儿文件"` | 确认框：标题「删除孤儿文件」、正文「确定要删除孤儿文件"11111111-2222-3333-4444-555555555555.bin"吗？该文件将被永久删除，不可恢复。」 |
| 26 | 点击「确认」 | snackbar「孤儿文件已删除」；警告区消失；脚本断言磁盘上该 `.bin` 文件已不存在 |

## 失败路径

| # | 失败场景 | 对应步骤 | 预期结果 |
|---|----------|----------|----------|
| F1 | 文本附件扩展名非法（`.exe`） | 3 | snackbar「文件名必须使用文本类型的扩展名（如 .txt、.md）」；不创建附件 |
| F2 | 重命名输入为空 | 9 | snackbar「文件名不能为空」；输入框标红；名称不变 |
| F3 | 导出目标已存在且用户取消覆盖 | 19 | 弹出「文件已存在」确认框；取消后不覆盖、原文件不变 |
| F4 | 预览有未保存修改时关闭 | 15 | 弹出「未保存的修改」确认框；取消则留在预览对话框 |
| F5 | 删除/永久删除确认框取消 | 20、23 | 附件保留在原位置 |

## 脚本资产需求

- `sample.txt`（UTF-8 文本，内容 `hello e2e attachment`），放入 `e2e\script\case_008\`
- 脚本需通过文件系统在数据目录附件文件夹写入伪造孤儿文件（uuid 名称 + `.bin` 后缀），并在删除后断言文件消失
- 脚本需读取 `<case>\output\sample-export.txt` 与资产内容比对
- 关键步骤截图（空态、创建文本附件、非法扩展名、导入后、重命名、排序后、预览编辑器、未保存标记、覆盖确认、回收站分区、孤儿警告、孤儿删除后）存入 `output\`

## 备注与风险

- CodeMirror 编辑器为 `contenteditable` 区域；若合成键盘输入未被编辑器接受（表现为「未保存」标记不出现），属于调试自动化输入能力问题，需上报并由脚本记录截图证据。
- 附件列表的顺序断言依赖文本行在 UI 树中的先后与 `bounds.top` 排序；拖拽排序的落点使用目标行的垂直中心。
- 孤儿文件检测为全局扫描（不限于当前节点），检测时机为对话框打开与每次操作成功后；伪造文件不会影响其他用例（用例数据目录独立）。
- 附件文件独立于 `user_database.sqlite` 即时落盘；本用例不做重启后附件持久化断言（可选项：Ctrl+S 后重启复验）。
- `AttachmentTooLarge`（单附件上限 100MB，服务端限制）需构造大于 100MB 资产，成本高，不在本用例覆盖。
- 文件选择器为应用内实现（不使用系统对话框），路径框支持绝对路径输入 + Enter 导航；save 模式需填写文件名，文件名非法时存在内联校验（「文件名包含非法字符，或以点号结尾」）。
