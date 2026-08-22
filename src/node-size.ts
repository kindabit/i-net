/**
 * 节点尺寸常量模块。
 *
 * 普通画布中的节点（DataNode.vue 渲染的数据节点/画布节点/影子节点）为固定尺寸，
 * 本模块的 px 数值是其唯一事实来源；rem 形式按项目 CSS 规范的 16px 基准派生，
 * 供组件内联样式使用。画布宇宙中的节点（CanvasNode.vue 渲染）为内容自适应尺寸，
 * 本模块仅提供自动布局读不到实测尺寸时使用的兜底估算值。
 */

/** rem 换算基准（项目 CSS 规范：16px = 1rem）。 */
const REM_BASE_PX = 16;

/** 数据节点固定宽度（px）。 */
export const DATA_NODE_WIDTH = 160;
/** 数据节点固定高度（px）。 */
export const DATA_NODE_HEIGHT = 80;
/** 数据节点半宽（px），用于由左上角坐标换算节点中心。 */
export const DATA_NODE_HALF_WIDTH = DATA_NODE_WIDTH / 2;
/** 数据节点半高（px），用于由左上角坐标换算节点中心。 */
export const DATA_NODE_HALF_HEIGHT = DATA_NODE_HEIGHT / 2;
/** 数据节点固定宽度（rem），供 DataNode.vue 根元素内联样式使用。 */
export const DATA_NODE_WIDTH_REM = `${DATA_NODE_WIDTH / REM_BASE_PX}rem`;
/** 数据节点固定高度（rem），供 DataNode.vue 根元素内联样式使用。 */
export const DATA_NODE_HEIGHT_REM = `${DATA_NODE_HEIGHT / REM_BASE_PX}rem`;

/** 画布宇宙节点兜底宽度（px）：CanvasNode 为内容自适应尺寸，自动布局读不到实测 dimensions 时使用。 */
export const CANVAS_NODE_FALLBACK_WIDTH = 130;
/** 画布宇宙节点兜底高度（px）：同 CANVAS_NODE_FALLBACK_WIDTH。 */
export const CANVAS_NODE_FALLBACK_HEIGHT = 60;
