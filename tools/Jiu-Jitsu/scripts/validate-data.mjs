import fs from "node:fs";
import { jointLimitViolations } from "../js/anatomy.js";
import { POSES } from "../js/poses.js";
import { POSE_SPECS, POSE_SPEC_ROLES } from "../js/poseSpecs.js";
import {
  BJJ_POSITION_FAMILIES,
  rolePairAllowed,
  rolePairRule,
  roleSpec,
} from "../js/positionCatalog.js";
import { OFFENSE_SCENARIOS, SCENARIOS } from "../js/techniques.js";

const poses = fs.readFileSync("js/poses.js", "utf8");
const techniques = fs.readFileSync("js/techniques.js", "utf8");
const game = fs.readFileSync("js/game.js", "utf8");
const fighter = fs.readFileSync("js/fighter.js", "utf8");
const audit = fs.readFileSync("_audit.html", "utf8");

const poseIds = [...poses.matchAll(/^  ([A-Za-z0-9_]+): P\(/gm)].map((m) => m[1]);
const poseSet = new Set(poseIds);
const poseSpecIds = Object.keys(POSE_SPECS);
const poseSpecSet = new Set(poseSpecIds);

const refs = [...techniques.matchAll(/(?:red|blue): "([A-Za-z0-9_]+)"/g)].map((m) => m[1]);
refs.push(...[...game.matchAll(/_pose\("([A-Za-z0-9_]+)"\)/g)].map((m) => m[1]));

const missing = [...new Set(refs.filter((id) => !poseSet.has(id)))].sort();
const missingPoseSpecs = poseIds.filter((id) => !poseSpecSet.has(id));
const extraPoseSpecs = poseSpecIds.filter((id) => !poseSet.has(id));
const missingReferencedPoseSpecs = [...new Set(refs.filter((id) => !poseSpecSet.has(id)))].sort();
const validPoseSpec = ([id, spec]) =>
  POSE_SPEC_ROLES.has(spec.role) &&
  spec.support &&
  typeof spec.support.base === "string" &&
  Number.isFinite(spec.support.radius) &&
  spec.support.radius >= 0.2 &&
  spec.support.radius <= 0.5 &&
  Array.isArray(spec.support.offset) &&
  spec.support.offset.length === 2 &&
  spec.support.offset.every((n) => Number.isFinite(n) && Math.abs(n) <= 0.25) &&
  typeof spec.support.load === "string" &&
  Array.isArray(spec.contacts) &&
  spec.contacts.length >= 1 &&
  spec.contacts.length <= 5 &&
  spec.contacts.every((contact) => typeof contact === "string" && contact.length >= 1) &&
  typeof spec.force === "string" &&
  spec.force.length >= 4 &&
  Array.isArray(spec.vector) &&
  spec.vector.length === 2 &&
  spec.vector.every((n) => Number.isFinite(n) && Math.abs(n) <= 0.5) &&
  Boolean(POSES[id]);
const invalidPoseSpecs = Object.entries(POSE_SPECS)
  .filter((entry) => !validPoseSpec(entry))
  .map(([id]) => id);
const knownPositionIds = new Set(BJJ_POSITION_FAMILIES.map((position) => position.id));
const implementedPositionIds = new Set(
  BJJ_POSITION_FAMILIES.filter((position) => position.implemented).map((position) => position.id),
);
const invalidPositionCatalog = BJJ_POSITION_FAMILIES
  .filter((position) =>
    typeof position.id !== "string" ||
    position.id.length < 3 ||
    typeof position.labelJp !== "string" ||
    position.labelJp.length < 2 ||
    typeof position.implemented !== "boolean",
  )
  .map((position) => position.id || String(position));
const invalidPoseRoleCatalog = Object.entries(POSE_SPECS)
  .filter(([, spec]) => {
    const role = roleSpec(spec.role);
    return !role || !knownPositionIds.has(role.family) || !implementedPositionIds.has(role.family);
  })
  .map(([id, spec]) => `${id}:${spec.role}:${roleSpec(spec.role)?.family || "unknown"}`);

const rootPos = (id) => POSES[id]?.root?.pos;
const finiteTuple = (value, length) =>
  Array.isArray(value) &&
  value.length === length &&
  value.every((n) => Number.isFinite(n));
const roleHeightRanges = {
  standing: [0.78, 1.06],
  "seated-front": [0.30, 0.52],
  "back-control-top": [0.30, 0.52],
  "back-defense-bottom": [0.30, 0.52],
  "submitted-bottom": [0.30, 0.52],
  "supine-bottom": [0.10, 0.24],
  "closed-guard-bottom": [0.10, 0.24],
  "open-guard-bottom": [0.10, 0.24],
  "side-control-bottom": [0.10, 0.24],
  "shrimp-bottom": [0.10, 0.24],
  "supine-bottom-arm-isolated": [0.10, 0.24],
  "prone-bottom": [0.20, 0.34],
  "mount-top": [0.44, 0.64],
  "mount-top-attacking-arm": [0.44, 0.64],
  "guard-top-after-sweep": [0.40, 0.58],
  "closed-guard-top": [0.38, 0.56],
  "guard-pass-top": [0.34, 0.50],
  "side-control-top": [0.20, 0.34],
  "armbar-attacker": [0.26, 0.42],
  "armbar-defender": [0.10, 0.24],
  "guard-armbar-attacker": [0.10, 0.24],
  "guard-armbar-defender": [0.20, 0.36],
  "triangle-attacker": [0.10, 0.24],
  "triangle-defender": [0.22, 0.38],
};
const invalidPoseGeometry = Object.entries(POSES)
  .filter(([id, pose]) => {
    const pos = pose?.root?.pos;
    const rot = pose?.root?.rot;
    const role = POSE_SPECS[id]?.role;
    const range = roleHeightRanges[role];
    if (!finiteTuple(pos, 3) || !finiteTuple(rot, 3) || !range) return true;
    const [x, y, z] = pos;
    return Math.abs(x) > 0.75 || Math.abs(z) > 0.75 || y < range[0] || y > range[1];
  })
  .map(([id]) => {
    const pos = POSES[id]?.root?.pos;
    const role = POSE_SPECS[id]?.role;
    return `${id}:${role}:pos=${JSON.stringify(pos)}`;
  });
const invalidJointLimits = Object.entries(POSES)
  .flatMap(([poseId, pose]) =>
    Object.entries(pose.joints || {}).flatMap(([joint, rot]) =>
      finiteTuple(rot, 3) ? jointLimitViolations(poseId, joint, rot) : []),
  );
const invalidRuntimeAnatomy = [
  {
    id: "runtime-joint-clamp",
    valid: fighter.includes("clampJointEuler(name"),
  },
  {
    id: "runtime-grounding-solver",
    valid: fighter.includes("_solveGroundedRootTarget") && fighter.includes("_floorProbeMinY"),
  },
  {
    id: "knee-hinge-shared-limits",
    valid: fs.readFileSync("js/anatomy.js", "utf8").includes("{ pattern: /^shin[LR]$/, x: [0, 150]"),
  },
]
  .filter((check) => !check.valid)
  .map((check) => check.id);
const topLikeRoles = new Set([
  "back-control-top",
  "mount-top",
  "mount-top-attacking-arm",
  "guard-top-after-sweep",
  "closed-guard-top",
  "guard-pass-top",
  "side-control-top",
  "armbar-attacker",
  "guard-armbar-attacker",
  "triangle-attacker",
]);
const bottomLikeRoles = new Set([
  "seated-front",
  "back-defense-bottom",
  "submitted-bottom",
  "supine-bottom",
  "closed-guard-bottom",
  "open-guard-bottom",
  "side-control-bottom",
  "shrimp-bottom",
  "supine-bottom-arm-isolated",
  "armbar-defender",
  "guard-armbar-defender",
  "prone-bottom",
  "triangle-defender",
]);
const pairEntries = [];
const scenarios = [...SCENARIOS, ...OFFENSE_SCENARIOS];
for (const scenario of scenarios) {
  pairEntries.push([scenario.id, "setup", scenario.setup.red, scenario.setup.blue]);
  pairEntries.push([scenario.id, "attack", scenario.attack.red, scenario.attack.blue]);
  for (const action of scenario.opponentActions || []) {
    if (action.attack) {
      pairEntries.push([scenario.id, `action:${action.id}`, action.attack.red, action.attack.blue]);
    }
  }
  for (const [index, option] of scenario.options.entries()) {
    pairEntries.push([scenario.id, `option${index}`, option.result.red, option.result.blue]);
  }
}
const invalidPosePairRoles = pairEntries
  .filter(([, , redId, blueId]) => {
    const redRole = POSE_SPECS[redId]?.role;
    const blueRole = POSE_SPECS[blueId]?.role;
    if (!redRole || !blueRole) return true;
    const hasTop = topLikeRoles.has(redRole) || topLikeRoles.has(blueRole);
    const hasBottom = bottomLikeRoles.has(redRole) || bottomLikeRoles.has(blueRole);
    return !hasTop || !hasBottom;
  })
  .map(([scenarioId, stage, redId, blueId]) => `${scenarioId}:${stage}:${redId}/${blueId}`);
const invalidExplicitRolePairs = pairEntries
  .filter(([, stage, redId, blueId]) => {
    const redRole = POSE_SPECS[redId]?.role;
    const blueRole = POSE_SPECS[blueId]?.role;
    if (!redRole || !blueRole) return true;
    if (stage !== "standing" && (redRole === "standing" || blueRole === "standing")) return true;
    return !rolePairAllowed(redRole, blueRole);
  })
  .map(([scenarioId, stage, redId, blueId]) => {
    const redRole = POSE_SPECS[redId]?.role;
    const blueRole = POSE_SPECS[blueId]?.role;
    return `${scenarioId}:${stage}:${redId}/${blueId}:${redRole}/${blueRole}`;
  });

const elevatedTopRules = {
  "mount-top": [0.28, 0.50],
  "mount-top-attacking-arm": [0.28, 0.50],
  "guard-top-after-sweep": [0.18, 0.42],
  "closed-guard-top": [0.20, 0.42],
  "guard-pass-top": [0.16, 0.38],
  "side-control-top": [0.04, 0.18],
  "armbar-attacker": [0.10, 0.30],
};
const closeRangeForPair = (redRole, blueRole) => {
  const roles = new Set([redRole, blueRole]);
  if (roles.has("mount-top") || roles.has("mount-top-attacking-arm")) return 0.34;
  if (roles.has("side-control-top") || roles.has("shrimp-bottom")) return 0.42;
  if (roles.has("back-control-top")) return 0.45;
  return 0.55;
};
const heightRelationshipValid = (redRole, blueRole, redY, blueY) => {
  const redRule = elevatedTopRules[redRole];
  if (redRule && bottomLikeRoles.has(blueRole)) {
    const rise = redY - blueY;
    return rise >= redRule[0] && rise <= redRule[1];
  }
  const blueRule = elevatedTopRules[blueRole];
  if (blueRule && bottomLikeRoles.has(redRole)) {
    const rise = blueY - redY;
    return rise >= blueRule[0] && rise <= blueRule[1];
  }
  if (redRole === "back-control-top" || blueRole === "back-control-top") {
    return Math.abs(redY - blueY) <= 0.12;
  }
  if (redRole === "guard-armbar-attacker" && blueRole === "guard-armbar-defender") {
    const defenderRise = blueY - redY;
    return defenderRise >= 0.05 && defenderRise <= 0.22;
  }
  if (blueRole === "guard-armbar-attacker" && redRole === "guard-armbar-defender") {
    const defenderRise = redY - blueY;
    return defenderRise >= 0.05 && defenderRise <= 0.22;
  }
  if (redRole === "triangle-attacker" && blueRole === "triangle-defender") {
    const defenderRise = blueY - redY;
    return defenderRise >= 0.05 && defenderRise <= 0.24;
  }
  if (blueRole === "triangle-attacker" && redRole === "triangle-defender") {
    const defenderRise = redY - blueY;
    return defenderRise >= 0.05 && defenderRise <= 0.24;
  }
  return true;
};
const invalidPosePairGeometry = pairEntries
  .filter(([, , redId, blueId]) => {
    const redPos = rootPos(redId);
    const bluePos = rootPos(blueId);
    const redRole = POSE_SPECS[redId]?.role;
    const blueRole = POSE_SPECS[blueId]?.role;
    if (!finiteTuple(redPos, 3) || !finiteTuple(bluePos, 3) || !redRole || !blueRole) return true;
    const horizontalDistance = Math.hypot(redPos[0] - bluePos[0], redPos[2] - bluePos[2]);
    if (horizontalDistance > closeRangeForPair(redRole, blueRole)) return true;
    return !heightRelationshipValid(redRole, blueRole, redPos[1], bluePos[1]);
  })
  .map(([scenarioId, stage, redId, blueId]) => {
    const redPos = rootPos(redId);
    const bluePos = rootPos(blueId);
    const dxz = finiteTuple(redPos, 3) && finiteTuple(bluePos, 3)
      ? Math.hypot(redPos[0] - bluePos[0], redPos[2] - bluePos[2]).toFixed(2)
      : "n/a";
    const dy = finiteTuple(redPos, 3) && finiteTuple(bluePos, 3)
      ? (redPos[1] - bluePos[1]).toFixed(2)
      : "n/a";
    return `${scenarioId}:${stage}:${redId}/${blueId}:xz=${dxz}:dy=${dy}`;
  });
const rotateX = ([x, y, z], a) => [
  x,
  y * Math.cos(a) - z * Math.sin(a),
  y * Math.sin(a) + z * Math.cos(a),
];
const rotateY = ([x, y, z], a) => [
  x * Math.cos(a) + z * Math.sin(a),
  y,
  -x * Math.sin(a) + z * Math.cos(a),
];
const rotateZ = ([x, y, z], a) => [
  x * Math.cos(a) - y * Math.sin(a),
  x * Math.sin(a) + y * Math.cos(a),
  z,
];
const rotateVector = (vector, rot) => {
  const afterX = rotateX(vector, rot[0]);
  const afterY = rotateY(afterX, rot[1]);
  return rotateZ(afterY, rot[2]);
};
const horizontalUnit = ([x, , z]) => {
  const length = Math.hypot(x, z);
  if (length < 0.001) return null;
  return [x / length, z / length];
};
const direction = (id, localVector) => {
  const rot = POSES[id]?.root?.rot;
  if (!finiteTuple(rot, 3)) return null;
  return horizontalUnit(rotateVector(localVector, rot));
};
const faceDir = (id) => direction(id, [0, 0, 1]);
const headDir = (id) => direction(id, [0, 1, 0]);
const deltaDir = (fromId, toId) => {
  const from = rootPos(fromId);
  const to = rootPos(toId);
  if (!finiteTuple(from, 3) || !finiteTuple(to, 3)) return null;
  return horizontalUnit([to[0] - from[0], 0, to[2] - from[2]]);
};
const dot2 = (a, b) => (a && b ? a[0] * b[0] + a[1] * b[1] : Number.NaN);
const pairOrientationValid = (topId, bottomId, topRole, bottomRole) => {
  const topFace = faceDir(topId);
  const bottomFace = faceDir(bottomId);
  const bottomHead = headDir(bottomId);
  const bottomToTop = deltaDir(bottomId, topId);
  const roles = new Set([topRole, bottomRole]);

  if (topRole === "back-control-top") {
    return dot2(topFace, bottomFace) >= 0.78 && dot2(bottomToTop, bottomFace) <= -0.45;
  }

  if (topRole === "mount-top" || topRole === "mount-top-attacking-arm") {
    return dot2(topFace, bottomHead) >= 0.65 && dot2(bottomToTop, bottomHead) >= 0.35;
  }

  if (
    topRole === "closed-guard-top" ||
    topRole === "guard-pass-top" ||
    topRole === "guard-top-after-sweep"
  ) {
    return dot2(topFace, bottomHead) >= 0.45;
  }

  if (topRole === "side-control-top") {
    return Math.abs(dot2(topFace, bottomHead)) <= 0.45;
  }

  if (topRole === "guard-armbar-attacker" || topRole === "triangle-attacker") {
    const topPos = rootPos(topId);
    const bottomPos = rootPos(bottomId);
    return (
      finiteTuple(topPos, 3) &&
      finiteTuple(bottomPos, 3) &&
      Math.abs(topPos[0] - bottomPos[0]) >= 0.08 &&
      Math.abs(dot2(topFace, bottomHead)) <= 0.75
    );
  }

  if (roles.has("standing")) return dot2(faceDir(topId), faceDir(bottomId)) <= -0.65;
  return true;
};
const invalidPosePairOrientation = pairEntries
  .filter(([, , redId, blueId]) => {
    const redRole = POSE_SPECS[redId]?.role;
    const blueRole = POSE_SPECS[blueId]?.role;
    if (!redRole || !blueRole) return true;
    if (topLikeRoles.has(redRole) && bottomLikeRoles.has(blueRole)) {
      return !pairOrientationValid(redId, blueId, redRole, blueRole);
    }
    if (topLikeRoles.has(blueRole) && bottomLikeRoles.has(redRole)) {
      return !pairOrientationValid(blueId, redId, blueRole, redRole);
    }
    return false;
  })
  .map(([scenarioId, stage, redId, blueId]) => {
    const redRole = POSE_SPECS[redId]?.role;
    const blueRole = POSE_SPECS[blueId]?.role;
    return `${scenarioId}:${stage}:${redId}/${blueId}:${redRole}/${blueRole}`;
  });
const invalidStandingOrientation = (() => {
  const redToBlue = deltaDir("standingRed", "standingBlue");
  const blueToRed = deltaDir("standingBlue", "standingRed");
  const redFacesBlue = dot2(faceDir("standingRed"), redToBlue);
  const blueFacesRed = dot2(faceDir("standingBlue"), blueToRed);
  const redPos = rootPos("standingRed");
  const bluePos = rootPos("standingBlue");
  const sameLine = finiteTuple(redPos, 3) && finiteTuple(bluePos, 3) && Math.abs(redPos[2] - bluePos[2]) <= 0.08;
  return redFacesBlue >= 0.82 && blueFacesRed >= 0.82 && sameLine
    ? []
    : [`standingRed/standingBlue:red=${redFacesBlue.toFixed(2)}:blue=${blueFacesRed.toFixed(2)}`];
})();
const standingPairRule = rolePairRule("standing", "standing");
const invalidStandingRolePair = standingPairRule ? [] : ["standing/standing"];
const supportCenter = (id) => {
  const pos = rootPos(id);
  const offset = POSE_SPECS[id]?.support?.offset;
  if (!finiteTuple(pos, 3) || !finiteTuple(offset, 2)) return null;
  return [pos[0] + offset[0], pos[2] + offset[1]];
};
const loadBearingValid = (loadId, baseId) => {
  const spec = POSE_SPECS[loadId];
  const center = supportCenter(loadId);
  const basePos = rootPos(baseId);
  if (!spec || !center || !finiteTuple(basePos, 3)) return false;
  if (!spec.support.load.includes("opponent")) return true;
  const distance = Math.hypot(center[0] - basePos[0], center[1] - basePos[2]);
  return distance <= spec.support.radius + 0.08;
};
const forceVectorValid = (loadId, baseId) => {
  const spec = POSE_SPECS[loadId];
  const vector = spec?.vector;
  const center = supportCenter(loadId);
  const basePos = rootPos(baseId);
  if (!spec?.support?.load.includes("opponent")) return true;
  if (!finiteTuple(vector, 2) || !center || !finiteTuple(basePos, 3)) return false;
  const force = horizontalUnit([vector[0], 0, vector[1]]);
  const towardOpponent = horizontalUnit([basePos[0] - center[0], 0, basePos[2] - center[1]]);
  return dot2(force, towardOpponent) >= 0.55;
};
const invalidPosePairSupport = pairEntries
  .filter(([, , redId, blueId]) => {
    const redRole = POSE_SPECS[redId]?.role;
    const blueRole = POSE_SPECS[blueId]?.role;
    if (!redRole || !blueRole) return true;
    if (topLikeRoles.has(redRole) && bottomLikeRoles.has(blueRole)) {
      return !loadBearingValid(redId, blueId);
    }
    if (topLikeRoles.has(blueRole) && bottomLikeRoles.has(redRole)) {
      return !loadBearingValid(blueId, redId);
    }
    return false;
  })
  .map(([scenarioId, stage, redId, blueId]) => {
    const redCenter = supportCenter(redId);
    const blueCenter = supportCenter(blueId);
    return `${scenarioId}:${stage}:${redId}/${blueId}:support=${JSON.stringify({ redCenter, blueCenter })}`;
  });
const invalidPosePairForce = pairEntries
  .filter(([, , redId, blueId]) => {
    const redRole = POSE_SPECS[redId]?.role;
    const blueRole = POSE_SPECS[blueId]?.role;
    if (!redRole || !blueRole) return true;
    if (topLikeRoles.has(redRole) && bottomLikeRoles.has(blueRole)) {
      return !forceVectorValid(redId, blueId);
    }
    if (topLikeRoles.has(blueRole) && bottomLikeRoles.has(redRole)) {
      return !forceVectorValid(blueId, redId);
    }
    return false;
  })
  .map(([scenarioId, stage, redId, blueId]) => {
    const redRole = POSE_SPECS[redId]?.role;
    const blueRole = POSE_SPECS[blueId]?.role;
    const forceId = topLikeRoles.has(redRole) ? redId : blueId;
    const baseId = forceId === redId ? blueId : redId;
    return `${scenarioId}:${stage}:${forceId}->${baseId}:vector=${JSON.stringify(POSE_SPECS[forceId]?.vector)}`;
  });
const auditBiomechanicsMarkers = [
  {
    id: "support-centered-force-arrow",
    valid: /new THREE\.Vector3\(supportX,\s*0\.1,\s*supportZ\)/.test(audit),
  },
  {
    id: "load-bearing-opponent-line",
    valid: audit.includes("spec?.support?.load?.includes(\"opponent\")") &&
      audit.includes("opponentPose?.root?.pos"),
  },
  {
    id: "support-and-opponent-load-points",
    valid: audit.includes("addPoint(scene, supportX") && audit.includes("addPoint(scene, opponentX"),
  },
];
const invalidAuditBiomechanics = auditBiomechanicsMarkers
  .filter((marker) => !marker.valid)
  .map((marker) => marker.id);
const scenarioIds = scenarios.map((scenario) => scenario.id);
const duplicateScenarioIds = scenarioIds.filter((id, i) => scenarioIds.indexOf(id) !== i);
const invalidRoles = scenarios
  .filter((scenario) => scenario.role !== "defense" && scenario.role !== "offense")
  .map((scenario) => scenario.id);

const stateEffectsFrom = (option) => [
  ...(option.stateEffects?.add || []),
  ...(option.stateEffects?.remove || []),
];
const stateRequirementsFrom = (option) => [
  ...(option.requiresState || []),
  ...(option.forbiddenState || []),
];
const validStateList = (value) =>
  value === undefined || (
    Array.isArray(value) &&
    value.length > 0 &&
    value.every((flag) => typeof flag === "string" && flag.length >= 3 && flag.length <= 24)
  );
const stateFlags = [...new Set(scenarios.flatMap((scenario) =>
  scenario.options.flatMap((option) => [...stateEffectsFrom(option), ...stateRequirementsFrom(option)]),
))].sort();
const stateFlagSet = new Set(stateFlags);
const scenariosWithStateBias = scenarios.filter((scenario) => Array.isArray(scenario.stateBias));
const invalidStateBiasScenarios = scenarios
  .filter((scenario) =>
    !validStateList(scenario.stateBias) ||
    (scenario.stateBias || []).some((flag) => !stateFlagSet.has(flag)),
  )
  .map((scenario) => scenario.id);
const stateVariants = [new Set(), ...stateFlags.map((flag) => new Set([flag]))];
const stateMatches = (option, state) =>
  (option.requiresState || []).every((flag) => state.has(flag)) &&
  (option.forbiddenState || []).every((flag) => !state.has(flag));
const validActionList = (value) =>
  value === undefined || (
    Array.isArray(value) &&
    value.length > 0 &&
    value.every((id) => typeof id === "string" && id.length >= 3 && id.length <= 40)
  );
const actionMatches = (option, actionId) =>
  (!(option.requiresAction || []).length || option.requiresAction.includes(actionId)) &&
  (option.forbiddenAction || []).every((id) => id !== actionId);
const validStateEffects = (effects) =>
  effects === undefined || (
    effects &&
    typeof effects === "object" &&
    validStateList(effects.add) &&
      validStateList(effects.remove)
  );
const visibleOptions = (scenario, mode, state = new Set(), actionId = undefined) =>
  scenario.options.filter((option) => {
    if (option.giOnly && mode !== "gi") return false;
    if (option.nogiOnly && mode !== "nogi") return false;
    if (!stateMatches(option, state)) return false;
    if (!actionMatches(option, actionId)) return false;
    return true;
  });
const invalidCorrectCounts = scenarios
  .flatMap((scenario) =>
    ["gi", "nogi"].flatMap((mode) =>
      stateVariants.flatMap((state) => {
        const actionVariants = [undefined, ...(scenario.opponentActions || []).map((action) => action.id)];
        return actionVariants
          .filter((actionId) =>
            visibleOptions(scenario, mode, state, actionId).filter((option) => option.correct).length !== 1,
          )
          .map((actionId) =>
            `${scenario.id}:${mode}:state=${[...state].join("+") || "none"}:action=${actionId || "none"}`,
          );
      }),
    ),
  );
const statefulChoices = scenarios.flatMap((scenario) =>
  scenario.options.filter((option) => option.stateEffects || option.requiresState || option.forbiddenState),
);
const actionGatedChoices = scenarios.flatMap((scenario) =>
  scenario.options.filter((option) => option.requiresAction || option.forbiddenAction),
);
const scenariosWithActionGates = scenarios.filter((scenario) =>
  scenario.options.some((option) => option.requiresAction || option.forbiddenAction),
);
const invalidStateChoices = scenarios.flatMap((scenario) =>
  scenario.options
    .filter((option) =>
      !validStateEffects(option.stateEffects) ||
      !validStateList(option.requiresState) ||
      !validStateList(option.forbiddenState),
    )
    .map((option) => `${scenario.id}:${option.jp}`),
);
const invalidActionChoices = scenarios.flatMap((scenario) => {
  const actionIds = new Set((scenario.opponentActions || []).map((action) => action.id));
  return scenario.options
    .filter((option) =>
      !validActionList(option.requiresAction) ||
      !validActionList(option.forbiddenAction) ||
      (option.requiresAction || []).some((id) => !actionIds.has(id)) ||
      (option.forbiddenAction || []).some((id) => !actionIds.has(id)),
    )
    .map((option) => `${scenario.id}:${option.jp}`);
});
const scenariosWithUniformBranch = scenarios.filter((scenario) =>
  scenario.options.some((option) => option.giOnly || option.nogiOnly),
);
const nextEntry = (entry) => {
  if (typeof entry === "string") return { id: entry, weight: 1, valid: true };
  if (!entry || typeof entry !== "object") return { id: String(entry), weight: 0, valid: false };
  const weight = entry.weight ?? 1;
  return {
    id: entry.id,
    weight,
    valid: typeof entry.id === "string" && Number.isFinite(weight) && weight > 0,
  };
};
const nextEntries = (option) => (option.next || []).map(nextEntry);
const invalidTimeLimits = scenarios
  .filter(
    (scenario) =>
      !Number.isInteger(scenario.timeLimitSec) ||
      scenario.timeLimitSec < 5 ||
      scenario.timeLimitSec > 12,
  )
  .map((scenario) => scenario.id);
const invalidPressure = scenarios
  .filter(
    (scenario) =>
      !scenario.pressure ||
      typeof scenario.pressure.early !== "string" ||
      typeof scenario.pressure.urgent !== "string" ||
      scenario.pressure.early.length < 8 ||
      scenario.pressure.urgent.length < 8,
  )
  .map((scenario) => scenario.id);
const invalidReadCues = scenarios
  .filter(
    (scenario) =>
      !Array.isArray(scenario.readCues) ||
      scenario.readCues.length < 2 ||
      scenario.readCues.length > 4 ||
      scenario.readCues.some((cue) => typeof cue !== "string" || cue.length < 1 || cue.length > 12),
  )
  .map((scenario) => scenario.id);
const validStyleIds = new Set(["pressure-passer", "choke-hunter", "guard-player"]);
const validTacticIds = new Set(["survive-first", "position-ladder", "submission-chain", "fast-scramble"]);
const actionAttackOverrides = scenarios.flatMap((scenario) =>
  (scenario.opponentActions || []).filter((action) => action.attack),
);
const scenariosWithActionVariation = scenarios.filter(
  (scenario) => (scenario.opponentActions || []).length >= 3,
);
const invalidOpponentActions = scenarios
  .filter(
    (scenario) =>
      !Array.isArray(scenario.opponentActions) ||
      scenario.opponentActions.length < 2 ||
      scenario.opponentActions.some((action) => {
        const attackValid = !action.attack || (
          typeof action.attack.red === "string" &&
          typeof action.attack.blue === "string" &&
          typeof action.attack.badge === "string" &&
          poseSet.has(action.attack.red) &&
          poseSet.has(action.attack.blue) &&
          action.attack.badge.length >= 4
        );
        const pressureValid = !action.pressure || (
          typeof action.pressure.early === "string" &&
          typeof action.pressure.urgent === "string" &&
          action.pressure.early.length >= 8 &&
          action.pressure.urgent.length >= 8
        );
        const readCuesValid = !action.readCues || (
          Array.isArray(action.readCues) &&
          action.readCues.length >= 2 &&
          action.readCues.length <= 4 &&
          action.readCues.every((cue) => typeof cue === "string" && cue.length >= 1 && cue.length <= 12)
        );
        const stylesValid = !action.styles || action.styles.every((id) => validStyleIds.has(id));
        const tacticsValid = !action.tactics || action.tactics.every((id) => validTacticIds.has(id));
        return (
          typeof action.id !== "string" ||
          typeof action.label !== "string" ||
          action.label.length < 2 ||
          typeof action.cue !== "string" ||
          action.cue.length < 12 ||
          action.cue.length > 80 ||
          !action.attack ||
          (action.weight !== undefined && (!Number.isFinite(action.weight) || action.weight <= 0)) ||
          !attackValid ||
          !pressureValid ||
          !readCuesValid ||
          !stylesValid ||
          !tacticsValid
        );
      }),
  )
  .map((scenario) => scenario.id);
const scenarioIdSet = new Set(scenarios.map((scenario) => scenario.id));
const invalidNextCounts = scenarios
  .flatMap((scenario) =>
    ["gi", "nogi"]
      .flatMap((mode) => visibleOptions(scenario, mode).filter((option) => option.correct))
      .filter((option) => !Array.isArray(option.next) || option.next.length === 0)
      .map((option) => `${scenario.id}:${option.giOnly ? "gi" : option.nogiOnly ? "nogi" : "all"}`),
  );
const invalidConsequenceNextCounts = scenarios
  .flatMap((scenario) =>
    ["gi", "nogi"]
      .flatMap((mode) => visibleOptions(scenario, mode).filter((option) => !option.correct))
      .filter((option) => !Array.isArray(option.next) || option.next.length === 0)
      .map((option) => `${scenario.id}:${option.giOnly ? "gi" : option.nogiOnly ? "nogi" : "all"}`),
  );
const invalidNextRefs = scenarios
  .flatMap((scenario) =>
    scenario.options.flatMap((option) =>
      nextEntries(option)
        .filter((entry) => entry.valid)
        .filter((entry) => !scenarioIdSet.has(entry.id))
        .map((entry) => `${scenario.id}->${entry.id}`),
    ),
  );
const invalidNextShapes = scenarios
  .flatMap((scenario) =>
    scenario.options.flatMap((option) =>
      nextEntries(option)
        .filter((entry) => !entry.valid)
        .map((entry) => `${scenario.id}->${entry.id}`),
    ),
  );
const weightedNextEntries = scenarios
  .flatMap((scenario) => scenario.options.flatMap((option) => nextEntries(option)))
  .filter((entry) => entry.valid && entry.weight !== 1);
const invalidReactions = scenarios
  .flatMap((scenario) =>
    ["gi", "nogi"]
      .flatMap((mode) => visibleOptions(scenario, mode).filter((option) => option.correct))
      .filter((option) => typeof option.reaction !== "string" || option.reaction.length < 12)
      .map((option) => `${scenario.id}:${option.giOnly ? "gi" : option.nogiOnly ? "nogi" : "all"}`),
  );
const invalidConsequences = scenarios
  .flatMap((scenario) =>
    ["gi", "nogi"]
      .flatMap((mode) => visibleOptions(scenario, mode).filter((option) => !option.correct))
      .filter((option) => typeof option.consequence !== "string" || option.consequence.length < 12)
      .map((option) => `${scenario.id}:${option.giOnly ? "gi" : option.nogiOnly ? "nogi" : "all"}`),
  );
const invalidStyleRefs = [...game.matchAll(/preferred:\s*\[([^\]]*)\]/g)]
  .flatMap(([, body]) => [...body.matchAll(/"([^"]+)"/g)].map((m) => m[1]))
  .filter((id) => !scenarioIdSet.has(id));
const missionBlocks = [...game.matchAll(/\{\s*id:\s*"[^"]+",\s*label:\s*"[^"]+",\s*text:\s*"[^"]+",\s*modes:\s*\[([^\]]+)\],\s*bonus:\s*(\d+),\s*target:\s*\{[^}]+\},\s*\}/g)];
const validModes = new Set(["mixed", "defense", "offense"]);
const invalidMissionModes = missionBlocks
  .flatMap(([, body]) => [...body.matchAll(/"([^"]+)"/g)].map((m) => m[1]))
  .filter((mode) => !validModes.has(mode));
const invalidMissionBonuses = missionBlocks
  .map(([, , bonus]) => Number(bonus))
  .filter((bonus) => !Number.isInteger(bonus) || bonus <= 0 || bonus > 20);
const tacticBlocks = [...game.matchAll(/\{\s*id:\s*"[^"]+",\s*label:\s*"[^"]+",\s*text:\s*"[^"]+",\s*modes:\s*\[([^\]]+)\],\s*timeDelta:\s*(-?\d+),/g)];
const invalidTacticModes = tacticBlocks
  .flatMap(([, body]) => [...body.matchAll(/"([^"]+)"/g)].map((m) => m[1]))
  .filter((mode) => !validModes.has(mode));
const invalidTacticTimeDeltas = tacticBlocks
  .map(([, , timeDelta]) => Number(timeDelta))
  .filter((timeDelta) => !Number.isInteger(timeDelta) || timeDelta < -2 || timeDelta > 2);

if (
  missing.length ||
  missingPoseSpecs.length ||
  extraPoseSpecs.length ||
  missingReferencedPoseSpecs.length ||
  invalidPoseSpecs.length ||
  invalidPositionCatalog.length ||
  invalidPoseRoleCatalog.length ||
  invalidPoseGeometry.length ||
  invalidJointLimits.length ||
  invalidPosePairRoles.length ||
  invalidExplicitRolePairs.length ||
  invalidPosePairGeometry.length ||
  invalidPosePairOrientation.length ||
  invalidStandingOrientation.length ||
  invalidStandingRolePair.length ||
  invalidPosePairSupport.length ||
  invalidPosePairForce.length ||
  invalidAuditBiomechanics.length ||
  duplicateScenarioIds.length ||
  invalidRoles.length ||
  invalidCorrectCounts.length ||
  statefulChoices.length < 8 ||
  invalidStateChoices.length ||
  actionGatedChoices.length < 18 ||
  scenariosWithActionGates.length < 6 ||
  invalidActionChoices.length ||
  scenariosWithStateBias.length < 6 ||
  invalidStateBiasScenarios.length ||
  scenariosWithUniformBranch.length === 0 ||
  invalidTimeLimits.length ||
  invalidPressure.length ||
  invalidReadCues.length ||
  actionAttackOverrides.length < scenarios.length * 2 ||
  scenariosWithActionVariation.length < 5 ||
  invalidOpponentActions.length ||
  invalidNextCounts.length ||
  invalidConsequenceNextCounts.length ||
  invalidNextRefs.length ||
  invalidNextShapes.length ||
  invalidReactions.length ||
  invalidConsequences.length ||
  invalidStyleRefs.length ||
  missionBlocks.length < 3 ||
  invalidMissionModes.length ||
  invalidMissionBonuses.length ||
  tacticBlocks.length < 3 ||
  invalidTacticModes.length ||
  invalidTacticTimeDeltas.length
) {
  console.error(
    JSON.stringify(
      {
        missing,
        missingPoseSpecs,
        extraPoseSpecs,
        missingReferencedPoseSpecs,
        invalidPoseSpecs,
        invalidPositionCatalog,
        invalidPoseRoleCatalog,
        invalidPoseGeometry,
        invalidJointLimits,
        invalidRuntimeAnatomy,
        invalidPosePairRoles,
        invalidExplicitRolePairs,
        invalidPosePairGeometry,
        invalidPosePairOrientation,
        invalidStandingOrientation,
        invalidStandingRolePair,
        invalidPosePairSupport,
        invalidPosePairForce,
        invalidAuditBiomechanics,
        duplicateScenarioIds,
        invalidRoles,
        invalidCorrectCounts,
        stateFlags,
        statefulChoices: statefulChoices.length,
        invalidStateChoices,
        actionGatedChoices: actionGatedChoices.length,
        actionGatedScenarios: scenariosWithActionGates.length,
        invalidActionChoices,
        stateBiasScenarios: scenariosWithStateBias.length,
        invalidStateBiasScenarios,
        missingUniformBranch: scenariosWithUniformBranch.length === 0,
        invalidTimeLimits,
        invalidPressure,
        invalidReadCues,
        actionAttackOverrides: actionAttackOverrides.length,
        actionVariationScenarios: scenariosWithActionVariation.length,
        invalidOpponentActions,
        invalidNextCounts,
        invalidConsequenceNextCounts,
        invalidNextRefs,
        invalidNextShapes,
        invalidReactions,
        invalidConsequences,
        invalidStyleRefs,
        missionCount: missionBlocks.length,
        invalidMissionModes,
        invalidMissionBonuses,
        tacticCount: tacticBlocks.length,
        invalidTacticModes,
        invalidTacticTimeDeltas,
      },
      null,
      2,
    ),
  );
  process.exit(1);
}

console.log(
  JSON.stringify(
    {
      poses: poseIds.length,
      referencedPoses: new Set(refs).size,
      scenarios: scenarioIds.length,
      missing: 0,
      missingPoseSpecs: 0,
      extraPoseSpecs: 0,
      missingReferencedPoseSpecs: 0,
      invalidPoseSpecs: 0,
      knownPositionFamilies: knownPositionIds.size,
      implementedPositionFamilies: implementedPositionIds.size,
      invalidPositionCatalog: 0,
      invalidPoseRoleCatalog: 0,
      invalidPoseGeometry: 0,
      invalidJointLimits: 0,
      invalidRuntimeAnatomy: 0,
      invalidPosePairRoles: 0,
      invalidExplicitRolePairs: 0,
      invalidPosePairGeometry: 0,
      invalidPosePairOrientation: 0,
      invalidStandingOrientation: 0,
      invalidStandingRolePair: 0,
      invalidPosePairSupport: 0,
      invalidPosePairForce: 0,
      invalidAuditBiomechanics: 0,
      duplicateScenarioIds: 0,
      invalidRoles: 0,
      invalidCorrectCounts: 0,
      stateFlags: stateFlags.length,
      statefulChoices: statefulChoices.length,
      invalidStateChoices: 0,
      actionGatedChoices: actionGatedChoices.length,
      actionGatedScenarios: scenariosWithActionGates.length,
      invalidActionChoices: 0,
      stateBiasScenarios: scenariosWithStateBias.length,
      invalidStateBiasScenarios: 0,
      uniformBranches: scenariosWithUniformBranch.length,
      invalidTimeLimits: 0,
      invalidPressure: 0,
      invalidReadCues: 0,
      actionAttackOverrides: actionAttackOverrides.length,
      actionVariationScenarios: scenariosWithActionVariation.length,
      invalidOpponentActions: 0,
      invalidNextCounts: 0,
      invalidConsequenceNextCounts: 0,
      invalidNextRefs: 0,
      invalidNextShapes: 0,
      weightedNextEntries: weightedNextEntries.length,
      invalidReactions: 0,
      invalidConsequences: 0,
      invalidStyleRefs: 0,
      missions: missionBlocks.length,
      invalidMissionModes: 0,
      invalidMissionBonuses: 0,
      tactics: tacticBlocks.length,
      invalidTacticModes: 0,
      invalidTacticTimeDeltas: 0,
    },
    null,
    2,
  ),
);
