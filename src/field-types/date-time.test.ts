import { afterEach, describe, expect, it } from "vitest";
import { DateTime, Settings } from "luxon";

import {
  assemblePartsToTime,
  daysInMonth,
  DEFAULT_LOCAL_PARTS,
  isLeapYear,
  isValidInstantValue,
  parseTzSuffix,
  resolveLocalRange,
  resolveLocalTime,
  sameLocalParts,
  shiftPartsToLocal,
  validateRangeValue,
  weekdayOf,
} from "./date-time";

/** 测试前保存的默认时区，用于 afterEach 恢复，避免污染其它测试。 */
const originalDefaultZone = Settings.defaultZone;

/** 固定当前系统时区；修改 Settings.defaultZone 后需重置 luxon 缓存才能生效。 */
function useDefaultZone(zone: string): void {
  Settings.defaultZone = zone;
  Settings.resetCaches();
}

afterEach(() => {
  Settings.defaultZone = originalDefaultZone;
  Settings.resetCaches();
});

/** 构造测试用本地时间部件。 */
function parts(
  year: number,
  month: number,
  day: number,
  hour: number,
  minute: number,
  second: number,
  millisecond: number,
) {
  return { year, month, day, hour, minute, second, millisecond };
}

describe("parseTzSuffix", () => {
  it("接受 currentTzSuffix 可生成的正负与小数偏移", () => {
    // 意图：正偏移、负偏移、半小时、四分之一小时、零偏移都是 currentTzSuffix 的合法输出，应解析为对应的偏移小时数。
    expect(parseTzSuffix("+8")).toBe(8);
    expect(parseTzSuffix("-5")).toBe(-5);
    expect(parseTzSuffix("+5.5")).toBe(5.5);
    expect(parseTzSuffix("+5.75")).toBe(5.75);
    expect(parseTzSuffix("+0")).toBe(0);
  });

  it("拒绝无符号与非数字格式", () => {
    // 意图：无符号的 "8"、纯字母、单独符号、空串都不是 currentTzSuffix 会生成的格式，应返回 null。
    expect(parseTzSuffix("8")).toBeNull();
    expect(parseTzSuffix("abc")).toBeNull();
    expect(parseTzSuffix("+")).toBeNull();
    expect(parseTzSuffix("")).toBeNull();
  });
});

describe("shiftPartsToLocal", () => {
  it("同一日期的正向平移（本地 UTC+9，源 +8）", () => {
    // 意图：源时区比本地慢 1 小时，墙钟 12:00 平移后为 13:00，年月日不变。
    useDefaultZone("UTC+9");
    expect(shiftPartsToLocal(parts(2025, 1, 1, 12, 0, 0, 0), 8)).toEqual(
      parts(2025, 1, 1, 13, 0, 0, 0),
    );
  });

  it("跨日平移（本地 UTC+8，源 +9）", () => {
    // 意图：源时区比本地快 1 小时，墙钟 00:30 平移后为前一日 23:30，验证跨日借位。
    useDefaultZone("UTC+8");
    expect(shiftPartsToLocal(parts(2025, 1, 1, 0, 30, 0, 0), 9)).toEqual(
      parts(2024, 12, 31, 23, 30, 0, 0),
    );
  });

  it("跨年平移（本地 UTC-5，源 +8）", () => {
    // 意图：本地比源慢 13 小时，2025-01-01 00:00 平移后为 2024-12-31 11:00，验证跨年月借位。
    useDefaultZone("UTC-5");
    expect(shiftPartsToLocal(parts(2025, 1, 1, 0, 0, 0, 0), 8)).toEqual(
      parts(2024, 12, 31, 11, 0, 0, 0),
    );
  });

  it("源偏移等于本地偏移时恒等", () => {
    // 意图：fromTzOffsetHours 与本地偏移一致时不产生平移，部件原样返回。
    useDefaultZone("UTC+9");
    const local = parts(2025, 1, 1, 12, 0, 0, 0);
    expect(shiftPartsToLocal(local, 9)).toEqual(local);
  });
});

describe("sameLocalParts", () => {
  const base = parts(2025, 1, 1, 12, 0, 0, 0);

  it("七位全部相等返回 true", () => {
    // 意图：全部字段相同的两个部件应视为相等。
    expect(sameLocalParts({ ...base }, { ...base })).toBe(true);
  });

  it("任一位不等返回 false", () => {
    // 意图：仅毫秒一位不同也足以判为不相等，避免编辑器递归提前终止。
    expect(sameLocalParts(base, { ...base, millisecond: 1 })).toBe(false);
  });

  it("双 null 返回 true", () => {
    // 意图：两端都为空时的递归终止条件，应判为相等。
    expect(sameLocalParts(null, null)).toBe(true);
  });

  it("单 null 返回 false", () => {
    // 意图：一端为空另一端有内容时必须判为不相等，避免递归误终止。
    expect(sameLocalParts(base, null)).toBe(false);
    expect(sameLocalParts(null, base)).toBe(false);
  });
});

describe("resolveLocalTime", () => {
  it("时区后缀与本地不同时转换部件并记录原后缀", () => {
    // 意图：本地 UTC+9、源 +8，12:00 转换后为本地 13:00，精度为 minute，convertedFromTz 记录 "+8"。
    useDefaultZone("UTC+9");
    expect(resolveLocalTime("2025-01-01 12:00|+8")).toEqual({
      parts: parts(2025, 1, 1, 13, 0, 0, 0),
      precision: "minute",
      convertedFromTz: "+8",
    });
  });

  it("时区后缀与本地一致时不转换", () => {
    // 意图：+9 与本地 UTC+9 相同，parts 原样返回且 convertedFromTz 为 null（未发生转换）。
    useDefaultZone("UTC+9");
    expect(resolveLocalTime("2025-01-01 12:00|+9")).toEqual({
      parts: parts(2025, 1, 1, 12, 0, 0, 0),
      precision: "minute",
      convertedFromTz: null,
    });
  });

  it("时区后缀缺失时宽容返回（视为整体非法值）", () => {
    // 意图：无 "|" 的旧格式值（splitTimeAndTz 返回 tz null）与时间部分非法同样宽容处理，parts 为 null，由调用方决定如何处置。
    useDefaultZone("UTC+9");
    expect(resolveLocalTime("2025-01-01 12:00")).toEqual({
      parts: null,
      precision: null,
      convertedFromTz: null,
    });
  });

  it("时区后缀非法时宽容返回（视为整体非法值）", () => {
    // 意图：无法解析的后缀（如 abc）与时间部分非法同样宽容处理，parts 为 null，由调用方决定如何处置。
    useDefaultZone("UTC+9");
    expect(resolveLocalTime("2025-01-01 12:00|abc")).toEqual({
      parts: null,
      precision: null,
      convertedFromTz: null,
    });
  });

  it("时间部分非法时宽容返回", () => {
    // 意图：时区合法但时间部分无法解析时应宽容（调用方显示为空），而不是报数据损坏。
    useDefaultZone("UTC+9");
    expect(resolveLocalTime("garbage|+9")).toEqual({
      parts: null,
      precision: null,
      convertedFromTz: null,
    });
  });
});

describe("resolveLocalRange", () => {
  it("两端同时按源后缀平移", () => {
    // 意图：本地 UTC+9、源 +8，起点与终点各 +1 小时，精度为 minute，convertedFromTz 记录 "+8"。
    useDefaultZone("UTC+9");
    expect(resolveLocalRange("2025-01-01 10:00 ~ 2025-01-01 12:00|+8")).toEqual({
      startParts: parts(2025, 1, 1, 11, 0, 0, 0),
      endParts: parts(2025, 1, 1, 13, 0, 0, 0),
      precision: "minute",
      convertedFromTz: "+8",
    });
  });

  it("时区后缀缺失时宽容返回（视为整体非法值）", () => {
    // 意图：无 "|" 的旧格式 range 值与值格式非法同样宽容处理，各字段为 null，由调用方决定如何处置。
    useDefaultZone("UTC+9");
    expect(resolveLocalRange("2025-01-01 10:00 ~ 2025-01-01 12:00")).toEqual({
      startParts: null,
      endParts: null,
      precision: null,
      convertedFromTz: null,
    });
  });

  it("值格式非法（无法解析为两端同精度区间）时宽容返回", () => {
    // 意图：时区合法但值格式非法（如无 " ~ " 分隔）时应宽容返回双 null，而不是报数据损坏。
    useDefaultZone("UTC+9");
    expect(resolveLocalRange("garbage|+9")).toEqual({
      startParts: null,
      endParts: null,
      precision: null,
      convertedFromTz: null,
    });
  });
});

describe("assemblePartsToTime", () => {
  it("各精度下按格式拼接并截断低位", () => {
    // 意图：millisecond 精度拼全七位，day 精度只拼到日、year 精度只拼年，截断结构与 TIME_FORMATS 一致。
    const value = parts(2025, 1, 15, 12, 34, 56, 789);
    expect(assemblePartsToTime(value, "millisecond")).toBe("2025-01-15 12:34:56.789");
    expect(assemblePartsToTime(value, "second")).toBe("2025-01-15 12:34:56");
    expect(assemblePartsToTime(value, "minute")).toBe("2025-01-15 12:34");
    expect(assemblePartsToTime(value, "hour")).toBe("2025-01-15 12");
    expect(assemblePartsToTime(value, "day")).toBe("2025-01-15");
    expect(assemblePartsToTime(value, "month")).toBe("2025-01");
    expect(assemblePartsToTime(value, "year")).toBe("2025");
  });

  it("单位数部件逐位补零", () => {
    // 意图：month、day 等不足两位的部件补 0 到 2 位，millisecond 补 0 到 3 位，year 补 0 到 4 位。
    expect(assemblePartsToTime(parts(5, 6, 7, 4, 3, 2, 5), "millisecond")).toBe(
      "0005-06-07 04:03:02.005",
    );
    expect(assemblePartsToTime(parts(2025, 6, 5, 0, 0, 0, 0), "day")).toBe("2025-06-05");
  });

  it("非法部件照拼为非法字符串，不做校验", () => {
    // 意图：month=0、day=0 等非法组合也照拼（如默认内部状态全 0 拼出 "0000-00-00 00:00:00.000"），不做合法性校验。
    expect(assemblePartsToTime(parts(2026, 0, 15, 0, 0, 0, 0), "day")).toBe("2026-00-15");
    expect(assemblePartsToTime(DEFAULT_LOCAL_PARTS, "millisecond")).toBe("0000-00-00 00:00:00.000");
  });
});

describe("isValidInstantValue", () => {
  it("合法的各精度值为 true", () => {
    // 意图：时间部分合法且时区后缀合法的各精度值都应判为合法。
    useDefaultZone("UTC+9");
    expect(isValidInstantValue("2025-01-01 12:00:00.000|+8")).toBe(true);
    expect(isValidInstantValue("2025-01|+9")).toBe(true);
    expect(isValidInstantValue("2025-01-01 12:00|+8")).toBe(true);
    expect(isValidInstantValue("2025|+8")).toBe(true);
  });

  it("时间部分非法为 false", () => {
    // 意图：时间部分无法解析为任何精度格式（如 "garbage"）时应判为非法。
    useDefaultZone("UTC+9");
    expect(isValidInstantValue("garbage|+9")).toBe(false);
  });

  it("时区后缀缺失或非法为 false", () => {
    // 意图：无 "|" 时 tz 缺失；"abc" 与无符号 "8" 都不是 currentTzSuffix 的输出格式，均应判为非法。
    useDefaultZone("UTC+9");
    expect(isValidInstantValue("2025-01-01")).toBe(false);
    expect(isValidInstantValue("2025-01-01|abc")).toBe(false);
    expect(isValidInstantValue("2025-01-01|8")).toBe(false);
  });

  it("不存在的日期为 false", () => {
    // 意图：2025-02-31 对不上任何真实日期，luxon 校验应判为非法。
    useDefaultZone("UTC+9");
    expect(isValidInstantValue("2025-02-31|+9")).toBe(false);
  });
});

describe("validateRangeValue", () => {
  it("合法的各精度区间（含起点等于终点）返回 null", () => {
    // 意图：两端同为任一合法精度（覆盖 minute/day/millisecond）且起点不晚于终点时都应判为合法，起点等于终点也是合法区间。
    useDefaultZone("UTC+9");
    expect(validateRangeValue("2025-01-01 10:00 ~ 2025-01-01 12:00|+8")).toBeNull();
    expect(validateRangeValue("2025-01-01 ~ 2025-01-01|+8")).toBeNull();
    expect(validateRangeValue("2025-01-01 10:00:00.000 ~ 2025-01-01 12:00:00.000|+8")).toBeNull();
  });

  it("起点晚于终点返回 order", () => {
    // 意图：同精度同格式下字典序即时间序，起点字符串大于终点字符串时属于顺序错误而非格式错误。
    useDefaultZone("UTC+9");
    expect(validateRangeValue("2025-01-02 ~ 2025-01-01|+8")).toBe("order");
  });

  it("无法解析为两端同精度区间返回 format", () => {
    // 意图：任一端格式非法（如 "garbage"）或两端精度不一致时都无法解析为两端同精度区间，属于格式错误。
    useDefaultZone("UTC+9");
    expect(validateRangeValue("garbage|+9")).toBe("format");
    expect(validateRangeValue("2025-01-01 ~ 2025-01-01 12:00|+8")).toBe("format");
  });

  it("时区后缀缺失或非法返回 format", () => {
    // 意图：无 "|" 或后缀无法解析（如 "abc"）时与格式非法同等对待，属于格式错误。
    useDefaultZone("UTC+9");
    expect(validateRangeValue("2025-01-01 ~ 2025-01-02")).toBe("format");
    expect(validateRangeValue("2025-01-01 ~ 2025-01-02|abc")).toBe("format");
  });
});

describe("calendar math", () => {
  it("isLeapYear 按格里历闰年规则判断", () => {
    // 意图：year 0 因 0 % 400 === 0 是闰年；1900 能被 100 整除但不能被 400 整除是非闰年；2000 能被 400 整除是闰年；2024 是闰年；2023 是非闰年。
    expect(isLeapYear(0)).toBe(true);
    expect(isLeapYear(1900)).toBe(false);
    expect(isLeapYear(2000)).toBe(true);
    expect(isLeapYear(2024)).toBe(true);
    expect(isLeapYear(2023)).toBe(false);
  });

  it("daysInMonth 平年逐月天数按常规则", () => {
    // 意图：平年 2023 全年各月天数依次为 31/28/31/30/31/30/31/31/30/31/30/31，2 月为 28 天。
    const expected = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for (let month = 1; month <= 12; month += 1) {
      expect(daysInMonth(2023, month)).toBe(expected[month - 1]);
    }
  });

  it("daysInMonth 闰年 2 月为 29 天", () => {
    // 意图：闰年 2024 的 2 月比平年多一天，返回 29。
    expect(daysInMonth(2024, 2)).toBe(29);
  });

  it("daysInMonth 非法月份抛 RangeError", () => {
    // 意图：month 为 0 或 13 时不在 1-12 合法范围内，属于失败路径，应抛 RangeError 而不是静默返回错误结果。
    expect(() => daysInMonth(2023, 0)).toThrow(RangeError);
    expect(() => daysInMonth(2023, 13)).toThrow(RangeError);
  });

  it("weekdayOf 锚点 0000-01-01 为周六", () => {
    // 意图：proleptic Gregorian 的 0000-01-01 是周六，ISO weekday 为 6；该锚点超出 luxon 支持范围，只能手工断言。
    expect(weekdayOf(0, 1, 1)).toBe(6);
  });

  it("weekdayOf 与 luxon 抽样对照", () => {
    // 意图：用 luxon 在 year 1-9999 支持范围内抽样对照，覆盖年初、闰日、普通日与年末，验证公式推导正确。
    const samples: Array<[number, number, number]> = [
      [1, 1, 1],
      [2000, 2, 29],
      [2024, 2, 29],
      [2026, 9, 3],
      [9999, 12, 31],
    ];
    for (const [year, month, day] of samples) {
      expect(weekdayOf(year, month, day)).toBe(DateTime.local(year, month, day).weekday);
    }
  });

  it("weekdayOf 非法月份抛 RangeError", () => {
    // 意图：month 为 13 时不在 1-12 合法范围内，属于失败路径，应抛 RangeError。
    expect(() => weekdayOf(2023, 13, 1)).toThrow(RangeError);
  });

  it("weekdayOf 非法日期抛 RangeError", () => {
    // 意图：day 为 0、或超过当月天数（2023-02-29 在平年不存在）时不在当月合法范围内，属于失败路径，应抛 RangeError。
    expect(() => weekdayOf(2023, 1, 0)).toThrow(RangeError);
    expect(() => weekdayOf(2023, 2, 29)).toThrow(RangeError);
  });
});