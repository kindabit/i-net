/**
 * 前端与 Tauri Rust 后端通信的 API 封装。
 *
 * 所有 invoke 调用统一集中在此文件，业务组件不直接调用 invoke。
 */
import { invoke } from "@tauri-apps/api/core";
import type {
  AttachmentVO,
  Canvas,
  CanvasColorEntry,
  Dictionary,
  Edge,
  LogPageResponse,
  Metadata,
  MoveNodeVO,
  Node,
  NodeColorEntry,
  NodeFieldVO,
  NodeVO,
  Template,
  TemplateFieldVO,
  Viewport,
  NodeSearchResponse,
} from "@/api-types";
import { currentLocale } from "@/i18n";

// ==================== preference ====================

/**
 * 查询偏好项的值。
 * @param name 偏好项名称
 * @returns 偏好项的值，不存在时返回 null
 */
export async function preferenceGet(name: string): Promise<string | null> {
  return invoke<string | null>("preference_get", { name });
}

/**
 * 插入或更新偏好项。
 * @param name 偏好项名称
 * @param value 偏好项值
 * @returns 无返回值
 */
export async function preferenceSet(name: string, value: string): Promise<void> {
  return invoke("preference_set", { name, value });
}

/**
 * 保存 preference 数据库至文件。
 * @returns 无返回值
 */
export async function preferenceSave(): Promise<void> {
  return invoke("preference_save");
}

// ==================== clipboard ====================

/**
 * 清空系统剪贴板内容。
 * @returns 无返回值
 */
export async function clipboardClear(): Promise<void> {
  return invoke("clipboard_clear");
}

// ==================== metadata ====================

/**
 * 注册一个用户数据库（只添加元数据记录，不创建数据库文件）。
 * @param name 数据库名称
 * @returns 新建记录的元数据
 */
export async function metadataRegister(name: string): Promise<Metadata> {
  return invoke<Metadata>("metadata_register", { name });
}

/**
 * 按归档状态查询用户数据库列表。
 * @param archived 归档状态，false 查询未归档，true 查询已归档
 * @returns 元数据列表
 */
export async function metadataList(archived: boolean): Promise<Metadata[]> {
  return invoke<Metadata[]>("metadata_list", { archived });
}

/**
 * 设置指定 id 的用户数据库的归档状态。
 * @param id 数据库 id
 * @param archived 归档状态，true 归档，false 解除归档
 * @returns 无返回值
 */
export async function metadataArchive(id: string, archived: boolean): Promise<void> {
  return invoke("metadata_archive", { id, archived });
}

/**
 * 物理删除一个用户数据库（数据库须已归档，且密码正确）。
 * @param id 数据库 id
 * @param password 该数据库的密码
 * @returns 无返回值
 */
export async function metadataPhysicalDelete(
  id: string,
  password: string,
): Promise<void> {
  return invoke("metadata_physical_delete", { id, password });
}

/**
 * 保存 metadata 数据库至文件。
 * @returns 无返回值
 */
export async function metadataSave(): Promise<void> {
  return invoke("metadata_save");
}

// ==================== user_database / lifecycle ====================

/**
 * 初始化（打开）一个用户数据库；数据库文件不存在时先创建。
 * @param id 数据库 id
 * @param password 数据库密码
 * @returns 该数据库的元数据
 */
export async function userDatabaseLifecycleInitialize(
  id: string,
  password: string,
): Promise<Metadata> {
  return invoke<Metadata>("user_database_lifecycle_initialize", {
    id,
    password,
  });
}

/**
 * 将内存中的用户数据库加密保存至文件。
 * @returns 无返回值
 */
export async function userDatabaseLifecycleSave(): Promise<void> {
  return invoke("user_database_lifecycle_save");
}

/**
 * 关闭当前用户数据库（清空后端会话状态，不保存）。
 * @returns 无返回值
 */
export async function userDatabaseLifecycleClose(): Promise<void> {
  return invoke("user_database_lifecycle_close");
}

// ==================== user_database / canvas ====================

/**
 * 在指定父画布下新建子画布（后端自动选取合适位置）。
 * @param parentId 父画布 id
 * @param name 画布名称
 * @returns 新建的画布
 */
export async function userDatabaseCanvasCreate(
  parentId: string,
  name: string,
): Promise<Canvas> {
  return invoke<Canvas>("user_database_canvas_create", { parentId, name });
}

/**
 * 修改画布的坐标。
 * @param id 画布 id
 * @param x 新 x 坐标
 * @param y 新 y 坐标
 * @returns 无返回值
 */
export async function userDatabaseCanvasMoveCanvas(
  id: string,
  x: number,
  y: number,
): Promise<void> {
  return invoke("user_database_canvas_move_canvas", { id, x, y });
}

/**
 * 批量移动画布的坐标。
 * @param items 画布坐标列表，每个元素包含 id、x、y
 * @returns 无返回值
 */
export async function userDatabaseCanvasMoveCanvases(items: MoveNodeVO[]): Promise<void> {
  return invoke("user_database_canvas_move_canvases", { items });
}

/**
 * 逻辑删除画布及其所有子画布。
 * @param id 画布 id
 * @returns 无返回值
 */
export async function userDatabaseCanvasLogicalDelete(
  id: string,
): Promise<void> {
  return invoke("user_database_canvas_logical_delete", { id });
}

/**
 * 恢复逻辑删除的画布（连同其逻辑删除的父画布），并移动至新坐标。
 * @param id 画布 id
 * @param x 新 x 坐标
 * @param y 新 y 坐标
 * @returns 无返回值
 */
export async function userDatabaseCanvasRestore(
  id: string,
  x: number,
  y: number,
): Promise<void> {
  return invoke("user_database_canvas_restore", { id, x, y });
}

/**
 * 物理删除画布及其所有子画布（含其中的节点和边）。
 * @param id 画布 id
 * @returns 无返回值
 */
export async function userDatabaseCanvasPhysicalDelete(
  id: string,
): Promise<void> {
  return invoke("user_database_canvas_physical_delete", { id });
}

/**
 * 重命名画布。
 * @param id 画布 id
 * @param name 新名称
 * @returns 无返回值
 */
export async function userDatabaseCanvasRename(
  id: string,
  name: string,
): Promise<void> {
  return invoke("user_database_canvas_rename", { id, name });
}

/**
 * 按逻辑删除状态查询画布列表。
 * @param deleted false 查询正常画布，true 查询已逻辑删除的画布
 * @returns 画布列表
 */
export async function userDatabaseCanvasList(
  deleted: boolean,
): Promise<Canvas[]> {
  return invoke<Canvas[]>("user_database_canvas_list", { deleted });
}

/**
 * 查询画布的自定义颜色列表（用于颜色组合历史聚合）。
 * @returns 带自定义颜色的画布条目列表
 */
export async function userDatabaseCanvasColorList(): Promise<
  CanvasColorEntry[]
> {
  return invoke<CanvasColorEntry[]>("user_database_canvas_color_list");
}

// ==================== user_database / viewport ====================

/**
 * 查询视口；不传画布 id 时返回画布宇宙的视口，不存在时返回默认值。
 * @param canvasId 画布 id，null 表示画布宇宙
 * @returns 视口
 */
export async function userDatabaseViewportGet(
  canvasId: string | null,
): Promise<Viewport> {
  return invoke<Viewport>("user_database_viewport_get", { canvasId });
}

/**
 * 插入或更新视口；不传画布 id 时作用于画布宇宙的视口。
 * @param canvasId 画布 id，null 表示画布宇宙
 * @param x 视口中心的 x 坐标
 * @param y 视口中心的 y 坐标
 * @param zoom 缩放比例
 * @returns 无返回值
 */
export async function userDatabaseViewportSet(
  canvasId: string | null,
  x: number,
  y: number,
  zoom: number,
): Promise<void> {
  return invoke("user_database_viewport_set", { canvasId, x, y, zoom });
}

// ==================== user_database / node ====================

/**
 * 在指定画布内新建节点。
 * @param canvasId 画布 id
 * @param title 节点标题
 * @param subTitle 节点副标题
 * @param x 节点的 x 坐标
 * @param y 节点的 y 坐标
 * @param templateId 模板 id，不指定时为 null
 * @param createCanvas 是否同时创建子画布（以 title 为基础名去重）
 * @returns 新建的节点
 */
export async function userDatabaseNodeCreate(
  canvasId: string,
  title: string,
  subTitle: string,
  x: number,
  y: number,
  templateId: string | null = null,
  createCanvas: boolean = false,
): Promise<Node> {
  return invoke<Node>("user_database_node_create", {
    canvasId,
    title,
    subTitle,
    x,
    y,
    templateId,
    createCanvas,
  });
}

/**
 * 在指定位置创建指定节点的副本。
 *
 * 副本继承源节点的标题、副标题、颜色和字段结构（不含字段值），不复制附件和边；
 * 影子节点与画布节点不允许复制。
 * @param id 被复制的节点 id
 * @param x 副本节点的 x 坐标
 * @param y 副本节点的 y 坐标
 * @returns 新建的副本节点
 */
export async function userDatabaseNodeCopy(
  id: string,
  x: number,
  y: number,
): Promise<Node> {
  return invoke<Node>("user_database_node_copy", { id, x, y });
}

/**
 * 修改节点的坐标。
 * @param id 节点 id
 * @param x 新 x 坐标
 * @param y 新 y 坐标
 * @returns 无返回值
 */
export async function userDatabaseNodeMoveNode(
  id: string,
  x: number,
  y: number,
): Promise<void> {
  return invoke("user_database_node_move_node", { id, x, y });
}

/**
 * 批量移动节点的坐标。
 * @param items 节点坐标列表，每个元素包含 id、x、y
 * @returns 无返回值
 */
export async function userDatabaseNodeMoveNodes(items: MoveNodeVO[]): Promise<void> {
  return invoke("user_database_node_move_nodes", { items });
}

/**
 * 批量跨画布迁移节点。
 *
 * items 中的 x/y 为前端算好的最终坐标（含目标画布视口中心定位与网格吸附），
 * 后端只负责校验与落库，不参与定位。
 * @param items 节点坐标列表，每个元素包含 id、x、y
 * @param targetCanvasId 目标画布 id
 * @returns 无返回值
 */
export async function userDatabaseNodeRelocateNodes(items: MoveNodeVO[], targetCanvasId: string): Promise<void> {
  return invoke("user_database_node_relocate_nodes", { items, targetCanvasId });
}

/**
 * 修改节点的标题和副标题。
 * @param id 节点 id
 * @param title 新标题
 * @param subTitle 新副标题
 * @returns 无返回值
 */
export async function userDatabaseNodeModify(
  id: string,
  title: string,
  subTitle: string,
): Promise<void> {
  return invoke("user_database_node_modify", { id, title, subTitle });
}

/**
 * 逻辑删除节点。
 * @param id 节点 id
 * @returns 逻辑删除后的节点对象
 */
export async function userDatabaseNodeLogicalDelete(id: string): Promise<Node> {
  return invoke<Node>("user_database_node_logical_delete", { id });
}

/**
 * 恢复逻辑删除的节点，并移动至新坐标。
 * @param id 节点 id
 * @param x 新 x 坐标
 * @param y 新 y 坐标
 * @returns 无返回值
 */
export async function userDatabaseNodeRestore(
  id: string,
  x: number,
  y: number,
): Promise<void> {
  return invoke("user_database_node_restore", { id, x, y });
}

/**
 * 物理删除节点和相连的边。
 * @param id 节点 id
 * @returns 无返回值
 */
export async function userDatabaseNodePhysicalDelete(
  id: string,
): Promise<void> {
  return invoke("user_database_node_physical_delete", { id });
}

/**
 * 查询指定画布内的节点列表。
 * 影子节点（返回项中 `shadow_id` 非 null 的项）的 title / sub_title / color /
 * canvas_ref_id 已被后端合并为原始节点的值；调用方无需另行解析原始节点。
 * @param canvasId 画布 id
 * @param deleted false 查询正常节点，true 查询已逻辑删除的节点
 * @returns 节点列表
 */
export async function userDatabaseNodeList(
  canvasId: string,
  deleted: boolean,
): Promise<NodeVO[]> {
  return invoke<NodeVO[]>("user_database_node_list", { canvasId, deleted });
}

/**
 * 在所有画布中按关键词搜索节点（关键词由后端按常见分隔符拆分，AND 匹配节点标题、副标题与画布名称）。
 * @param query 用户输入的原始查询字符串
 * @returns 搜索结果列表（按画布名、节点标题排序，上限 50 条）
 */
export async function userDatabaseNodeSearch(
  query: string,
): Promise<NodeSearchResponse[]> {
  return invoke<NodeSearchResponse[]>("user_database_node_search", { query });
}

/**
 * 设置指定节点的自定义颜色。
 * @param id 节点 id
 * @param color 规范化后的颜色字符串
 * @returns 无返回值
 */
export async function userDatabaseNodeSetColor(
  id: string,
  color: string,
): Promise<void> {
  return invoke("user_database_node_set_color", { id, color });
}

/**
 * 设置指定画布节点的自定义颜色。
 * @param id 画布 id
 * @param color 规范化后的颜色字符串
 * @returns 无返回值
 */
export async function userDatabaseCanvasSetColor(
  id: string,
  color: string,
): Promise<void> {
  return invoke("user_database_canvas_set_color", { id, color });
}

/**
 * 查询画布内节点的自定义颜色列表（用于颜色组合历史聚合）。
 * @returns 带自定义颜色的节点条目列表
 */
export async function userDatabaseNodeColorList(): Promise<NodeColorEntry[]> {
  return invoke<NodeColorEntry[]>("user_database_node_color_list");
}

// ==================== user_database / edge ====================

/**
 * 在指定画布内新建边（后端校验重复边与成环）。
 * @param canvasId 画布 id
 * @param sourceId 源节点 id
 * @param sourcePort 源节点连接桩
 * @param targetId 目标节点 id
 * @param targetPort 目标节点连接桩
 * @returns 新建的边
 */
export async function userDatabaseEdgeCreate(
  canvasId: string,
  sourceId: string,
  sourcePort: string,
  targetId: string,
  targetPort: string,
): Promise<Edge> {
  return invoke<Edge>("user_database_edge_create", {
    canvasId,
    sourceId,
    sourcePort,
    targetId,
    targetPort,
  });
}

/**
 * 物理删除边（边没有逻辑删除状态）。
 * 删除边会级联删除被引用子画布内的影子节点；若影子在子画布内有关联节点且
 * `confirmed` 为 false，后端返回 ErrorCode `EdgeDeleteDisconnectsNodes`，
 * 其 data 为 `{ nodes: string[] }`，列出将失去连接的节点标题。
 * @param id 边 id
 * @param confirmed 是否确认级联断开节点连接
 * @returns 无返回值
 */
export async function userDatabaseEdgeDelete(
  id: string,
  confirmed: boolean,
): Promise<void> {
  return invoke("user_database_edge_delete", { id, confirmed });
}

/**
 * 查询指定画布内的所有边。
 * @param canvasId 画布 id
 * @returns 该画布内的边列表
 */
export async function userDatabaseEdgeList(canvasId: string): Promise<Edge[]> {
  return invoke<Edge[]>("user_database_edge_list", { canvasId });
}

/**
 * 更新指定边的标题和详情。
 * @param id 边 id
 * @param title 新标题
 * @param description 新详情
 * @returns 无返回值
 */
export async function userDatabaseEdgeUpdate(
  id: string,
  title: string,
  description: string,
): Promise<void> {
  return invoke("user_database_edge_update", { id, title, description });
}

// ==================== user_database / registry ====================

/**
 * 查询用户数据库 registry 中指定变量的值。
 * @param name 变量名称
 * @returns 变量的值，不存在时返回 null
 */
export async function userDatabaseRegistryGet(name: string): Promise<string | null> {
  return invoke<string | null>("user_database_registry_get", { name });
}

/**
 * 写入用户数据库 registry 变量（随用户数据库统一保存）。
 * @param name 变量名称
 * @param value 变量值
 * @returns 无返回值
 */
export async function userDatabaseRegistrySet(name: string, value: string): Promise<void> {
  return invoke("user_database_registry_set", { name, value });
}

// ==================== user_database / log ====================

/**
 * 分页查询日志（按时间倒序）。
 * @param offset 偏移量
 * @param limit 数量上限
 * @returns 日志分页列表（含总数）
 */
export async function userDatabaseLogList(
  offset: number,
  limit: number,
): Promise<LogPageResponse> {
  return invoke<LogPageResponse>("user_database_log_list", { offset, limit });
}

// ==================== user_database / node_field ====================

/**
 * 查询指定节点的全部字段。
 * @param nodeId 节点 id
 * @returns 节点字段列表（按位置排序）
 */
export async function userDatabaseNodeFieldGet(
  nodeId: string,
): Promise<NodeFieldVO[]> {
  return invoke<NodeFieldVO[]>("user_database_node_field_get", { nodeId });
}

/**
 * 替换指定节点的全部字段。
 * @param nodeId 节点 id
 * @param fields 新的字段列表（按位置排序）
 * @returns 无返回值
 */
export async function userDatabaseNodeFieldSet(
  nodeId: string,
  fields: NodeFieldVO[],
): Promise<void> {
  return invoke("user_database_node_field_set", { nodeId, fields });
}

// ==================== user_database / dictionary ====================

/**
 * 查询全部字典条目。
 * @returns 字典条目列表（树形组织，按位置排序）
 */
export async function userDatabaseDictionaryList(): Promise<Dictionary[]> {
  return invoke<Dictionary[]>("user_database_dictionary_list");
}

/**
 * 替换全部字典条目。
 * @param entries 新的字典条目列表（树形组织，按位置排序）
 * @returns 无返回值
 */
export async function userDatabaseDictionarySet(
  entries: Dictionary[],
): Promise<void> {
  return invoke("user_database_dictionary_set", { entries });
}

// ==================== user_database / template ====================

/**
 * 新建一个空的模板。
 * @param name 模板名称
 * @returns 新建的模板
 */
export async function userDatabaseTemplateCreate(
  name: string,
): Promise<Template> {
  return invoke<Template>("user_database_template_create", { name });
}

/**
 * 从指定节点的字段结构创建模板。
 * @param nodeId 节点 id
 * @param name 模板名称
 * @returns 新建的模板
 */
export async function userDatabaseTemplateCreateFromNode(
  nodeId: string,
  name: string,
): Promise<Template> {
  return invoke<Template>("user_database_template_create_from_node", {
    nodeId,
    name,
  });
}

/**
 * 重命名模板。
 * @param id 模板 id
 * @param newName 新名称
 * @returns 无返回值
 */
export async function userDatabaseTemplateRename(
  id: string,
  newName: string,
): Promise<void> {
  return invoke("user_database_template_rename", { id, newName });
}

/**
 * 删除模板。
 * @param id 模板 id
 * @returns 无返回值
 */
export async function userDatabaseTemplateDelete(id: string): Promise<void> {
  return invoke("user_database_template_delete", { id });
}

/**
 * 查询全部模板。
 * @returns 模板列表（按 order 排序）
 */
export async function userDatabaseTemplateList(): Promise<Template[]> {
  return invoke<Template[]>("user_database_template_list");
}

/**
 * 查询指定模板的全部字段。
 * @param id 模板 id
 * @returns 模板字段列表（按位置排序）
 */
export async function userDatabaseTemplateGetFields(
  id: string,
): Promise<TemplateFieldVO[]> {
  return invoke<TemplateFieldVO[]>("user_database_template_get_fields", { id });
}

/**
 * 替换指定模板的全部字段。
 * @param id 模板 id
 * @param fields 新的字段列表（按位置排序）
 * @returns 无返回值
 */
export async function userDatabaseTemplateSetFields(
  id: string,
  fields: TemplateFieldVO[],
): Promise<void> {
  return invoke("user_database_template_set_fields", { id, fields });
}

/**
 * 导出模板数据：由后端弹出系统保存对话框，将 template、template_field、dictionary 数据导出到 SQLite 文件。
 * @returns 导出成功返回 true，用户取消返回 false
 */
export async function userDatabaseTemplateExport(): Promise<boolean> {
  return invoke<boolean>("user_database_template_export");
}

/**
 * 导入模板数据：由后端弹出系统文件选择对话框，读取 SQLite 文件并替换当前 template、template_field、dictionary 数据。
 * @returns 导入成功返回 true，用户取消返回 false
 */
export async function userDatabaseTemplateImport(): Promise<boolean> {
  return invoke<boolean>("user_database_template_import");
}

// ==================== user_database / attachment ====================

/**
 * 导入附件：由后端弹出系统文件选择对话框，将用户选中的文件加密后存为节点的附件。
 * @param nodeId 节点 id
 * @returns 新建附件的值对象；用户在系统对话框中取消选择时返回 null
 */
export async function userDatabaseAttachmentImport(
  nodeId: string,
): Promise<AttachmentVO | null> {
  return invoke<AttachmentVO | null>("user_database_attachment_import", {
    nodeId,
  });
}

/**
 * 按逻辑删除状态查询节点的附件列表（按导入时间升序）。
 * @param nodeId 节点 id
 * @param deleted false 查询正常附件，true 查询回收站中已逻辑删除的附件
 * @returns 附件值对象列表
 */
export async function userDatabaseAttachmentList(
  nodeId: string,
  deleted: boolean,
): Promise<AttachmentVO[]> {
  return invoke<AttachmentVO[]>("user_database_attachment_list", {
    nodeId,
    deleted,
  });
}

/**
 * 加载附件明文内容（后端解密后返回 IPC 二进制响应）。
 * 前端 invoke 实测收到的类型为 ArrayBuffer
 * （Tauri 自定义协议路径经 fetch response.arrayBuffer() 交付）。
 * @param id 附件 id
 * @returns 附件明文的二进制内容
 */
export async function userDatabaseAttachmentLoad(
  id: string,
): Promise<ArrayBuffer> {
  return invoke<ArrayBuffer>("user_database_attachment_load", { id });
}

/**
 * 导出附件：由后端弹出系统保存对话框，将附件明文写入用户选择的目标文件。
 * @param id 附件 id
 * @returns 是否完成导出；用户在系统对话框中取消选择时返回 false
 */
export async function userDatabaseAttachmentExport(
  id: string,
): Promise<boolean> {
  return invoke<boolean>("user_database_attachment_export", { id });
}

/**
 * 逻辑删除附件（移入回收站，附件文件保留）。
 * @param id 附件 id
 * @returns 无返回值
 */
export async function userDatabaseAttachmentLogicalDelete(
  id: string,
): Promise<void> {
  return invoke("user_database_attachment_logical_delete", { id });
}

/**
 * 恢复回收站中的附件。
 * @param id 附件 id
 * @returns 无返回值
 */
export async function userDatabaseAttachmentRestore(id: string): Promise<void> {
  return invoke("user_database_attachment_restore", { id });
}

/**
 * 物理删除附件（同时删除加密附件文件，不可恢复）。
 * @param id 附件 id
 * @returns 无返回值
 */
export async function userDatabaseAttachmentPhysicalDelete(
  id: string,
): Promise<void> {
  return invoke("user_database_attachment_physical_delete", { id });
}

/**
 * 列出孤儿附件文件（附件目录中存在但没有对应元数据的文件 id）。
 * 仅上报，由用户显式删除。
 * @returns 孤儿文件的 id 列表
 */
export async function userDatabaseAttachmentListOrphanFiles(): Promise<
  string[]
> {
  return invoke<string[]>("user_database_attachment_list_orphan_files");
}

/**
 * 删除指定的孤儿附件文件（不动元数据表，不记日志）。
 * @param id 孤儿文件 id（uuid）
 * @returns 无返回值
 */
export async function userDatabaseAttachmentRemoveOrphanFile(
  id: string,
): Promise<void> {
  return invoke("user_database_attachment_remove_orphan_file", { id });
}

/**
 * 更新附件文件内容：将新的明文内容加密后覆盖附件文件，并更新元数据中的文件大小。
 * @param id 附件 id
 * @param content 新的明文内容（UTF-8 字节）
 * @returns 无返回值
 */
export async function userDatabaseAttachmentUpdateFile(
  id: string,
  content: Uint8Array,
): Promise<void> {
  return invoke("user_database_attachment_update_file", { id, content });
}

/**
 * 交换两个附件的排序位置。
 * @param id1 附件1 id
 * @param id2 附件2 id
 * @returns 无返回值
 */
export async function userDatabaseAttachmentSwapSortOrder(
  id1: string,
  id2: string,
): Promise<void> {
  return invoke("user_database_attachment_swap_sort_order", { id1, id2 });
}

// ==================== user_database / export ====================

/** 数据库导出模式。 */
export type DatabaseExportMode =
  | "exclude-fields"
  | "mask-values"
  | "include-values";

/**
 * 导出整个用户数据库为 markdown 文件；由后端弹出系统保存对话框，用户在系统保存对话框中取消时返回 false。
 * 导出文件的固定文案语言取当前 i18n 语言（前端充当语言 gate）。
 * @param mode 字段导出模式
 * @returns 导出成功返回 true，用户取消返回 false
 */
export async function userDatabaseExport(
  mode: DatabaseExportMode,
): Promise<boolean> {
  return invoke<boolean>("user_database_export", {
    mode,
    locale: currentLocale.value,
  });
}

// ==================== backup ====================

/**
 * 全量备份数据目录（除日志外）；由后端弹出系统保存对话框，用户在系统保存对话框中取消时返回 false。
 * 备份过程中会通过 Tauri Event 上报进度（事件名 "backup-progress"）。
 * @param redundancyRatio 冗余比例，范围 (0, 1)，如 0.05 表示增加 5% 体积
 * @returns 备份完成返回 true，用户取消系统对话框返回 false
 */
export async function backupBackup(redundancyRatio: number): Promise<boolean> {
  return invoke<boolean>("backup_backup", { redundancyRatio });
}

/**
 * 全量还原数据目录；由后端执行完整还原流程。
 * 路径必选（由 `restoreProbe` 选定），不再有"后端弹对话框"的路径。
 * 还原过程中通过 Tauri Event 上报进度（事件名 "restore-progress"）。
 * @param sourcePath 备份文件绝对路径（必选，由 restoreProbe 选定）
 * @returns 还原完成即返回；失败抛 ErrorCode
 */
export async function backupRestore(sourcePath: string): Promise<void> {
  return invoke<void>("backup_restore", { sourcePath });
}

/** 后端探测备份文件的返回结构。 */
export interface RestoreProbeResult {
  /** 是否可还原（损坏 shard 数 ≤ parity）。 */
  recoverable: boolean;
  /** 损坏 shard 数。 */
  lost: number;
  /** 可恢复上限（parity shard 数）。 */
  limit: number;
  /** 探测通过的文件路径，可继续用于 restoreDataDirectory。 */
  source_path: string;
}

/**
 * 探测备份文件：弹文件对话框、校验 Header 与 shard SHA-256，但不替换数据。
 * @returns 用户取消返回 null；否则返回探测结果（含可继续使用的源路径）
 */
export async function backupRestoreProbe(): Promise<RestoreProbeResult | null> {
  return invoke<RestoreProbeResult | null>("backup_restore_probe");
}

/**
 * 查询当前数据目录（除日志外）的总字节数，用于前端预估备份体积。
 * @returns 数据目录总字节数
 */
export async function backupDataDirectorySize(): Promise<number> {
  return invoke<number>("backup_data_directory_size");
}

/**
 * 还原后刷新 preference 模块的内存 connection，让它重新持有磁盘文件的所有权。
 * 必须与 reclaimMetadata / reclaimUserDatabase 一起调用，否则 exit-save 会用陈旧内存覆盖还原结果。
 */
export async function reclaimPreference(): Promise<void> {
  return invoke<void>("reclaim_preference");
}

/**
 * 还原后刷新 metadata 模块的内存 connection，让它重新持有磁盘文件的所有权。
 */
export async function reclaimMetadata(): Promise<void> {
  return invoke<void>("reclaim_metadata");
}

/**
 * 还原后刷新 user_database 状态：若当前有 user_database 处于打开状态则强制关闭。
 * Home.vue 路径下通常无 user_database 打开，属于防御性兜底。
 */
export async function reclaimUserDatabase(): Promise<void> {
  return invoke<void>("reclaim_user_database");
}


