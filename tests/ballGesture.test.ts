import { describe, expect, it } from "vitest";
import { movedBeyondClick } from "../src/widget/ballGesture";

describe("悬浮球点击与拖动", () => {
  it("允许点击时的小幅抖动，超过阈值才拖动", () => {
    expect(movedBeyondClick({ x: 100, y: 100 }, { x: 103, y: 102 })).toBe(false);
    expect(movedBeyondClick({ x: 100, y: 100 }, { x: 103, y: 104 })).toBe(true);
    expect(movedBeyondClick({ x: -1200, y: 80 }, { x: -1206, y: 80 })).toBe(true);
  });
});
