/**
 * 备份/还原进度事件订阅 composable。
 *
 * 通过 Tauri Event 监听后端在打包/解包阶段上报的进度，统一对外暴露 ref 状态。
 * 组件挂载时订阅、卸载时取消订阅；多次调用会得到独立的 ref 与监听器。
 */
import { onMounted, onUnmounted, Ref, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** 后端进度事件阶段标识（与 Rust 端 Phase 枚举的 snake_case 一一对应）。 */
export type BackupPhase =
  | "backup_pack"
  | "backup_encode"
  | "backup_write"
  | "restore_read_header"
  | "restore_verify"
  | "restore_decode"
  | "restore_unpack"
  | "restore_clear"
  | "restore_move";

/** 进度事件 payload。 */
export interface BackupProgressPayload {
  phase: BackupPhase;
  progress: number;
}

/**
 * 订阅指定事件名的进度事件。
 *
 * @param event 后端事件名（使用 `backup-progress` 或 `restore-progress`）。
 * @returns 当前 phase、progress 两个 ref，以及手动取消监听的 unlisten。
 */
export function useBackupProgress(event: "backup-progress" | "restore-progress"): {
  phase: Ref<BackupPhase | null>;
  progress: Ref<number>;
  unlisten: () => void;
} {
  const phase = ref<BackupPhase | null>(null);
  const progress = ref(0);
  let stop: UnlistenFn | undefined;

  onMounted(() => {
    void listen<BackupProgressPayload>(event, (e) => {
      phase.value = e.payload.phase;
      progress.value = e.payload.progress;
    }).then((fn) => {
      stop = fn;
    });
  });
  onUnmounted(() => {
    stop?.();
    stop = undefined;
  });

  return {
    phase,
    progress,
    unlisten: () => stop?.(),
  };
}
