import { Vector3 } from "three";
import type { Stage } from "../content/types";
import { solveBody, type LimbTarget } from "./mannequin";
import { backPair, guardPair, kneeling, mountPair, pair, sidePair, supine, type PracticePair } from "./practicePose";

const v = (x: number, y: number, z: number) => new Vector3(x, y, z);
const limb = (end: Vector3, pole: Vector3): LimbTarget => ({ end, pole });

function armbar(): PracticePair {
  const blue = supine(0.20);
  const red = supine();
  red.pelvis = v(-0.37, 0.13, -0.12);
  red.up = v(-0.8, 0.6, 0); red.front = v(0.6, 0.8, 0);
  red.legs.L = limb(v(0.32, 0.29, -0.45), v(0, 0.39, -0.45));
  red.legs.R = limb(v(0.32, 0.17, 0.17), v(0.01, 0.36, 0.18));
  blue.arms.R = limb(v(-0.49, 0.33, -0.12), v(-0.27, 0.18, -0.16));
  red.arms.L = limb(v(-0.49, 0.33, -0.15), v(-0.65, 0.30, -0.38));
  red.arms.R = limb(v(-0.49, 0.33, -0.09), v(-0.65, 0.30, 0.15));
  const r = solveBody(red), b = solveBody(blue);
  return {
    red: r, blue: b,
    supports: [{ actor: "blue", joint: "ankleL", height: 0.055 }, { actor: "blue", joint: "ankleR", height: 0.055 }],
    cues: [{ label: "両手で手首を保持", point: b.wristR, kind: "contact" }, { label: "頭側と胸側に脚を置く", point: r.kneeL, kind: "contact" }],
  };
}

function triangle(): PracticePair {
  const red = supine(0, 0.24);
  const blue = kneeling(v(0, 0.38, 0.54), v(0, 0.20, -1));
  red.legs.L = limb(v(-0.10, 0.56, 0.14), v(0.37, 0.50, 0.20));
  red.legs.R = limb(v(0.12, 0.57, 0.44), v(-0.38, 0.56, 0.09));
  blue.arms.R = limb(v(-0.10, 0.25, 0.24), v(0.06, 0.34, 0.11));
  blue.arms.L = limb(v(-0.40, 0.16, 0.45), v(-0.40, 0.35, 0.15));
  red.arms.L = limb(v(0.10, 0.40, 0), v(0.34, 0.24, -0.15));
  red.arms.R = limb(v(-0.10, 0.40, 0), v(-0.34, 0.24, -0.15));
  const result = pair(blue, red, "blue");
  result.cues = [{ label: "首と片腕を脚で囲う", point: result.blue.neck, kind: "contact" }, { label: "もう一方の腕は外側", point: result.blue.elbowL, kind: "space" }];
  return result;
}

function openGuard(): PracticePair {
  const red = supine();
  red.legs.L = limb(v(0.43, 0.055, 0.89), v(0.44, 0.43, 0.47));
  red.legs.R = limb(v(-0.43, 0.055, 0.89), v(-0.44, 0.43, 0.47));
  const blue = kneeling(v(0.24, 0.38, 0.57));
  blue.arms.L = limb(v(-0.07, 0.30, 0.30), v(-0.17, 0.48, 0.51));
  blue.arms.R = limb(v(0.43, 0.28, 0.47), v(0.55, 0.48, 0.48));
  const result = pair(blue, red, "blue");
  result.cues = [{ label: "足の交差がほどけた", point: result.red.ankleL, kind: "floor" }];
  return result;
}

/** 応用ロールも、二人分の支持点と骨長を一緒に解く。Euler 補間は行わない。 */
export function rollPair(stage: Stage): PracticePair {
  switch (`${stage.red}/${stage.blue}`) {
    case "standingRed/standingBlue": return backPair("safe");
    case "redMountTop/blueUnderMount": return mountPair(stage.body === "high-mount" ? "high" : "low", "red");
    case "redMountArmbar/blueUnderMount": return mountPair("isolated", "red");
    case "redRolledBottom/blueUpaTop": return guardPair("stable", "blue");
    case "redClosedGuardBottom/blueTopInGuard": return guardPair(stage.body === "guard-angle" ? "angle" : "stable", "blue");
    case "redGuardArmbarFinish/blueGuardArmbarCaught":
      if (stage.body === "guard-pull" || stage.body === "guard-angle") return guardPair(stage.body === "guard-pull" ? "pull" : "angle", "blue");
      return armbar();
    case "redTriangleFinish/blueCaughtInTriangle": return triangle();
    case "redGuardOpened/blueGuardPass": return openGuard();
    case "redSideControl/blueUnderSide": return sidePair("pressured", "red");
    case "redSideControl/blueShrimpRecover": return sidePair("framed", "red");
    case "redBackControl/blueSeatedFront": return backPair("reach");
    case "redBackControl/blueBackDefend": return backPair(stage.body === "released" ? "safe" : "defended");
    case "redBackControl/blueTapped": return backPair(stage.body === "released" ? "safe" : "locked");
    case "redBackControl/blueGivesBack": return backPair("reach");
    default: throw new Error(`No roll mannequins for: ${stage.red}/${stage.blue}`);
  }
}
