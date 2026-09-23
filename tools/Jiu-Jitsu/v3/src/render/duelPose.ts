import type { DuelState } from "../engine/duel";
import { backPair, guardPair, mountPair, sidePair, type PracticePair } from "./practicePose";
import { SUGGESTED_ROUTES } from "../content/routes";
import { inspectRoutePose } from "../engine/route";

export function duelPair(state: DuelState): PracticePair {
  if (state.winner && state.preparation[state.winner] < 2) return backPair("safe");
  if (state.winner && ["mount", "side"].includes(state.position)) {
    const pose = SUGGESTED_ROUTES.find((r) => r.id === "armbar-shape")!.frames().at(-1)!.pose;
    return inspectRoutePose(state.winner === "blue" ? pose : { blue: pose.red, red: pose.blue }).pair;
  }
  switch (state.position) {
    case "guard": return guardPair(state.control === 0 ? "angle" : "stable", state.top);
    case "side": return sidePair(state.control === 0 ? "framed" : "pinned", state.top);
    case "mount": return mountPair(state.preparation[state.top] > 0 ? "isolated" : state.control === 0 ? "mounted" : "low", state.top);
    case "back": {
      const pair = backPair(state.winner ? "locked" : state.control < 2 ? "defended" : "reach");
      if (state.top === "red") return pair;
      return { blue: pair.red, red: pair.blue, cues: pair.cues, supports: pair.supports.map((s) => ({ ...s, actor: s.actor === "blue" ? "red" : "blue" })) };
    }
  }
}
