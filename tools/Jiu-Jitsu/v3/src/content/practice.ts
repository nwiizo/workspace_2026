import type { AnatomyJointId } from "../anatomy/types";
import { ADVANCED_PRACTICE } from "./advancedPractice";
import { SITUATIONAL_PRACTICE } from "./situationalPractice";

export interface PracticeAction {
  id: string;
  label: string;
  description: string;
  blocked: string;
}

export interface PracticeNode {
  visual?: { family: "mount" | "guard" | "side" | "back" | "half" | "turtle" | "open"; state: string; top?: "blue" | "red"; swap?: boolean };
  title: string;
  situation: string;
  facts: readonly string[];
  focus: AnatomyJointId;
  hint: string;
  progress: number;
  end?: "goal" | "safety";
  moves: Record<string, { to: string; feedback: string; observation?: boolean }>;
  setbacks?: Record<string, { to: string; feedback: string }>;
}

export interface PracticeLesson {
  id: string;
  title: string;
  position: string;
  goal: string;
  principle: string;
  transfer: string;
  steps: readonly string[];
  actions: readonly PracticeAction[];
  starts: readonly [string, string];
  nodes: Record<string, PracticeNode>;
  source?: { title: string; url: string; section: string; checked: string; scope: string };
}

export const PRACTICE_LESSONS: readonly PracticeLesson[] = [
  {
    id: "side-space", title: "まず、動ける隙間を作る", position: "サイドコントロールの下",
    goal: "相手との間に膝を戻し、ガードを回復する。",
    principle: "支えを作る → 腰を離す → 膝を戻す。膝を急いでも、通る空間がなければ戻らない。",
    transfer: "道場では、横から抑えられたときに前腕の支えと腰の空間を観察してみよう。",
    steps: ["前腕で支える", "腰の空間を作る", "膝を戻す"],
    actions: [
      { id: "frame", label: "前腕で支える", description: "肘を体の近くに置き、相手の肩と腰を支える。", blocked: "支えはできている。次はその空間を使って、自分の腰を動かそう。" },
      { id: "hip", label: "腰を引く", description: "海老（エビ）：横を向き、自分の腰を相手から離す。", blocked: "支えがないまま腰だけ動かすと、相手の胸がついてきて空間が消える。" },
      { id: "knee", label: "膝を間に戻す", description: "自分と相手の胴の間へ膝を入れる。", blocked: "膝の通り道がまだない。腕の支えと、腰の空間を先に確かめよう。" },
      { id: "push", label: "腕を伸ばして押す", description: "相手の胸を押し、自分から遠ざける。", blocked: "赤は伸びた腕を外へ流し、胸を寄せた。腕力で押し続けるより、肘を近くに戻して支えよう。" },
    ], starts: ["pinned", "heavy"], nodes: {
      pinned: { title: "胸が近く、腰が動かない", situation: "赤が横から胸を重ねている。あなたの脚は相手の外にある。", facts: ["前腕の支え：なし", "腰の隙間：なし", "膝：相手の外"], focus: "elbow", hint: "まず肘を近くに置き、前腕を二人の間の支えにする。", progress: 0, moves: { frame: { to: "framed", feedback: "前腕が支えになった。赤は胸を寄せようとするが、小さな隙間が残っている。" } } },
      heavy: { title: "赤が腰まで追いかける", situation: "赤は胸を寄せ、あなたの腰の動きにもついてくる。", facts: ["前腕の支え：なし", "赤：腰を追う", "膝：相手の外"], focus: "elbow", hint: "最初は前腕の支え。相手の反応を見て、その支えを保とう。", progress: 0, moves: { frame: { to: "pressured", feedback: "支えを作った。赤は上から体重を移し、あなたの肘を外へずらしにきた。" } } },
      framed: { title: "支えができた", situation: "赤の胸が前腕で止まっている。脚を戻すには、もう少し腰の空間がいる。", facts: ["前腕の支え：あり", "腰の隙間：小さい", "膝：まだ外"], focus: "hip", hint: "腕を伸ばさず、支えを残して自分の腰を引く。", progress: 1, moves: { hip: { to: "space", feedback: "腰を引くと胸の圧から少し外れた。赤が追いかける前に膝を差し込める。" } } },
      pressured: { title: "支えを潰されかけた", situation: "赤が体重を移し、あなたの肘が外へ流れた。さっきの隙間が消えかけている。", facts: ["前腕の支え：崩れかけ", "赤：胸を寄せ直す", "腰の隙間：なし"], focus: "elbow", hint: "手順を急がず、肘を戻して前腕の支えを作り直す。", progress: 1, moves: { frame: { to: "framed", feedback: "肘を戻して支え直した。赤の胸が止まり、今度は腰を動かせる。" } } },
      space: { title: "膝の通り道が開いた", situation: "青の腰が赤の胸の下から離れた。赤は距離を詰め直そうとしている。", facts: ["前腕の支え：あり", "腰の隙間：あり", "膝：差し込める"], focus: "knee", hint: "作った空間へ膝を戻し、脚を二人の間の壁にする。", progress: 2, moves: { knee: { to: "recovered", feedback: "膝が間に入り、脚で距離を保てた。ここでは両脚を戻してガードを回復。まだ下だが、横からの抑え込みを外せた。" } }, setbacks: { push: { to: "pinned", feedback: "伸ばした腕を赤が外へ流し、胸を寄せ直した。前腕の支えと腰の空間を失ったので、支えを作るところからやり直そう。" } } },
      recovered: { title: "ガードを回復した", situation: "青の脚が赤との間に戻った。ここでいったん区切り、同じ開始位置からまた試せる。", facts: ["脚が間にある", "胸の抑え込み：解除", "青：ガードの下"], focus: "knee", hint: "", progress: 3, end: "goal", moves: {} },
    },
  },
  {
    id: "mount-base", title: "返す前に、支えを見る", position: "マウントの下",
    goal: "相手の支え方に合わせて、マウントから脱出する。",
    principle: "ブリッジは、相手が手足をつけるかで結果が変わる。高いマウントでは膝肘の空間を探す。",
    transfer: "相手の膝の高さと、返す側の手足が床につけるかを指導者と確認しよう。",
    steps: ["肘を守る", "相手の支えを読む", "脱出する"],
    actions: [
      { id: "elbows", label: "肘を体に戻す", description: "腕を伸ばさず、肘を胴の近くへ戻す。", blocked: "肘は戻っている。赤の膝の高さと、床につく手足を見よう。" },
      { id: "trap", label: "同じ側の手足を止める", description: "返す側の腕と足を一緒に押さえる。", blocked: "この形では手足をまとめて止められない。肘の位置と、相手の膝の高さを確認しよう。" },
      { id: "bridge", label: "ブリッジで返す", description: "腰を上げ、支えを止めた側へ回る。", blocked: "赤が手か足を床について残った。腰を上げるだけでは、相手の支えは消えない。" },
      { id: "knee", label: "肘と膝を近づける", description: "横を向き、脚を戻すための空間を探す。", blocked: "まず肘を守ろう。この練習では、今示されている相手の支え方に合わせて脱出する。" },
    ], starts: ["low", "high"], nodes: {
      low: { title: "低いマウントで腕が浮いた", situation: "赤の腰はあなたの腰の近く。片腕を取られる前に、自分の肘を戻したい。", facts: ["肘：体から離れた", "赤の膝：腰の近く", "赤の手足：自由"], focus: "elbow", hint: "相手を押し上げず、まず自分の肘を戻す。", progress: 0, moves: { elbows: { to: "read-low", feedback: "肘を戻せた。赤は低い位置に残り、片側の腕と足を押さえられる距離にいる。" } } },
      high: { title: "膝が脇へ上がってくる", situation: "赤は高いマウントへ移動中。あなたの肘が持ち上がり始めた。", facts: ["肘：浮き始め", "赤の膝：脇へ移動中", "赤の重心：胸側"], focus: "elbow", hint: "膝が脇へ入り切る前に肘を戻す。", progress: 0, moves: { elbows: { to: "read-high", feedback: "肘を戻して膝の上昇を止めた。赤は胸側に残っていて、低いマウントと同じ返し方はしづらい。" } } },
      "read-low": { title: "返す側に支えが残っている", situation: "赤は低いマウントのまま。片側の腕と同じ側の足を止められる。", facts: ["肘：体の近く", "赤の膝：低い", "返す側の支え：残っている"], focus: "ankle", hint: "ブリッジの前に、同じ側の腕と足を止める。", progress: 1, moves: { trap: { to: "trapped", feedback: "片側の腕と足を止めた。赤はその側に手足をついて支えにくくなった。" } } },
      "read-high": { title: "相手の重心が胸側にある", situation: "赤の膝は高めで、手も遠くにつける。腰だけを上げても赤の重心まで届きにくい。", facts: ["肘：体の近く", "赤の膝：高め", "膝肘の空間：作れる"], focus: "hip", hint: "膝肘エスケープへ切り替え、脚を戻す空間を作る。", progress: 1, moves: { knee: { to: "knee-space", feedback: "横を向き、肘と膝の間に赤の脚を捉えた。上を返す代わりに、自分の脚を間へ戻す道ができた。" } } },
      trapped: { title: "同じ側の支えを止めた", situation: "赤の片腕と同じ側の足が止まっている。止めた側へ体重を移せる。", facts: ["片腕：止めた", "同側の足：止めた", "赤の膝：低い"], focus: "hip", hint: "止めた側へブリッジで回る。首だけに体重を乗せない。", progress: 2, moves: { bridge: { to: "top", feedback: "上下が入れ替わった。赤は脚であなたを囲んだので、次はガード内の上。マウントを取ったわけではない。" } } },
      "knee-space": { title: "脚を戻す隙間ができた", situation: "肘と膝で作った隙間から、青の脚が赤の脚の内側へ入り始めた。", facts: ["横向き：できた", "膝肘の空間：あり", "脚：内側へ戻し中"], focus: "knee", hint: "膝肘の動きを続けて脚を間に戻す。", progress: 2, moves: { knee: { to: "guard", feedback: "脚を戻してガードを回復した。実際はハーフガードを経ることもある。今回は脚を戻し切るまでを一手として示している。" } } },
      top: { title: "ガード内の上へ脱出", situation: "青が上になった。赤の脚はまだ青の腰を囲んでいる。", facts: ["マウント下：脱出", "青：ガード内の上", "パス：これから"], focus: "hip", hint: "", progress: 3, end: "goal", moves: {} },
      guard: { title: "脚を戻して脱出", situation: "青は下のまま、ガードを回復した。脱出の目的は上を取ることだけではない。", facts: ["マウント下：脱出", "青：ガードの下", "脚が間にある"], focus: "knee", hint: "", progress: 3, end: "goal", moves: {} },
    },
  },
  {
    id: "guard-posture", title: "上でも、まだ自由ではない", position: "クローズドガード内の上",
    goal: "相手の崩しに合わせて、姿勢と両肘を保つ。",
    principle: "上にいるだけでは支配できない。脚に囲まれている間は、姿勢・肘・相手の角度を見る。",
    transfer: "ガードの上下を入れ替えて、下の人が脚と角度で何を止めているか見てみよう。",
    steps: ["土台を整える", "崩しに対応する", "両肘を守る"],
    actions: [
      { id: "base", label: "膝で土台を整える", description: "膝と腰の位置を整え、体が流れないようにする。", blocked: "土台は整っている。赤が頭を引くのか、腰を横へ動かすのかを見よう。" },
      { id: "posture", label: "上体を起こす", description: "腰の上に頭を戻し、姿勢を取り戻す。", blocked: "土台や相手との向きが整っていない。上体だけ起こそうとしても崩される。" },
      { id: "square", label: "相手に向き直る", description: "相手の腰の動きに合わせて正面へ向き直る。", blocked: "今は向き直るより、頭と腰の位置を戻す必要がある。" },
      { id: "elbows", label: "両肘を内側に保つ", description: "腕を一本だけ脚の間に残さず、肘を体の近くに置く。", blocked: "肘だけ引いても姿勢の崩れは残る。まず土台と、頭・腰の向きを整えよう。" },
    ], starts: ["pull", "angle"], nodes: {
      pull: { title: "赤が頭を引いてくる", situation: "青は上にいるが、赤の脚が腰を囲んでいる。頭を引かれ、上体が前へ流れた。", facts: ["脚の囲い：閉じている", "頭：腰より前", "土台：不安定"], focus: "hip", hint: "上体を反らす前に膝と腰の土台を整える。", progress: 0, moves: { base: { to: "base-pull", feedback: "膝と腰が安定した。赤は頭を引き続けるが、腰まで前へ流れなくなった。" } } },
      angle: { title: "赤が腰を横へずらす", situation: "赤は脚を閉じたまま腰を横へ動かし、青の片腕を狙っている。", facts: ["脚の囲い：閉じている", "赤の腰：横へ移動", "土台：不安定"], focus: "hip", hint: "まず土台を作り、相手が作った角度を観察する。", progress: 0, moves: { base: { to: "base-angle", feedback: "土台はできたが、赤の腰は横へずれたまま。正面を向いたつもりでも、片腕に角度を作られている。" } } },
      "base-pull": { title: "腰は止まり、頭が前にある", situation: "赤は頭を引いている。膝と腰の土台を使い、頭を腰の上へ戻せる。", facts: ["土台：安定", "赤の腰：正面", "頭：前へ引かれる"], focus: "hip", hint: "整えた土台の上へ上体を戻す。", progress: 1, moves: { posture: { to: "upright", feedback: "上体を起こせた。赤は次に腕を一本だけ脚の内側へ残そうとしている。" } } },
      "base-angle": { title: "赤が片腕へ角度を作った", situation: "赤の腰が横へずれ、青の片腕の外側に回り込んでいる。", facts: ["土台：安定", "赤の腰：横向き", "片腕：狙われている"], focus: "hip", hint: "上体を起こすだけでなく、赤の腰へ向き直って角度を消す。", progress: 1, moves: { square: { to: "upright", feedback: "赤の腰に向き直り、姿勢を戻した。片腕の外側へ回り込む角度を小さくできた。" } } },
      upright: { title: "姿勢が戻った。次は肘", situation: "頭が腰の上へ戻った。赤は脚と手で青の片腕を内側へ引き込もうとする。", facts: ["姿勢：戻った", "脚の囲い：閉じたまま", "片肘：引き込まれかけ"], focus: "elbow", hint: "両肘を近くに置き、一本だけ脚の中に取り残されないようにする。", progress: 2, moves: { elbows: { to: "stable", feedback: "両肘を近くに保てた。姿勢と腕の位置が整ったので、ここからガードを開く攻防へ進める。まだパスはしていない。" } } },
      stable: { title: "ガードの中で姿勢を保てた", situation: "赤の脚の囲いは残っている。今回の目標は、それを越える前の準備。", facts: ["土台：安定", "姿勢と両肘：保てた", "ガード：未通過"], focus: "elbow", hint: "", progress: 3, end: "goal", moves: {} },
    },
  },
  {
    id: "side-control", title: "進むか、抑え直すか", position: "サイドコントロールの上",
    goal: "相手の膝を見て、上のポジションを保ちながら攻める。",
    principle: "上からは、相手が脚を戻す道を塞ぐ。進めないときに抑え直す判断も、攻防の一手。",
    transfer: "相手が膝を戻したら、技を急ぐ前に二人の間にできた空間を見てみよう。",
    steps: ["腰の動きを止める", "膝の戻りを見る", "上を安定させる"],
    actions: [
      { id: "hip", label: "腰の動きを止める", description: "赤が腰を引けないよう、胸の位置と腰の支えを整える。", blocked: "腰は止まっている。赤の膝が二人の間へ戻っていないか確認しよう。" },
      { id: "clear", label: "膝の壁を外す", description: "胸の支配を残し、二人の間に入った膝の外へ回る。", blocked: "今は膝の壁を外す段階ではない。腰の支配と相手の脚の位置を見よう。" },
      { id: "mount", label: "胴をまたぐ", description: "相手の膝に止められない道から、マウントへ進む。", blocked: "赤が膝を間に戻して進路を塞いだ。腰と膝を無視してまたぐと、脚を捕まえられる。" },
      { id: "settle", label: "上で安定させる", description: "攻めを急がず、相手の動きに合わせて支えを整える。", blocked: "相手の腰が動けるうちは、待つだけでは抑え込みが深まらない。" },
    ], starts: ["open", "shield"], nodes: {
      open: { title: "赤が腰を引こうとしている", situation: "青が横から抑えている。赤の膝はまだ外だが、腰を引いて脚を戻そうとしている。", facts: ["青：脚の外で上", "赤の腰：動ける", "赤の膝：外"], focus: "hip", hint: "まず赤の腰の動きを止める。", progress: 0, moves: { hip: { to: "clear-path", feedback: "腰の動きを止めた。赤の膝は外に残り、胴をまたぐ道が開いている。" } } },
      shield: { title: "赤が膝を戻し始めた", situation: "青が横から抑える間に、赤が腰を引いて膝を二人の間へ戻そうとしている。", facts: ["青：サイドの上", "赤の腰：動ける", "赤の膝：内側へ移動"], focus: "hip", hint: "まず腰を止める。その後で、すでに入った膝を確認しよう。", progress: 0, moves: { hip: { to: "blocked-path", feedback: "腰の動きは止めたが、赤の膝が先に二人の間へ入った。このまま胴をまたぐ道は塞がれている。" } } },
      "clear-path": { title: "胴をまたぐ道が開いた", situation: "赤の腰は動きにくく、膝は二人の間に入っていない。", facts: ["腰の支配：あり", "膝の壁：なし", "胴への道：開いている"], focus: "knee", hint: "胸の支配を残して胴をまたぐ。", progress: 1, moves: { mount: { to: "mounted", feedback: "赤の胴をまたいだ。赤は腰を持ち上げて返そうとする。次は上で安定させよう。" } } },
      "blocked-path": { title: "膝の壁が道を塞いでいる", situation: "赤の膝が青の胸と腰の間に入っている。無理にまたぐと脚を捕まえられる。", facts: ["腰の支配：あり", "膝の壁：あり", "胴への道：閉じている"], focus: "knee", hint: "先に膝の壁の外へ回り、サイドの支配を回復する。", progress: 1, moves: { clear: { to: "side-again", feedback: "膝の外へ回り、横から抑える位置を取り戻した。赤がもう一度動く前に安定させる。" } } },
      mounted: { title: "マウントに入った直後", situation: "青が胴をまたいだ。赤は腰を持ち上げ、青を横へ返そうとしている。", facts: ["青：マウントの上", "赤：ブリッジ中", "安定：これから"], focus: "hip", hint: "技へ急がず、相手の動きに合わせて上で安定させる。", progress: 2, moves: { settle: { to: "mount-held", feedback: "相手の動きに合わせて支えを整え、上を保てた。位置に入ることと、保つことを分けて考えよう。" } } },
      "side-again": { title: "サイドを取り戻した直後", situation: "赤の膝は外へ出たが、赤はまた腰を引こうとしている。", facts: ["青：サイドの上", "膝の壁：外れた", "安定：これから"], focus: "hip", hint: "今回の目標は支配を保つこと。まずサイドを安定させる。", progress: 2, moves: { settle: { to: "side-held", feedback: "サイドを安定させた。マウントを急がず、ガード回復を止めて上を保つ判断ができた。" } } },
      "mount-held": { title: "マウントを保てた", situation: "相手の反応に合わせて上を保った。ここで練習を区切る。", facts: ["青：マウントの上", "上の支え：安定", "相手の反応に対応"], focus: "hip", hint: "", progress: 3, end: "goal", moves: {} },
      "side-held": { title: "サイドを保てた", situation: "相手の膝に合わせて抑え直した。前へ進まなくても、今回の目標は達成。", facts: ["青：サイドの上", "膝の回復：止めた", "上の支え：安定"], focus: "hip", hint: "", progress: 3, end: "goal", moves: {} },
    },
  },
  {
    id: "back-safety", title: "逃げるより先に、首を守る", position: "バックを取られたところ",
    goal: "首への攻撃を優先して見て、続けられないときはタップする。",
    principle: "首を守る手を、脚のために離さない。苦しさや痛みを待たず、不安な段階でタップしてよい。",
    transfer: "練習相手とタップの合図を確認し、タップされたらすぐに力を抜こう。",
    steps: ["首への手を止める", "攻撃の変化を見る", "安全に区切る"],
    actions: [
      { id: "hands", label: "両手で首への腕を止める", description: "首へ向かう腕を押さえ、絞めを組ませない。", blocked: "首への腕は止めている。相手のもう一方の手がどこへ動くかを見よう。" },
      { id: "follow", label: "持ち替えた手を追う", description: "相手の手の変化に合わせ、首の前の守りを保つ。", blocked: "まず、すでに首へ向かっている腕を止めよう。" },
      { id: "feet", label: "足の絡みを外す", description: "手を下げて、相手の脚をほどきにいく。", blocked: "手を下げると首の前が空く。バックでは、脚をほどく前に首への攻撃への対応が必要。" },
      { id: "chin", label: "顎を引いて待つ", description: "手を使わず、顎だけで首を隠す。", blocked: "顎だけでは絞めを止められない。首への腕を手で止めるか、危険ならタップで止めよう。" },
    ], starts: ["reach", "locked"], nodes: {
      reach: { title: "赤の腕が首へ向かっている", situation: "赤が背後から腕を回し始めた。まだ絞めは組まれていない。", facts: ["首への腕：接近中", "青の両手：使える", "赤の脚：絡んでいる"], focus: "neck", hint: "脚より先に、両手で首へ向かう腕を止める。", progress: 0, moves: { hands: { to: "switch", feedback: "首への腕を止めた。赤はもう一方の手を持ち替えて、守りの外から首へ近づこうとする。" } } },
      switch: { title: "赤が手を持ち替えた", situation: "最初の腕は止めているが、赤のもう一方の手が首へ近づいている。", facts: ["最初の腕：止めた", "もう一方の手：首へ接近", "脚：後回し"], focus: "wrist", hint: "相手の持ち替えに合わせ、首を守る手の位置を変える。", progress: 1, moves: { follow: { to: "defended", feedback: "持ち替えにも手を合わせ、首の前の守りを保った。今回は絞めの準備を止めるところで終了。脱出そのものは次の課題。" } } },
      locked: { title: "すでに絞めを組まれている", situation: "赤の腕が首に入り、両手が組まれた。あなたは防御の手を戻せない。", facts: ["絞め：組まれた", "防御の手：戻せない", "練習を止める場面"], focus: "neck", hint: "痛みや苦しさを待たず、タップして相手に止めてもらう。", progress: 1, moves: { tap: { to: "safe", feedback: "タップを伝え、赤がすぐに離した。止める判断も練習の成果。苦しくなるまで待つ必要はない。" } } },
      defended: { title: "首の守りを保てた", situation: "絞めの準備を止めたところで練習を区切る。バックから脱出できたわけではない。", facts: ["首への攻撃：止めた", "青：まだバックを取られている", "今回の練習：終了"], focus: "neck", hint: "", progress: 3, end: "goal", moves: {} },
      safe: { title: "タップで安全に止めた", situation: "相手がすぐに離した。状況を確認してから、また練習を始められる。", facts: ["タップ：伝えた", "相手：離した", "我慢せず止められた"], focus: "neck", hint: "", progress: 3, end: "safety", moves: {} },
    },
  },
  ...ADVANCED_PRACTICE,
  ...SITUATIONAL_PRACTICE,
];

export function practiceLesson(id: string): PracticeLesson {
  const lesson = PRACTICE_LESSONS.find((item) => item.id === id);
  if (!lesson) throw new Error(`Unknown practice lesson: ${id}`);
  return lesson;
}
