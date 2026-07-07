import type { Scenario } from "../types";

export const attackFromSide: Scenario = {
  id: "attack-from-side",
  role: "offense",
  belt: "白帯〜青帯",
  positionJp: "サイドコントロールからの攻め",
  positionEn: "Side Control Offense",
  term: "横四方固め / cem quilos",
  focusJoints: ["shoulder", "elbow"],
  stateBias: ["guard-recovered", "knee-shield", "top-base"],
  setup: { red: "redSideControl", blue: "blueUnderSide", badge: "赤: <b>横四方</b>で抑え込み" },
  attack: { red: "redMountTop", blue: "blueUnderMount", badge: "赤: <b>マウント</b>へ移行" },
  timeLimitSec: 8,
  pressure: {
    early: "青がフレームを作り、膝を差し込む空間を探している",
    urgent: "青の膝が戻り始めている。腰を制しないとガードに戻される",
  },
  opponentActions: [
    {
      id: "frame-recovery",
      label: "青が首と腰にフレーム",
      cue: "フレームと膝の間を潰し、腰を制して位置を上げる",
      weight: 2,
      attack: { red: "redSideControl", blue: "blueShrimpRecover", badge: "青が<b>フレーム</b>で膝を戻す" },
      readCues: ["フレーム", "腰", "膝"],
      pressure: {
        early: "青が首と腰にフレームを作り、膝を差し込む空間を探す",
        urgent: "膝が戻る。腰を制して位置を上げる判断が必要",
      },
    },
    {
      id: "turn-away",
      label: "青が背を向ける",
      cue: "背中が見えたら腰を追い、バックかマウントへ進む",
      weight: 1,
      attack: { red: "redBackControl", blue: "blueGivesBack", badge: "青が背を向け、赤は<b>バック</b>へ追う" },
      readCues: ["背中", "腰", "フック"],
      pressure: {
        early: "青が圧から逃げるため背を向け、バックの入口を作っている",
        urgent: "背中かマウントか、位置を失う前に上位へ進む",
      },
    },
    {
      id: "knee-shield-insert",
      label: "青が膝盾を差す",
      cue: "膝が差さったら胸圧だけでなく腰を戻して潰す",
      weight: 1,
      attack: { red: "redSideControl", blue: "blueShrimpRecover", badge: "青が<b>膝盾</b>を差し、赤は腰を潰す" },
      readCues: ["膝盾", "腰", "胸圧"],
      pressure: {
        early: "青が膝を差し込み、腰の前にシールドを作り始める",
        urgent: "膝盾を許すとガードに戻る。腰を制して角度を潰す",
      },
    },
  ],
  readCues: ["フレーム", "腰", "膝"],
  situation:
    "あなた (赤) はサイドで上。青は首と腰にフレームを作り、膝を差し込もうとしています。",
  prompt: "相手のガードリカバリを防ぎながら攻めるには？",
  options: [
    {
      jp: "フレームを潰して腰を制し、ニーオンベリーまたはマウントへ段階的に上がる",
      en: "Flatten the frames, control the hips, then climb to mount",
      correct: true,
      forbiddenState: ["guard-recovered"],
      forbiddenAction: ["turn-away", "knee-shield-insert"],
      stateEffects: { add: ["top-base"], remove: ["knee-shield", "guard-recovered"] },
      next: [{ id: "attack-from-mount", weight: 3 }, { id: "attack-from-back", weight: 1 }],
      reaction: "青は膝を差すか背を向けて逃げる。あなたは腰を潰し続け、マウントかバックへ上がる",
      result: { red: "redMountTop", blue: "blueUnderMount", badge: "赤: <b>マウント</b>へ前進" },
      feedback:
        "正解。サイドでは極めを急がず、相手のフレームと腰を潰して膝の差し込みを消し、より上位の位置へ進む。",
    },
    {
      jp: "前局面で戻された膝を潰し直し、腰を固定してからマウントへ上がる",
      en: "Re-smash the recovered knee, pin the hips, then climb",
      correct: true,
      requiresState: ["guard-recovered"],
      forbiddenAction: ["turn-away"],
      stateEffects: { add: ["top-base"], remove: ["knee-shield", "guard-recovered"] },
      next: [{ id: "attack-from-mount", weight: 3 }, { id: "attack-from-back", weight: 1 }],
      reaction: "青は一度戻した膝を潰され、背を向けるか肘を守って下から耐える",
      result: { red: "redMountTop", blue: "blueUnderMount", badge: "赤: 膝を潰し直し<b>マウント</b>へ" },
      feedback:
        "正解。前局面でガードを戻された流れでは、胸圧だけで登ると膝盾が残ります。膝を潰し直して腰を固定してから位置を上げる。",
    },
    {
      jp: "背中が見えた瞬間に腰を追い、フックを入れてバックコントロールへ移る",
      en: "Follow the hips as they turn away, insert hooks, and take the back",
      correct: true,
      requiresAction: ["turn-away"],
      stateEffects: { add: ["back-exposed"], remove: ["knee-shield", "guard-recovered"] },
      next: [{ id: "attack-from-back", weight: 3 }, { id: "back-defense", weight: 1 }],
      reaction: "青は圧から逃げようとして背中を見せる。あなたは腰を追ってバックを取り、首と防御手の攻防へ入る",
      result: { red: "redBackControl", blue: "blueGivesBack", badge: "赤: 背を向けた青を追って<b>バック</b>へ" },
      feedback:
        "正解。相手が背を向けたら、マウントへ固執せず腰を追ってバックを取る。逃げ道を追う攻めで、位置階層をさらに上げられます。",
    },
    {
      jp: "差し込まれた膝盾を腰の外へ潰し、胸圧を戻してから前進する",
      en: "Smash the knee shield outside the hip, restore chest pressure, then climb",
      correct: true,
      requiresAction: ["knee-shield-insert"],
      forbiddenState: ["guard-recovered"],
      stateEffects: { add: ["top-base"], remove: ["knee-shield", "guard-recovered"] },
      next: [{ id: "attack-from-mount", weight: 2 }, { id: "attack-from-side", weight: 1 }],
      reaction: "青は膝盾で距離を作ろうとする。あなたは膝と腰を潰し直し、上位支配を保つ",
      result: { red: "redSideControl", blue: "blueUnderSide", badge: "赤: <b>膝盾を潰し</b>サイドを再固定" },
      feedback:
        "正解。膝盾が入った瞬間に胸だけで登るとガードへ戻されます。膝を腰の外へ潰し、腰を固定してから位置を上げる。",
    },
    {
      jp: "相手を抱きしめて胸だけで押さえ続ける",
      en: "Hug tightly and hold chest-to-chest only",
      correct: false,
      stateEffects: { add: ["knee-shield"], remove: ["top-base"] },
      next: [{ id: "attack-armbar-guard", weight: 1 }, { id: "attack-triangle-guard", weight: 2 }],
      consequence: "青が腰を抜いてガードを戻す。あなたは下からの腕十字や三角の条件を警戒する攻防へ移る",
      result: { red: "redSideControl", blue: "blueShrimpRecover", badge: "青: 空間を作り<b>膝を差す</b>" },
      feedback:
        "悪手。腰を制していないと海老で角度を作られます。胸の圧だけでなく、腰と膝のラインを管理する。",
    },
    {
      jp: "下の腕だけを狙って体重を前に流す",
      en: "Chase only the far arm and let weight drift forward",
      correct: false,
      stateEffects: { add: ["angle-created"], remove: ["top-base"] },
      next: [{ id: "attack-triangle-guard", weight: 2 }, { id: "attack-armbar-guard", weight: 1 }],
      consequence: "体重が前に流れ、青に角度を作られる。ガードからの三角や腕十字を受けやすい",
      result: { red: "redSideControl", blue: "blueShrimpRecover", badge: "青: 腰を抜いて<b>回復</b>" },
      feedback:
        "悪手。腕に集中しすぎると腰の支配が抜け、ガードを戻されます。攻めの前に位置の固定。",
    },
  ],
  principle:
    "<b>位置を上げる攻撃。</b> サイドの価値は極めだけでなく、マウントやバックへ進む足場になること。",
};
