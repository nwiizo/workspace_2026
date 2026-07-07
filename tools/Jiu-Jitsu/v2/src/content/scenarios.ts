import type { Scenario, ScenarioId } from "./types";
import { backDefense } from "./scenarios/backDefense";
import { mountEscape } from "./scenarios/mountEscape";
import { sideEscape } from "./scenarios/sideEscape";
import { closedGuardPosture } from "./scenarios/closedGuardPosture";
import { attackFromMount } from "./scenarios/attackFromMount";
import { attackFromBack } from "./scenarios/attackFromBack";
import { attackFromSide } from "./scenarios/attackFromSide";
import { attackArmbarGuard } from "./scenarios/attackArmbarGuard";
import { attackTriangleGuard } from "./scenarios/attackTriangleGuard";

export const SCENARIOS: Scenario[] = [
  backDefense,
  mountEscape,
  sideEscape,
  closedGuardPosture,
  attackFromMount,
  attackFromBack,
  attackFromSide,
  attackArmbarGuard,
  attackTriangleGuard,
];

const BY_ID = new Map<ScenarioId, Scenario>(SCENARIOS.map((s) => [s.id, s]));

export function scenarioById(id: ScenarioId): Scenario {
  const s = BY_ID.get(id);
  if (!s) throw new Error(`unknown scenario: ${id}`);
  return s;
}
