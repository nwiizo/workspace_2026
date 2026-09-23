import { describe, expect, it } from "vitest";
import { cloneRoutePose, inspectRoutePose, inspectTransition, interpolateRoute, routePose } from "../src/engine/route";
import { backPair } from "../src/render/practicePose";
import { SUGGESTED_ROUTES, ROUTE_COURSES, courseRoutes } from "../src/content/routes";

const start = () => routePose(backPair("safe"));

describe("自作ルートの動作検査", () => {
  it.each(ROUTE_COURSES)("連続例 $id の全局面が成立し、順序と色を保つ", (course) => {
    const steps = courseRoutes(course);
    expect(steps).toHaveLength(course.steps.length);
    for (const step of steps) for (const frame of step.frames()) expect(inspectRoutePose(frame.pose).issues.filter((i) => i.level === "blocked")).toEqual([]);
    if (course.id === "top-exchange") expect(steps.map((step) => step.top)).toEqual(["red", "blue", "red", "blue"]);
  });
  it.each(SUGGESTED_ROUTES)("登録例 $id の各姿勢を編集できる", (route) => {
    for (const frame of route.frames()) expect(inspectRoutePose(frame.pose).issues.filter((i) => i.level === "blocked"), frame.label).toEqual([]);
  });
  it.each(SUGGESTED_ROUTES)("登録例 $id の途中の動きで貫通しない", (route) => {
    const frames = route.frames();
    for (let i = 1; i < frames.length; i++) expect(inspectTransition(frames[i - 1]!.pose, frames[i]!.pose), frames[i]!.label).toBeNull();
  });
  it("届かない手の目標を黙って縮めず、理由と場所を返す", () => {
    const pose = start();
    pose.blue.arms.L.end.x += 2;
    const result = inspectRoutePose(pose);
    expect(result.issues.some((i) => i.level === "blocked" && i.actor === "blue" && i.message.includes("届きません"))).toBe(true);
    expect(result.pair.blue.shoulderL.distanceTo(result.pair.blue.elbowL)).toBeCloseTo(0.28, 6);
  });
  it("足の目標が床下なら通さない", () => {
    const pose = start(); pose.blue.legs.L.end.y = -0.2;
    expect(inspectRoutePose(pose).issues.some((i) => i.level === "blocked" && i.message.includes("床"))).toBe(true);
  });
  it("手首が床上でも手の厚みが床へ入る目標を拒否する", () => {
    const p = start(); p.blue.arms.L.end.y = 0.032;
    expect(inspectRoutePose(p).issues.some((i) => i.level === "blocked" && i.message.includes("手の目標"))).toBe(true);
  });
  it("静止した座位は初期状態で登録できる", () => {
    expect(inspectRoutePose(start()).issues.filter((i) => i.level === "blocked")).toEqual([]);
  });
  it("編集用の複製と途中の姿勢が、登録済みの姿勢を書き換えない", () => {
    const original = start(), copy = cloneRoutePose(original);
    copy.blue.arms.L.end.x += 0.1;
    expect(original.blue.arms.L.end.x).toBeCloseTo(-0.35);
    const middle = interpolateRoute(original, copy, 0.5);
    expect(middle.blue.arms.L.end.x).toBeCloseTo(-0.30);
    expect(original.blue.arms.L.end.x).toBeCloseTo(-0.35);
  });
  it("両端が成立していても途中で二人がすり抜ける移動を検出する", () => {
    const a = start(), b = { blue: a.red, red: a.blue };
    expect(inspectRoutePose(b).issues.filter((i) => i.level === "blocked")).toEqual([]);
    const problem = inspectTransition(a, b);
    expect(problem).not.toBeNull();
    expect(problem!.fraction).toBeGreaterThan(0);
    expect(problem!.fraction).toBeLessThan(1);
  });
});
