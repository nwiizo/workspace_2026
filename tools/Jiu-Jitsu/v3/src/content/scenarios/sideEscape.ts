import type { Scenario } from "../types";

// C. サイドコントロール脱出 (守) — フレームと海老でガード回復
export const sideEscape: Scenario = {
  id: "side-escape",
  role: "defense",
  belt: "白帯〜青帯",
  positionJp: "サイドコントロール (横から抑えられた)",
  positionEn: "Side Control — escaping",
  term: "横四方固め / cem quilos",
  focusJoints: ["shoulder"],
  stateBias: ["frame-lost", "knee-shield"],
  setup: { red: "redSideControl", blue: "blueUnderSide", badge: "赤: <b>横四方</b>で抑え込み" },
  attack: { red: "redSideControl", blue: "blueUnderSide", badge: "赤: 体重をかけ<b>キムラ</b>を窺う" },
  timeLimitSec: 9,
  pressure: {
    early: "赤が胸圧を強め、下の腕を孤立させようとしている",
    urgent: "腰が止められ、腕を取られ始めている。空間を作る必要がある",
  },
  opponentActions: [
    {
      id: "crossface-pressure",
      label: "肩圧で顔を向ける",
      cue: "首フレームを失う前に顔と腰を同じ方向へ戻す",
      weight: 2,
      attack: { red: "redSideControl", blue: "blueUnderSide", badge: "赤: <b>肩圧</b>で顔と腰を分断" },
      readCues: ["首フレーム", "肩圧", "腰"],
      pressure: {
        early: "赤が肩で顔を反対へ向け、腰の動きを止めようとする",
        urgent: "首と腰が分断され、膝を戻す空間が消えかけている",
      },
    },
    {
      id: "kimura-threat",
      label: "下の腕を孤立",
      cue: "下の腕を相手の下に差さず、腰フレームを保つ",
      weight: 1,
      attack: { red: "redSideControl", blue: "blueUnderSide", badge: "赤: 下の腕を浮かせ<b>キムラ</b>へ" },
      readCues: ["下の腕", "腰フレーム", "膝"],
      pressure: {
        early: "赤が下の腕を浮かせ、キムラの支点を探している",
        urgent: "肩のラインを取られ始めている。腕を相手の下へ差すと危険",
      },
    },
    {
      id: "overcommit-pressure",
      label: "胸圧を前へ流す",
      cue: "圧が前へ流れた瞬間に首と腰を同じ方向へずらす",
      weight: 1,
      attack: { red: "redSideControl", blue: "blueShrimpRecover", badge: "赤: 胸圧を前へ流し、青は<b>角度</b>を探す" },
      readCues: ["圧の方向", "首", "腰"],
      pressure: {
        early: "赤の胸圧が前へ流れ、体重が一方向に寄っている",
        urgent: "圧の方向を読めないと首と腰が分断されたまま潰される",
      },
    },
  ],
  readCues: ["首フレーム", "腰フレーム", "膝の差し込み"],
  situation:
    "アッパで一度は返したものの、攻防の中で赤がガードを越え、横から抑え込んできました。胸で胸を潰され、動きを止められる前に動きたい局面です。",
  prompt: "ガードを取り戻すには？",
  options: [
    {
      jp: "首と腰にフレーム (肘・前腕の支え) を作り、海老で腰を引いて膝を割り込ませる",
      en: "Build frames, shrimp (hip escape), insert the knee",
      correct: true,
      stateEffects: { add: ["guard-recovered"], remove: ["frame-lost", "arm-exposed"] },
      next: [
        { id: "attack-armbar-guard", weight: 2 },
        { id: "attack-triangle-guard", weight: 2 },
        { id: "back-defense", weight: 1 },
      ],
      reaction: "赤は膝を戻される前に再び圧をかける。あなたはガードから攻めるか、首を守る局面へ戻される",
      result: { red: "redSideControl", blue: "blueShrimpRecover", badge: "青: <b>ガードリカバリ</b> ▸ ロールを生き延びた" },
      feedback:
        "正解。フレームで相手との間に空間を作り、海老 (シュリンプ) で腰を後ろへ抜きながら膝を相手と自分の間に差し込む。空間 → 膝 → ガードの順。これがガードリカバリの王道で、ロールを生き延びる基礎です。",
    },
    {
      jp: "相手を抱きしめて密着し、力で押し返す",
      en: "Hug them tight and muscle them back",
      correct: false,
      stateEffects: { add: ["frame-lost"], remove: ["guard-recovered"] },
      next: [{ id: "mount-escape", weight: 2 }, { id: "back-defense", weight: 1 }],
      consequence: "フレームが消えて赤の圧が通る。赤はマウントへ上がるか、首のラインを狙いやすくなる",
      result: { red: "redSideControl", blue: "blueUnderSide", badge: "赤: さらに<b>圧迫</b>を強める" },
      feedback:
        "悪手。抱え込むと自分のフレーム (突っ張り) が消え、相手の体重をもろに受けます。柔術では「抱きしめる」より「突っ張って空間を作る」。",
    },
    {
      jp: "下の手を相手の体の下に深く差し込んで引き寄せる",
      en: "Reach your bottom arm under them",
      correct: false,
      stateEffects: { add: ["arm-exposed"], remove: ["guard-recovered"] },
      next: ["mount-escape", "side-escape"],
      consequence: "差した腕が孤立し、赤は肩か上位支配を継続する。再び横かマウント下から守る展開になる",
      result: { red: "redSideControl", blue: "blueUnderSide", badge: "赤: 差した腕に<b>キムラ</b>" },
      feedback:
        "悪手。下から腕を差すとキムラ (肩関節技) やアームトラップの的になります。サイド下では腕を相手の下に通さないのが鉄則。",
    },
  ],
  principle:
    "<b>フレームと空間。</b> 抑え込みからの脱出は力で押すのでなく、骨格のフレームで space を作り、海老で角度を変える。守れる人だけが、次に攻められる。",
};
