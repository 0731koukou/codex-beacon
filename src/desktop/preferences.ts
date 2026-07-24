export const MIN_BACKGROUND_OPACITY = 30;
export const MAX_BACKGROUND_OPACITY = 100;
export const DEFAULT_BACKGROUND_OPACITY = 98;

const BACKGROUND_OPACITY_KEY = "codex-beacon.background-opacity";
const WINDOW_POSITION_KEY = "codex-beacon.window-position";
const MIN_VISIBLE_WIDTH = 80;
const MIN_VISIBLE_HEIGHT = 40;

type StorageReader = Pick<Storage, "getItem">;
type StorageWriter = Pick<Storage, "setItem">;

export type Point = {
  x: number;
  y: number;
};

export type Dimensions = {
  width: number;
  height: number;
};

export type ScreenRect = Point & Dimensions;

export function normalizeBackgroundOpacity(value: unknown) {
  const number = Number(value);
  return Number.isFinite(number)
    ? Math.min(
        MAX_BACKGROUND_OPACITY,
        Math.max(MIN_BACKGROUND_OPACITY, number),
      )
    : DEFAULT_BACKGROUND_OPACITY;
}

export function readBackgroundOpacity(storage: StorageReader) {
  try {
    const value = storage.getItem(BACKGROUND_OPACITY_KEY);
    return value === null
      ? DEFAULT_BACKGROUND_OPACITY
      : normalizeBackgroundOpacity(value);
  } catch {
    return DEFAULT_BACKGROUND_OPACITY;
  }
}

export function saveBackgroundOpacity(
  storage: StorageWriter,
  value: number,
) {
  try {
    storage.setItem(
      BACKGROUND_OPACITY_KEY,
      String(normalizeBackgroundOpacity(value)),
    );
  } catch {
    // Keep the in-memory setting when storage is unavailable.
  }
}

export function readWindowPosition(storage: StorageReader): Point | null {
  try {
    const value = storage.getItem(WINDOW_POSITION_KEY);
    if (!value) {
      return null;
    }
    const position = JSON.parse(value) as Partial<Point>;
    return Number.isFinite(position.x) && Number.isFinite(position.y)
      ? { x: position.x as number, y: position.y as number }
      : null;
  } catch {
    return null;
  }
}

export function saveWindowPosition(
  storage: StorageWriter,
  position: Point,
) {
  if (Number.isFinite(position.x) && Number.isFinite(position.y)) {
    try {
      storage.setItem(WINDOW_POSITION_KEY, JSON.stringify(position));
    } catch {
      // Dragging must still work when storage is unavailable.
    }
  }
}

export function toScreenRect(area: {
  position: Point;
  size: Dimensions;
}): ScreenRect {
  return {
    x: area.position.x,
    y: area.position.y,
    width: area.size.width,
    height: area.size.height,
  };
}

export function isWindowPositionVisible(
  position: Point,
  size: Dimensions,
  monitors: ScreenRect[],
) {
  return monitors.some((monitor) => {
    const overlapWidth =
      Math.min(position.x + size.width, monitor.x + monitor.width) -
      Math.max(position.x, monitor.x);
    const overlapHeight =
      Math.min(position.y + size.height, monitor.y + monitor.height) -
      Math.max(position.y, monitor.y);
    return (
      overlapWidth >= MIN_VISIBLE_WIDTH &&
      overlapHeight >= MIN_VISIBLE_HEIGHT
    );
  });
}
