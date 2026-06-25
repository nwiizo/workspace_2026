// anatomy.js
// Simplified anatomical constraints shared by rendering and data validation.
// Angles are intentionally conservative for the primitive display rig.

export const deg = (d) => (d * Math.PI) / 180;
export const radToDeg = (r) => (r * 180) / Math.PI;

export const JOINT_LIMITS_DEG = [
  { pattern: /^neck$/, x: [-35, 35], y: [-30, 30], z: [-30, 30] },
  { pattern: /^upperArm[LR]$/, x: [-125, 125], y: [-70, 70], z: [-80, 80] },
  { pattern: /^forearm[LR]$/, x: [-120, 120], y: [-55, 55], z: [-35, 35] },
  { pattern: /^hand[LR]$/, x: [-55, 55], y: [-45, 45], z: [-45, 45] },
  { pattern: /^thigh[LR]$/, x: [-130, 145], y: [-55, 55], z: [-80, 80] },
  { pattern: /^shin[LR]$/, x: [0, 150], y: [-30, 30], z: [-25, 25] },
  { pattern: /^foot[LR]$/, x: [-55, 55], y: [-55, 55], z: [-55, 55] },
];

const axes = ["x", "y", "z"];
const limitForJoint = (joint) => JOINT_LIMITS_DEG.find((limit) => limit.pattern.test(joint));
const clamp = (value, min, max) => Math.min(max, Math.max(min, value));

export function clampJointEuler(joint, radians) {
  const limit = limitForJoint(joint);
  if (!limit) return radians;
  return radians.map((value, index) => {
    const [min, max] = limit[axes[index]];
    return deg(clamp(radToDeg(value), min, max));
  });
}

export function jointLimitViolations(poseId, joint, radians) {
  const limit = limitForJoint(joint);
  if (!limit || !Array.isArray(radians) || radians.length !== 3) return [];
  return radians.flatMap((value, index) => {
    const axis = axes[index];
    const degrees = radToDeg(value);
    const [min, max] = limit[axis];
    return degrees < min || degrees > max
      ? [`${poseId}:${joint}.${axis}=${degrees.toFixed(1)} outside ${min}..${max}`]
      : [];
  });
}
