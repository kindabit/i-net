/**
 * vue-flow 视口持久化组合式函数。
 *
 * 封装视口加载、坐标转换（中心语义 ↔ 屏幕偏移）、
 * 防抖保存等逻辑，供每个使用 vue-flow 的页面复用。
 */
import { ref, type Ref } from "vue";
import debounce from "lodash/debounce";
import { userDatabaseViewportGet, userDatabaseViewportSet } from "@/api";
import { snackbarErrorCode } from "@/composables/use-snackbar";

export function useViewportPersistence(
  canvasId: string | null,
  containerRef: Ref<HTMLElement | undefined>,
) {
  const initial = ref<{ x: number; y: number; zoom: number }>({
    x: 0,
    y: 0,
    zoom: 1,
  });
  const current = ref<{ x: number; y: number; zoom: number }>({
    x: 0,
    y: 0,
    zoom: 1,
  });

  function getHalfSize(): { hw: number; hh: number } {
    if (!containerRef.value) return { hw: 0, hh: 0 };
    const { clientWidth: w, clientHeight: h } = containerRef.value;
    return { hw: w / 2, hh: h / 2 };
  }

  async function load() {
    try {
      const vp = await userDatabaseViewportGet(canvasId);
      const { hw, hh } = getHalfSize();
      initial.value = { x: vp.x + hw, y: vp.y + hh, zoom: vp.zoom };
    } catch {
      const { hw, hh } = getHalfSize();
      initial.value = { x: hw, y: hh, zoom: 1 };
    }
    current.value = { ...initial.value };
  }

  const save = debounce(
    (vp: { x: number; y: number; zoom: number }) => {
      current.value = vp;
      const { hw, hh } = getHalfSize();
      userDatabaseViewportSet(canvasId, vp.x - hw, vp.y - hh, vp.zoom).catch(
        snackbarErrorCode,
      );
    },
    500,
  );

  function flush() {
    save.flush();
  }

  return { initial, current, load, save, flush };
}
