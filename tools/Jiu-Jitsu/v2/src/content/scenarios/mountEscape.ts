import type { Scenario } from "../types";

// B. マウント脱出 / アッパ (守) — 上を奪取し攻めへ転じる
export const mountEscape: Scenario = {
  id: "mount-escape",
  role: "defense",
  belt: "白帯",
  positionJp: "マウント (馬乗りされた)",
  positionEn: "Mount — escaping",
  term: "縦四方固め / montada",
  focusJoints: ["elbow", "shoulder"],
  stateBias: ["arm-exposed", "back-exposed"],
  setup: { red: "redMountTop", blue: "blueUnderMount", badge: "赤: <b>マウント</b>確保 (上位)" },
  attack: { red: "redMountArmbar", blue: "blueUnderMount", badge: "赤: <b>腕十字</b>を狙い腕を取りにくる" },
  timeLimitSec: 9,
  pressure: {
    early: "赤が膝を高く上げ、あなたの肘を体から剥がしにくる",
    urgent: "腕が伸ばされ始めている。橋を作るなら今",
  },
  opponentActions: [
    {
      id: "arm-isolation",
      label: "片腕を隔離",
      cue: "伸ばされる腕と同側の足を先に封じて橋を作る",
      weight: 2,
      attack: { red: "redMountArmbar", blue: "blueUnderMount", badge: "赤: <b>片腕を隔離</b>して腕十字へ" },
      readCues: ["片腕", "同側の足", "腰の橋"],
      pressure: {
        early: "赤が片腕を体から剥がし、腕十字へ角度を作る",
        urgent: "肘が伸び始めている。封じる腕と足を今決める",
      },
    },
    {
      id: "high-mount-climb",
      label: "高いマウントへ上がる",
      cue: "膝が脇へ上がる前に肘を戻し、腰の橋を残す",
      weight: 1,
      attack: { red: "redMountTop", blue: "blueUnderMount", badge: "赤: <b>高いマウント</b>へ上がる" },
      readCues: ["膝の位置", "腰", "肘"],
      pressure: {
        early: "赤が膝を脇へ上げ、腰を重くして橋を殺しにくる",
        urgent: "膝が高くなり、腕と首の逃げ道が消え始めている",
      },
    },
    {
      id: "grapevine-base",
      label: "脚で腰を伸ばす",
      cue: "足を絡められたら膝肘で空間を作り、橋だけに固執しない",
      weight: 1,
      attack: { red: "redMountTop", blue: "blueUnderMount", badge: "赤: 脚で腰を伸ばし<b>橋を殺す</b>" },
      readCues: ["足の絡み", "腰", "肘"],
      pressure: {
        early: "赤が脚であなたの腰を伸ばし、橋の爆発力を消しにくる",
        urgent: "腰が伸ばされ、片腕を封じる前に橋の支点がなくなっている",
      },
    },
  ],
  readCues: ["片腕", "同側の足", "腰の橋"],
  situation:
    "脱出の流れで赤があなたの腹の上＝マウントへ。赤は体重を預け、伸びた腕を狙って腕十字に来ます。",
  prompt: "どう返す？ (マウント下の最優先は?)",
  options: [
    {
      jp: "相手の片腕と同側の足を封じ、強く橋を作って封じた側へ返す (アッパ)",
      en: "Trap arm & leg, bridge and roll — Upa",
      correct: true,
      forbiddenAction: ["high-mount-climb", "grapevine-base"],
      stateEffects: { add: ["top-base"], remove: ["arm-exposed", "back-exposed"] },
      next: [
        { id: "attack-from-mount", weight: 3 },
        { id: "attack-from-side", weight: 2 },
        { id: "side-escape", weight: 1 },
      ],
      reaction: "赤は返されながらガードやフレームで止めようとする。上を取ったあなたは次の支配位置を選ぶ",
      result: { red: "redRolledBottom", blue: "blueUpaTop", badge: "青: <b>スイープ成功</b> 上を奪取 ▸ 攻めへ転じる" },
      feedback:
        "正解。相手の片腕と同じ側の足を封じ、橋 (ブリッジ) を作って封じた側へ返す。テコと体重移動で力を使わず上下を入れ替える柔術の代表的エスケープ「アッパ」。これで<b>あなたが上</b>になり、攻めに転じます。",
    },
    {
      jp: "膝が脇へ上がる前に肘を戻し、片膝を差して膝肘エスケープへ切り替える",
      en: "Recover the elbows before high mount settles, insert a knee, and switch to knee-elbow escape",
      correct: true,
      requiresAction: ["high-mount-climb"],
      stateEffects: { add: ["guard-recovered"], remove: ["arm-exposed", "back-exposed"] },
      next: [
        { id: "side-escape", weight: 2 },
        { id: "attack-armbar-guard", weight: 1 },
        { id: "attack-triangle-guard", weight: 1 },
      ],
      reaction: "赤は高いマウントで腕を狙うが、あなたは肘と膝を戻して空間を作る。次は横圧かガードの攻防へ戻る",
      result: { red: "redSideControl", blue: "blueShrimpRecover", badge: "青: 肘と膝を戻し<b>空間を作る</b>" },
      feedback:
        "正解。高いマウントで膝が脇へ上がると、単純な橋は効きにくくなります。先に肘を体へ戻し、膝を差して腰を逃がす。橋だけに固執しない判断です。",
    },
    {
      jp: "足の絡みを外して膝肘で空間を作り、橋ではなく腰を横へ逃がす",
      en: "Clear the grapevines, build knee-elbow space, and hip escape instead of forcing the bridge",
      correct: true,
      requiresAction: ["grapevine-base"],
      stateEffects: { add: ["guard-recovered"], remove: ["arm-exposed", "back-exposed"] },
      next: [
        { id: "side-escape", weight: 2 },
        { id: "closed-guard-posture", weight: 1 },
        { id: "attack-armbar-guard", weight: 1 },
      ],
      reaction: "赤は脚で腰を伸ばして橋を殺す。あなたは足の絡みを外し、膝肘で空間を作って下の構造を戻す",
      result: { red: "redSideControl", blue: "blueShrimpRecover", badge: "青: 脚の絡みを外し<b>膝肘で回復</b>" },
      feedback:
        "正解。グレープバインで腰を伸ばされたら、アッパの支点が消えます。まず足の絡みを外し、膝肘で空間を作って腰を横へ逃がす。",
    },
    {
      jp: "両手で相手の胸を全力で押し上げて引き剥がす",
      en: "Bench-press them off with both arms",
      correct: false,
      stateEffects: { add: ["arm-exposed"], remove: ["top-base"] },
      next: [{ id: "back-defense", weight: 2 }, { id: "side-escape", weight: 1 }],
      consequence: "伸びた腕を取られ、赤は腕十字か上位支配を継続する。あなたは背中か横からの圧を受けやすい",
      result: { red: "redMountArmbar", blue: "blueUnderMount", badge: "赤: 伸びた腕を<b>腕十字</b>で取得" },
      feedback:
        "悪手。腕を伸ばして押すと、その腕を腕十字 (アームバー) で取られます。マウント下で腕を伸ばすのは最も危険な行為のひとつ。",
    },
    {
      jp: "うつ伏せに寝返って背中を相手に向ける",
      en: "Roll to your stomach / give up the back",
      correct: false,
      stateEffects: { add: ["back-exposed"], remove: ["top-base"] },
      next: ["back-defense"],
      consequence: "背中を向けたことで赤はバックへ回る。次は首を守る判断から立て直す必要がある",
      result: { red: "redBackControl", blue: "blueGivesBack", badge: "赤: <b>バック</b>へ移行" },
      feedback:
        "悪手。背を向けるとバックコントロール (さらに上位) を献上し、裸絞めの餌食に。マウントよりバックを取られる方が危険。",
    },
  ],
  principle:
    "<b>エスケープに腕力は要らない。</b> 橋 (ブリッジ) と海老 (シュリンプ) が下からの二大基本。返したら攻守交代 — 今度はあなたが位置を支配する番。",
};
