import { watch, type Ref } from "vue";

/**
 * 菜单弹层"点击外部关闭"的兜底实现。
 * vuetify 4.x 的 vClickOutside 会把点击任意 .v-overlay__content（如对话框内容区）
 * 豁免为"内部"，导致对话框内的 VMenu 无法通过点击对话框区域关闭。
 * 此 composable 在菜单打开时挂 document 级 mousedown 监听：
 * 点击目标不在弹层内容（popperSelector）内时关闭菜单。
 * @param menuOpen 菜单打开状态的 ref
 * @param popperSelector 弹层内容容器的选择器（点击其内部不关闭）
 */
export function useMenuDismiss(menuOpen: Ref<boolean>, popperSelector: string): void {
  watch(menuOpen, (open, _prev, onCleanup) => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if ((e.target as HTMLElement | null)?.closest(popperSelector)) return;
      menuOpen.value = false;
    };
    // 延迟一帧挂载，避免打开菜单的同一次 activator 点击冒泡触发关闭
    const frame = requestAnimationFrame(() => {
      document.addEventListener("mousedown", handler, true);
    });
    onCleanup(() => {
      cancelAnimationFrame(frame);
      document.removeEventListener("mousedown", handler, true);
    });
  });
}
