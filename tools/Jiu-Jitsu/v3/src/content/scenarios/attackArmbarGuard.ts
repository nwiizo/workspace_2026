import type { Scenario } from "../types";

export const attackArmbarGuard: Scenario = {
  id: "attack-armbar-guard",
  role: "offense",
  belt: "青帯",
  positionJp: "クローズドガードからの腕十字",
  positionEn: "Closed Guard Armbar",
  term: "閉じガード / juji-gatame",
  focusJoints: ["elbow"],
  stateBias: ["angle-created", "posture-broken", "arm-exposed"],
  setup: { red: "redClosedGuardBottom", blue: "blueTopInGuard", badge: "赤: 下から<b>クローズドガード</b>" },
  attack: { red: "redGuardArmbarFinish", blue: "blueGuardArmbarCaught", badge: "赤: <b>腕十字</b>へ移行" },
  timeLimitSec: 8,
  pressure: {
    early: "青が姿勢を起こし、肘を中心線へ戻そうとしている",
    urgent: "青の肘が抜ける。角度を作らないと腕十字が消える",
  },
  opponentActions: [
    {
      id: "posture-rise",
      label: "青が姿勢を戻す",
      cue: "肘が戻る前に腰角度を切り、脚を顔側へ回す",
      weight: 1,
      attack: { red: "redClosedGuardBottom", blue: "blueTopInGuard", badge: "青が<b>姿勢</b>を戻し肘を隠す" },
      readCues: ["姿勢", "肘", "腰角度"],
      pressure: {
        early: "青が背筋を立て、肘を中心線へ戻そうとしている",
        urgent: "肘が抜ける。角度と脚の回し込みを先に作る",
      },
    },
    {
      id: "arm-pull-free",
      label: "青が腕を引き抜く",
      cue: "腕が抜ける反応を三角の入口として読み替える",
      weight: 1,
      attack: { red: "redTriangleFinish", blue: "blueCaughtInTriangle", badge: "腕を抜く反応に赤が<b>三角</b>へ切替" },
      readCues: ["肘", "片腕", "三角の入口"],
      pressure: {
        early: "青が腕を引き抜き、腕十字の線を消そうとしている",
        urgent: "腕が抜ける反応で、三角へ切り替える条件が見える",
      },
    },
    {
      id: "stack-defense",
      label: "青が重ねて潰す",
      cue: "重ねられたら脚を閉じたまま角度を戻し、三角へ切り替える",
      weight: 1,
      attack: { red: "redTriangleFinish", blue: "blueCaughtInTriangle", badge: "青の<b>重ね圧</b>に赤が三角へ切替" },
      readCues: ["重ね圧", "角度", "三角"],
      pressure: {
        early: "青が体重を前へ重ね、腕十字の角度を潰そうとする",
        urgent: "角度が潰れる前に脚を閉じ直し、三角の線へ切り替える",
      },
    },
  ],
  readCues: ["姿勢", "肘", "腰角度"],
  situation:
    "あなた (赤) は下のクローズドガード。青は姿勢を立てようとしながら片腕を前に残しています。",
  prompt: "腕十字に入る正しい攻め方は？",
  options: [
    {
      jp: "相手の姿勢を崩し、肘を中心線から外して脚を顔へ回す",
      en: "Break posture, move the elbow off-center, swing the leg over",
      correct: true,
      stateEffects: { add: ["angle-created"], remove: ["posture-safe", "stack-pressure"] },
      next: [{ id: "attack-triangle-guard", weight: 3 }, { id: "attack-from-back", weight: 1 }],
      reaction: "青は肘を抜いて姿勢を戻そうとする。その反応で三角や背中への角度が生まれる",
      result: { red: "redGuardArmbarFinish", blue: "blueGuardArmbarCaught", badge: "赤: 角度を作り<b>腕十字</b>" },
      feedback:
        "正解。腕十字は腕だけでなく姿勢と角度の技。相手の肘を中心線から外し、腰を切って脚を顔にかける。",
    },
    {
      jp: "腕だけを両手で引っ張り、脚は閉じたままにする",
      en: "Pull the arm with both hands while keeping guard closed",
      correct: false,
      stateEffects: { add: ["posture-safe"], remove: ["angle-created"] },
      next: [{ id: "attack-triangle-guard", weight: 2 }, { id: "attack-from-side", weight: 1 }],
      consequence: "青が姿勢を戻す。あなたは三角の条件を探すか、上を取る展開へ切り替える",
      result: { red: "redClosedGuardBottom", blue: "blueTopInGuard", badge: "青: 姿勢を立て<b>防御</b>" },
      feedback:
        "悪手。脚と腰の角度がなければ肘を伸ばせません。腕力だけでは相手に姿勢を戻されます。",
    },
    {
      jp: "相手の両腕を同時に追って体を正面に残す",
      en: "Chase both arms while staying square",
      correct: false,
      stateEffects: { add: ["posture-safe"], remove: ["angle-created"] },
      next: ["attack-triangle-guard", "attack-from-side"],
      consequence: "正面のまま青にベースを戻される。あなたは腕の配置を作り直すか、上への展開を狙う",
      result: { red: "redClosedGuardBottom", blue: "blueTopInGuard", badge: "青: ベースを戻し<b>安定</b>" },
      feedback:
        "悪手。正面のままでは角度が足りません。一つの腕を孤立させ、腰を切って相手の姿勢を折る。",
    },
  ],
  principle:
    "<b>角度が極めを作る。</b> 下からの攻撃でも、相手の姿勢を崩してから一つの関節を孤立させる。",
};
