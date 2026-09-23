import { describe, expect, it } from "vitest";
import { loadRoute, ROUTE_KEY, saveRoute } from "../src/engine/routeStorage";
import { SUGGESTED_ROUTES } from "../src/content/routes";

const memory = () => {
  const items = new Map<string, string>();
  return { getItem: (key: string) => items.get(key) ?? null, setItem: (key: string, value: string) => { items.set(key, value); } };
};
describe("自作ルートの保存", () => {
  it("名前と順番・時間・手足の目標を再読み込みできる", () => {
    const store = memory(), route = { name: "<自作ルート>", frames: SUGGESTED_ROUTES[1]!.frames() };
    saveRoute(store, route);
    const saved = loadRoute(store)!;
    expect(saved?.name).toBe(route.name);
    expect(saved?.frames.length).toBe(3);
    expect(saved.frames[1]!.pose.blue.arms.L.end.distanceTo(route.frames[1]!.pose.blue.arms.L.end)).toBeLessThan(1e-8);
  });
  it("壊れたデータや未対応の版を消さずに拒否する", () => {
    const store = memory();
    for (const bad of ['{"version":2}', '{"version":1,"frames":[]}', 'not-json']) {
      store.setItem(ROUTE_KEY, bad);
      expect(() => loadRoute(store)).toThrow();
      expect(store.getItem(ROUTE_KEY)).toBe(bad);
    }
  });
  it("不正な座標・回転・秒数を読み込まない", () => {
    const store = memory(); saveRoute(store, { name: "確認", frames: SUGGESTED_ROUTES[1]!.frames() });
    const valid = store.getItem(ROUTE_KEY)!;
    for (const change of [
      (data: any) => { data.frames[0].blue.pelvis[0] = null; },
      (data: any) => { data.frames[0].red.rotation = [0, 0, 0, 0]; },
      (data: any) => { data.frames[0].seconds = 0; },
      (data: any) => { data.frames[0].blue.legs = []; },
    ]) {
      const data = JSON.parse(valid); change(data); store.setItem(ROUTE_KEY, JSON.stringify(data));
      expect(() => loadRoute(store)).toThrow();
    }
  });
  it("保存失敗時には以前の内容が残る", () => {
    const store = memory(), route = { name: "元のルート", frames: SUGGESTED_ROUTES[1]!.frames() };
    saveRoute(store, route); const original = store.getItem(ROUTE_KEY);
    route.frames[0]!.pose.blue.pelvis.x = NaN;
    expect(() => saveRoute(store, route)).toThrow();
    expect(store.getItem(ROUTE_KEY)).toBe(original);
  });
});
