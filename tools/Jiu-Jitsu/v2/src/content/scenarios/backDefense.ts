import type { Scenario } from "../types";

// A. バックディフェンス (守) — 首の安全が最優先
export const backDefense: Scenario = {
  id: "back-defense",
  role: "defense",
  belt: "白帯",
  positionJp: "バックコントロール (背後を取られた)",
  positionEn: "Back Control — defending",
  term: "背後位 / pegada nas costas",
  focusJoints: ["neck"],
  stateBias: ["neck-exposed", "back-exposed"],
  setup: { red: "redBackControl", blue: "blueSeatedFront", badge: "赤: <b>バック</b>確保＋両フック" },
  attack: { red: "redBackControl", blue: "blueSeatedFront", badge: "赤: <b>裸絞め</b>を狙い首へ" },
  timeLimitSec: 8,
  pressure: {
    early: "赤が片手を首元へ滑り込ませ、防御手を剥がしにくる",
    urgent: "絞め腕が顎下に入り始めている。首を守る判断が遅い",
  },
  opponentActions: [
    {
      id: "choke-hand-entry",
      label: "絞め手を入れる",
      cue: "顎下へ入る手を最優先で止め、腰より先に首を守る",
      weight: 2,
      attack: { red: "redBackControl", blue: "blueTapped", badge: "赤: 初動で<b>絞め手</b>を顎下へ" },
      readCues: ["首", "防御手", "頭の位置"],
      pressure: {
        early: "赤が絞め手を顎下へ差し込み、防御手を一枚ずつ剥がす",
        urgent: "顎下に腕が入り、首の逃げ道が狭くなっている",
      },
    },
    {
      id: "hook-ride",
      label: "腰のフックで追う",
      cue: "首を閉じたまま、フックの弱い側へ腰をずらす",
      weight: 1,
      attack: { red: "redBackControl", blue: "blueBackDefend", badge: "赤: <b>腰のフック</b>で背中を追う" },
      readCues: ["腰", "フック", "弱い側"],
      pressure: {
        early: "赤が脚フックであなたの腰を引き戻し、背中を保とうとする",
        urgent: "腰を固定され、首と背中の両方を守る必要がある",
      },
    },
    {
      id: "seatbelt-tighten",
      label: "シートベルトで固定",
      cue: "首を閉じたまま肩を床へ戻し、密着の線をずらす",
      weight: 1,
      attack: { red: "redBackControl", blue: "blueBackDefend", badge: "赤: <b>シートベルト</b>で肩を固定" },
      readCues: ["肩", "シートベルト", "腰"],
      pressure: {
        early: "赤がシートベルトを締め、肩を背中側へ引き戻そうとする",
        urgent: "肩と腰を固定されると、首を守っても脱出方向が消える",
      },
    },
  ],
  readCues: ["首", "防御手", "腰の逃げ道"],
  situation:
    "ロール開始。赤があなた(青)の背後を取り、両足のフックを入れ、腕を首に回してきます。バックは階層最上位。裸絞め (マタレオン) は数秒で効きます。",
  prompt: "今この瞬間、最優先で守るべきは？",
  options: [
    {
      jp: "顎を引き、両手で首と襟を防御。頭を絞め腕側へ寄せ、弱い側へ尻を抜き始める",
      en: "Chin down, hand-fight the choke, escape to the weak side",
      correct: true,
      giOnly: true,
      forbiddenAction: ["hook-ride", "seatbelt-tighten"],
      stateEffects: { add: ["neck-safe"], remove: ["neck-exposed", "back-exposed"] },
      next: [{ id: "mount-escape", weight: 3 }, { id: "side-escape", weight: 2 }],
      reaction: "赤は絞めを諦めず上から追い、あなたの脱出方向に合わせてマウント/サイドへ圧を変える",
      result: { red: "redBackControl", blue: "blueBackDefend", badge: "青: <b>首を守り</b>脱出開始 ▸ 局面を引き戻す" },
      feedback:
        "正解。バックディフェンスの鉄則は「まず首を守る (Defense first)」。顎を引いて絞め腕の挿入を防ぎ、頭を絞め腕側へ。背中をマットへ着けて弱い側へ抜けると、相手は上下を保てずマウントの攻防へ移ります。",
    },
    {
      jp: "顎を引き、両手で絞め腕の手首と前腕をつかむ。頭を絞め腕側へ寄せ、弱い側へ尻を抜き始める",
      en: "Chin down, two-on-one the choking wrist and forearm, escape to the weak side",
      correct: true,
      nogiOnly: true,
      forbiddenAction: ["hook-ride", "seatbelt-tighten"],
      stateEffects: { add: ["neck-safe"], remove: ["neck-exposed", "back-exposed"] },
      next: [{ id: "mount-escape", weight: 3 }, { id: "side-escape", weight: 2 }],
      reaction: "赤は手首を切られても密着を保ち、あなたの腰逃げに合わせてマウント/サイドへ圧を変える",
      result: { red: "redBackControl", blue: "blueBackDefend", badge: "青: <b>首を守り</b>脱出開始 ▸ 局面を引き戻す" },
      feedback:
        "正解。ノーギでは襟を使えないため、絞め腕の手首と前腕を二対一で止める。顎を引き、頭を絞め腕側へ寄せてから弱い側へ腰を抜く。",
    },
    {
      jp: "首の防御手を残したまま弱い側のフックを外し、肩をマットへ戻して背中の密着を切る",
      en: "Keep the neck hand in place, clear the weak-side hook, and put your shoulder to the mat",
      correct: true,
      requiresAction: ["hook-ride"],
      stateEffects: { add: ["neck-safe"], remove: ["neck-exposed", "back-exposed"] },
      next: [{ id: "side-escape", weight: 3 }, { id: "mount-escape", weight: 2 }],
      reaction: "赤は腰の追尾を失い、上から押さえ直すためにサイド/マウントへ切り替える",
      result: { red: "redBackControl", blue: "blueBackDefend", badge: "青: <b>フックを外し</b>肩を床へ戻す" },
      feedback:
        "正解。首を空けずに、相手が追ってくるフック側を先に弱くする。肩をマットへ戻すと背中の密着が切れ、相手はバックを保つより上から押さえ直す展開になります。",
    },
    {
      jp: "顎を閉じて上側の腕を二対一で止め、肩を床へ戻しながら腰をシートベルトの外へずらす",
      en: "Close the neck, two-on-one the top arm, and slide your hips outside the seatbelt line",
      correct: true,
      requiresAction: ["seatbelt-tighten"],
      stateEffects: { add: ["neck-safe"], remove: ["neck-exposed", "back-exposed"] },
      next: [{ id: "mount-escape", weight: 2 }, { id: "side-escape", weight: 2 }],
      reaction: "赤は肩の固定を失い、バックを捨てて上の圧へ移るか、絞め手を作り直そうとする",
      result: { red: "redBackControl", blue: "blueBackDefend", badge: "青: <b>肩を床へ</b>戻し固定を切る" },
      feedback:
        "正解。シートベルトを締められた時は、首だけでなく肩の線を戻す必要があります。上側の腕を止め、肩を床へ戻しながら腰をずらすと、背中を固定され続ける形を避けられます。",
    },
    {
      jp: "相手のフック (足) を先に外そうと両手を下げる",
      en: "Drop both hands to strip the leg hooks first",
      correct: false,
      stateEffects: { add: ["neck-exposed"], remove: ["neck-safe"] },
      next: [{ id: "mount-escape", weight: 2 }, { id: "side-escape", weight: 1 }],
      consequence: "首を守れず赤に主導権が残る。タップ後の再開でも上から圧を受ける局面になりやすい",
      result: { red: "redBackControl", blue: "blueTapped", badge: "赤: <b>裸絞め</b>成功 (タップ)" },
      feedback:
        "悪手。首を空けて手を下げた瞬間に絞め腕が入りタップに至ります。順序が逆。<b>首 → 上半身 → 下半身</b>の順に守るのが原則。",
    },
    {
      jp: "体を前に倒して一気に立ち上がろうとする",
      en: "Lurch forward and try to stand up",
      correct: false,
      stateEffects: { add: ["neck-exposed", "back-exposed"], remove: ["neck-safe"] },
      next: [{ id: "back-defense", weight: 3 }, { id: "mount-escape", weight: 1 }],
      consequence: "前傾で首を差し出し、赤は絞めか上への追いかけを継続する。次も首か上位支配から守る展開になる",
      result: { red: "redBackControl", blue: "blueTapped", badge: "赤: 絞めが<b>深く</b>決まる" },
      feedback:
        "悪手。前傾は首を相手の腕に差し出す形。フックされた相手はついてきて、むしろ絞めが深くなります。",
    },
  ],
  principle:
    "<b>首を守ることがバックの全て。</b> 守れたら慌てず相手の体勢を崩し、局面を引き戻す。守りは攻めへの入口。",
};
