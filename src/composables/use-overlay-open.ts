/**
 * 判断当前是否有对话框或菜单处于打开状态。
 * Vuetify 的 VDialog 与 VMenu 均基于 VOverlay，打开时根元素带 v-overlay--active 类、
 * 关闭即移除；v-snackbar 与 v-tooltip 同样带 v-overlay--active，
 * 故用 v-dialog/v-menu 类名限定以排除提示条与悬浮提示。
 * 该判据覆盖应用内所有层级的 VDialog/VMenu（含嵌套对话框与父级页面的对话框）。
 * @returns 存在打开的对话框或菜单时返回 true
 */
export function hasActiveDialogOrMenu(): boolean {
  return document.querySelector(".v-overlay--active.v-dialog, .v-overlay--active.v-menu") !== null;
}
