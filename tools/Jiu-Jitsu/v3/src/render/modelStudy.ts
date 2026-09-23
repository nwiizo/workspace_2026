import { Vector3 } from "three";
import { solveBody, type BodyPose } from "./mannequin";

/** 膝をわずかに緩めたAポーズ。保存用の練習ルートとは独立した形状確認用。 */
export function neutralModelPose(): BodyPose {
  return solveBody({
    pelvis: new Vector3(0, 0.855, 0), up: new Vector3(0, 1, 0), front: new Vector3(0, 0, 1),
    arms: {
      L: { end: new Vector3(0.428, 0.806, 0.012), pole: new Vector3(0.38, 1.04, -0.03) },
      R: { end: new Vector3(-0.428, 0.806, 0.012), pole: new Vector3(-0.38, 1.04, -0.03) },
    },
    legs: {
      L: { end: new Vector3(0.14, 0.055, 0), pole: new Vector3(0.14, 0.46, 0.15) },
      R: { end: new Vector3(-0.14, 0.055, 0), pole: new Vector3(-0.14, 0.46, 0.15) },
    },
  });
}
