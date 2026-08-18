<!--
  数据目录还原对话框。

  流程：
  1. 用户点击"选择备份文件"，后端弹出系统打开对话框，做 header + shard 校验。
  2. 校验通过 → 显示校验结论 + "确认还原"按钮。
  3. 校验不通过 → 显示不可还原结论，"确认还原"按钮禁用。
  4. 用户点击"确认还原" → 后端执行完整还原流程。
  5. 还原完成后 emit "success" 事件，Home.vue 弹出"还原成功"对话框。
-->
<script setup lang="ts">
import { computed, ref } from "vue";
import { t } from "@/i18n";
import {
  backupRestore,
  backupRestoreProbe,
  type RestoreProbeResult,
} from "@/api";
import { snackbarErrorCode } from "@/composables/use-snackbar";
import { useBackupProgress } from "@/composables/use-backup-progress";

const emit = defineEmits<{
  /** 还原成功后通知父组件弹出"还原成功"对话框 */
  success: [];
}>();

const dialog = ref(false);
/** 探测结果：null 表示尚未探测。结果中的 `source_path` 供"确认还原"阶段使用。 */
const probeResult = ref<RestoreProbeResult | null>(null);
/** 是否处于还原流程中（用于禁用按钮与显示进度）。 */
const restoring = ref(false);

const { phase: progressPhase, progress: progressValue } = useBackupProgress(
  "restore-progress",
);

/** 当前阶段对应的 i18n 文案。 */
const phaseText = computed(() => {
  switch (progressPhase.value) {
    case "restore_read_header":
      return t("home.restore.phase-restore-read-header");
    case "restore_verify":
      return t("home.restore.phase-restore-verify");
    case "restore_decode":
      return t("home.restore.phase-restore-decode");
    case "restore_unpack":
      return t("home.restore.phase-restore-unpack");
    case "restore_clear":
      return t("home.restore.phase-restore-clear");
    case "restore_move":
      return t("home.restore.phase-restore-move");
    default:
      return t("home.restore.in-progress");
  }
});

/** 校验结论文案。 */
const probeText = computed(() => {
  const r = probeResult.value;
  if (!r) return "";
  if (!r.recoverable) return t("home.restore.verify-result-irrecoverable");
  if (r.lost === 0) return t("home.restore.verify-result-recoverable");
  return t("home.restore.verify-result-recoverable-with-loss", {
    lost: r.lost,
  });
});

/** 还原进度百分比（0~100）。 */
const progressPercent = computed(() =>
  Math.round((progressValue.value ?? 0) * 100),
);

/** 是否可以执行"确认还原"按钮（已探测且可还原）。 */
const canConfirm = computed(() => probeResult.value?.recoverable === true);

/**
 * 打开对话框，重置状态。
 */
function open() {
  dialog.value = true;
  probeResult.value = null;
  restoring.value = false;
}

/**
 * 触发"选择备份文件"按钮：调 `restoreProbe` 让后端弹对话框并做校验。
 */
async function pickAndProbe() {
  try {
    const result = await backupRestoreProbe();
    if (!result) return; // 用户在系统对话框中取消
    probeResult.value = result;
  } catch (error) {
    snackbarErrorCode(error);
  }
}

/**
 * 触发"确认还原"按钮：调 `restoreDataDirectory` 执行完整还原。
 */
async function confirmRestore() {
  if (!probeResult.value || !canConfirm.value) return;
  restoring.value = true;
  try {
    await backupRestore(probeResult.value.source_path);
    restoring.value = false;
    dialog.value = false;
    emit("success");
  } catch (error) {
    restoring.value = false;
    snackbarErrorCode(error);
  }
}

/** 关闭对话框（仅在非还原状态下生效）。 */
function close() {
  if (restoring.value) return;
  dialog.value = false;
}

defineExpose({ open, close });
</script>

<template>
  <VDialog v-model="dialog" max-width="32rem" persistent>
    <VCard>
      <VCardTitle>{{ t("home.restore.title") }}</VCardTitle>
      <VCardText class="restore-text">
        <div class="warning">{{ t("home.restore.warning-text") }}</div>
        <div v-if="probeResult" class="probe-result">
          {{ probeText }}
        </div>
        <div v-if="restoring" class="progress-block">
          <div class="phase-label">{{ phaseText }}</div>
          <VProgressLinear
            :model-value="progressPercent"
            color="primary"
            height="0.5rem"
            rounded
          />
        </div>
      </VCardText>
      <VCardActions>
        <VSpacer />
        <VBtn variant="text" :disabled="restoring" @click="close">
          {{ t("home.restore.cancel") }}
        </VBtn>
        <VBtn
          color="primary"
          variant="text"
          :disabled="restoring || probeResult !== null"
          @click="pickAndProbe"
        >
          {{ t("home.restore.select-file") }}
        </VBtn>
        <VBtn
          color="primary"
          variant="flat"
          :disabled="!canConfirm || restoring"
          :loading="restoring"
          @click="confirmRestore"
        >
          {{ t("home.restore.confirm-button") }}
        </VBtn>
      </VCardActions>
    </VCard>
  </VDialog>
</template>

<style lang="scss" scoped>
.restore-text {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.warning {
  font-size: 0.875rem;
  color: rgb(var(--v-theme-error));
}

.probe-result {
  font-size: 0.875rem;
}

.progress-block {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.phase-label {
  font-size: 0.875rem;
}
</style>