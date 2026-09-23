/** 説明用の静力学・平面リンクモデル。人体の強度や安全性は計算しない。 */
export function momentArm(distanceCm: number, forearmAngleDeg: number): number {
  return distanceCm * Math.abs(Math.cos(forearmAngleDeg * Math.PI / 180));
}

export function supportMargin(widthCm: number, centerOffsetCm: number): number {
  return widthCm / 2 - Math.abs(centerOffsetCm);
}

export function legPoints(hipDeg: number, kneeDeg: number): {
  knee: { x: number; y: number };
  ankle: { x: number; y: number };
} {
  const hip = hipDeg * Math.PI / 180;
  const shin = (hipDeg - kneeDeg) * Math.PI / 180;
  const knee = { x: 40 * Math.cos(hip), y: 40 * Math.sin(hip) };
  return { knee, ankle: { x: knee.x + 40 * Math.cos(shin), y: knee.y + 40 * Math.sin(shin) } };
}

export type BodyExperiment = "lever" | "base" | "leg";

export function movementTarget(experiment: BodyExperiment, values: readonly [number, number]): boolean {
  if (experiment === "lever") return momentArm(values[0], values[1]) <= 12;
  if (experiment === "base") return supportMargin(values[0], values[1]) >= 10;
  const p = legPoints(values[0], values[1]);
  return Math.abs(p.knee.x) <= 20 && p.knee.y >= 30 && p.ankle.x >= 0 && p.ankle.x <= 55 && p.ankle.y >= 15 && p.ankle.y <= 50;
}
