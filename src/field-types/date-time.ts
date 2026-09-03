/**
 * 字段值的日期时间工具：instant 系列值的时间部分（本地时间格式化字符串，格式即精度）与时区后缀的解析、推断与按本地时区转换。
 *
 * instant 与 instant-range 字段的值的时间部分按 tz 后缀所表示的时区解释，
 * 编辑展示时转换为本地时间，写入时以本地时间与本地时区后缀合成；
 * 时区后缀缺失或非法时与时间部分非法同样按整体非法宽容处理（解析结果为 null），由调用方决定如何处置。
 */
import { DateTime } from "luxon";

/** instant 系列字段的精度，决定值时间部分的格式与编辑控件显示到哪一位。 */
export type Precision =
  | "year"
  | "month"
  | "day"
  | "hour"
  | "minute"
  | "second"
  | "millisecond";

/** 本地时间部件。month 取值范围为 1-12。 */
export interface LocalParts {
  year: number;
  month: number;
  day: number;
  hour: number;
  minute: number;
  second: number;
  millisecond: number;
}

/** instant 系列编辑器内部状态的默认本地时间部件（全部位为 0）。属非法组合（month/day 取值下限为 1），经组装后得到非法时间字符串，充当"字段值为空"的初始内部状态。 */
export const DEFAULT_LOCAL_PARTS: LocalParts = {
  year: 0,
  month: 0,
  day: 0,
  hour: 0,
  minute: 0,
  second: 0,
  millisecond: 0,
};

/** 各显示精度对应的本地时间格式。 */
const TIME_FORMATS: Record<Precision, string> = {
  year: "yyyy",
  month: "yyyy-MM",
  day: "yyyy-MM-dd",
  hour: "yyyy-MM-dd HH",
  minute: "yyyy-MM-dd HH:mm",
  second: "yyyy-MM-dd HH:mm:ss",
  millisecond: "yyyy-MM-dd HH:mm:ss.SSS",
};

/** instant-range 完整值中起点与终点之间的分隔符。 */
const RANGE_SEPARATOR = " ~ ";

/** 完整值中时间部分与时区后缀之间的分隔符。 */
const TZ_SEPARATOR = "|";

/**
 * 获取当前系统时区的时区后缀。
 * @returns 时区后缀字符串，格式为 "+8"、"-5"、"+5.5" 等
 */
export function currentTzSuffix(): string {
  const hours = DateTime.local().offset / 60;
  return (hours >= 0 ? "+" : "") + hours;
}

/**
 * 解析时区后缀为 UTC 偏移小时数，与 currentTzSuffix 互为逆运算。
 * @param tz 时区后缀字符串，仅接受 currentTzSuffix 会生成的格式（必须带符号；正数和零带 "+"），即 "^[+-]\\d+(\\.\\d+)?$"
 * @returns UTC 偏移小时数；格式非法（与 currentTzSuffix 的输出不互逆）时返回 null
 */
export function parseTzSuffix(tz: string): number | null {
  if (!/^[+-]\d+(\.\d+)?$/.test(tz)) return null;
  return Number(tz);
}

/**
 * 获取当前系统时区的 UTC 偏移小时数。
 * @returns UTC 偏移小时数，即 DateTime.local().offset / 60
 */
export function localTzOffsetHours(): number {
  return DateTime.local().offset / 60;
}

/**
 * 分离完整值为时间部分与时区后缀。
 * @param value instant 类型的完整值字符串，格式为 "<时间部分>|<时区后缀>"
 * @returns 包含时间部分和时区后缀的对象；无 "|" 时 tz 为 null（宽容处理）
 */
export function splitTimeAndTz(value: string): { time: string; tz: string | null } {
  const lastSepIndex = value.lastIndexOf(TZ_SEPARATOR);
  if (lastSepIndex === -1) {
    return { time: value, tz: null };
  }
  return {
    time: value.substring(0, lastSepIndex),
    tz: value.substring(lastSepIndex + 1),
  };
}

/**
 * 从时间部分的格式推断精度。
 * @param time 时间部分字符串
 * @returns 推断出的精度；无法匹配任何精度格式时返回 null
 */
export function precisionOfTime(time: string): Precision | null {
  for (const p of Object.keys(TIME_FORMATS) as Precision[]) {
    const dt = DateTime.fromFormat(time, TIME_FORMATS[p]);
    if (dt.isValid) {
      return p;
    }
  }
  return null;
}

/**
 * 按推断精度将时间部分解析为本地时间部件。
 * @param time 时间部分字符串
 * @returns 本地时间部件；格式非法时返回 null。未指明的低位取最小值（月=1、日=1、时分秒毫秒=0），由 luxon fromFormat 自然完成
 */
export function parseTimeToParts(time: string): LocalParts | null {
  const precision = precisionOfTime(time);
  if (!precision) return null;
  const dt = DateTime.fromFormat(time, TIME_FORMATS[precision]);
  if (!dt.isValid) return null;
  return {
    year: dt.year,
    month: dt.month,
    day: dt.day,
    hour: dt.hour,
    minute: dt.minute,
    second: dt.second,
    millisecond: dt.millisecond,
  };
}

/**
 * 将本地时间部件按指定精度简单拼接为时间部分字符串（逐位补零、按精度截断低位）。
 * @param parts 本地时间部件
 * @param precision 目标精度
 * @returns 拼接出的时间部分字符串；不做合法性校验，非法部件组合照拼为非法字符串，合法性由值的消费方判定
 */
export function assemblePartsToTime(parts: LocalParts, precision: Precision): string {
  const PART_ORDER: Precision[] = [
    "year",
    "month",
    "day",
    "hour",
    "minute",
    "second",
    "millisecond",
  ];
  const pad = (n: number, width: number) => String(n).padStart(width, "0");
  const segments: string[] = [
    pad(parts.year, 4),
    pad(parts.month, 2),
    pad(parts.day, 2),
    pad(parts.hour, 2),
    pad(parts.minute, 2),
    pad(parts.second, 2),
    pad(parts.millisecond, 3),
  ];
  const end = PART_ORDER.indexOf(precision);
  const date = segments.slice(0, 3).join("-");
  const time = segments.slice(3, 6).join(":");
  if (end < 3) {
    return segments.slice(0, end + 1).join("-");
  }
  if (end === 6) {
    return `${date} ${time}.${segments[6]}`;
  }
  return `${date} ${segments.slice(3, end + 1).join(":")}`;
}

/**
 * 把时间部件（视为 fromTzOffsetHours 时区的墙钟时间）平移为本系统时区的墙钟时间部件。
 * 用 UTC 区做纯算术平移，避开本地时区 DST 的干扰；跨日/月/年由 luxon 自然处理。
 * @param parts 视为 fromTzOffsetHours 时区的墙钟时间部件
 * @param fromTzOffsetHours 源时区的 UTC 偏移小时数
 * @returns 平移后的本地时间部件
 */
export function shiftPartsToLocal(parts: LocalParts, fromTzOffsetHours: number): LocalParts {
  const shifted = DateTime.fromObject(parts, { zone: "utc" }).plus({
    minutes: Math.round((localTzOffsetHours() - fromTzOffsetHours) * 60),
  });
  return {
    year: shifted.year,
    month: shifted.month,
    day: shifted.day,
    hour: shifted.hour,
    minute: shifted.minute,
    second: shifted.second,
    millisecond: shifted.millisecond,
  };
}

/**
 * 逐字段数值比较两个本地时间部件是否相等。
 * @param a 本地时间部件或 null
 * @param b 本地时间部件或 null
 * @returns 双 null 为 true，单 null 为 false，否则七个字段全部相等时返回 true
 */
export function sameLocalParts(a: LocalParts | null, b: LocalParts | null): boolean {
  if (a === null || b === null) return a === b;
  return (
    a.year === b.year &&
    a.month === b.month &&
    a.day === b.day &&
    a.hour === b.hour &&
    a.minute === b.minute &&
    a.second === b.second &&
    a.millisecond === b.millisecond
  );
}

/**
 * 校验 instant 完整值是否合法。
 * @param value instant 类型的完整值字符串，格式为 "<时间部分>|<时区后缀>"
 * @returns 时区后缀存在且合法、且时间部分可解析为合法时间时为 true
 */
export function isValidInstantValue(value: string): boolean {
  const { time, tz } = splitTimeAndTz(value);
  if (tz === null) return false;
  if (parseTzSuffix(tz) === null) return false;
  if (parseTimeToParts(time) === null) return false;
  return true;
}

/** instant-range 完整值的非法类别："format" 表示时区后缀缺失或非法、或无法解析为两端同精度区间；"order" 表示起点晚于终点。 */
export type RangeValueError = "format" | "order";

/**
 * 校验 instant-range 完整值并区分非法类别。
 * @param value instant-range 类型的完整值字符串，格式为 "<开始时间> ~ <结束时间>|<时区后缀>"
 * @returns 合法时返回 null；非法时返回非法类别（"format" 或 "order"）
 */
export function validateRangeValue(value: string): RangeValueError | null {
  const { tz } = splitTimeAndTz(value);
  if (tz === null) return "format";
  if (parseTzSuffix(tz) === null) return "format";
  const range = parseRangeValue(value);
  if (range === null) return "format";
  if (range.start > range.end) return "order";
  return null;
}

/**
 * 解析 instant-range 完整值。
 * @param value instant-range 类型的完整值字符串，格式为 "<开始时间> ~ <结束时间>|<时区后缀>"
 * @returns 包含开始和结束时间部分的对象（不含时区后缀）；任一端格式非法或两端精度不一致时返回 null
 */
export function parseRangeValue(value: string): { start: string; end: string } | null {
  const { time } = splitTimeAndTz(value);
  const parts = time.split(RANGE_SEPARATOR);
  if (parts.length !== 2) return null;
  const start = parts[0]!;
  const end = parts[1]!;
  const startPrecision = precisionOfTime(start);
  const endPrecision = precisionOfTime(end);
  if (!startPrecision || !endPrecision || startPrecision !== endPrecision) {
    return null;
  }
  return { start, end };
}

/**
 * 由两端时间部分合成 instant-range 完整值。
 * @param startTime 开始时间部分字符串
 * @param endTime 结束时间部分字符串
 * @returns 完整的 instant-range 值字符串，格式为 "<开始时间> ~ <结束时间>|<当前系统时区后缀>"
 */
export function formatRangeValue(startTime: string, endTime: string): string {
  return `${startTime}${RANGE_SEPARATOR}${endTime}${TZ_SEPARATOR}${currentTzSuffix()}`;
}

/**
 * 由时间部分合成 instant 完整值。
 * @param time 时间部分字符串
 * @returns 完整的 instant 值字符串，格式为 "<时间部分>|<当前系统时区后缀>"
 */
export function formatInstantValue(time: string): string {
  return `${time}${TZ_SEPARATOR}${currentTzSuffix()}`;
}

/** resolveLocalTime 的解析结果。 */
export interface ResolvedLocalTime {
  /** 本地时间部件；时区后缀缺失或非法、或时间部分格式非法时为 null（宽容，调用方显示为空） */
  parts: LocalParts | null;
  /** 从时间部分格式推断的精度；值非法时为 null */
  precision: Precision | null;
  /** 发生了时区转换时的原时区后缀；未发生转换时为 null */
  convertedFromTz: string | null;
}

/**
 * 解析 instant 完整值为本地时间部件，必要时按 tz 后缀表示的时区转换为本地时间。
 * @param value instant 类型的完整值字符串，格式为 "<时间部分>|<时区后缀>"
 * @returns 解析结果；时区后缀缺失或非法、或时间部分格式非法时宽容返回 parts 为 null
 */
export function resolveLocalTime(value: string): ResolvedLocalTime {
  const { time, tz } = splitTimeAndTz(value);
  const tzHours = tz === null ? null : parseTzSuffix(tz);
  if (tzHours === null) {
    return { parts: null, precision: null, convertedFromTz: null };
  }
  const parts = parseTimeToParts(time);
  if (parts === null) {
    return { parts: null, precision: null, convertedFromTz: null };
  }
  const precision = precisionOfTime(time)!;
  if (tzHours !== localTzOffsetHours()) {
    return {
      parts: shiftPartsToLocal(parts, tzHours),
      precision,
      convertedFromTz: tz,
    };
  }
  return { parts, precision, convertedFromTz: null };
}

/** resolveLocalRange 的解析结果。 */
export interface ResolvedLocalRange {
  /** 起点本地时间部件；值格式非法（无法解析为两端同精度区间）或时区后缀缺失/非法时为 null */
  startParts: LocalParts | null;
  /** 终点本地时间部件；同上 */
  endParts: LocalParts | null;
  /** 两端共享精度；值非法时为 null */
  precision: Precision | null;
  /** 发生了时区转换时的原时区后缀；未发生转换时为 null */
  convertedFromTz: string | null;
}

/**
 * 解析 instant-range 完整值为两端的本地时间部件，必要时按 tz 后缀表示的时区转换为本地时间。
 * @param value instant-range 类型的完整值字符串，格式为 "<开始时间> ~ <结束时间>|<时区后缀>"
 * @returns 解析结果；时区后缀缺失或非法、或值格式非法（无法解析为两端同精度区间）时宽容返回（各字段为 null）
 */
export function resolveLocalRange(value: string): ResolvedLocalRange {
  const { tz } = splitTimeAndTz(value);
  const tzHours = tz === null ? null : parseTzSuffix(tz);
  if (tzHours === null) {
    return {
      startParts: null,
      endParts: null,
      precision: null,
      convertedFromTz: null,
    };
  }
  const range = parseRangeValue(value);
  if (range === null) {
    return {
      startParts: null,
      endParts: null,
      precision: null,
      convertedFromTz: null,
    };
  }
  const precision = precisionOfTime(range.start)!;
  const shouldConvert = tzHours !== localTzOffsetHours();
  return {
    startParts: shouldConvert
      ? shiftPartsToLocal(parseTimeToParts(range.start)!, tzHours)
      : parseTimeToParts(range.start)!,
    endParts: shouldConvert
      ? shiftPartsToLocal(parseTimeToParts(range.end)!, tzHours)
      : parseTimeToParts(range.end)!,
    precision,
    convertedFromTz: shouldConvert ? tz : null,
  };
}

/**
 * 判断指定年份在 proleptic Gregorian 历下是否为闰年（支持 year 0）。
 * 项目的 instant 字段值覆盖 year 0-9999，JS Date 与 luxon 均不能安全承载 year 0，
 * 因此日历数学全部基于 proleptic Gregorian 自研实现。
 * @param year 年份，合法取值 0-9999
 * @returns 能被 4 整除但不能被 100 整除、或能被 400 整除时为 true；year 0 满足 0 % 400 === 0 因此是闰年
 */
export function isLeapYear(year: number): boolean {
  return year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
}

/**
 * 获取指定年份指定月份的天数（proleptic Gregorian，支持 year 0）。
 * @param year 年份，合法取值 0-9999
 * @param month 月份，合法取值 1-12
 * @returns 该月的天数；2 月按 isLeapYear 的结果返回 29 或 28，其余月份按常规则返回 30 或 31
 * @throws month 不在 1-12 时抛出 RangeError（失败路径）
 */
export function daysInMonth(year: number, month: number): number {
  if (month < 1 || month > 12) {
    throw new RangeError(`month must be in range 1-12, got ${month}`);
  }
  if (month === 2) {
    return isLeapYear(year) ? 29 : 28;
  }
  return [4, 6, 9, 11].includes(month) ? 30 : 31;
}

/**
 * 计算 proleptic Gregorian 历下指定年份之前（[0, year) 区间）的总天数。
 * @param year 年份，合法取值 0-9999
 * @returns year 之前的总天数；year 0 返回 0，year >= 1 时按该区间内闰年个数（含 year 0 本身为闰年）累加
 */
function daysBeforeYear(year: number): number {
  return (
    365 * year +
    (year === 0
      ? 0
      : Math.floor((year - 1) / 4) - Math.floor((year - 1) / 100) + Math.floor((year - 1) / 400) + 1)
  );
}

/**
 * 计算指定日期在 ISO 8601 历法下的星期序号（proleptic Gregorian，支持 year 0）。
 * 以 0000-01-01 为周六（ISO weekday 6）作锚点，先累计 year 之前的天数与当年 month 之前各月天数，
 * 再换算为 1=周一 … 7=周日的星期序号，与 luxon 的 DateTime.weekday 在 year 1-9999 区间一致。
 * @param year 年份，合法取值 0-9999
 * @param month 月份，合法取值 1-12
 * @param day 日期，合法取值 1-daysInMonth(year, month)
 * @returns ISO 8601 星期序号，1=周一 … 7=周日
 * @throws month 不在 1-12、或 day 不在当月合法范围内时抛出 RangeError（失败路径）
 */
export function weekdayOf(year: number, month: number, day: number): number {
  if (month < 1 || month > 12) {
    throw new RangeError(`month must be in range 1-12, got ${month}`);
  }
  const daysInCurrentMonth = daysInMonth(year, month);
  if (day < 1 || day > daysInCurrentMonth) {
    throw new RangeError(`day must be in range 1-${daysInCurrentMonth}, got ${day}`);
  }
  let daysInEarlierMonths = 0;
  for (let m = 1; m < month; m += 1) {
    daysInEarlierMonths += daysInMonth(year, m);
  }
  const totalDays = daysBeforeYear(year) + daysInEarlierMonths + (day - 1);
  return ((5 + totalDays) % 7) + 1;
}