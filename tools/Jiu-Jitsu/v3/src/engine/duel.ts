import type { Actor } from "../render/practicePose";
import { mulberry32, type Rng } from "./rng";
export type DuelAction = "secure" | "frame" | "escape" | "advance" | "attack" | "rest" | "posture" | "deny" | "isolate" | "bridge" | "hip-escape" | "hand-fight";
export type DuelPosition = "guard" | "side" | "mount" | "back";
export interface DuelState {
  position: DuelPosition; top: Actor; control: number; turn: number;
  preparation: Record<Actor, number>; energy: Record<Actor, number>; points: Record<Actor, number>;
  winner: Actor | null; ended: boolean;
}
export interface DuelBeat { actor: Actor; action: DuelAction; message: string; before: DuelState; after: DuelState }
export const DUEL_ACTIONS: Record<DuelAction, { label: string; cost: number; description: string }> = {
  secure: { label: "上の土台を整える", cost: 5, description: "支配を1段階上げる。位置を進める前の準備。" },
  frame: { label: "フレームで崩す", cost: 5, description: "相手の支配を下げ、返しの準備を1段階進める。" },
  escape: { label: "返し・ガード回復", cost: 12, description: "準備が2、相手の支配が2以下なら進める。サイド・バックではまずガード回復。" },
  advance: { label: "ポジションを進める", cost: 10, description: "支配が2以上なら、ガード内 → サイド → マウント → バックへ。" },
  attack: { label: "極めの準備・仕掛け", cost: 12, description: "支配2以上で準備を進め、2回の仕掛けが通ると一本。" },
  rest: { label: "力を緩めて回復", cost: 0, description: "余力を22戻す。相手にも一手の機会がある。" },
  posture: { label: "上体を起こして姿勢を戻す", cost: 14, description: "ガード内で支配を2段階戻す。土台を整えるより余力を使う。" },
  deny: { label: "返しの支えを外す", cost: 8, description: "相手の返しの準備を0に戻す。自分の極めの準備も1段階下がる。" },
  isolate: { label: "片腕をコントロールする", cost: 8, description: "支配2以上で極めの準備を1段階進める。手を使うぶん支配が1下がる。" },
  bridge: { label: "ブリッジで支えを崩す", cost: 14, description: "サイド・マウントの下で支配を2下げ、返しの準備を1進める。余力を多く使う。" },
  "hip-escape": { label: "腰を引いて膝の空間を作る", cost: 9, description: "支配1以下なら返しの準備が2になる。押さえ込まれていると支配を1下げるだけ。" },
  "hand-fight": { label: "肘を戻して腕を守る", cost: 6, description: "相手の極めの準備を0に戻す。位置や返しの準備は進まない。" },
};
export class DuelEngine {
  state: DuelState = { position: "guard", top: "red", control: 1, turn: 0, preparation: { blue: 0, red: 0 }, energy: { blue: 100, red: 100 }, points: { blue: 0, red: 0 }, winner: null, ended: false };
  readonly history: DuelBeat[] = [];
  private readonly rng: Rng;
  opponentPlan: DuelAction = "advance";
  constructor(seed = 1, private readonly style: "pressure" | "counter" = "pressure", start: "guard-bottom" | "guard-top" | "mount-bottom" | "side-top" = "guard-bottom") {
    this.rng = mulberry32(seed);
    if (start === "guard-top" || start === "side-top") this.state.top = "blue";
    if (start === "mount-bottom") this.state.position = "mount";
    if (start === "side-top") this.state.position = "side";
    this.opponentPlan = this.plan();
  }
  available(actor: Actor = "blue"): DuelAction[] {
    if (this.state.ended) return [];
    const position = this.state.position;
    const choices: DuelAction[] = this.state.top === actor
      ? ["secure", position === "guard" ? "posture" : "isolate", "deny", ...(position !== "back" ? ["advance" as const] : []), ...(position !== "guard" ? ["attack" as const] : []), "rest"]
      : ["frame", ...(["side", "mount"].includes(position) ? ["bridge" as const] : []), ...(position !== "back" ? ["hip-escape" as const] : []), "hand-fight", "escape", "rest"];
    return choices.filter((a) => this.state.energy[actor] >= DUEL_ACTIONS[a].cost);
  }
  turn(action: DuelAction): DuelBeat[] {
    if (!this.available().includes(action)) throw new Error("この局面では使えない行動です。");
    this.state.turn++;
    const beats = [this.act("blue", action)];
    if (!this.state.ended) {
      const reply = this.available("red").includes(this.opponentPlan) ? this.opponentPlan : this.plan();
      beats.push(this.act("red", reply));
    }
    if (this.state.turn >= 12) this.state.ended = true;
    beats[beats.length - 1]!.after = structuredClone(this.state);
    this.history.push(...beats);
    this.opponentPlan = this.plan();
    return beats;
  }
  tap(): void { if (!this.state.ended) { this.state.ended = true; this.state.winner = "red"; } }
  private plan(): DuelAction {
    if (this.state.energy.red < 15) return "rest";
    const top = this.state.top === "red";
    if (!top) {
      if (this.state.preparation.blue > 0 && this.state.control >= 2) return "hand-fight";
      if (this.state.preparation.red >= 2) return "escape";
      if (this.style === "counter" && this.state.control <= 1 && this.state.position !== "back") return "hip-escape";
      if (this.state.control === 3 && ["mount", "side"].includes(this.state.position)) return "bridge";
      return this.rng() < (this.style === "counter" ? 0.65 : 0.35) ? "escape" : "frame";
    }
    if (this.style === "counter" && this.state.preparation.blue >= 2) return "deny";
    if (this.state.position === "guard" && this.state.control === 0) return "posture";
    if (this.state.control < 2 && this.rng() < 0.6) return "secure";
    if (this.state.position !== "guard" && this.rng() < 0.75) return "attack";
    return this.state.position === "back" ? "secure" : "advance";
  }
  private act(actor: Actor, action: DuelAction): DuelBeat {
    const s = this.state, other = actor === "blue" ? "red" : "blue", name = actor === "blue" ? "青" : "赤";
    const before = structuredClone(s);
    s.energy[actor] -= DUEL_ACTIONS[action].cost;
    let message: string;
    switch (action) {
      case "rest": s.energy[actor] = Math.min(100, s.energy[actor] + 22); message = `${name}が力を緩め、余力を戻した。`; break;
      case "secure": s.control = Math.min(3, s.control + 1); message = `${name}が膝と腰の支えを整えた。`; break;
      case "posture": s.control = Math.min(3, s.control + 2); message = `${name}が土台を使って上体を起こし、ガード内の姿勢を戻した。`; break;
      case "deny":
        s.preparation[other] = 0; s.preparation[actor] = Math.max(0, s.preparation[actor] - 1);
        message = `${name}が仕掛けをいったん緩め、返しに使う相手の手足の支えを外した。`; break;
      case "isolate":
        if (s.control >= 2) { s.preparation[actor] = Math.min(2, s.preparation[actor] + 1); s.control--; message = `${name}が片腕を保持した。支えの手を使ったため、位置の安定が下がった。`; }
        else message = `${name}は位置が安定せず、相手の腕を保持できなかった。`;
        break;
      case "bridge":
        s.control = Math.max(0, s.control - 2); s.preparation[actor] = Math.min(2, s.preparation[actor] + 1);
        message = `${name}が足と背中を支えに腰を上げ、相手の土台を崩した。まだ上下は変わっていない。`; break;
      case "hip-escape":
        if (s.control <= 1) { s.preparation[actor] = 2; message = `${name}が空いた方向へ腰を引き、膝を使う準備を整えた。`; }
        else { s.control--; message = `${name}は腰を引く空間を塞がれ、先に押さえ込みを緩めた。`; }
        break;
      case "hand-fight":
        s.preparation[other] = 0; message = `${name}が肘と手の位置を戻して腕を守り、相手の極めの準備を外した。`; break;
      case "frame":
        s.control = Math.max(0, s.control - 1); s.preparation[actor] = Math.min(2, s.preparation[actor] + 1); s.preparation[other] = Math.max(0, s.preparation[other] - 1);
        message = `${name}が前腕と腰で空間を作り、返しの準備を進めた。`; break;
      case "escape":
        if (s.preparation[actor] >= 2 && s.control <= 2) {
          const reversal = s.position === "guard" || s.position === "mount";
          if (reversal) s.top = actor;
          s.position = "guard"; s.control = 1; s.preparation = { blue: 0, red: 0 }; s.points[actor]++;
          message = reversal ? `${name}が返してトップを取った。相手のガードは残っている。` : `${name}が脚を戻し、下のままガードを回復した。`;
        } else { s.preparation[actor] = Math.min(2, s.preparation[actor] + 1); message = `${name}はまだ返せず、相手の支えを外す準備を続けた。`; }
        break;
      case "advance":
        if (s.control >= 2) {
          s.position = s.position === "guard" ? "side" : s.position === "side" ? "mount" : "back";
          s.control = 1; s.preparation = { blue: 0, red: 0 }; s.points[actor]++;
          message = `${name}が${s.position === "side" ? "膝の線を越えてサイドへ" : s.position === "mount" ? "胴をまたいでマウントへ" : "背後へ"}進んだ。`;
        } else { s.control = Math.min(3, s.control + 1); message = `${name}は相手に止められ、先に土台を整え直した。`; }
        break;
      case "attack":
        if (s.control >= 2) {
          s.preparation[actor] = Math.min(2, s.preparation[actor] + 1);
          if (s.preparation[actor] === 2) { s.winner = actor; s.ended = true; message = `${name}の仕掛けが通り、相手がタップ。一本で終了。`; }
          else message = `${name}が上の位置を保ち、極めに向けて腕を保持した。`;
        } else message = `${name}は支配が足りず、相手に腕を守られた。`;
        break;
    }
    return { actor, action, message, before, after: structuredClone(s) };
  }
}
