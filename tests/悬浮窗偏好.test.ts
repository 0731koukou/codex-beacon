import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const preferences = await import("../src/desktop/preferences.ts").catch(
  () => ({}),
);
const presentation = await import("../src/codex/presentation.ts").catch(
  () => ({}),
);
const shellStyles = await readFile(
  new URL("../src/styles/shell.css", import.meta.url),
  "utf8",
);

test("透明度限制在 30 到 100，并在无效值时回退 98", () => {
  assert.equal(typeof preferences.normalizeBackgroundOpacity, "function");
  assert.equal(preferences.normalizeBackgroundOpacity(12), 30);
  assert.equal(preferences.normalizeBackgroundOpacity(72), 72);
  assert.equal(preferences.normalizeBackgroundOpacity(140), 100);
  assert.equal(preferences.normalizeBackgroundOpacity("bad"), 98);
});

test("读取保存的透明度并限制边界", () => {
  assert.equal(typeof preferences.readBackgroundOpacity, "function");
  assert.equal(
    preferences.readBackgroundOpacity({ getItem: () => "24" }),
    30,
  );
  assert.equal(
    preferences.readBackgroundOpacity({ getItem: () => "76" }),
    76,
  );
  assert.equal(
    preferences.readBackgroundOpacity({ getItem: () => null }),
    98,
  );
  assert.equal(
    preferences.readBackgroundOpacity({
      getItem: () => {
        throw new Error("storage unavailable");
      },
    }),
    98,
  );
});

test("窗口位置只接受有限数值", () => {
  assert.equal(typeof preferences.readWindowPosition, "function");
  assert.deepEqual(
    preferences.readWindowPosition({
      getItem: () => '{"x":-1700,"y":80}',
    }),
    { x: -1700, y: 80 },
  );
  assert.equal(
    preferences.readWindowPosition({
      getItem: () => '{"x":"wrong","y":80}',
    }),
    null,
  );
});

test("窗口至少保留 80×40 像素时视为可见", () => {
  assert.equal(typeof preferences.isWindowPositionVisible, "function");
  const monitors = [{ x: 0, y: 0, width: 1920, height: 1080 }];
  const size = { width: 820, height: 460 };

  assert.equal(
    preferences.isWindowPositionVisible({ x: 100, y: 100 }, size, monitors),
    true,
  );
  assert.equal(
    preferences.isWindowPositionVisible({ x: 1840, y: 1040 }, size, monitors),
    true,
  );
  assert.equal(
    preferences.isWindowPositionVisible({ x: 1841, y: 1041 }, size, monitors),
    false,
  );
});

test("显示器工作区映射保留负坐标", () => {
  assert.equal(typeof preferences.toScreenRect, "function");
  assert.deepEqual(
    preferences.toScreenRect({
      position: { x: -1920, y: 0 },
      size: { width: 1920, height: 1080 },
    }),
    { x: -1920, y: 0, width: 1920, height: 1080 },
  );
});

test("透明度只线性作用于悬浮窗主背景", () => {
  assert.match(shellStyles, /--surface:\s*rgba\(7,\s*12,\s*14,\s*var\(--surface-opacity\)\)/);
  assert.doesNotMatch(shellStyles, /--content-protection/);
  assert.doesNotMatch(
    shellStyles,
    /calc\(\(1\s*-\s*var\(--surface-opacity\)\)/,
  );
});

test("任务标题纵向轮播在末项后回到第一项", () => {
  assert.equal(typeof presentation.moveSessionIndex, "function");
  assert.equal(presentation.moveSessionIndex(0, 0), 0);
  assert.equal(presentation.moveSessionIndex(0, 1), 0);
  assert.equal(presentation.moveSessionIndex(0, 3), 1);
  assert.equal(presentation.moveSessionIndex(2, 3), 0);
  assert.equal(presentation.moveSessionIndex(0, 3, -1), 2);
});

test("全部任务完成六秒后才进入紧凑待命状态", () => {
  const completed = {
    phase: "completed",
    attention: "",
    updatedAt: 1_000,
  };
  assert.equal(presentation.shouldCompactIsland([], 10_000), true);
  assert.equal(
    presentation.shouldCompactIsland([completed], 6_999),
    false,
  );
  assert.equal(
    presentation.shouldCompactIsland([completed], 7_000),
    true,
  );
  assert.equal(
    presentation.shouldCompactIsland(
      [{ ...completed, phase: "running" }],
      20_000,
    ),
    false,
  );
  assert.equal(
    presentation.shouldCompactIsland(
      [{ ...completed, phase: "failed" }],
      20_000,
    ),
    false,
  );
});
