import assert from "node:assert/strict";
import test from "node:test";

const preferences = await import("../src/desktop/preferences.ts").catch(
  () => ({}),
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
