<!--
  数据目录备份对话框。

  流程：
  1. 打开时异步查询当前数据目录大小。
  2. 用户调整冗余比例（默认 5%），实时显示预估备份大小。
  3. 点击"开始备份"调用后端命令，后端弹出系统保存对话框。
  4. 备份过程中通过 "backup-progress" 事件更新进度条与阶段文案。
  5. 完成后 snackbar 提示并自动关闭对话框。
-->
<script setup lang="ts">
import { computed, ref } from "vue";
import { t } from "@/i18n";
import {
  backupBackup,
  backupDataDirectorySize,
} from "@/api";
import { snackbarErrorCode, snackbarText } from "@/composables/use-snackbar";
import { useBackupProgress } from "@/composables/use-backup-progress";

const dialog = ref(false);
const dataSize = ref<number | null>(null);
const redundancyRatio = ref(0.05);
const backing = ref(false);

const {
  phase: progressPhase,
  progress: progressValue,
} = useBackupProgress("backup-progress");

/** 把字节数格式化为人类可读的字符串。 */
function formatSize(bytes: number | null): string {
  if (bytes === null) return "…";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

/**
 * 预估备份大小。
 *
 * 备份文件 ≈ 原始数据 × (1 + 冗余比例)（shard 区 + 少量元数据；忽略 tar 头开销）。
 */
const estimatedSize = computed(() => {
  if (dataSize.value === null) return null;
  return Math.ceil(dataSize.value * (1 + redundancyRatio.value));
});

/** 当前阶段对应的 i18n 文案。 */
const phaseText = computed(() => {
  switch (progressPhase.value) {
    case "backup_pack":
      return t("home.backup.phase-backup-pack");
    case "backup_encode":
      return t("home.backup.phase-backup-encode");
    case "backup_write":
      return t("home.backup.phase-backup-write");
    default:
      return t("home.backup.in-progress");
  }
});

/** 当前阶段的进度百分比（0~100）。 */
const progressPercent = computed(() =>
  Math.round((progressValue.value ?? 0) * 100),
);

/**
 * 打开对话框；首次打开时拉取当前数据目录大小。
 */
async function open() {
  dialog.value = true;
  if (dataSize.value === null) {
    try {
      dataSize.value = await backupDataDirectorySize();
    } catch (error) {
      snackbarErrorCode(error);
      dialog.value = false;
    }
  }
}

function close() {
  if (backing.value) return;
  dialog.value = false;
}

defineExpose({ open, close });

/**
 * 触发后端备份命令；用户取消系统对话框时不视为错误。
 *
 * 不监听进度事件做收尾：invoke 返回即意味着后端已走完整流程；
 * 进度事件只用于进度条展示。
 */
async function start() {
  backing.value = true;
  try {
    const saved = await backupBackup(redundancyRatio.value);
    if (saved) {
      snackbarText(t("home.backup.success"), "success");
    } else {
      snackbarText(t("home.backup.cancelled"), "info");
    }
  } catch (error) {
    snackbarErrorCode(error);
  } finally {
    backing.value = false;
    dialog.value = false;
  }
}
</script>

<template>
  <VDialog v-model="dialog" max-width="32rem" persistent>
    <VCard>
      <VCardTitle>{{ t("home.backup.title") }}</VCardTitle>
      <VCardText class="backup-text">
        <div class="info-line">
          {{ t("home.backup.data-size", { size: formatSize(dataSize) }) }}
        </div>
        <div class="ratio-row">
          <VTextField
            v-model.number="redundancyRatio"
            type="number"
            :label="t('home.backup.ratio-label')"
            :hint="t('home.backup.ratio-hint')"
            :disabled="backing"
            :min="0.01"
            :max="0.5"
            :step="0.01"
            variant="outlined"
            persistent-hint
            class="ratio-input"
          />
          <div class="estimated-size">
            {{ t("home.backup.estimated-size", { size: formatSize(estimatedSize) }) }}
          </div>
          <div class="volume-structure">
            {{ t("home.backup.volume-structure") }}
          </div>
        </div>
        <div v-if="backing" class="progress-block">
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
        <VBtn variant="text" :disabled="backing" @click="close">
          {{ t("home.backup.cancel") }}
        </VBtn>
        <VBtn
          color="primary"
          variant="flat"
          :loading="backing"
          :disabled="backing || dataSize === null"
          @click="start"
        >
          {{ t("home.backup.start") }}
        </VBtn>
      </VCardActions>
    </VCard>
  </VDialog>
</template>

<style lang="scss" scoped>
.backup-text {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.info-line {
  font-size: 0.875rem;
  opacity: 0.85;
}

.ratio-row {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.ratio-input {
  width: 100%;
}

.estimated-size {
  font-size: 0.875rem;
  opacity: 0.75;
}

.volume-structure {
  font-size: 0.75rem;
  opacity: 0.6;
  line-height: 1.4;
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