import { Euler, Quaternion, Vector3 } from "three";
import { cloneRoutePose, inspectRoutePose, inspectTransition, interpolateRoute, routePose, type RouteFrame, type RoutePose } from "../engine/route";
import { SUGGESTED_ROUTES, ROUTE_COURSES, courseRoutes, type SuggestedRoute } from "../content/routes";
import { RoutePlayback } from "../engine/routePlayback";
import { BodyScene } from "../render/bodyScene";
import { backPair, guardPair, mountPair, sidePair, type Actor } from "../render/practicePose";
import { h } from "./dom";
import { addSavedRoute, decodeRoute, encodeRoute, loadRoute, loadSavedRoutes, removeSavedRoute, saveRoute, type SavedRoute } from "../engine/routeStorage";
import type { KeyValueStore } from "../engine/storage";

const POINTS = [["pelvis", "腰"], ["wristL", "左手"], ["wristR", "右手"], ["elbowL", "左肘の向き"], ["elbowR", "右肘の向き"], ["ankleL", "左足"], ["ankleR", "右足"], ["kneeL", "左膝の向き"], ["kneeR", "右膝の向き"]] as const;
type PointId = typeof POINTS[number][0];
const PRESETS = [
  ["二人が離れた座位", () => backPair("safe")], ["マウント", () => mountPair("low", "red")],
  ["サイド", () => sidePair("pinned", "red")], ["クローズドガード", () => guardPair("stable", "blue")], ["バック", () => backPair("reach")],
] as const;

export class RouteTab {
  readonly root: HTMLElement;
  private readonly scene: BodyScene;
  private frames: RouteFrame[] = [{ label: "開始", seconds: 2, pose: routePose(backPair("safe")) }];
  private draft = cloneRoutePose(this.frames[0]!.pose);
  private selected = 0;
  private actor: Actor = "blue";
  private point: PointId = "wristL";
  private undo: RoutePose | null = null;
  private bones = false;
  private faded = false;
  private active = false;
  private playing: number | null = null;
  private watching: { items: SuggestedRoute[]; index: number; frames: RouteFrame[]; title: string } | null = null;
  private readonly viewing = h("section", { class: "route-viewing", attrs: { "aria-label": "いま見ている攻防" } });
  private readonly speed = h("select", { attrs: { "aria-label": "再生速度" } }, ...[0.25, 0.5, 1, 2].map((value) => h("option", { text: `${value}倍`, attrs: { value: String(value) } })));
  private readonly repeat = h("input", { attrs: { type: "checkbox" } });
  private dirty = false;
  private canSave = true;
  private saved = false;
  private unsaved = false;
  private readonly saveStatus = h("p", { class: "route-status", attrs: { role: "status" } });
  private readonly savedRoutes = h("div", { class: "route-saved-cards" });
  private readonly savedSummary = h("summary", { text: "保存したルート" });
  private readonly libraryStatus = h("p", { class: "route-status", attrs: { role: "status" } });
  private readonly timeline = h("ol", { class: "route-timeline" });
  private readonly issues = h("div", { class: "route-issues", attrs: { "aria-live": "polite", "aria-atomic": "true" } });
  private readonly caption = h("p", { class: "practice-caption" });
  private readonly pointSelect = h("select", { attrs: { "aria-label": "動かす部位" } });
  private readonly coordinates: HTMLInputElement[] = [];
  private readonly name = h("input", { attrs: { "aria-label": "ルート名", maxlength: "80", value: "自分のルート" } });
  private readonly label = h("input", { attrs: { "aria-label": "姿勢の名前", maxlength: "80", value: "開始" } });
  private readonly seconds = h("input", { attrs: { type: "number", min: "0.5", max: "10", step: "0.5", value: "2", "aria-label": "この姿勢へ動く秒数" } });
  private readonly add = h("button", { class: "btn btn-start", text: "この姿勢を後ろに追加", onClick: () => this.register(false) });
  private readonly update = h("button", { class: "btn", text: "選んだ姿勢を更新", onClick: () => this.register(true) });
  private readonly play = h("button", { class: "btn btn-start", text: "自動再生", onClick: () => {
    if (this.playing !== null) { this.stopPlayback(); this.status.textContent = "一時停止しました。「再開」で続きから見られます。"; }
    else this.startPlayback();
  } });
  private readonly status = h("p", { class: "route-status", attrs: { role: "status" } });
  private readonly scrub = h("input", { attrs: { type: "range", min: "0", max: "1", step: "0.01", value: "0", "aria-label": "ルートの途中を確認" } });

  constructor(private readonly store: KeyValueStore, private readonly page: "edit" | "watch", private readonly openOther: (item: SuggestedRoute) => void) {
    this.speed.value = "1";
    try {
      const saved = loadRoute(store);
      if (saved) { this.frames = saved.frames; this.draft = cloneRoutePose(this.frames[0]!.pose); this.name.value = saved.name; this.saved = true; }
      this.saveStatus.textContent = saved ? "作成中のルートを自動保存から読み込みました。" : "姿勢を登録すると作成中のルートを自動保存します。残したいルートは「一覧に保存」してください。";
    } catch { this.canSave = false; this.saveStatus.textContent = "保存済みルートを読み取れません。元の保存内容を保護し、この画面では上書きしません。"; }
    this.name.addEventListener("change", () => { if (!this.name.value.trim()) this.name.value = "自分のルート"; this.persist(); });
    this.label.addEventListener("input", () => { this.dirty = true; });
    this.seconds.addEventListener("input", () => { this.dirty = true; });
    const canvas = h("canvas", { attrs: { "aria-label": page === "edit" ? "点をドラッグして人形を動かす。部位と座標の入力でも操作できます。" : "ルートの二人。ドラッグで視点を変更できます。" } });
    const pins = h("div", { class: "practice-pins", attrs: { "aria-hidden": "true" } });
    const cameras = h("div", { class: "practice-cameras" });
    for (const [label, view] of [["斜め", "diagonal"], ["横", "side"], ["真上", "top"]] as const) cameras.append(h("button", { class: "btn", text: label, onClick: () => this.scene.setCamera(view) }));
    const layers = h("div", { class: "practice-display" });
    for (const [label, bones] of [["身体", false], ["骨格", true]] as const) layers.append(h("button", { class: "btn", text: label, attrs: { "aria-pressed": String(!bones) }, onClick: () => {
      this.bones = bones;
      layers.querySelectorAll("button").forEach((button) => button.setAttribute("aria-pressed", String(button.textContent === label)));
      this.display();
    } }));
    const faded = h("input", { attrs: { type: "checkbox" } });
    faded.addEventListener("change", () => { this.faded = faded.checked; this.display(); });
    layers.append(h("label", {}, faded, "もう一人を透かす"));
    const actor = h("select", { attrs: { "aria-label": "動かす人形" } }, h("option", { text: "青を動かす", attrs: { value: "blue" } }), h("option", { text: "赤を動かす", attrs: { value: "red" } }));
    actor.addEventListener("change", () => { this.actor = actor.value as Actor; this.draw(); });
    for (const [value, label] of POINTS) this.pointSelect.append(h("option", { text: label, attrs: { value } }));
    this.pointSelect.value = this.point;
    this.pointSelect.addEventListener("change", () => { this.point = this.pointSelect.value as PointId; this.draw(); });
    const axes = h("div", { class: "route-axes" });
    for (const [axis, label] of [["x", "マットの横"], ["y", "高さ"], ["z", "マットの奥行き"]] as const) {
      const input = h("input", { attrs: { type: "number", min: "-3", max: "3", step: "0.02", "aria-label": label } });
      input.addEventListener("change", () => {
        if (!Number.isFinite(input.valueAsNumber)) return this.draw();
        this.remember(); const p = this.target().clone(); p[axis] = Math.max(-3, Math.min(3, input.valueAsNumber)); this.move(this.point, p);
      });
      axes.append(h("label", {}, label, input, "m")); this.coordinates.push(input);
    }
    const rotate = h("div", { class: "route-rotation" }, h("p", { text: "胴体の向き（手足の目標を保つ）" }));
    for (const [axis, label] of [["x", "前後に傾ける"], ["y", "左右へ向く"], ["z", "横に傾ける"]] as const) {
      const row = h("div", {}, h("span", { text: label }));
      for (const sign of [-1, 1]) row.append(h("button", { class: "btn", text: `${sign > 0 ? "+" : "−"}5°`, attrs: { "aria-label": `${label} ${sign * 5}度` }, onClick: () => {
        this.remember(); const e = new Euler(); e[axis] = sign * Math.PI / 36;
        const q = new Quaternion().setFromEuler(e); this.draft[this.actor].up.applyQuaternion(q); this.draft[this.actor].front.applyQuaternion(q);
        this.dirty = true; this.draw();
      } }));
      rotate.append(row);
    }
    const preset = h("select", { attrs: { "aria-label": "姿勢の出発点" } });
    PRESETS.forEach(([label], index) => preset.append(h("option", { text: label, attrs: { value: String(index) } })));
    const courses = h("div", { class: "route-courses" }, h("h3", { text: "ポジションから先の攻防を見る" }), h("p", { text: "何を整えて次へ進むかを、順番に観察します。局面間の返し・パス・脚の組み替えは段階表示です。" }));
    for (const course of ROUTE_COURSES) courses.append(h("article", {}, h("h4", { text: course.name }), h("p", { text: course.description }),
      h("button", { class: "btn", text: "例題を自動再生", attrs: { "aria-label": `${course.name}を自動再生` }, onClick: () => this.watch(courseRoutes(course), course.name) })));
    const library = h("details", { class: "route-library" }, h("summary", { text: `状況別の${SUGGESTED_ROUTES.length}ルート` }), h("p", { text: "自動再生では自作ルートを残したまま観察できます。大会報告を参考にした3例には出典を表示しています。" }));
    const category = h("select", { attrs: { "aria-label": "ルートの状況" } }, h("option", { text: "すべての状況", attrs: { value: "all" } }),
      ...[...new Set(SUGGESTED_ROUTES.map((r) => r.category))].map((value) => h("option", { text: value, attrs: { value: value! } })));
    const cards = h("div", { class: "route-cards" });
    const filtered = () => SUGGESTED_ROUTES.filter((r) => category.value === "all" || r.category === category.value);
    const showCards = () => {
      cards.replaceChildren();
      for (const item of filtered()) {
        const card = h("article", {}, h("h3", { text: item.name }), h("p", { text: item.fact }), h("p", { text: item.exercise }));
        if (item.source) card.append(h("a", { text: `${item.sourceTitle}（${item.published}）`, attrs: { href: item.source, target: "_blank", rel: "noopener noreferrer" } }));
        card.append(h("button", { class: "btn", text: "自動再生", attrs: { "aria-label": `${item.name}を自動再生` }, onClick: () => this.watch([item], item.name) }),
          h("button", { class: "btn", text: "編集用に開く", attrs: { "aria-label": `${item.name}を編集用に開く` }, onClick: () => this.editExample(item) }));
        cards.append(card);
      }
    }; category.addEventListener("change", showCards); showCards();
    library.append(h("div", { class: "route-library-actions" }, category, h("button", { class: "btn btn-start", text: "表示中のルートを順番に自動再生", onClick: () => this.watch(filtered(), "状況別ルートの連続再生") })), cards,
        h("p", {}, "2026年9月7日時点で、ADCC世界大会は9月12〜13日の開催前です。結果を予測してルートへ反映してはいません。", h("a", { text: "ADCC公式・9月6日更新", attrs: { href: "https://adcombat.com/official-competitor-list-for-the-adcc-world-championship-2026-updated/", target: "_blank", rel: "noopener noreferrer" } })));
    this.scrub.addEventListener("input", () => { this.stopPlayback(); this.preview(Number(this.scrub.value)); });
    const savedDetails = h("details", {}, this.savedSummary, this.savedRoutes);
    const file = h("input", { attrs: { type: "file", accept: ".json,application/json", hidden: "", "aria-label": "ルートのJSONファイル" } });
    const importButton = h("button", { class: "btn", text: "ファイルを読み込む", onClick: () => file.click() });
    file.addEventListener("change", async () => {
      const selected = file.files?.[0];
      if (!selected) return;
      importButton.disabled = true;
      try {
        if (selected.size > 200_000) throw new Error("File is too large");
        const route = decodeRoute(await selected.text());
        addSavedRoute(this.store, crypto.randomUUID(), route);
        this.refreshSavedRoutes(); savedDetails.open = true;
        this.libraryStatus.textContent = `「${route.name}」を一覧へ読み込みました。作成中のルートは残っています。再生すると動きも検査します。`;
      } catch { this.libraryStatus.textContent = "読み込めませんでした。柔術道場のJSONファイル（200KB以内）か、保存上限・ブラウザの保存設定を確認してください。元のルートは変更していません。"; }
      finally { file.value = ""; importButton.disabled = false; }
    });
    const savedLibrary = h("section", { class: "route-saved", attrs: { "aria-label": "自作ルートの保存と読み込み" } },
      h("h3", { text: "自作ルート" }), h("p", { text: "作成中の自動保存とは別に、最大20件を一覧に残せます。ファイルに書き出すと他のブラウザでも使えます。" }),
      h("div", { class: "route-library-actions" },
        ...(page === "edit" ? [h("button", { class: "btn", text: "一覧に保存", onClick: () => {
          if (!this.registeredForFile()) return;
          try {
            addSavedRoute(this.store, crypto.randomUUID(), { name: this.name.value.trim() || "自分のルート", frames: this.frames });
            this.refreshSavedRoutes(); savedDetails.open = true;
            this.libraryStatus.textContent = "現在のルートを一覧に保存しました。作成中のルートを変更しても、一覧の内容は変わりません。";
          } catch { this.libraryStatus.textContent = "一覧に保存できませんでした。20件の上限・保存容量・ブラウザの設定を確認してください。元の一覧は変更していません。"; }
        } }), h("button", { class: "btn", text: "作成中のルートを書き出す", onClick: () => {
          if (this.registeredForFile()) this.exportRoute({ name: this.name.value.trim() || "自分のルート", frames: this.frames });
        } })] : []), importButton, file), this.libraryStatus, savedDetails);
    this.root = h("div", { class: `route-editor route-${page}` },
      h("div", { class: "route-heading" }, h("h2", { text: page === "edit" ? "人形で、動きのルートを作る" : "ルートを確認する" }),
        h("p", { text: page === "edit" ? "点をドラッグして姿勢を登録。再生は「ルート確認」で行えます。何もない場所のドラッグは視点を回します。" : "例題や自作ルートを再生して、ポジションの次に何を整えるかを観察します。" }),
        h("button", { class: "btn btn-start", text: page === "edit" ? "このルートを確認する" : "作成中のルートを確認", onClick: () => {
          if (page === "edit") {
            if (this.dirty) { this.status.textContent = "動かした姿勢を登録してから確認へ進んでください。"; return; }
            this.openOther(this.customExample());
          } else {
            try {
              const saved = loadRoute(store);
              if (!saved) { this.status.textContent = "保存したルートがありません。「ルート作成」で姿勢を登録してください。"; return; }
              this.frames = saved.frames; this.name.value = saved.name; this.watch([this.customExample()], "自作ルートの確認");
            } catch { this.status.textContent = "保存したルートを読み込めませんでした。保存内容は変更していません。"; }
          }
        } })),
      savedLibrary,
      ...(page === "watch" ? [courses, library] : []),
      h("div", { class: "route-workbench" },
        h("div", { class: "route-observation" }, this.viewing, layers, h("div", { class: "route-view" }, canvas, pins, cameras, this.caption),
          ...(page === "watch" ? [h("div", { class: "route-playback" }, this.play, h("button", { class: "btn", text: "先頭から", onClick: () => this.startPlayback(true) }),
            h("button", { class: "btn", text: "停止", onClick: () => { this.stopPlayback(); this.status.textContent = "停止しました。繰り返し再生も止まっています。"; } }),
            h("button", { class: "btn", text: "このルートを編集する", onClick: () => this.editExample(this.watching?.items[this.watching.index] ?? this.customExample()) }),
            h("label", {}, "速度", this.speed), h("label", {}, this.repeat, "繰り返し"), this.scrub)] : []),
          this.status, ...(page === "edit" ? [this.saveStatus] : []), this.timeline),
        ...(page === "edit" ? [h("fieldset", { class: "route-controls", attrs: { "aria-label": "人形とルートの編集" } },
          h("h3", { text: "人形を動かす" }), actor, this.pointSelect, axes, rotate,
          h("p", { class: "practice-model-note", text: "腰を動かすと、手足を目標に残して身体が動きます。肘・膝の点は曲がる方向の指定です。" }),
          h("button", { class: "btn", text: "動かす前に戻す", onClick: () => { if (this.undo) { this.stopPlayback(); this.draft = cloneRoutePose(this.undo); this.undo = null; this.dirty = true; this.draw(); } } }),
          this.issues,
          h("h3", { text: "姿勢を登録" }), h("label", {}, "ルート名", this.name), h("label", {}, "姿勢の名前", this.label), h("label", {}, "この姿勢へ動く秒数", this.seconds),
          this.add, this.update, h("details", {}, h("summary", { text: "別の姿勢を出発点にする" }), preset, h("button", { class: "btn", text: "この姿勢を試す", onClick: () => { this.remember(); this.draft = routePose(PRESETS[Number(preset.value)]![1]()); this.dirty = true; this.draw(); } })))] : []),
      ),
      h("p", { class: "route-model-note", text: "判定できるのは、この模式図での到達距離・表示用の屈曲範囲・床や身体内部への貫通などです。筋力、柔軟性、肩や股関節の回旋、摩擦、相手の反応は再現していません。指摘なしでも実技で可能・安全とは限りません。" }),
    );
    this.scene = new BodyScene(canvas, pins); this.selectFrame(0); this.refreshSavedRoutes();
    window.addEventListener("beforeunload", (event) => { if (page === "edit" && (this.dirty || this.unsaved)) { event.preventDefault(); event.returnValue = ""; } });
  }

  setActive(active: boolean): void {
    this.active = active;
    if (active) this.refreshSavedRoutes();
    if (!active && this.playing !== null) { this.stopPlayback(); this.status.textContent = "画面を離れたため一時停止しました。「再開」で続けられます。"; }
    this.scene.setActive(active);
  }
  openCourse(id: string): void {
    const course = ROUTE_COURSES.find((c) => c.id === id);
    if (course) this.watch(courseRoutes(course), course.name);
  }
  openRoute(item: SuggestedRoute): void { this.watch([item], item.name); }
  private customExample(route: SavedRoute = { name: this.name.value, frames: this.frames }): SuggestedRoute {
    const frames = route.frames.map((f) => ({ ...f, pose: cloneRoutePose(f.pose) }));
    return { id: "custom", name: route.name, fact: "ルート作成で登録した姿勢を順番に再生します。", exercise: "途中に指摘があれば、作成ページで中間姿勢を追加して調整できます。", frames: () => frames.map((f) => ({ ...f, pose: cloneRoutePose(f.pose) })) };
  }
  private registeredForFile(): boolean {
    if (!this.dirty) return true;
    this.libraryStatus.textContent = "未登録の編集があります。先に姿勢を登録するか、登録済みの姿勢へ戻してください。";
    return false;
  }
  private refreshSavedRoutes(): void {
    this.savedRoutes.replaceChildren();
    try {
      const entries = loadSavedRoutes(this.store);
      this.savedSummary.textContent = `保存したルート（${entries.length} / 20）`;
      if (!entries.length) this.savedRoutes.append(h("p", { text: "一覧は空です。ルート作成で「一覧に保存」するか、書き出したファイルを読み込んでください。" }));
      for (const { id, route } of entries) {
        this.savedRoutes.append(h("article", {}, h("h4", { text: route.name }), h("p", { text: `${route.frames.length}姿勢` }),
          h("div", { class: "route-library-actions" },
            h("button", { class: "btn", text: "確認する", attrs: { "aria-label": `${route.name}を確認する` }, onClick: () => {
              const item = this.customExample(route);
              if (this.page === "edit") this.openOther(item); else this.watch([item], item.name);
            } }),
            h("button", { class: "btn", text: "編集用に開く", attrs: { "aria-label": `${route.name}を編集用に開く` }, onClick: () => this.editExample(this.customExample(route)) }),
            h("button", { class: "btn", text: "書き出す", attrs: { "aria-label": `${route.name}を書き出す` }, onClick: () => this.exportRoute(route) }),
            h("button", { class: "btn", text: "削除", attrs: { "aria-label": `${route.name}を一覧から削除` }, onClick: () => {
              if (!window.confirm(`「${route.name}」を一覧から削除しますか？ 作成中のルートと書き出したファイルは残ります。`)) return;
              try { removeSavedRoute(this.store, id); this.refreshSavedRoutes(); this.libraryStatus.textContent = "一覧から削除しました。"; }
              catch { this.refreshSavedRoutes(); this.libraryStatus.textContent = "削除できませんでした。保存設定や、別のページで一覧が変わっていないか確認してください。"; }
            } }))));
      }
    } catch { this.savedSummary.textContent = "保存したルートを読み込めません"; this.savedRoutes.append(h("p", { text: "元の保存内容を保護し、一覧の追加・削除は行いません。作成中のルートはファイルに書き出せます。" })); }
  }
  private exportRoute(route: SavedRoute): void {
    try {
      const url = URL.createObjectURL(new Blob([encodeRoute(route)], { type: "application/json" }));
      const link = h("a", { attrs: { href: url, download: `${route.name.replace(/[\\/:*?"<>|\u0000-\u001f]/g, "_")}.json` } });
      document.body.append(link); link.click(); link.remove();
      window.setTimeout(() => URL.revokeObjectURL(url), 1000);
      this.libraryStatus.textContent = "JSONファイルを書き出しました。別のブラウザでも「ファイルを読み込む」で使えます。";
    } catch { this.libraryStatus.textContent = "書き出せませんでした。登録済みの姿勢とブラウザの設定を確認してください。"; }
  }
  private currentFrames(): RouteFrame[] { return this.watching?.frames ?? this.frames; }
  private watch(items: SuggestedRoute[], title: string): void {
    if (!items.length) return;
    this.stopPlayback(); this.watching = { items, index: 0, frames: items[0]!.frames(), title };
    this.showWatching(); this.startPlayback(true);
    this.root.querySelector(".route-workbench")!.scrollIntoView({ block: "start" });
  }
  private showWatching(): void {
    const watching = this.watching!;
    const item = watching.items[watching.index]!;
    this.viewing.replaceChildren(h("p", { class: "route-stage-count", text: `${watching.title} · ${watching.index + 1} / ${watching.items.length}` }),
      h("h3", { text: item.name }), h("p", { class: item.top ? `route-top-${item.top}` : "", text: item.top ? `${item.top === "blue" ? "青" : "赤"}がトップ` : "身体の位置と手足の関係を観察" }),
      h("p", { text: item.fact }), h("p", { class: "route-transition-note", text: item.exercise }),
      h("p", { text: "自作ルートはそのまま残っています。" }));
    const navigation = h("div", { class: "route-stage-nav" });
    watching.items.forEach((entry, index) => navigation.append(h("button", { class: "btn", text: entry.name, attrs: { "aria-pressed": String(index === watching.index), "aria-label": `局面${index + 1}：${entry.name}` }, onClick: () => {
      this.stopPlayback(); watching.index = index; watching.frames = entry.frames(); this.showWatching(); this.status.textContent = "局面を選びました。「自動再生」で動きを確認できます。";
    } })));
    if (watching.items.length > 1) {
      const previous = h("button", { class: "btn", text: "前の局面", onClick: () => this.jumpStage(-1) }); previous.disabled = watching.index === 0;
      const next = h("button", { class: "btn", text: "次の局面", onClick: () => this.jumpStage(1) }); next.disabled = watching.index === watching.items.length - 1;
      this.viewing.append(h("div", { class: "route-stage-actions" }, previous, next), h("details", {}, h("summary", { text: "全局面を見る" }), navigation));
    }
    this.scrub.value = "0"; this.renderTimeline(); this.preview(0);
  }
  private jumpStage(delta: number): void {
    const watching = this.watching!; this.stopPlayback(); watching.index += delta;
    watching.frames = watching.items[watching.index]!.frames(); this.showWatching();
    this.status.textContent = "局面を切り替えました。「自動再生」で動きを確認できます。";
  }
  editExample(item: SuggestedRoute): void {
    if (this.page === "watch") { this.stopPlayback(); this.openOther(item); return; }
    if ((this.dirty || this.saved || this.unsaved) && !window.confirm("作成中のルートとその自動保存を置き換えますか？ 一覧に保存したルートは残ります。")) return;
    this.stopPlayback(); this.watching = null; this.viewing.replaceChildren();
    this.frames = item.frames(); this.name.value = item.name; this.selectFrame(0); this.persist();
    this.root.querySelector(".route-workbench")!.scrollIntoView({ block: "start" });
  }
  private target(point = this.point): Vector3 {
    const p = this.draft[this.actor];
    if (point === "pelvis") return p.pelvis;
    const side = point.endsWith("L") ? "L" : "R";
    return p[point.startsWith("wrist") || point.startsWith("elbow") ? "arms" : "legs"][side][point.startsWith("elbow") || point.startsWith("knee") ? "pole" : "end"];
  }
  private remember(): void { this.stopPlayback(); this.undo = cloneRoutePose(this.draft); }
  private move(id: string, point: Vector3): void {
    this.target(id as PointId).copy(point).clampScalar(-3, 3); this.dirty = true; this.draw();
  }
  private display(): void { this.scene.setDisplay(this.bones, this.faded, true, this.actor === "blue" ? "red" : "blue"); }
  private draw(): void {
    if (this.page === "watch") { this.preview(this.selected); return; }
    this.root.querySelector<HTMLFieldSetElement>(".route-controls")!.disabled = false;
    const { pair, issues } = inspectRoutePose(this.draft);
    this.scene.show(pair); this.display();
    this.scene.setHandles(POINTS.map(([id]) => ({ id, point: this.target(id), selected: id === this.point })),
      (id) => { this.remember(); this.point = id as PointId; this.pointSelect.value = id; this.draw(); }, (id, point) => this.move(id, point));
    const point = this.target(); this.coordinates.forEach((input, index) => { input.value = point.getComponent(index).toFixed(3); });
    const blocked = issues.some((i) => i.level === "blocked");
    this.add.disabled = blocked || this.frames.length >= 40; this.update.disabled = blocked;
    this.caption.textContent = blocked ? `登録不可：${issues.find((i) => i.level === "blocked")!.message}` : `${this.actor === "blue" ? "青" : "赤"}の${POINTS.find(([id]) => id === this.point)![1]}を編集中${this.dirty ? " · 未登録" : ""}`;
    this.issues.classList.toggle("has-blocked", blocked);
    this.issues.replaceChildren(h("strong", { text: blocked ? "このままでは登録できません" : issues.length ? "確認したい点があります" : "この姿勢で検出した問題はありません" }),
      ...issues.slice(0, 8).map((issue, index) => h("p", { text: `${index + 1}. ${issue.actor === "blue" ? "青" : "赤"}：${issue.message}` })));
  }
  private selectFrame(index: number): void {
    this.stopPlayback(); this.selected = index; const frame = this.frames[index]!;
    this.draft = cloneRoutePose(frame.pose); this.label.value = frame.label; this.seconds.value = String(frame.seconds);
    this.dirty = false; this.undo = null; this.scrub.value = String(index); this.draw(); this.renderTimeline();
  }
  private register(replace: boolean): void {
    if (inspectRoutePose(this.draft).issues.some((i) => i.level === "blocked")) return;
    const seconds = this.seconds.valueAsNumber;
    if (!Number.isFinite(seconds) || seconds < 0.5 || seconds > 10) { this.status.textContent = "秒数は0.5〜10の範囲で入力してください。"; return; }
    const frame = { label: this.label.value.trim() || `姿勢 ${this.frames.length + 1}`, seconds, pose: cloneRoutePose(this.draft) };
    if (replace) this.frames[this.selected] = frame;
    else { if (this.frames.length >= 40) return; this.frames.splice(this.selected + 1, 0, frame); this.selected++; }
    this.selectFrame(this.selected); this.persist(); this.status.textContent = "姿勢を登録しました。途中の指摘も確認してください。";
  }
  private renderTimeline(): void {
    const frames = this.currentFrames();
    this.timeline.replaceChildren(); this.scrub.max = String(Math.max(0, frames.length - 1));
    this.play.disabled = frames.length < 2; this.scrub.disabled = frames.length < 2;
    frames.forEach((frame, index) => {
      const issue = index ? inspectTransition(frames[index - 1]!.pose, frame.pose) : null;
      const item = h("li", { class: index === this.selected ? "selected" : "" },
        h("button", { class: "route-frame", text: `${index + 1}. ${frame.label}`, attrs: { "aria-pressed": String(index === this.selected) }, onClick: () => {
          if (this.page === "watch") { this.stopPlayback(); this.scrub.value = String(index); this.preview(index); return; }
          if (this.dirty && !window.confirm("未登録の姿勢を破棄して、登録済みの姿勢へ移りますか？")) return;
          this.selectFrame(index);
        } }), h("span", { class: issue ? "route-problem" : "route-frame-info", text: issue ? "途中で干渉・届かない目標あり" : `${frame.seconds}秒` }));
      if (this.page === "watch") { this.timeline.append(item); return; }
      for (const [label, delta] of [["前へ", -1], ["後へ", 1]] as const) {
        const button = h("button", { class: "btn", text: label, attrs: { "aria-label": `姿勢${index + 1}を${label}` }, onClick: () => {
          if (this.dirty) { this.status.textContent = "先に姿勢を登録するか、登録済みの姿勢へ戻してください。"; return; }
          this.stopPlayback(); const next = index + delta;
          [this.frames[index], this.frames[next]] = [this.frames[next]!, this.frames[index]!]; this.selectFrame(next); this.persist();
        } }); button.disabled = index + delta < 0 || index + delta >= this.frames.length; item.append(button);
      }
      const remove = h("button", { class: "btn", text: "削除", attrs: { "aria-label": `姿勢${index + 1}を削除` }, onClick: () => {
        if (this.dirty) { this.status.textContent = "未登録の編集があります。先に姿勢を登録するか、登録済みの姿勢へ戻してください。"; return; }
        if (!window.confirm(`「${frame.label}」を削除しますか？`)) return;
        this.frames.splice(index, 1); this.selectFrame(Math.min(index, this.frames.length - 1)); this.persist();
      } }); remove.disabled = this.frames.length <= 1; item.append(remove); this.timeline.append(item);
    });
  }
  private preview(position: number): boolean {
    const controls = this.root.querySelector<HTMLFieldSetElement>(".route-controls");
    if (controls) controls.disabled = true;
    const frames = this.currentFrames();
    const index = Math.min(Math.floor(position), frames.length - 1), fraction = position - index;
    const pose = index >= frames.length - 1 ? cloneRoutePose(frames[index]!.pose) : interpolateRoute(frames[index]!.pose, frames[index + 1]!.pose, fraction);
    const { pair, issues } = inspectRoutePose(pose); this.scene.show(pair); this.display(); this.scene.setHandles([], () => {}, () => {});
    const blocked = issues.find((i) => i.level === "blocked");
    this.caption.textContent = blocked ? `停止位置：${blocked.message}` : `${frames[index]!.label}${fraction > 0 ? ` → ${frames[index + 1]!.label}（${Math.round(fraction * 100)}%）` : ""}`;
    this.issues.classList.toggle("has-blocked", !!blocked);
    this.issues.replaceChildren(h("strong", { text: issues.length ? "途中の動きへの指摘" : "この時点で検出した問題はありません" }), ...issues.slice(0, 6).map((i) => h("p", { text: `${i.actor === "blue" ? "青" : "赤"}：${i.message}` })));
    this.add.disabled = true; this.update.disabled = true;
    this.timeline.querySelectorAll("li").forEach((li, i) => { li.classList.toggle("selected", i === index); li.querySelector("button")?.setAttribute("aria-pressed", String(i === index)); });
    return !!blocked;
  }
  private startPlayback(fromStart = false): void {
    if (!this.watching && this.dirty) { this.status.textContent = "動かした姿勢を登録してから再生してください。"; return; }
    const frames = this.currentFrames();
    if (frames.length < 2) { this.status.textContent = "姿勢を2つ以上登録すると自動再生できます。"; return; }
    this.stopPlayback();
    let position = Number(this.scrub.value);
    if (fromStart || position >= frames.length - 1) position = 0;
    let player = new RoutePlayback(frames, position), previous = performance.now(), finishedAt: number | null = null;
    this.play.textContent = "一時停止";
    this.status.textContent = "自動再生中。問題を検出した位置で停止します。";
    const tick = (now: number) => {
      if (!this.active) { this.stopPlayback(); return; }
      if (finishedAt !== null) {
        if (now - finishedAt >= 800) {
          const watching = this.watching;
          if (watching && watching.index + 1 < watching.items.length) watching.index++;
          else if (this.repeat.checked) { if (watching) watching.index = 0; }
          else { this.stopPlayback(); this.play.textContent = "自動再生"; this.status.textContent = "最後まで再生しました。"; return; }
          if (watching) { watching.frames = watching.items[watching.index]!.frames(); this.showWatching(); }
          player = new RoutePlayback(this.currentFrames()); finishedAt = null;
          this.status.textContent = watching ? `局面 ${watching.index + 1} / ${watching.items.length} に切り替えました。` : "先頭へ戻って繰り返しています。";
        }
        previous = now; this.playing = requestAnimationFrame(tick); return;
      }
      const step = player.advance((now - previous) / 1000 * Number(this.speed.value)); previous = now;
      this.scrub.value = String(step.position);
      const blocked = this.preview(step.position);
      if (step.problem || blocked) { this.stopPlayback(); this.status.textContent = step.problem ? `途中で停止：${step.problem.message}` : "途中の動きで停止しました。表示した位置を確認してください。"; return; }
      if (step.done) finishedAt = now;
      this.playing = requestAnimationFrame(tick);
    }; this.playing = requestAnimationFrame(tick);
  }
  private stopPlayback(): void {
    if (this.playing !== null) cancelAnimationFrame(this.playing);
    this.playing = null; this.play.textContent = Number(this.scrub.value) > 0 && Number(this.scrub.value) < Number(this.scrub.max) ? "再開" : "自動再生";
  }
  private persist(): void {
    this.unsaved = true;
    if (!this.canSave) return;
    try { saveRoute(this.store, { name: this.name.value.trim() || "自分のルート", frames: this.frames }); this.saved = true; this.unsaved = false; this.saveStatus.textContent = "このブラウザに保存しました。"; }
    catch { this.saveStatus.textContent = "保存できませんでした。この画面のルートは残っています。ブラウザの保存設定を確認してください。"; }
  }
}
