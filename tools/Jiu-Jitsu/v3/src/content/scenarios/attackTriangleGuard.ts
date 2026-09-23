import type { Scenario } from "../types";

export const attackTriangleGuard: Scenario = {
  id: "attack-triangle-guard",
  role: "offense",
  belt: "青帯",
  positionJp: "三角絞めへの連携",
  positionEn: "Triangle Choke Offense",
  term: "三角絞め / triângulo",
  focusJoints: ["neck", "shoulder"],
  stateBias: ["angle-created", "posture-broken", "stack-pressure"],
  setup: { red: "redClosedGuardBottom", blue: "blueTopInGuard", badge: "赤: 下から<b>三角</b>の入口を作る" },
  attack: { red: "redTriangleFinish", blue: "blueCaughtInTriangle", badge: "赤: <b>首と片腕</b>を脚で挟む" },
  timeLimitSec: 8,
  pressure: {
    early: "青が片腕を外へ戻し、膝の間から頭を抜こうとしている",
    urgent: "青の頭が抜ける。首と片腕を閉じる判断が必要",
  },
  opponentActions: [
    {
      id: "head-posture",
      label: "青が頭を抜く",
      cue: "頭が抜ける前に片腕を残し、脚ロックの角度を切る",
      weight: 1,
      attack: { red: "redTriangleFinish", blue: "blueCaughtInTriangle", badge: "青が頭を抜く前に赤が<b>脚を閉じる</b>" },
      readCues: ["首", "姿勢", "脚ロック"],
      pressure: {
        early: "青が頭を上げ、膝の間から首を抜こうとしている",
        urgent: "頭が抜ける。首と片腕を閉じる角度を先に作る",
      },
    },
    {
      id: "arm-hide",
      label: "青が腕を隠す",
      cue: "腕を隠す反応を腕十字へ戻す入口として使う",
      weight: 2,
      attack: { red: "redGuardArmbarFinish", blue: "blueGuardArmbarCaught", badge: "青の腕隠しに赤が<b>腕十字</b>へ戻す" },
      readCues: ["片腕", "肩", "腕十字"],
      pressure: {
        early: "青が片腕を隠し、三角の首肩ラインをほどこうとしている",
        urgent: "腕が動く反応で、腕十字へ戻す入口が開いている",
      },
    },
    {
      id: "stack-pressure",
      label: "青が重ねて潰す",
      cue: "重ねられたら腰角度を作り直すか、腕十字へ戻す",
      weight: 1,
      attack: { red: "redGuardArmbarFinish", blue: "blueGuardArmbarCaught", badge: "青の<b>重ね圧</b>に赤が腕十字へ戻す" },
      readCues: ["重ね圧", "腰角度", "腕十字"],
      pressure: {
        early: "青が体重を前へ重ね、三角の脚ロックを潰そうとする",
        urgent: "重ね圧で首の角度が潰れる。腕十字へ戻す入口を読む",
      },
    },
  ],
  readCues: ["首", "片腕", "脚ロック"],
  situation:
    "あなた (赤) は下。青の片腕が内側、もう片腕が外側に分かれ、三角絞めの条件が見えています。",
  prompt: "三角絞めを完成させるには？",
  options: [
    {
      jp: "姿勢を崩して片腕を中に残し、脚で首と肩を閉じて角度を作る",
      en: "Break posture, trap one arm inside, lock the legs and cut the angle",
      correct: true,
      stateEffects: { add: ["angle-created"], remove: ["posture-safe", "stack-pressure"] },
      next: [{ id: "attack-armbar-guard", weight: 2 }, { id: "attack-from-side", weight: 1 }],
      reaction: "青は首を抜こうとして腕を伸ばす。あなたは腕十字へ戻すか、上を取る展開へ繋げる",
      result: { red: "redTriangleFinish", blue: "blueCaughtInTriangle", badge: "赤: 角度を作り<b>三角</b>" },
      feedback:
        "正解。三角は片腕を内側に残し、相手の姿勢を前に折り、脚の角度で肩と首を閉じる技。",
    },
    {
      jp: "相手の頭だけを両手で引き続ける",
      en: "Pull only the head with both hands",
      correct: false,
      stateEffects: { add: ["posture-safe"], remove: ["angle-created"] },
      next: [{ id: "attack-armbar-guard", weight: 2 }, { id: "attack-from-side", weight: 1 }],
      consequence: "青が姿勢を戻して首を抜く。あなたは腕十字へ切り替えるか、上への攻防へ進む",
      result: { red: "redClosedGuardBottom", blue: "blueTopInGuard", badge: "青: 姿勢を立て<b>解除</b>" },
      feedback:
        "悪手。頭だけを引くと相手は姿勢を戻せます。腕の配置と腰の角度を同時に作る必要があります。",
    },
    {
      jp: "両腕が外にあるまま脚を閉じる",
      en: "Close the legs while both arms stay outside",
      correct: false,
      stateEffects: { add: ["posture-safe"], remove: ["angle-created"] },
      next: ["attack-armbar-guard", "attack-from-side"],
      consequence: "三角の条件がなく青に安全な姿勢を戻される。腕十字や上への切り替えを作り直す必要がある",
      result: { red: "redClosedGuardBottom", blue: "blueTopInGuard", badge: "青: 腕を戻し<b>安全</b>" },
      feedback: "悪手。三角は首と片腕を挟む技。両腕が外なら絞めの構造ができません。",
    },
  ],
  principle:
    "<b>条件を見て攻める。</b> サブミッションは形だけでなく、相手の腕・首・姿勢の条件が揃って初めて成立する。",
};
