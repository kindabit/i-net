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
  /** 视口中心的 x 坐标 */
  x: number;
  /** 视口中心的 y 坐标 */
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
  sub_title: string;
  /** 节点引用的子画布 id，仅画布节点有值 */
  canvas_ref_id: string | null;
  /** 是否逻辑删除 */
  deleted: boolean;
  /** 序列化自定义颜色，空串 = 默认 */
  color: string;
  /** 影子节点指向的原始节点 id；null 表示普通节点 */
  shadow_id: string | null;
}

/** 节点列表（user_database_node_list）的返回项：在 Node 基础上附带影子节点的展示信息。
 * 影子节点的 title / sub_title / color 已被后端合并为根本体节点的值（canvas_ref_id 不合并，恒为 null；
 * 出向影子根本体引用的子画布 id 由 shadow_origin_canvas_ref_id 单独携带）；
 * 普通节点的扩展字段均为 null。 */
export interface NodeVO extends Node {
  /** 影子节点根本体节点的 id；仅影子节点有值（沿产生边链解析到非影子节点） */
  shadow_origin_id: string | null;
  /** 影子节点的原始节点是否已被逻辑删除；仅影子节点有值 */
  shadow_origin_deleted: boolean | null;
  /** 影子节点的方向；仅影子节点有值 */
  shadow_direction: "inflow" | "outflow" | null;
  /** 影子节点根本体（画布节点）引用的子画布 id；仅出向影子有值，供双击影子节点时跳转定位 */
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
  /** 源节点连接桩 */
  source_port: string;
  /** 目标节点 id */
  target_id: string;
  /** 目标节点连接桩 */
  target_port: string;
  /** 边的标题，始终显示在边上 */
  title: string;
  /** 边的详情，鼠标悬浮时显示 */
  description: string;
}

/**
 * 日志行为（对应后端 Action 枚举），采用 serde tag = "variant"、content = "data"
 * 的序列化格式；variant 和 data 直接透传给 i18n 模块做文案插值（见 i18n 的 log 模块）。
 */
export type LogAction =
  | { variant: string; data: Record<string, unknown> }
  | { variant: "NodeFieldsModify"; data: { node_title: string; changes: NodeFieldChange[] } };

/** 日志条目（对应后端 LogListResponse） */
export interface LogListResponse {
  /** 日志 id（uuid） */
  id: string;
  /** 被操作对象的 id */
  object_id: string;
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

/** 字段值，前后端传输的值载体。variant 名与字段类型 schema 中的底层数据类型 key 一致。 */
export type FieldValue =
  | { variant: "string"; data: string | null }
  | { variant: "decimal"; data: string | null }
  | { variant: "instant"; data: number | null }
  | { variant: "instantRange"; data: [number, number] | null };

/** 节点字段变更，记录一次字段编辑中单个字段的变化。 */
export type NodeFieldChange =
  | { variant: "Added"; data: { name: string; field_type: string; value: FieldValue } }
  | { variant: "Modified"; data: { name: string; old_field_type: string; new_field_type: string; old_value: FieldValue; new_value: FieldValue } }
  | { variant: "Removed"; data: { name: string; field_type: string; old_value: FieldValue } };

/** 节点字段值对象。字段顺序由数组位置表达。 */
export interface NodeFieldVO {
  name: string;
  field_type: string;
  type_config: Record<string, unknown> | null;
  value: FieldValue;
  dictionary_id: string | null;
}

/** 模板字段值对象。模板字段只定义结构，不含值。 */
export interface TemplateFieldVO {
  name: string;
  field_type: string;
  type_config: Record<string, unknown> | null;
  dictionary_id: string | null;
}

/** 模板。 */
export interface Template {
  id: string;
  name: string;
  order: number;
}

/** 字典条目，树形组织。 */
export interface Dictionary {
  id: string;
  parent_id: string | null;
  value: string;
  order: number;
}

/** 节点连接桩（上下左右各一个） */
export type NodePort = "top" | "right" | "bottom" | "left";

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
  sub_title: string;
  /** 节点引用的子画布 id，仅画布节点有值 */
  canvas_ref_id: string | null;
  /** 节点所在画布的名称 */
  canvas_name: string;
}

/** 带自定义颜色的节点条目（node_color_list 的返回项） */
export interface NodeColorEntry {
  /** 节点标题 */
  title: string;
  /** 节点自定义颜色字符串 */
  color: string;
}

/** 带自定义颜色的画布条目（canvas_color_list 的返回项） */
export interface CanvasColorEntry {
  /** 画布名称 */
  name: string;
  /** 父画布 id，根画布为 null */
  parent_id: string | null;
  /** 画布自定义颜色字符串 */
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
