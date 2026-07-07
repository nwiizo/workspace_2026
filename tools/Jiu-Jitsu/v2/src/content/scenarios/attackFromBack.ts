import type { Scenario } from "../types";

export const attackFromBack: Scenario = {
  id: "attack-from-back",
  role: "offense",
  belt: "白帯",
  positionJp: "バックからの攻め",
  positionEn: "Back Control Offense",
  term: "背後位 / mata-leão",
  focusJoints: ["neck"],
  stateBias: ["back-exposed", "neck-exposed"],
  setup: { red: "redBackControl", blue: "blueSeatedFront", badge: "赤: <b>バック</b>確保＋両フック" },
  attack: { red: "redBackControl", blue: "blueTapped", badge: "赤: <b>裸絞め</b>へ進行" },
  timeLimitSec: 8,
  pressure: {
    early: "青が顎を引き、弱い側へ腰をずらそうとしている",
    urgent: "青の背中が床へ向き始める。フックと防御手の処理が必要",
  },
  opponentActions: [
    {
      id: "hand-fight",
      label: "青が防御手を重ねる",
      cue: "首をこじ開けず、防御手を一枚ずつ剥がす",
      weight: 2,
      attack: { red: "redBackControl", blue: "blueBackDefend", badge: "青が<b>防御手</b>を重ね、赤は剥がしに行く" },
      readCues: ["防御手", "首", "シートベルト"],
      pressure: {
        early: "青が両手を首元へ重ね、絞め腕の入口を塞いでいる",
        urgent: "防御手を剥がさず首だけ狙うと逃げられる",
      },
    },
    {
      id: "hip-slide",
      label: "青が弱い側へ腰を抜く",
      cue: "腰が抜ける前にフックを保つか上位へ切り替える",
      weight: 1,
      attack: { red: "redBackControl", blue: "blueBackDefend", badge: "青が弱い側へ<b>腰を抜く</b>" },
      readCues: ["フック", "腰", "背中"],
      pressure: {
        early: "青が弱い側へ尻を抜き、背中を床へ向け始める",
        urgent: "腰が抜ける前にフックか上位ポジションへ切り替える",
      },
    },
  ],
  readCues: ["フック", "防御手", "首"],
  situation:
    "あなた (赤) はバックを取っています。青は両手で首を守り、弱い側へ尻を抜こうとしています。",
  prompt: "安全に攻めを進めるなら？",
  options: [
    {
      jp: "シートベルトを保ち、片手で相手の防御手を剥がしてから絞め腕を入れる",
      en: "Keep the seatbelt, strip a defending hand, then enter the choke",
      correct: true,
      forbiddenAction: ["hip-slide"],
      next: [{ id: "attack-from-mount", weight: 2 }, { id: "attack-from-side", weight: 1 }],
      reaction: "青は首を守りながら背中を床へ向ける。あなたはフックを保つか、上位ポジションへ移る",
      result: { red: "redBackControl", blue: "blueTapped", badge: "赤: 防御手を剥がし<b>絞め</b>へ" },
      feedback:
        "正解。バック攻撃は首を直接こじ開けるのでなく、シートベルトとフックで背中を保ち、防御手を一枚ずつ剥がす。",
    },
    {
      jp: "腰が抜ける側のフックを追い、背中が床へ向くならマウントへ切り替える",
      en: "Follow the escaping hip with the hook, and switch to mount if their back reaches the mat",
      correct: true,
      requiresAction: ["hip-slide"],
      stateEffects: { add: ["top-base"], remove: ["back-exposed", "neck-exposed"] },
      next: [{ id: "attack-from-mount", weight: 3 }, { id: "attack-from-side", weight: 1 }],
      reaction: "青は弱い側へ腰を抜く。あなたはバックに固執せず、背中が床へ向く流れをマウントへ変換する",
      result: { red: "redMountTop", blue: "blueUnderMount", badge: "赤: 腰を追って<b>マウント</b>へ変換" },
      feedback:
        "正解。腰が抜け始めたら首だけを追うと位置を失います。フックで腰を追い、背中が床へ向くならマウントへ切り替えると上位を保てます。",
    },
    {
      jp: "両足のフックを外して腕だけで首を取りにいく",
      en: "Remove both hooks and chase the neck with arms only",
      correct: false,
      next: [{ id: "attack-from-mount", weight: 2 }, { id: "attack-from-side", weight: 1 }],
      consequence: "フックを捨てたことで青の腰が逃げる。あなたは上位ポジションへ移って支配を作り直す",
      result: { red: "redBackControl", blue: "blueBackDefend", badge: "青: 弱い側へ<b>脱出</b>" },
      feedback: "悪手。フックを捨てると相手の腰が逃げます。バックは脚で位置、腕で首を管理する。",
    },
    {
      jp: "相手の顎を無理に押し上げて首をこじ開ける",
      en: "Force the chin up and pry the neck open",
      correct: false,
      next: ["attack-from-side", "attack-from-mount"],
      consequence: "力任せで青に防御手を戻される。あなたは位置を保って別の上位支配から攻め直す",
      result: { red: "redBackControl", blue: "blueBackDefend", badge: "青: 顎を引いて<b>防御</b>" },
      feedback:
        "悪手。力任せは危険で再現性が低い。防御手と肩のラインを崩して、相手が守れない角度を作る。",
    },
  ],
  principle:
    "<b>支配と安全。</b> 絞めは危険を伴うため、練習ではゆっくり入り、相手のタップに即座に反応する。",
};
