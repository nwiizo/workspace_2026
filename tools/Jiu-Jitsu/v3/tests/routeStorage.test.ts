import { describe, expect, it } from "vitest";
import { addSavedRoute, decodeRoute, encodeRoute, loadRoute, loadSavedRoutes, removeSavedRoute, ROUTE_KEY, ROUTE_LIBRARY_KEY, saveRoute } from "../src/engine/routeStorage";
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

describe("複数の自作ルートとファイル", () => {
  const route = (name: string) => ({ name, frames: SUGGESTED_ROUTES[1]!.frames() });
  it("別名で残した二つのルートが作業中データの編集・削除と独立する", () => {
    const store = memory();
    saveRoute(store, route("作成中"));
    addSavedRoute(store, "first", route("ハーフガードから"));
    addSavedRoute(store, "second", route("バックから"));
    saveRoute(store, route("次の作業"));
    expect(loadSavedRoutes(store).map((entry) => entry.route.name)).toEqual(["ハーフガードから", "バックから"]);
    removeSavedRoute(store, "first");
    expect(loadSavedRoutes(store).map((entry) => entry.id)).toEqual(["second"]);
    expect(loadRoute(store)?.name).toBe("次の作業");
  });
  it("読み出した姿勢の編集では保存した例を変更しない", () => {
    const store = memory(); addSavedRoute(store, "first", route("元の例"));
    const opened = loadSavedRoutes(store)[0]!.route;
    opened.frames[0]!.pose.blue.pelvis.x = 2;
    expect(loadSavedRoutes(store)[0]!.route.frames[0]!.pose.blue.pelvis.x).not.toBe(2);
  });
  it("同じIDや壊れたルートを追加しても以前の一覧を失わない", () => {
    const store = memory(); addSavedRoute(store, "first", route("残す例"));
    const original = store.getItem(ROUTE_LIBRARY_KEY);
    expect(() => addSavedRoute(store, "first", route("重複"))).toThrow();
    const invalid = route("不正"); invalid.frames[0]!.seconds = Infinity;
    expect(() => addSavedRoute(store, "second", invalid)).toThrow();
    expect(store.getItem(ROUTE_LIBRARY_KEY)).toBe(original);
  });
  it("壊れた一覧を読み書きで消さない", () => {
    const store = memory();
    for (const bad of ['not-json', '{"version":2,"routes":[]}', '{"version":1,"routes":[{}]}']) {
      store.setItem(ROUTE_LIBRARY_KEY, bad);
      expect(() => loadSavedRoutes(store)).toThrow();
      expect(() => addSavedRoute(store, "new", route("追加"))).toThrow();
      expect(() => removeSavedRoute(store, "old")).toThrow();
      expect(store.getItem(ROUTE_LIBRARY_KEY)).toBe(bad);
    }
  });
  it("20件を超える保存を拒否し、すでに保存した分を保つ", () => {
    const store = memory();
    for (let i = 0; i < 20; i++) addSavedRoute(store, String(i), route(`例 ${i}`));
    const original = store.getItem(ROUTE_LIBRARY_KEY);
    expect(() => addSavedRoute(store, "overflow", route("追加"))).toThrow();
    expect(store.getItem(ROUTE_LIBRARY_KEY)).toBe(original);
  });
  it("ファイルの往復で姿勢と時間を保ち、従来の作業中データも読める", () => {
    const original = route("<ガードの練習>");
    original.frames[1]!.seconds = 3.5;
    const imported = decodeRoute(encodeRoute(original));
    expect(imported.name).toBe(original.name);
    expect(imported.frames[1]!.seconds).toBe(3.5);
    expect(imported.frames[1]!.pose.red.arms.R.end.toArray()).toEqual(original.frames[1]!.pose.red.arms.R.end.toArray());
    const store = memory(); saveRoute(store, original);
    expect(decodeRoute(store.getItem(ROUTE_KEY)!).name).toBe(original.name);
    expect(() => decodeRoute(' '.repeat(200_001))).toThrow();
    expect(() => decodeRoute('{"version":9}')).toThrow();
  });
});
