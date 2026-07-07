import type { Scenario } from "../types";

// D. クローズドガード内の姿勢防御 — 下からの極めを消してパスへ
export const closedGuardPosture: Scenario = {
  id: "closed-guard-posture",
  role: "defense",
  belt: "白帯〜青帯",
  positionJp: "クローズドガード内 (下から捕まった)",
  positionEn: "Inside Closed Guard — posture defense",
  term: "閉じガード内 / dentro da guarda fechada",
  focusJoints: ["elbow", "neck"],
  stateBias: ["posture-broken", "arm-exposed"],
  setup: { red: "redClosedGuardBottom", blue: "blueTopInGuard", badge: "赤: 下から<b>クローズドガード</b>で制御" },
  attack: { red: "redClosedGuardBottom", blue: "blueTopInGuard", badge: "赤: 姿勢を崩して<b>腕十字/三角</b>を狙う" },
  timeLimitSec: 8,
  pressure: {
    early: "赤が袖/手首を引き、あなたの頭と肘を前へ崩そうとしている",
    urgent: "頭が落ち、片腕が中心線から外れ始めている。姿勢を戻す必要がある",
  },
  opponentActions: [
    {
      id: "posture-break",
      label: "姿勢を折る",
      cue: "頭が腰より前へ落ちたら、肘を内側へ戻して姿勢を立てる",
      weight: 2,
      attack: { red: "redGuardArmbarFinish", blue: "blueGuardArmbarCaught", badge: "赤: 頭を下げさせ<b>姿勢を折る</b>" },
      readCues: ["姿勢", "肘", "頭"],
      pressure: {
        early: "赤が手首を引き、頭を下げさせてガードの中へ折り込む",
        urgent: "頭が腰より前へ落ち、片腕が孤立し始めている",
      },
    },
    {
      id: "angle-cut",
      label: "腰角度を作る",
      cue: "相手の腰角度と片腕の孤立を見て、肘を中心線へ戻す",
      weight: 1,
      attack: { red: "redGuardArmbarFinish", blue: "blueGuardArmbarCaught", badge: "赤: 腰角度を作り<b>片腕を孤立</b>" },
      readCues: ["腰角度", "片腕", "膝"],
      pressure: {
        early: "赤が腰を切り、あなたの片腕を中心線から外そうとしている",
        urgent: "角度を作られ、腕十字と三角の二択が近い",
      },
    },
    {
      id: "hip-bump-threat",
      label: "起き上がって崩す",
      cue: "相手が起き上がったら手をマットに出さず、腰を制する",
      weight: 1,
      attack: { red: "redClosedGuardBottom", blue: "blueTopInGuard", badge: "赤: 起き上がって<b>姿勢を崩す</b>" },
      readCues: ["起き上がり", "手のベース", "腰"],
      pressure: {
        early: "赤が上体を起こし、手をマットにつかせる形で姿勢を崩す",
        urgent: "手を外へ出すと、腕十字やスイープの支点を渡してしまう",
      },
    },
  ],
  readCues: ["姿勢", "肘", "腰のベース"],
  situation:
    "あなた(青)は赤のクローズドガードの中。赤は下から姿勢を折り、片腕を孤立させて腕十字や三角へ繋げようとしています。",
  prompt: "下からの極めを消しながらガードを開くには？",
  options: [
    {
      jp: "背筋を立て、肘を内側に戻し、相手の腰を制してから安全にガードを開く",
      en: "Posture up, elbows in, control the hips, then open the guard",
      correct: true,
      forbiddenAction: ["angle-cut", "hip-bump-threat"],
      stateEffects: { add: ["posture-safe"], remove: ["posture-broken", "arm-exposed"] },
      next: [
        { id: "side-escape", weight: 1 },
        { id: "attack-from-side", weight: 2 },
        { id: "attack-from-mount", weight: 1 },
      ],
      reaction: "赤はガードを開かれてフレームを戻す。あなたはパスを進めるか、再び横の攻防へ移る",
      result: { red: "redGuardOpened", blue: "blueGuardPass", badge: "青: 姿勢を守り<b>ガードを開く</b>" },
      feedback:
        "正解。クローズドガード内では、頭を下げず肘を内側へ。姿勢とベースを保ってから腰を制し、安全にガードを開く。先に腕を差し出すと極めの入口になります。",
    },
    {
      jp: "相手の腰角度を正面に戻し、孤立した肘を中心線へ戻してから姿勢を立てる",
      en: "Square their hip angle, bring the isolated elbow back inside, then posture up",
      correct: true,
      requiresAction: ["angle-cut"],
      stateEffects: { add: ["posture-safe"], remove: ["posture-broken", "arm-exposed", "angle-created"] },
      next: [
        { id: "attack-from-side", weight: 2 },
        { id: "attack-from-mount", weight: 1 },
        { id: "side-escape", weight: 1 },
      ],
      reaction: "赤は角度を戻され、片腕の孤立を失う。あなたは姿勢を立て直してパスへ進む",
      result: { red: "redGuardOpened", blue: "blueGuardPass", badge: "青: 腰角度を戻し<b>腕を救出</b>" },
      feedback:
        "正解。相手が角度を切ったら、先に腰の角度を正面へ戻し、孤立した肘を中心線へ戻す。姿勢だけ立てても腕が残ると腕十字や三角へ繋がります。",
    },
    {
      jp: "手をマットにつかず、相手の腰を制して起き上がりを止めてから姿勢を戻す",
      en: "Do not post on the mat; pin the hips, stop the sit-up, then recover posture",
      correct: true,
      requiresAction: ["hip-bump-threat"],
      stateEffects: { add: ["posture-safe"], remove: ["posture-broken", "arm-exposed"] },
      next: [
        { id: "attack-from-side", weight: 2 },
        { id: "side-escape", weight: 1 },
        { id: "attack-from-mount", weight: 1 },
      ],
      reaction: "赤は起き上がりを止められ、手をマットにつかせる支点を作れない。あなたは腰を制してガードを開きにいく",
      result: { red: "redGuardOpened", blue: "blueGuardPass", badge: "青: 起き上がりを止め<b>腰を制御</b>" },
      feedback:
        "正解。ヒップバンプ気味に起き上がられたら、手をマットへ出さない。腰を制して相手の上体を寝かせ、肘を内へ戻してから姿勢を作る。",
    },
    {
      jp: "頭を下げて胸で押し込み、両手をマットにつく",
      en: "Drive your head down and post both hands on the mat",
      correct: false,
      stateEffects: { add: ["posture-broken"], remove: ["posture-safe"] },
      next: [{ id: "attack-triangle-guard", weight: 2 }, { id: "attack-armbar-guard", weight: 1 }],
      consequence: "頭が落ち、片腕が内側に残る。赤は三角絞めか腕十字へ連携しやすくなる",
      result: { red: "redTriangleFinish", blue: "blueCaughtInTriangle", badge: "赤: <b>三角</b>の形を作る" },
      feedback:
        "悪手。頭を下げて手をマットにつくと、首と片腕を脚で挟まれます。ガード内では姿勢を守ることが最初の防御です。",
    },
    {
      jp: "片腕だけを強く引き抜こうとして肘を外へ出す",
      en: "Yank one arm free and let the elbow drift outside",
      correct: false,
      stateEffects: { add: ["arm-exposed", "posture-broken"], remove: ["posture-safe"] },
      next: [{ id: "attack-armbar-guard", weight: 2 }, { id: "attack-triangle-guard", weight: 1 }],
      consequence: "肘が中心線から外れ、赤に腕を孤立させられる。腕十字か三角の二択を受けやすい",
      result: { red: "redGuardArmbarFinish", blue: "blueGuardArmbarCaught", badge: "赤: 孤立した腕へ<b>腕十字</b>" },
      feedback:
        "悪手。腕だけを抜こうとすると肘が外へ出て、関節技の支点を作られます。肘を内側へ戻し、姿勢と腰の制御を先に作る。",
    },
  ],
  principle:
    "<b>ガード内では姿勢が命。</b> 頭・肘・腰の線を守れば下からの極めは消え、パスの入口が生まれる。",
};
