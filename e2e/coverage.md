# e2e 测试用例覆盖台账

本台账对账范围：`src\api.ts` 的后端命令与前端功能面（页面、对话框、画布交互），逐项标注对应测试用例或未覆盖原因。

说明：原 85 条命令中的 `user_database_canvas_create` 已于 2026-09-21 随收尾整理从代码中删除（前端无 UI 调用入口）；2026-09-22 节点标签与书签功能新增 6 条命令（#86–#91），现行命令 90 条。下表保留已删除命令行以说明去向。

## 一、后端命令覆盖表

| # | 命令（invoke 名） | 对应 case | 覆盖说明 / 未覆盖原因 |
|---|------------------|-----------|----------------------|
| 1 | `preference_get` | 002、014、015 | 启动加载语言偏好（014 断言切换后持久化；002 为启动隐含步骤） |
| 2 | `preference_set` | 014 | 语言、主题、剪贴板超时写入 |
| 3 | `preference_save` | 014 | `storePreference` = set + save，设置保存后重开断言持久化 |
| 4 | `clipboard_clear` | 005 | 剪贴板倒计时结束自动清空（脚本读系统剪贴板验证） |
| 5 | `metadata_register` | 001 | 首页新建数据库 |
| 6 | `metadata_list` | 002、013 | 首页候选与归档管理列表 |
| 7 | `metadata_archive` | 013 | 归档 / 解除归档 |
| 8 | `metadata_physical_delete` | 013 | 归档后删除（名称与密码两类校验路径） |
| 9 | `metadata_save` | 001、002、013 | 异步落盘（fire-and-forget），以重启后的数据结构断言（015 间接验证） |
| 10 | `user_database_lifecycle_initialize` | 001、002、012、015 | 新建并解锁 / 解锁 / 还原后重新解锁 |
| 11 | `user_database_lifecycle_save` | 012、015 | 保存并退出（012）、Ctrl+S / 保存并退出 / 保存并关闭（015） |
| 12 | `user_database_lifecycle_close` | 015 | 保存并退出、不保存关闭、保存并关闭 |
| 13 | `user_database_canvas_create` | 已删除 | 该接口于 2026-09-21 从代码中移除（前端无 UI 调用入口）；建画布能力经 `user_database_node_create(createCanvas=true)` 覆盖 |
| 14 | `user_database_canvas_move_canvases` | 003 | 画布节点拖拽与自动布局 |
| 15 | `user_database_canvas_logical_delete` | 003 | 删除画布（无确认框） |
| 16 | `user_database_canvas_restore` | 003 | 回收站恢复画布 |
| 17 | `user_database_canvas_physical_delete` | 003 | 永久删除画布（确认框） |
| 18 | `user_database_canvas_rename` | 003 | 重命名画布（名称唯一性校验） |
| 19 | `user_database_canvas_list` | 002、003、007、015 | 宇宙加载、`lastScene` 校验、回收站面板 |
| 20 | `user_database_canvas_node_color_list` | 003 | 打开画布配色对话框时聚合历史颜色 |
| 21 | `user_database_canvas_node_set_color` | 003 | 画布配色保存 |
| 22 | `user_database_viewport_get` | 003、004、007 | 宇宙/画布视口加载；迁移目标视口中心换算 |
| 23 | `user_database_viewport_set` | 003、004 | 视口缩放/平移防抖写入（500ms） |
| 24 | `user_database_node_create` | 003、004、006、007、009、010、011、015 | 拖拽创建（含模板、画布数据节点） |
| 25 | `user_database_node_copy` | 004、016 | 悬浮按钮复制（副本到当前画布视口中央、20px 取整）；快捷键 Ctrl+C/V 复制粘贴（副本到指定目标画布 canvas_id、落点为鼠标位置、20px 取整）；004 覆盖按钮入口，016 覆盖快捷键入口与跨画布 / 源节点已删除路径 |
| 26 | `user_database_node_move_nodes` | 004、007、010 | 框选批量移动、非法迁移回退、日志准备拖动 |
| 27 | `user_database_node_relocate_nodes` | 007 | Alt 跨画布迁移（合法双路径 + 非法三提示） |
| 28 | `user_database_node_modify` | 004、006、007、009、010、015 | 标题/副标题/画布名修改 |
| 29 | `user_database_node_logical_delete` | 004 | 逻辑删除节点 |
| 30 | `user_database_node_restore` | 004 | 回收站恢复节点 |
| 31 | `user_database_node_physical_delete` | 004 | 永久删除节点（含确认框取消） |
| 32 | `user_database_node_list` | 002、003、004、005、006、007、008、009、010、011、015 | 各画布加载、回收站、影子合并展示 |
| 33 | `user_database_node_search` | 010、017 | 全局搜索（防抖、候选、无匹配）；017 覆盖节点标签的 OR 匹配维度与结果副标题的根画布本地化显示 |
| 34 | `user_database_data_node_set_color` | 004 | 数据节点自定义颜色 |
| 35 | `user_database_data_node_color_list` | 004 | 配色对话框历史颜色 |
| 36 | `user_database_edge_create` | 006、007 | 新建、同向更新、反向替换（含断连确认） |
| 37 | `user_database_edge_delete` | 006 | 删除边（无断连 / 断连确认两阶段） |
| 38 | `user_database_edge_get` | 007（006 隐含） | 影子虚拟边点击后定位产生边 |
| 39 | `user_database_edge_list` | 006、007 | 画布边加载 |
| 40 | `user_database_edge_update` | 006 | 编辑边标题与详情 |
| 41 | `user_database_registry_get` | 002、015 | 读取 `lastScene` |
| 42 | `user_database_registry_set` | 002、003、004、006、007、015 | 路由场景变化写入 `lastScene` |
| 43 | `user_database_log_list` | 010 | 日志分页、日期/行为/关键词筛选 |
| 44 | `user_database_node_field_get` | 005、009、011 | 字段读取（编辑对话框、模板继承验证、导出准备） |
| 45 | `user_database_node_field_set` | 005、009、010、011 | 字段保存（含值格式校验、敏感值日志渲染准备） |
| 46 | `user_database_dictionary_list` | 005、009 | 字典加载（绑定面板、导入替换） |
| 47 | `user_database_dictionary_set` | 005、009 | 字典保存（含空值与未保存确认） |
| 48 | `user_database_template_create` | 009 | 新建空模板 |
| 49 | `user_database_template_create_from_node` | 009 | 从节点保存为模板 |
| 50 | `user_database_template_rename` | 009 | 重命名模板 |
| 51 | `user_database_template_delete` | 009 | 删除模板（确认框） |
| 52 | `user_database_template_list` | 009 | 模板列表 |
| 53 | `user_database_template_get_fields` | 009 | 模板字段结构读取 |
| 54 | `user_database_template_set_fields` | 009 | 模板字段保存（校验） |
| 55 | `user_database_template_export` | 009 | 导出 SQLite |
| 56 | `user_database_template_import` | 009 | 导入替换（确认框） |
| 57 | `user_database_attachment_create` | 008 | 创建文本附件（扩展名校验） |
| 58 | `user_database_attachment_import` | 008 | 导入附件（FilePicker） |
| 59 | `user_database_attachment_list` | 008 | 正常与回收站列表 |
| 60 | `user_database_attachment_load` | 008 | 预览时解密读取 |
| 61 | `user_database_attachment_export` | 008 | 导出附件（覆盖确认） |
| 62 | `user_database_attachment_logical_delete` | 008 | 删除附件（确认框） |
| 63 | `user_database_attachment_restore` | 008 | 恢复附件 |
| 64 | `user_database_attachment_physical_delete` | 008 | 永久删除附件（确认框） |
| 65 | `user_database_attachment_rename` | 008 | 重命名附件（空名拦截） |
| 66 | `user_database_attachment_list_orphan_files` | 008 | 打开对话框自动检测孤儿文件 |
| 67 | `user_database_attachment_remove_orphan_file` | 008 | 删除孤儿文件（确认框 + 磁盘断言） |
| 68 | `user_database_attachment_update_file` | 008 | 文本附件编辑保存 |
| 69 | `user_database_attachment_swap_sort_order` | 008 | 拖拽排序 |
| 70 | `user_database_export_export` | 011 | 三种导出模式与产物断言 |
| 71 | `user_database_migration_read_file` | 011 | 读取 `.kdbx`（扩展名校验） |
| 72 | `user_database_migration_import_keepass2` | 011 | 单画布导入（含密码错误路径） |
| 73 | `user_database_migration_import_keepass2_canvases` | 011 | 多画布导入 |
| 74 | `backup_backup` | 012 | 备份（比例、进度、产物） |
| 75 | `backup_restore` | 012 | 确认还原（进度、成功对话框、数据回退） |
| 76 | `backup_restore_probe` | 012 | 探测通过 / 不可恢复两条路径 |
| 77 | `backup_data_directory_size` | 012 | 打开备份对话框时显示数据目录大小 |
| 78 | `reclaim_preference` | 012 | 还原成功后自动调用（无独立 UI 入口） |
| 79 | `reclaim_metadata` | 012 | 同上 |
| 80 | `reclaim_user_database` | 012 | 同上 |
| 81 | `fatal_exit` | 不可覆盖 | 仅由 `DataCorruption*` 错误触发（正常 UI 无法制造数据损坏），且会强制退出进程；安全红线行为，测试不构造 |
| 82 | `file_system_list_directory` | 008、009、011、012 | FilePickerDialog 目录导航（导入/导出/选择备份） |
| 83 | `file_system_roots` | 008、009、011、012 | FilePickerDialog 驱动器列表 |
| 84 | `file_system_path_exists` | 008、009、011、012 | save 模式覆盖检查 |
| 85 | `app_info_get` | 014 | 关于对话框信息 |
| 86 | `user_database_node_set_bookmarked` | 017 | 编辑对话框书签切换（收藏/取消）；取消后书签列表重载为空态；无变化不写日志由 017 的日志计数间接覆盖 |
| 87 | `user_database_node_list_bookmarked` | 017 | 书签列表加载（按画布分组、根画布本地化组名）、取消书签后空态、跨画布跳转入口 |
| 88 | `user_database_node_tag_list` | 017 | 标签云数据源与字号分档（node_count 比例分 5 档） |
| 89 | `user_database_node_tag_list_for_node` | 017 | 编辑对话框标签 chip 回填（保存后重开） |
| 90 | `user_database_node_tag_list_nodes` | 017 | 「标签：{tag}」分组列表加载（含根画布分组名） |
| 91 | `user_database_node_tag_set_for_node` | 017 | 标签保存（trim/去空/去重、全量覆盖、无变化不写日志由日志计数间接覆盖） |

**命令覆盖结论**：现行 90 条中，89 条由至少一个用例覆盖；其中 `reclaim_*` 无独立入口（由 case_012 还原流程覆盖）、`metadata_save` / `user_database_lifecycle_save` 为 fire-and-forget（以持久化结果间接覆盖）；`fatal_exit` 不可覆盖（原因见上）。原 `user_database_canvas_create` 已于 2026-09-21 从代码中删除；2026-09-22 新增的 6 条标签与书签命令（#86–#91）均由 case_017 覆盖。

## 二、前端功能面覆盖表

### 页面与框架

| 功能面 | 对应 case |
|--------|-----------|
| Home.vue：名称步/密码步、新建与解锁、候选下拉、归档/备份/还原入口 | 001、002、012、013、014、015 |
| Home.vue：挂载时自动填最后打开的未归档数据库（冷启动与保存并退出重挂载均生效、跨会话持久化、空列表不填、自动填后停留名称步骤、归档库排除、归档管理对话框关闭不覆盖用户输入） | 018 |
| DatabaseView.vue：工具菜单、日志/保存并退出、Ctrl+S、关闭拦截、场景路由 | 003–011、015 |
| CanvasUniverseView.vue：画布节点、自动布局、画布回收站、宇宙视口 | 003、007 |
| CanvasView.vue：模板面板、节点/边/附件/字典/导出/搜索等画布操作、书签与标签云入口 | 004–011、015、017 |
| App.vue：右上角语言/设置/主题/关于、Snackbar 队列、剪贴板倒计时条 | 005、014（Snackbar 由各 case 断言） |

### 对话框与组件

| 组件 | 对应 case |
|------|-----------|
| HomeComponents：ArchiveManagementDialog / DeleteDatabaseDialog | 013 |
| HomeComponents：BackupDialog / RestoreDialog / RestoreSuccessDialog | 012 |
| components：AboutDialog / SettingsDialog | 014 |
| components：ThemeManagerDialog / ThemeEditDialog / ThemeImportDialog | 014 |
| components：FilePickerDialog | 008、009、011、012 |
| components：NameInputDialog / ConfirmDialog | 003、004、006、007、008、009、014 |
| components：AutoCompleteField / PasswordField | 001、002、011 |
| components：NodeField / TemplateField / TreeSelect | 005、009 |
| field-editors：StringSingleLineEditor / StringPasswordEditor / PasswordGeneratorDialog / StringEmailEditor | 005 |
| DatabaseComponents：EditNodeDialog / RecycleBinPanel | 004、005、009、017（017：书签切换按钮与标签 combobox） |
| DatabaseComponents：DictionaryManagerDialog | 005 |
| DatabaseComponents：NodeGroupListDialog / TagCloudPanel | 017 |
| DatabaseComponents：NodeTemplatePanel / TemplateManagerDialog | 004、006、007、009 |
| DatabaseComponents：EdgeContextMenu / EditEdgeDialog / CustomEdge | 006、007 |
| DatabaseComponents：DataNode / CanvasNode / CanvasBreadcrumb / CanvasRecycleBinPanel | 003、004、006、007、015、017（017：卡片书签图标） |
| DatabaseComponents：ExportDialog / GlobalSearch / LogDialog / LogActionContent / Topbar | 010、011、017（017：搜索标签匹配、标签与书签日志详情） |
| DatabaseComponents\attachment：AttachmentDialog / AttachmentPreviewDialog / AttachmentViewerText | 008 |
| node-colors：EditDataNodeColorDialog / EditCanvasNodeColorDialog | 004、003 |
| migration：KeePass2Import | 011 |
| composables：use-viewport / use-auto-layout / use-canvas-recycle-bin / use-recycle-bin / use-edge-delete / use-node-move-and-relocate / use-relocate-animation / use-clipboard-clear / use-backup-progress / use-node-copy-paste / use-overlay-open | 003、004、005、006、007、012、016（随 UI 操作覆盖） |

### 画布交互

| 交互 | 对应 case |
|------|-----------|
| 模板面板拖拽创建（数据节点 / 画布数据节点） | 003、004、006、007、009 |
| 节点双击进入子画布 / 编辑、面包屑钻入钻出与折叠 | 003、007 |
| 节点拖动、框选批量移动、20px 网格吸附 | 004 |
| 节点复制、逻辑删除/恢复/永久删除、配色 | 004 |
| 节点标签输入与书签切换（编辑对话框 combobox 自由输入、trim/去空/去重、仅变更项保存、取消不写库；卡片书签图标） | 017 |
| 书签列表与标签云（按画布分组、行内编辑后整表重载、跨画布跳转居中、空态；标签字号分档、面板互斥、分组列表关闭后重载） | 017 |
| 快捷键 Ctrl/Cmd+C 复制选中数据节点、Ctrl/Cmd+V 在鼠标位置粘贴（含生效门控：对话框 / 模板面板 / 回收站面板 / 边右键菜单 / 输入框焦点；跨画布粘贴归属当前画布；源节点逻辑删除或物理删除后跳过） | 016 |
| 连接桩拖拽建边、同向更新、反向替换 | 006、007 |
| 边右键菜单、编辑、删除与断连确认 | 006、007 |
| 影子节点产生、影子虚拟边点击跳转（入向/出向）、影子删除 | 006、007 |
| Alt 跨画布迁移与非法提示 | 007 |
| 视口缩放/平移持久化 | 003、004 |
| 附件拖拽排序、文本编辑 | 008 |
| 画布缩放控件（vue-flow Controls） | 未覆盖：滚轮缩放已覆盖（003、004），控件按钮为等价缩放入口，未单独点击 |

### 未纳入本期计划的功能面（含原因）

| 功能面 | 原因 |
|--------|------|
| field-editors 其余类型：StringUrlEditor、StringMultipleLineEditor、StringSecretEditor、DecimalDecimalEditor、Instant* 系列、DateCalendar / YearPicker / MonthPicker / WheelColumn / InstantPickerPanel | 本期 14 个主题只要求字段增删改通用能力；时间类编辑器的日历/滚轮交互成本高。case_005 已覆盖单行文本、密码、邮箱三种编辑器（含格式校验、生成器、复制） |
| NodeField 字段行拖拽排序（drag-start / drop-on） | 属 case_005 可扩展项，本期未列入主题 |
| PasswordGeneratorDialog 长度滑块调整 | case_005 只验证默认值与「使用」回填；同类滑块操作已在 case_014 覆盖 |
| ThemeEditDialog 的颜色取色编辑（VColorPicker） | case_014 只覆盖显示名修改；取色器交互列为可选扩展 |
| 主题编辑时「主题标识」输入框 disabled 断言 | case_014 未显式断言（记录为补充点） |
| CanvasView 内自动布局按钮 | 未点击；宇宙自动布局（同一 composable 与后端命令 `user_database_canvas_move_canvases`）已由 case_003 覆盖 |
| 附件「文件丢失」标记（missing_file） | 需删除磁盘附件文件后重开对话框；case_008 未构造（可选扩展） |
| 附件 Omni 预览（图片/音视频/Office/PDF） | 需构造对应二进制资产；case_008 仅覆盖文本查看器 |
| FilePickerDialog 的驱动器/主目录/上级目录按钮导航 | case_008/009/011/012 使用路径框导航；按钮导航为等价入口，未单独覆盖 |
| NodeDebugOverlay / ViewportDebugOverlay | `#if [DEBUG]` 条件编译的调试浮层，非用户功能；e2e 使用预构建 exe（生产前端构建）时不存在，不属于测试目标 |
| FatalErrorDialog + 受控崩溃链路 | 仅 `DataCorruption*` 错误触发，正常 UI 不可达；安全红线行为，不构造 |
| 日志「AttachmentUpdate」行为类型筛选 | i18n 缺少该 key，筛选下拉中无此项（已知缺陷，无法选择） |
| 快捷键复制对影子节点的过滤（`data.shadowOriginId` 分支） | 需要构造跨画布边产生影子节点（case_006/007 主题），构造成本高；该过滤与画布数据节点分支共用同一表达式（`!data.shadowOriginId && !data.canvasRefId`），case_016 已覆盖画布数据节点分支 |
| 快捷键粘贴在「鼠标不在画布容器内」时回退视口中心 | 画布容器 `.canvas-view` 宽高 100%，指针恒在容器内，该分支不可经 UI 触达；case_016 以「副本中心落在鼠标落点附近且远离视口中心」的落点断言覆盖指针语义 |
| 节点标签的移除路径（NodeTagsModify 的 removed 非空） | 删除已有标签 chip 需点击 chip 的关闭图标，该图标位于 action-leaf combobox 内部、UI 树不展开，无法稳定定位；case_017 仅覆盖新增路径（removed 为空显示「—」），后端 `set_for_node` 的 added/removed 差集计算已有单元测试覆盖 |
| GlobalSearch 搜索结果中的节点标签 chips 展示（命中关键词的标签优先、其余字典序、单行右对齐、最多 3 个加省略号 chip） | 2026-09-22 新增的展示行为，当日经一次性调试自动化验证通过（含排序、截断、对齐断言），尚未补入正式用例；建议后续并入 case_017 S6 |
| 影子节点只读对话框中的书签按钮禁用与标签 combobox 只读 | 构造影子节点需要跨画布边（case_006/007 主题），构造成本高；后端 `NodeIsShadow` 已有单元测试覆盖 |
| 编辑对话框「取消」分支不写库的直接观测 | 取消没有可观测的副作用（后端无变化时也不写日志），case_017 由「标签云状态保持」「书签列表状态保持」「日志计数」间接佐证 |

## 三、覆盖结论

1. 后端命令：现行 90 条均有归属说明（另有 1 条已于 2026-09-21 删除）；89 条有 UI 可达的覆盖路径，`fatal_exit` 为唯一不可覆盖项（原因：受控崩溃仅由数据损坏触发）。
2. 前端功能面：4 个页面、全部面向用户的主要对话框与画布交互均已分配用例；未覆盖项均为「等价入口」「复杂交互扩展」或「不可构造/不可触达」，已逐条列明原因。
3. 失败路径：全部 18 份计划均含成功路径与失败路径（含前端校验、后端错误码、确认对话框取消分支）。
4. 已知偏差（与任务书描述的差异）已在各计划「备注与风险」中记录，主要为：fixture `lastScene` 非空（002）、画布/节点无右键菜单（003、004）、模板拖拽不吸附网格（004）、删除数据库名称不匹配无文案（013）、「关于」仓库链接无 href 且禁止点击（014）、无顶栏保存按钮且保存并退出位于右下角（015）。
5. 测试工程维护（2026-09-22 节点标签与书签功能接入）：
   - fixture `base` 因用户数据库新增 `node.bookmarked` 列与 `node_tag` 表而按 `_fixtures\README.md` 用 `_fixtures\build.js` 重建（旧 fixture 打开后节点列表查询失败、画布无节点）；
   - case_010 的全局搜索副标题断言按「根画布显示本地化名称」升级为「根画布」，其第二个搜索素材落点与日志分页按钮判据按当前窗口尺寸/DPI 修正（原固定坐标在新环境下分别落入底部工具栏与判据阈值之外）。
6. 测试工程维护（2026-09-22 首页自动填最后打开的数据库接入）：新增 case_018（三段会话覆盖冷启动、保存并退出重挂载、跨会话持久化、空列表不填、归档库排除、归档管理对话框关闭不覆盖输入），未引入新的后端命令；「前端功能面覆盖表」新增 Home.vue 自动填行（判定依据为 UI 树的名称步可编辑控件集合与系统剪贴板读回的名称值）。
