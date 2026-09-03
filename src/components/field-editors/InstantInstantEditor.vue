<!--
  instant:instant 字段值编辑器。

  值格式为 "<时间部分>|<时区后缀>"，时间部分按 tz 后缀所表示的时区解释；
  编辑展示时转换为本地时间（内部状态恒为本地时间部件），写入时以本地时间与本地时区后缀合成。
  内部状态恒非空（初始全 0 的 DEFAULT_LOCAL_PARTS 表示"字段值为空"）。

  本组件遵循"单向非法值拦截"模型：
  - 数据下行（modelValue → 内部状态）：非法 modelValue 不进入内部状态，解析失败时不更新任何内部状态；
  - 数据上行（内部状态 → emit）：内部非法中间态照常经简单字符串拼接组装后传出，最终合法性由保存时的校验判定。

  精度下拉框控制显示与编辑到哪一位，切换精度只更新显示精度，值的格式转换由监听链路完成
  （降级截断低位、升级保留低位原值）；值变更时非本地时区会被规范化为本地时区并提示用户，
  时区后缀缺失或非法与时间部分非法同样按整体非法宽容处理（不进入内部状态）。
-->
<script setup lang="ts">
import { ref, watch } from "vue";
import { t } from "@/i18n";
import { DEFAULT_PRECISION, PRECISIONS } from "@/field-types/catalog";
import {
  assemblePartsToTime,
  DEFAULT_LOCAL_PARTS,
  formatInstantValue,
  resolveLocalTime,
  sameLocalParts,
} from "@/field-types/date-time";
import type { LocalParts, Precision } from "@/field-types/date-time";
import InstantEndpointEditor from "./InstantEndpointEditor.vue";

const props = defineProps<{
  modelValue: string | null;
  /** 值错误高亮：输入控件进入错误高亮状态（不显示错误信息）。 */
  errorHighlight?: boolean;
  readonly?: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string | null];
}>();

/** 当前显示精度。 */
const precision = ref<Precision>(DEFAULT_PRECISION);
/** 当前值的本地时间部件（已按值的时区后缀转换为本地时间），各部件恒为数字；初始全 0 表示"字段值为空"。 */
const timePart = ref<LocalParts>({ ...DEFAULT_LOCAL_PARTS });
/** 最近一次从值中检测到非本地时区时的原时区后缀，用于向用户提示发生过时区转换；仅在实际发生转换时更新，设置后不清除（组件随对话框关闭而卸载）。 */
const convertedFromTz = ref<string | null>(null);

/**
 * 将 modelValue 转换为组件内部状态（单向拦截：非法值不进入内部状态）。
 * @param value 字段值字符串；null 表示字段值为空
 */
function receiveModelValue(value: string | null): void {
  if (value === null) {
    precision.value = DEFAULT_PRECISION;
    timePart.value = { ...DEFAULT_LOCAL_PARTS };
    convertedFromTz.value = null;
    return;
  }
  const resolved = resolveLocalTime(value);
  // 时区后缀缺失/非法或时间部分非法：整个值视为非法，不进入内部状态。
  if (resolved.parts === null) return;
  if (resolved.precision !== null && resolved.precision !== precision.value) {
    precision.value = resolved.precision;
  }
  if (!sameLocalParts(resolved.parts, timePart.value)) timePart.value = resolved.parts;
  if (resolved.convertedFromTz !== null) convertedFromTz.value = resolved.convertedFromTz;
}

// 初始化时处理一次初始值；随后监听 modelValue 的外部变化（应用内理论上不会发生，出于组件功能完整性保留）。
receiveModelValue(props.modelValue);
watch(() => props.modelValue, receiveModelValue);

// 内部状态 → emit：只做简单组装（不做合法性校验）；与 modelValue 相同时跳过，与 receiveModelValue 的判等更新共同构成递归终止条件。
watch([precision, timePart], () => {
  const candidate = formatInstantValue(assemblePartsToTime(timePart.value, precision.value));
  if (candidate === props.modelValue) return;
  emit("update:modelValue", candidate);
});

/**
 * 切换精度：只更新显示精度，值的格式转换由监听链路完成（降级截断低位、升级保留低位原值）。
 * @param val 新精度
 */
function onPrecisionChange(val: Precision): void {
  precision.value = val;
}
</script>

<template>
  <div class="instant-editor">
    <div class="instant-top-row">
      <VSelect
        class="precision-select"
        :model-value="precision"
        :items="PRECISIONS"
        :item-title="(p: Precision) => t(`database.field-type.precision-${p}`)"
        :item-value="(p: Precision) => p"
        :label="t('database.field-type.precision')"
        :readonly="readonly"
        variant="outlined"
        density="compact"
        hide-details="auto"
        @update:model-value="onPrecisionChange"
      />
      <InstantEndpointEditor
        class="endpoint"
        v-model="timePart"
        :precision="precision"
        :error-highlight="errorHighlight"
        :readonly="readonly"
      />
    </div>
    <div v-if="convertedFromTz !== null" class="text-caption text-secondary">
      {{ t("database.field-editor.tz-converted-hint", { tz: convertedFromTz }) }}
    </div>
  </div>
</template>

<style lang="scss" scoped>
.instant-editor {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.instant-top-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.precision-select {
  flex: none;
  width: 7.5rem;
}

.endpoint {
  flex: 1;
  min-width: 0;
}
</style>
