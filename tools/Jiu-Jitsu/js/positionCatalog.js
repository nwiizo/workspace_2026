// positionCatalog.js
// BJJ の体勢分類と、このゲームで 3D 実装済みの role / role ペア制約。
// 「既知だが未実装」と「現在ゲームに出してよい」を分け、未定義の組み合わせを検証で落とす。

export const BJJ_POSITION_FAMILIES = [
  { id: "standing-neutral", labelJp: "立位/組み手", implemented: true },
  { id: "closed-guard", labelJp: "クローズドガード", implemented: true },
  { id: "open-guard", labelJp: "オープンガード/パス入口", implemented: true },
  { id: "half-guard", labelJp: "ハーフガード/ニーシールド", implemented: false },
  { id: "butterfly-guard", labelJp: "バタフライガード", implemented: false },
  { id: "dlr-guard", labelJp: "デラヒーバ/リバースデラヒーバ", implemented: false },
  { id: "spider-lasso-guard", labelJp: "スパイダー/ラッソーガード", implemented: false },
  { id: "x-guard", labelJp: "Xガード/SLX", implemented: false },
  { id: "guard-pass", labelJp: "ガードパス/上のベース", implemented: true },
  { id: "side-control", labelJp: "サイドコントロール", implemented: true },
  { id: "north-south", labelJp: "ノースサウス", implemented: false },
  { id: "knee-on-belly", labelJp: "ニーオンベリー", implemented: false },
  { id: "mount", labelJp: "マウント/高いマウント", implemented: true },
  { id: "technical-mount", labelJp: "テクニカルマウント", implemented: false },
  { id: "back-control", labelJp: "バックコントロール", implemented: true },
  { id: "turtle", labelJp: "タートル/背中露出", implemented: true },
  { id: "front-headlock", labelJp: "フロントヘッドロック", implemented: false },
  { id: "crucifix", labelJp: "クルシフィックス", implemented: false },
  { id: "armbar", labelJp: "腕十字", implemented: true },
  { id: "triangle", labelJp: "三角絞め", implemented: true },
  { id: "omoplata", labelJp: "オモプラッタ", implemented: false },
  { id: "kimura", labelJp: "キムラ/肩固め系", implemented: false },
  { id: "guillotine", labelJp: "ギロチン", implemented: false },
  { id: "darce-anaconda", labelJp: "ダース/アナコンダ", implemented: false },
  { id: "ashi-garami", labelJp: "アシガラミ/足関入口", implemented: false },
  { id: "inside-ashi", labelJp: "インサイドアシ/サドル/411", implemented: false },
  { id: "outside-ashi", labelJp: "アウトサイドアシ", implemented: false },
  { id: "fifty-fifty", labelJp: "50/50/バックサイド50/50", implemented: false },
  { id: "leg-lock-finish", labelJp: "足関節フィニッシュ", implemented: false },
];

export const POSE_ROLE_CATALOG = {
  standing: { family: "standing-neutral", level: "neutral", orientation: "facing" },
  "seated-front": { family: "back-control", level: "bottom", orientation: "same-direction" },
  "back-control-top": { family: "back-control", level: "top", orientation: "same-direction-behind" },
  "back-defense-bottom": { family: "back-control", level: "bottom", orientation: "same-direction" },
  "submitted-bottom": { family: "back-control", level: "bottom", orientation: "same-direction" },
  "supine-bottom": { family: "mount", level: "bottom", orientation: "headline" },
  "mount-top": { family: "mount", level: "top", orientation: "toward-head" },
  "mount-top-attacking-arm": { family: "mount", level: "top", orientation: "toward-head" },
  "guard-top-after-sweep": { family: "guard-pass", level: "top", orientation: "toward-head" },
  "closed-guard-bottom": { family: "closed-guard", level: "bottom", orientation: "guard-wrap" },
  "closed-guard-top": { family: "closed-guard", level: "top", orientation: "toward-head" },
  "guard-pass-top": { family: "guard-pass", level: "top", orientation: "toward-head" },
  "open-guard-bottom": { family: "open-guard", level: "bottom", orientation: "frames-legs" },
  "side-control-top": { family: "side-control", level: "top", orientation: "perpendicular" },
  "side-control-bottom": { family: "side-control", level: "bottom", orientation: "pinned" },
  "shrimp-bottom": { family: "side-control", level: "bottom", orientation: "hip-escape" },
  "supine-bottom-arm-isolated": { family: "mount", level: "bottom", orientation: "headline" },
  "armbar-attacker": { family: "armbar", level: "attacker", orientation: "joint-line" },
  "armbar-defender": { family: "armbar", level: "defender", orientation: "joint-line" },
  "guard-armbar-attacker": { family: "armbar", level: "attacker", orientation: "guard-angle" },
  "guard-armbar-defender": { family: "armbar", level: "defender", orientation: "posture-broken" },
  "prone-bottom": { family: "turtle", level: "bottom", orientation: "back-exposed" },
  "triangle-attacker": { family: "triangle", level: "attacker", orientation: "guard-angle" },
  "triangle-defender": { family: "triangle", level: "defender", orientation: "posture-broken" },
};

export const POSE_SPEC_ROLES = new Set(Object.keys(POSE_ROLE_CATALOG));

export const ALLOWED_ROLE_PAIR_RULES = [
  { red: "standing", blue: "standing", relation: "standing-opponents" },
  { red: "back-control-top", blue: "seated-front", relation: "back-control-established" },
  { red: "back-control-top", blue: "back-defense-bottom", relation: "back-control-defended" },
  { red: "back-control-top", blue: "submitted-bottom", relation: "back-choke-finish" },
  { red: "back-control-top", blue: "prone-bottom", relation: "back-take-from-turtle" },
  { red: "mount-top", blue: "supine-bottom", relation: "mount-established" },
  { red: "mount-top-attacking-arm", blue: "supine-bottom", relation: "mount-arm-isolation" },
  { red: "supine-bottom", blue: "guard-top-after-sweep", relation: "sweep-to-top-guard" },
  { red: "closed-guard-bottom", blue: "closed-guard-top", relation: "closed-guard" },
  { red: "open-guard-bottom", blue: "guard-pass-top", relation: "opened-guard-pass" },
  { red: "side-control-top", blue: "side-control-bottom", relation: "side-control-established" },
  { red: "side-control-top", blue: "shrimp-bottom", relation: "side-control-escape" },
  { red: "guard-armbar-attacker", blue: "guard-armbar-defender", relation: "guard-armbar-finish" },
  { red: "armbar-attacker", blue: "armbar-defender", relation: "top-armbar-finish" },
  { red: "triangle-attacker", blue: "triangle-defender", relation: "guard-triangle-finish" },
];

const familyIds = new Set(BJJ_POSITION_FAMILIES.map((position) => position.id));
const implementedFamilies = new Set(
  BJJ_POSITION_FAMILIES.filter((position) => position.implemented).map((position) => position.id),
);
const allowedPairs = new Set(ALLOWED_ROLE_PAIR_RULES.map((rule) => `${rule.red}/${rule.blue}`));

export const implementedPositionIds = () => implementedFamilies;
export const knownPositionIds = () => familyIds;
export const roleSpec = (role) => POSE_ROLE_CATALOG[role] || null;
export const rolePairRule = (redRole, blueRole) =>
  ALLOWED_ROLE_PAIR_RULES.find((rule) => rule.red === redRole && rule.blue === blueRole) || null;
export const rolePairAllowed = (redRole, blueRole) => allowedPairs.has(`${redRole}/${blueRole}`);
