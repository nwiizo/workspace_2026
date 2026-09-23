import type { Scenario } from "../types";

export const attackFromMount: Scenario = {
  id: "attack-from-mount",
  role: "offense",
  belt: "白帯",
  positionJp: "マウントからの攻め",
  positionEn: "Mount Offense",
  term: "縦四方固め / montada",
  focusJoints: ["elbow"],
  stateBias: ["top-base", "arm-exposed"],
  setup: { red: "redMountTop", blue: "blueUnderMount", badge: "赤: <b>マウント</b>から攻めを組み立てる" },
  attack: { red: "redMountArmbar", blue: "blueUnderMount", badge: "赤: 腕を隔離し<b>腕十字</b>へ" },
  timeLimitSec: 8,
  pressure: {
    early: "青が橋を作る準備をし、肘を体へ戻そうとしている",
    urgent: "青の腰が跳ねる。位置を失う前に攻めを組み立てる",
  },
  opponentActions: [
    {
      id: "elbow-hide",
      label: "青が肘を戻す",
      cue: "胸圧で肘を体から剥がしてから極めへ進む",
      weight: 1,
      attack: { red: "redMountArmbar", blue: "blueUnderMount", badge: "青が肘を戻す前に赤が<b>胸圧</b>で剥がす" },
      readCues: ["胸圧", "肘", "ベース"],
      pressure: {
        early: "青が肘を肋骨へ戻し、橋であなたのベースをずらそうとしている",
        urgent: "青の腰が跳ねる。先に胸圧で肘を剥がす必要がある",
      },
    },
    {
      id: "bridge-threat",
      label: "青が橋を作る",
      cue: "腰が跳ねる方向を読み、ベースを残してから攻める",
      weight: 1,
      attack: { red: "redMountTop", blue: "blueUnderMount", badge: "青が<b>橋</b>を作り、赤はベースを保つ" },
      readCues: ["同側の足", "腰", "肘"],
      pressure: {
        early: "青が片側へ橋を作り、あなたの膝と手のベースを崩しにくる",
        urgent: "体重が流れる。極めだけ追うと返される",
      },
    },
  ],
  readCues: ["胸圧", "肘", "ベース"],
  situation:
    "あなた (赤) はマウントで上。青は腕を縮めて守りながら脱出の橋を狙っています。腕力で押さえ込むだけでは返されます。",
  prompt: "攻めを成立させる正しい順序は？",
  options: [
    {
      jp: "胸で圧をかけて姿勢を崩し、肘を体から離してから腕十字へ移る",
      en: "Break posture, isolate the elbow, then attack the armbar",
      correct: true,
      forbiddenAction: ["bridge-threat"],
      next: [{ id: "attack-armbar-guard", weight: 1 }, { id: "attack-from-back", weight: 2 }],
      reaction: "青は腕を戻すか背を向けて逃げる。あなたは腕を追うか、背中への支配へ切り替える",
      result: { red: "redMountArmbar", blue: "blueUnderMount", badge: "赤: <b>腕を隔離</b>して攻撃継続" },
      feedback:
        "正解。上の支配は「相手の防御構造を崩してから極める」。腕だけを引っ張らず、胸圧と角度で肘を体から離してから腕十字へ移る。",
    },
    {
      jp: "橋の方向へ手足のベースを置き、腰を沈め直してから肘を隔離する",
      en: "Post toward the bridge, settle your hips again, then isolate the elbow",
      correct: true,
      requiresAction: ["bridge-threat"],
      stateEffects: { add: ["top-base"], remove: ["arm-exposed", "back-exposed"] },
      next: [{ id: "attack-from-side", weight: 1 }, { id: "attack-from-back", weight: 2 }],
      reaction: "青は橋を潰されて肘を戻すか背を向ける。あなたはベースを失わず、腕か背中への攻めへ戻る",
      result: { red: "redMountTop", blue: "blueUnderMount", badge: "赤: <b>橋を潰し</b>ベースを回復" },
      feedback:
        "正解。橋が始まった瞬間に腕十字へ飛ぶと返されます。まず橋の方向へベースを置き、骨盤を沈め直してから肘を隔離する。",
    },
    {
      jp: "いきなり両手で相手の腕を引っ張り上げる",
      en: "Yank both arms upward immediately",
      correct: false,
      next: [{ id: "attack-from-side", weight: 1 }, { id: "attack-from-back", weight: 2 }],
      consequence: "青に橋で崩される。あなたはスクランブルで横や背中への支配を取り直す必要がある",
      result: { red: "redRolledBottom", blue: "blueUpaTop", badge: "青: 橋で<b>返す</b>" },
      feedback:
        "悪手。上半身だけで引くとベースが浮き、アッパで返されます。マウント攻撃は自分のベースを失わないことが前提。",
    },
    {
      jp: "首だけを狙って前のめりに体重を預ける",
      en: "Lean forward and chase only the neck",
      correct: false,
      next: ["attack-from-side", "attack-armbar-guard"],
      consequence: "前のめりで腰が軽くなり、青が逃げる。あなたは横から潰すか、ガード攻防へ切り替える",
      result: { red: "redRolledBottom", blue: "blueUpaTop", badge: "青: 体重移動を使い<b>脱出</b>" },
      feedback:
        "悪手。前のめりになると腰が軽くなり、相手の橋とロールに乗せられます。極める前に安定した位置を保つ。",
    },
  ],
  principle:
    "<b>Position before submission.</b> 攻める側も、極めだけを急ぐと位置を失う。支配を保ってから関節を孤立させる。",
};
