// techniques.js
// 教育コンテンツの核。「一本の連続したロール (スパーリング)」として設計。
// 防御/攻撃/ミックスのフォーカスで、局面列を毎回ランダムに構成する。
//
// role: "defense"(守) / "offense"(攻)。position before submission を攻めパートで体現する。
//
// 局面: { id, role, belt, positionJp/En, term, situation, prompt, readCues, timeLimitSec, pressure, options[], principle }
//   setup  : 局面開始のポーズ { red, blue, badge }
//   attack : 「決断の瞬間」のポーズ (守=相手が仕掛ける / 攻=あなたが前進する)
//   pressure: 判断タイマー中に表示する、相手の能動アクション { early, urgent }
//   opponentActions[]: 同じ局面内で相手が選ぶ初動。style/tactic/weight で出現傾向を変え、必要なら attack pose も上書きする。
//   options[].result = { red, blue, badge }
//   options[].giOnly / nogiOnly : そのモード専用の選択肢 (技セット分岐)。
//   options[].reaction / consequence : 選択後に相手がどう反応し、次局面へ繋がるか。
//   options[].next : "scenario-id" または { id, weight }。自然な展開ほど weight を高くする。
export const SCENARIOS = [
  // === A. バックディフェンス (守) — 首の安全が最優先 =====================
  {
    id: "back-defense",
    role: "defense",
    belt: "白帯",
    positionJp: "バックコントロール (背後を取られた)",
    positionEn: "Back Control — defending",
    term: "背後位 / pegada nas costas",
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
        styles: ["choke-hunter"],
        tactics: ["fast-scramble"],
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
        styles: ["pressure-passer"],
        tactics: ["survive-first"],
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
        styles: ["choke-hunter"],
        tactics: ["survive-first"],
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
  },

  // === B. マウント脱出 / アッパ (守) — 上を奪取し攻めへ転じる ============
  {
    id: "mount-escape",
    role: "defense",
    belt: "白帯",
    positionJp: "マウント (馬乗りされた)",
    positionEn: "Mount — escaping",
    term: "縦四方固め / montada",
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
        styles: ["choke-hunter"],
        tactics: ["submission-chain", "fast-scramble"],
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
        styles: ["pressure-passer"],
        tactics: ["position-ladder"],
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
        styles: ["pressure-passer"],
        tactics: ["survive-first"],
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
  },

  // === C. サイドコントロール脱出 (守) — フレームと海老でガード回復 =======
  {
    id: "side-escape",
    role: "defense",
    belt: "白帯〜青帯",
    positionJp: "サイドコントロール (横から抑えられた)",
    positionEn: "Side Control — escaping",
    term: "横四方固め / cem quilos",
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
        styles: ["pressure-passer"],
        tactics: ["position-ladder"],
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
        styles: ["choke-hunter"],
        tactics: ["submission-chain"],
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
        styles: ["pressure-passer"],
        tactics: ["fast-scramble"],
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
  },

  // === D. クローズドガード内の姿勢防御 — 下からの極めを消してパスへ ====
  {
    id: "closed-guard-posture",
    role: "defense",
    belt: "白帯〜青帯",
    positionJp: "クローズドガード内 (下から捕まった)",
    positionEn: "Inside Closed Guard — posture defense",
    term: "閉じガード内 / dentro da guarda fechada",
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
        styles: ["guard-player"],
        tactics: ["submission-chain"],
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
        styles: ["guard-player"],
        tactics: ["fast-scramble"],
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
        styles: ["guard-player"],
        tactics: ["fast-scramble"],
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
  },
];

export const OFFENSE_SCENARIOS = [
  {
    id: "attack-from-mount",
    role: "offense",
    belt: "白帯",
    positionJp: "マウントからの攻め",
    positionEn: "Mount Offense",
    term: "縦四方固め / montada",
    points: "4点ポジション (最上位の一つ)",
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
        styles: ["pressure-passer"],
        tactics: ["position-ladder"],
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
        styles: ["guard-player"],
        tactics: ["fast-scramble"],
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
  },
  {
    id: "attack-from-back",
    role: "offense",
    belt: "白帯",
    positionJp: "バックからの攻め",
    positionEn: "Back Control Offense",
    term: "背後位 / mata-leão",
    points: "4点ポジション (最上位)",
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
        styles: ["choke-hunter"],
        tactics: ["submission-chain"],
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
        styles: ["pressure-passer"],
        tactics: ["position-ladder"],
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
        feedback:
          "悪手。フックを捨てると相手の腰が逃げます。バックは脚で位置、腕で首を管理する。",
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
  },
  {
    id: "attack-from-side",
    role: "offense",
    belt: "白帯〜青帯",
    positionJp: "サイドコントロールからの攻め",
    positionEn: "Side Control Offense",
    term: "横四方固め / cem quilos",
    points: "ガードパス成立後の支配位置",
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
        styles: ["pressure-passer"],
        tactics: ["position-ladder"],
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
        styles: ["choke-hunter"],
        tactics: ["fast-scramble"],
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
        styles: ["guard-player"],
        tactics: ["position-ladder"],
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
  },
  {
    id: "attack-armbar-guard",
    role: "offense",
    belt: "青帯",
    positionJp: "クローズドガードからの腕十字",
    positionEn: "Closed Guard Armbar",
    term: "閉じガード / juji-gatame",
    points: "下からの代表的サブミッション",
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
        styles: ["guard-player"],
        tactics: ["submission-chain"],
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
        styles: ["choke-hunter"],
        tactics: ["fast-scramble"],
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
        styles: ["pressure-passer"],
        tactics: ["fast-scramble"],
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
  },
  {
    id: "attack-triangle-guard",
    role: "offense",
    belt: "青帯",
    positionJp: "三角絞めへの連携",
    positionEn: "Triangle Choke Offense",
    term: "三角絞め / triângulo",
    points: "下からの代表的サブミッション",
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
        styles: ["guard-player"],
        tactics: ["fast-scramble"],
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
        styles: ["choke-hunter"],
        tactics: ["submission-chain"],
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
        styles: ["pressure-passer"],
        tactics: ["fast-scramble"],
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
        feedback:
          "悪手。三角は首と片腕を挟む技。両腕が外なら絞めの構造ができません。",
      },
    ],
    principle:
      "<b>条件を見て攻める。</b> サブミッションは形だけでなく、相手の腕・首・姿勢の条件が揃って初めて成立する。",
  },
];

// 道場の心得 (フッターで開閉) — 柔術の教育的な普遍原則
export const DOJO_NOTES = {
  hierarchy: [
    "バックコントロール (背後位) — 最上位",
    "マウント (縦四方)",
    "ニーオンベリー (膝を腹に)",
    "サイドコントロール (横四方)",
    "ハーフガード",
    "クローズド/オープンガード — 下だが攻防の起点",
  ],
  principles: [
    ["Position before submission", "極める前に良い位置を取る。位置の優劣が柔術の背骨。"],
    ["テコ > 力", "小さな力で大きな相手を制す。角度・支点・体重移動で勝つ。"],
    ["Defense first", "まず生き残る。守れる人だけが攻められる。"],
    ["攻守は繋がる", "守って終わりではない。脱出は上を取る好機。返したら攻めへ転じる。"],
    ["タップは学び", "関節・絞めは危険。タップ (参った) は敗北でなく安全装置であり次への学び。"],
  ],
  glossary: [
    ["柔術 (じゅうじゅつ)", "日本古来の徒手武術。地上での関節技・絞めを体系化。BJJ の源流。"],
    ["アッパ / Upa", "ブリッジ＆ロール。マウント脱出の基本。"],
    ["海老 / Shrimp", "ヒップエスケープ。腰を抜いて空間と角度を作る最重要動作。"],
    ["マタレオン / Mata-leão", "裸絞め (Rear Naked Choke)。"],
    ["十字固め / Juji-gatame", "アームバー。柔道由来の関節技。"],
    ["ニーオンベリー", "膝を相手の腹に乗せる抑え込み。サイドとマウントの中継点。"],
  ],
  research: [
    [
      "「ポジション優先」は上半身の技でこそ成立",
      "競技 no-gi の動作分析 (Spanias ら 2022) では、位置の優位は上半身のサブミッション (絞め・腕関節) と相関 (r=0.50, p<0.05)。一方ヒールフック等の下半身関節技は位置支配なしでも決まる例が多い。本道場が扱う上半身の攻防は、まさに position before submission が効く領域。",
    ],
    [
      "腕十字は最多の受傷機転 — だから「腕を隔離してから」",
      "競技傷害調査 (Scoggin ら 2014) では整形外科的傷害の 38.9% が肘で、その肘傷害 14 例中 10 例が腕十字 (アームバー) によるもの。攻める側も「位置を固めてから」極めるのが安全。",
    ],
    [
      "早期タップが第一の傷害予防",
      "同調査は「傷害予防の第一段階は選手自身」とし、我慢せず早くタップする文化を推奨。BJJ の競技傷害率は約 9.2/1000 exposure で、MMA (236–286) や柔道より大幅に低い。タップは安全装置。",
    ],
    [
      "教育ツールとしての効果 (学校導入 RCT ほか)",
      "アブダビの学校での BJJ プログラム RCT では児童の認知的自己制御・教室での行動の改善が報告。意思決定・問題解決・レジリエンス育成に寄与する教育的価値が学術的に検討されている。",
    ],
    [
      "現代の指導法: 状況から解を発見する (Constraints-Led / 生態学的アプローチ)",
      "孤立した型の反復より、制約を与えた状況スパーで解を自ら発見させる方が技能の定着・転移・創造的問題解決に優れるとされる。本ゲームの「連続したロールの中で判断する」形式はこの思想に沿う。",
    ],
  ],
};
