import type { PracticeLesson } from "./practice";

export const ADVANCED_PRACTICE: PracticeLesson[] = [
  {
    id: "mount-attack", title: "マウントから片腕を分ける", position: "マウントの上", goal: "上を保ち、腕十字へ進む前の片腕の保持まで確認する。",
    principle: "上を安定させる → 肘を分ける → 両手で位置を保つ。", transfer: "次はルート確認の「サイド → マウント → 腕十字」で段階のつながりを見る。",
    steps: ["上を安定させる", "片腕を分ける", "位置を保つ"], starts: ["start", "pressure"],
    actions: [
      { id: "base", label: "膝と腰を整える", description: "相手の腰の動きに合わせて上の支えを整える。", blocked: "土台はある。次は相手の肘と、上の位置を同時に見る。" },
      { id: "isolate", label: "片肘を胴から分ける", description: "上の支えを残して、片腕の保持へ進む。", blocked: "まだ上が不安定。先に腰と膝の位置を整える。" },
      { id: "hold", label: "両手で腕の位置を保つ", description: "肘を伸ばし切らず、次の脚の位置を考える。", blocked: "片腕の位置をまだ確保できていない。" },
      { id: "rush", label: "すぐに寝転んで腕を引く", description: "上の支えを離して腕だけを狙う。", blocked: "位置を失いかけた。支えを整える段階へ戻ろう。" },
    ], nodes: {
      start: { title: "青がマウントに入った", situation: "赤は腰を動かして返そうとしている。", facts: ["青：上", "赤：腰を動かす", "片腕：未保持"], focus: "hip", hint: "まず上の支えを整える。", progress: 0, visual: { family: "mount", state: "mounted", top: "blue" }, moves: { base: { to: "base", feedback: "腰と両膝を整え、腕を見る余裕ができた。" } } },
      pressure: { title: "赤の腕は見えるが土台が不安定", situation: "青は腕を狙える位置にいるが、赤の腰の動きに遅れている。", facts: ["青：上", "腕：浮いている", "土台：先に確認"], focus: "hip", hint: "腕より先に上の支え。", progress: 0, visual: { family: "mount", state: "low", top: "blue" }, moves: { base: { to: "base", feedback: "上を保った。次に片肘が胴から離れる場面を見る。" } } },
      base: { title: "上が整った", situation: "赤の片肘を胴から離す攻防へ進める。", facts: ["土台：あり", "片腕：これから"], focus: "elbow", hint: "片腕を分ける。", progress: 1, visual: { family: "mount", state: "mount-held", top: "blue" }, moves: { isolate: { to: "arm", feedback: "片腕を保持した局面へ進んだ。途中の腕の攻防は段階表示。" } }, setbacks: { rush: { to: "start", feedback: "上の支えを失いかけたため、マウントを整え直す。" } } },
      arm: { title: "片腕を保持した", situation: "青の両手が赤の片腕へ集まっている。", facts: ["青：マウント", "赤の片腕：胴から離れた"], focus: "wrist", hint: "保持位置を確認して区切る。", progress: 2, visual: { family: "mount", state: "isolated", top: "blue" }, moves: { hold: { to: "end", observation: true, feedback: "腕十字へ進む準備まで確認できた。旋回と脚の置き換えは次の段階。" } } },
      end: { title: "極め技の前の保持位置", situation: "肘を伸ばし切らず、この位置で区切る。", facts: ["片腕：保持", "次：身体の向きと脚"], focus: "elbow", hint: "", progress: 3, end: "goal", visual: { family: "mount", state: "isolated", top: "blue" }, moves: {} },
    },
  },
  {
    id: "guard-attack", title: "下から角度を作って攻める", position: "クローズドガードの下", goal: "相手の姿勢と腕を見ながら、腰の角度を作る。",
    principle: "脚の囲い → 腰の角度 → 相手の反応。下からでも攻めは続く。", transfer: "ルート確認で、片腕を分けた後の三角の形と比較する。",
    steps: ["囲いを確認", "腰の角度を作る", "相手の反応を見る"], starts: ["start", "pressure"],
    actions: [
      { id: "control", label: "脚と腕の位置を確認", description: "相手の腰を脚で囲い、両腕の位置を見る。", blocked: "囲いはある。次は腰の角度。" },
      { id: "angle", label: "腰を横へずらす", description: "相手の正面から少し外れて角度を作る。", blocked: "まず脚の囲いと相手の姿勢を確認する。" },
      { id: "read", label: "相手の土台を観察", description: "相手が膝を広げたか、向き直ったかを見る。", blocked: "先に腰の角度を作り、相手の反応を引き出す。" },
      { id: "pull", label: "腕だけ強く引く", description: "腰の角度を変えず、手だけで相手を動かす。", blocked: "腕だけでは角度は変わらない。腰と脚の位置を見直す。" },
    ], nodes: {
      start: { title: "青が下で脚を閉じている", situation: "赤は上で姿勢を保っている。", facts: ["青：下", "赤：上", "脚：腰を囲う"], focus: "hip", hint: "脚と腕の配置を確認。", progress: 0, visual: { family: "guard", state: "stable", top: "red" }, moves: { control: { to: "control", observation: true, feedback: "腰と両腕の位置を確認した。次は角度を変える。" } } },
      pressure: { title: "赤の上体が近づいた", situation: "青が下から上体を近づけた場面。まだ腰の角度はない。", facts: ["赤：前傾", "青の腰：正面"], focus: "hip", hint: "脚と腕の配置を見直す。", progress: 0, visual: { family: "guard", state: "pull", top: "red" }, moves: { control: { to: "control", feedback: "赤が上体を戻した局面へ。姿勢が変わっても腰の角度を観察する。" } } },
      control: { title: "脚の囲いが残っている", situation: "赤の腕と腰の向きを見ながら、青の腰を動かせる。", facts: ["囲い：あり", "腰：正面"], focus: "hip", hint: "腰を横へ。", progress: 1, visual: { family: "guard", state: "stable", top: "red" }, moves: { angle: { to: "angle", feedback: "青の腰が横へずれた。赤は膝の支えを広げようとする。" } } },
      angle: { title: "赤が支え直そうとしている", situation: "青が角度を作ったが、赤は土台を戻そうとしている。", facts: ["青の腰：横", "赤：土台を調整"], focus: "knee", hint: "相手の土台を見る。", progress: 2, visual: { family: "guard", state: "angle", top: "red" }, moves: { read: { to: "end", feedback: "赤が支え直す反応を確認した。必ず返せる、必ず極まるとは決まっていない。" } } },
      end: { title: "角度と反応を読めた", situation: "ここから腕を分けるか、崩し直すかの攻防になる。", facts: ["青：下のまま", "次：相手の反応に合わせる"], focus: "hip", hint: "", progress: 3, end: "goal", visual: { family: "guard", state: "base-angle", top: "red" }, moves: {} },
    },
  },
  {
    id: "mount-balance", title: "返されそうな上を立て直す", position: "マウントの上", goal: "相手が返そうとしたときに、極め技を急がず上を保つ。",
    principle: "相手の腰を見る → 支えを戻す → 腕を急がない。", transfer: "トップの取り合いの例で、支え直せた展開と交代する展開を比べる。",
    steps: ["腰の動きを読む", "支えを戻す", "上を保つ"], starts: ["start", "pressure"],
    actions: [
      { id: "read", label: "腰の動きを見る", description: "相手がどちらへ体重を移したかを見る。", blocked: "反応は見えた。次に支えを戻す。" },
      { id: "base", label: "膝と手の支えを戻す", description: "腕への攻めをいったん止めて、上の位置を整える。", blocked: "まず腰の動きを確認する。" },
      { id: "settle", label: "位置を保って区切る", description: "相手の動きに対応した位置を確認する。", blocked: "支えを戻す段階が先。" },
      { id: "chase", label: "腕を追って上体を伸ばす", description: "相手の腰を見ず、腕への攻めを続ける。", blocked: "相手の腰についていけず、返されやすくなる。" },
    ], nodes: {
      start: { title: "赤がブリッジを始めた", situation: "青は上。赤は腰を上げ、青を横へ返そうとしている。", facts: ["青：上", "赤：腰が上がる"], focus: "hip", hint: "腰の動きを見る。", progress: 0, visual: { family: "mount", state: "mounted", top: "blue" }, moves: { read: { to: "read", observation: true, feedback: "腰の変化を見た。腕への攻めより土台を優先する。" } } },
      pressure: { title: "腕に気を取られた", situation: "青が赤の腕へ注意を向ける間に、赤が腰を動かそうとしている。", facts: ["青：上", "赤の腰：観察が必要"], focus: "hip", hint: "腰の動きを見る。", progress: 0, visual: { family: "mount", state: "low", top: "blue" }, moves: { read: { to: "read", feedback: "相手が返そうとする局面へ。支えを整え直す。" } } },
      read: { title: "支えを戻す場面", situation: "青が上体を伸ばし続けると、赤の動きに遅れる。", facts: ["攻め：いったん止める", "支え：調整する"], focus: "knee", hint: "膝と手で支えを戻す。", progress: 1, visual: { family: "mount", state: "mounted", top: "blue" }, moves: { base: { to: "base", feedback: "青が上を保った局面へ進んだ。必ず返されるわけではない。" } }, setbacks: { chase: { to: "start", feedback: "土台を見失いかけた。腰の動きを読む段階へ戻る。" } } },
      base: { title: "上の支えが戻った", situation: "青はマウントを保ち、赤の腰は床へ戻った。", facts: ["青：トップを維持", "赤：下"], focus: "hip", hint: "位置を確認して区切る。", progress: 2, visual: { family: "mount", state: "mount-held", top: "blue" }, moves: { settle: { to: "end", observation: true, feedback: "上を保てた。次の腕の攻防は、この位置からやり直せる。" } } },
      end: { title: "トップを維持した", situation: "返しへの対応を終えた。", facts: ["上：青のまま", "次：攻め直す"], focus: "hip", hint: "", progress: 3, end: "goal", visual: { family: "mount", state: "mount-held", top: "blue" }, moves: {} },
    },
  },
  {
    id: "back-attack", title: "バックから手の攻防へ", position: "バックを取った側", goal: "相手の防御に合わせて手を持ち替え、保持位置で区切る。",
    principle: "背後の位置 → 防御の手を見る → 持ち替え。", transfer: "攻守を入れ替え、首を守る練習と比較する。",
    steps: ["手の防御を見る", "持ち替える", "位置を確認して止める"], starts: ["start", "pressure"],
    actions: [
      { id: "read", label: "相手の防御の手を見る", description: "首の前にある相手の手を見る。", blocked: "防御は見えた。次は持ち替え。" },
      { id: "switch", label: "手を持ち替える", description: "相手の手に合わせて保持位置を変える。", blocked: "先に防御の手を見る。" },
      { id: "stop", label: "形を確認して止める", description: "締め付けず、この局面で区切る。", blocked: "持ち替えた後の位置を先に見る。" },
      { id: "force", label: "腕だけで押し切る", description: "相手の防御を無視して腕を引く。", blocked: "相手の手を見ずに進めない。保持位置を観察する。" },
    ], nodes: {
      start: { title: "青が背後を取った", situation: "赤は首の前へ手を戻そうとしている。", facts: ["青：背後", "赤：首を守る"], focus: "wrist", hint: "相手の手を見る。", progress: 0, visual: { family: "back", state: "reach", swap: true }, moves: { read: { to: "read", feedback: "赤が防御の手を戻した局面を確認する。" } } },
      pressure: { title: "赤が両手を戻した", situation: "青の最初の攻めは止められた。", facts: ["首の前：赤の手", "青：持ち替えを考える"], focus: "wrist", hint: "防御の手を見る。", progress: 0, visual: { family: "back", state: "defended", swap: true }, moves: { read: { to: "read", observation: true, feedback: "防御の位置を確認した。手を持ち替える攻防へ。" } } },
      read: { title: "持ち替える前", situation: "最初の腕を押さえられたので、もう一方の手の位置を見る。", facts: ["最初の腕：止まる", "もう一方：使える"], focus: "wrist", hint: "手を持ち替える。", progress: 1, visual: { family: "back", state: "defended", swap: true }, moves: { switch: { to: "switch", feedback: "持ち替えた。相手も手を動かすので、常に成功する動作ではない。" } } },
      switch: { title: "持ち替えた後の保持位置", situation: "青と赤の手の位置が変わった。", facts: ["青：持ち替え", "赤：防御を継続"], focus: "wrist", hint: "締め付けず区切る。", progress: 2, visual: { family: "back", state: "switch", swap: true }, moves: { stop: { to: "end", observation: true, feedback: "手の攻防を観察して終了。タップはどの段階でも使える。" } } },
      end: { title: "手の攻防を確認した", situation: "絞め切らずに練習を区切った。", facts: ["青：背後のまま", "次：攻守を入れ替えて比較"], focus: "wrist", hint: "", progress: 3, end: "goal", visual: { family: "back", state: "switch", swap: true }, moves: {} },
    },
  },
];
