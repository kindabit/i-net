/**
 * Tauri 后端类型在前端的定义。
 *
 * 字段命名与后端 serde 序列化结果保持一致（snake_case 字段名）。
 * ErrorCode 相关定义见 @/error-code。
 */

/** 用户数据库元数据（对应后端 Metadata 实体） */
export interface Metadata {
  /** 数据库 id（uuid） */
  id: string;
  /** 数据库名称 */
  name: string;
  /** 是否归档 */
  archived: boolean;
  /** 创建时间，毫秒时间戳 */
  create_time: number;
  /** 修改时间，毫秒时间戳 */
  modify_time: number;
  /** 最后打开时间，毫秒时间戳 */
  last_open_time: number;
}

/** 画布（对应后端 Canvas 实体） */
export interface Canvas {
  /** 画布 id（uuid） */
  id: string;
  /** 父画布 id，根画布为 null */
  parent_id: string | null;
  /** 画布名称（唯一） */
  name: string;
  /** 画布在画布宇宙中的 x 坐标 */
  x: number;
  /** 画布在画布宇宙中的 y 坐标 */
  y: number;
  /** 是否逻辑删除 */
  deleted: boolean;
  /** 序列化自定义颜色，空串 = 默认 */
  color: string;
}

/** 视口（对应后端 Viewport 实体） */
export interface Viewport {
  /** 画布 id（特殊值表示画布宇宙的视口） */
  canvas_id: string;
  /** 视口 x（屏幕坐标：画布原点相对视口中心的水平偏移） */
  x: number;
  /** 视口 y（屏幕坐标：画布原点相对视口中心的垂直偏移） */
  y: number;
  /** 缩放比例 */
  zoom: number;
}

/** 节点（对应后端 Node 实体） */
export interface Node {
  /** 节点 id（uuid） */
  id: string;
  /** 所属画布 id */
  canvas_id: string;
  /** 节点在画布中的 x 坐标 */
  x: number;
  /** 节点在画布中的 y 坐标 */
  y: number;
  /** 节点标题 */
  title: string;
  /** 节点副标题 */
  subtitle: string;
  /** 节点引用的子画布 id，仅画布数据节点有值 */
  canvas_ref_id: string | null;
  /** 是否逻辑删除 */
  deleted: boolean;
  /** 序列化自定义颜色，空串 = 默认 */
  color: string;
  /** 产生该影子节点的边 id；null 表示数据节点（影子的产生边为其存在依据，与其同生共死） */
  shadow_producing_edge_id: string | null;
}

/** 节点列表（user_database_node_list）的返回项：在 Node 基础上附带影子节点的展示信息。
 * 影子节点的 title / subtitle / color 已被后端合并为本体节点的值（canvas_ref_id 不合并，恒为 null；
 * 出向影子本体引用的子画布 id 由 shadow_origin_canvas_ref_id 单独携带）；
 * 数据节点的扩展字段均为 null。 */
export interface NodeVO extends Node {
  /** 影子节点本体节点的 id；仅影子节点有值（沿产生边链解析到非影子节点） */
  shadow_origin_id: string | null;
  /** 影子节点的本体节点是否已被逻辑删除；仅影子节点有值 */
  shadow_origin_deleted: boolean | null;
  /** 影子节点的方向；仅影子节点有值 */
  shadow_direction: "inflow" | "outflow" | null;
  /** 影子节点本体（画布数据节点）引用的子画布 id；仅出向影子有值，供双击影子节点时跳转定位 */
  shadow_origin_canvas_ref_id: string | null;
}

/** 边（对应后端 Edge 实体） */
export interface Edge {
  /** 边 id（uuid） */
  id: string;
  /** 所属画布 id */
  canvas_id: string;
  /** 源节点 id */
  source_id: string;
  /** 源连接桩 */
  source_handle: string;
  /** 目标节点 id */
  target_id: string;
  /** 目标连接桩 */
  target_handle: string;
  /** 边的标题，始终显示在边上 */
  title: string;
  /** 边的详情，鼠标悬浮时显示 */
  description: string;
}

/**
 * 日志行为（对应后端 Action 枚举），采用 serde tag = "variant"、content = "data"
 * 的序列化格式；variant 和 data 直接透传给 i18n 模块做文案插值（见 i18n 的 log 模块）。
 * data 为不透明键值对；个别行为（如节点字段编辑）的嵌套数据形状由使用处自行收窄。
 */
export interface LogAction {
  /** 行为名（Action 枚举的 variant 名） */
  variant: string;
  /** 行为数据载荷 */
  data: Record<string, unknown>;
}

/** 日志条目（对应后端 LogListResponse） */
export interface LogListResponse {
  /** 日志 id（uuid） */
  id: string;
  /** 日志行为及其数据 */
  action: LogAction;
  /** 时间，毫秒时间戳 */
  time: number;
}

/** 日志分页列表的响应结构：包含当前页的日志列表和日志总条数。 */
export interface LogPageResponse {
  /** 当前页的日志列表，按时间倒序排序。 */
  items: LogListResponse[];
  /** 日志总条数。 */
  total: number;
}

/** 节点字段变更，记录一次字段编辑中单个字段的变化。value 为字段值字符串（格式见 field-types 模块），null 表示无值。 */
export type NodeFieldChange =
  | { variant: "Added"; data: { name: string; field_type: string; value: string | null } }
  | { variant: "Modified"; data: { name: string; old_field_type: string; new_field_type: string; old_value: string | null; new_value: string | null } }
  | { variant: "Removed"; data: { name: string; field_type: string; old_value: string | null } };

/** 节点字段值对象。字段顺序由数组位置表达。value 为字段值字符串（格式由前端字段类型系统定义，后端不解析其内容），null 表示无值。 */
export interface NodeFieldVO {
  name: string;
  field_type: string;
  value: string | null;
  dictionary_id: string | null;
}

/** 迁移导入的节点值对象：前端构造好的节点数据（含布局坐标与字段列表），传给后端聚合导入接口。 */
export interface ImportedNodeVO {
  /** 节点标题 */
  title: string;
  /** 节点副标题 */
  subtitle: string;
  /** 节点在目标画布中的 x 坐标 */
  x: number;
  /** 节点在目标画布中的 y 坐标 */
  y: number;
  /** 节点字段列表，顺序即存储顺序 */
  fields: NodeFieldVO[];
}

/**
 * 迁移导入的边值对象：以节点列表下标表达父子关系（source_index 为父、target_index 为子），
 * 恒满足 source_index < target_index（节点按深度优先先序排列，父节点下标恒小于子节点）。
 */
export interface ImportedEdgeVO {
  /** 父节点在节点列表中的下标 */
  source_index: number;
  /** 子节点在节点列表中的下标 */
  target_index: number;
}

/**
 * 多画布导入的画布数据节点值对象：在父画布中创建一个引用子画布的画布数据节点。
 * ref_index 必须指向自身画布的直接子画布（被引用画布的 parent_index 必须指回自身画布）；
 * 标题不随数据传入，后端写入时取被引用画布去重后的最终名称。
 */
export interface ImportedCanvasNodeVO {
  /** 画布数据节点在其画布中的 x 坐标 */
  x: number;
  /** 画布数据节点在其画布中的 y 坐标 */
  y: number;
  /** 被引用子画布在导入画布列表中的下标 */
  ref_index: number;
}

/**
 * 多画布导入的画布值对象：按迁移源 group 层级构造好的画布数据。
 * 列表按深度优先先序排列（父画布下标恒小于子画布），第一个元素为迁移源根 group
 * 对应的画布（parent_index 为 null，挂到根画布）；宇宙坐标由前端按 group 层级
 * 以树形布局计算，画布内节点坐标由前端以矩阵布局计算。
 */
export interface ImportedCanvasVO {
  /** 画布名称（重名时后端自动追加 " 2"、" 3"…） */
  name: string;
  /** 画布在画布宇宙中的 x 坐标 */
  x: number;
  /** 画布在画布宇宙中的 y 坐标 */
  y: number;
  /** 父画布在导入画布列表中的下标；null 表示挂到根画布 */
  parent_index: number | null;
  /** 画布内的数据节点列表（矩阵布局坐标） */
  nodes: ImportedNodeVO[];
  /** 画布内的画布数据节点列表（引用直接子画布，矩阵布局坐标） */
  canvas_nodes: ImportedCanvasNodeVO[];
}

/** 模板字段值对象。模板字段只定义结构，不含值。 */
export interface TemplateFieldVO {
  name: string;
  field_type: string;
  dictionary_id: string | null;
}

/** 模板。 */
export interface Template {
  id: string;
  name: string;
  sort_order: number;
}

/** 字典条目，树形组织。 */
export interface Dictionary {
  id: string;
  parent_id: string | null;
  value: string;
  sort_order: number;
}

/** 连接桩（上下左右各一个） */
export type Handle = "top" | "right" | "bottom" | "left";

/** 节点全局搜索结果项（对应后端 NodeSearchResponse） */
export interface NodeSearchResponse {
  /** 节点 id（uuid） */
  id: string;
  /** 所属画布 id */
  canvas_id: string;
  /** 节点在画布中的 x 坐标 */
  x: number;
  /** 节点在画布中的 y 坐标 */
  y: number;
  /** 节点标题 */
  title: string;
  /** 节点副标题 */
  subtitle: string;
  /** 节点引用的子画布 id，仅画布数据节点有值 */
  canvas_ref_id: string | null;
  /** 节点所在画布的名称 */
  canvas_name: string;
}

/** 带自定义颜色的数据节点条目（data_node_color_list 的返回项） */
export interface DataNodeColorEntry {
  /** 节点标题 */
  title: string;
  /** 数据节点自定义颜色字符串 */
  color: string;
}

/** 带自定义颜色的画布节点条目（canvas_node_color_list 的返回项） */
export interface CanvasNodeColorEntry {
  /** 画布名称 */
  name: string;
  /** 父画布 id，根画布为 null */
  parent_id: string | null;
  /** 画布节点自定义颜色字符串 */
  color: string;
}

/** 批量移动节点/画布时单个条目的值对象（与后端 node::vo::MoveNodeVO、canvas::vo::MoveNodeVO 对应）。 */
export interface MoveNodeVO {
  id: string;
  x: number;
  y: number;
}

/** 附件值对象（对应后端 AttachmentVO） */
export interface AttachmentVO {
  /** 附件 id（uuid） */
  id: string;
  /** 附件的原始文件名 */
  file_name: string;
  /** 附件明文内容的大小，单位为字节 */
  size: number;
  /** 附件的导入时间，毫秒时间戳 */
  create_time: number;
  /** 附件文件是否丢失（元数据存在但附件目录中没有对应文件） */
  missing_file: boolean;
}
