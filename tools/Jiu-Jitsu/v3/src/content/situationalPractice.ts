import type { PracticeLesson } from "./practice";

const checked = "2026-09-07";
const course = "https://www.gracieuniversity.com/Pages/Public/course?enc=pZ3uf5EaC9beRyE3Wwgx0w%3D%3D";
const scope = "英語の公開説明から練習の着眼点を採用。ボタンの手順・相手の反応・3D配置は本アプリの練習例で、教材動画の再現ではありません。";

export const SITUATIONAL_PRACTICE: PracticeLesson[] = [
  {
    id: "half-bottom", title: "ハーフガードで膝の壁を戻す", position: "ハーフガードの下",
    goal: "片脚を残した状態から、前腕と膝で空間を作りガードへ戻る。",
    principle: "片脚を捕らえるだけで止まらず、上半身と膝の距離を見る。", transfer: "同じ位置の上側の課題と比べ、膝の壁が何を止めるか観察する。",
    source: { title: "VR Jiu-Jitsu — Maintaining Knee Shield Half Guard", url: "https://www.vrjiujitsu.online/packages/vr-jiu-jitsu-fundamentals/videos/kneeshield-half-guard", section: "横向きと上下の膝の配置", checked, scope },
    steps: ["前腕で空間を守る", "膝の壁を戻す", "ガードを回復する"], starts: ["framed", "flat"],
    actions: [
      { id: "frame", label: "前腕の支えを作る", description: "上半身の間に空間を残す。", blocked: "支えはある。次は膝の位置。" },
      { id: "knee", label: "上の膝を間へ戻す", description: "できた空間へ膝を置き、距離を保つ。", blocked: "上半身を押さえられたまま膝だけでは戻しにくい。" },
      { id: "recover", label: "脚を戻してガードへ", description: "空間を使って脚を戻した局面を確認する。", blocked: "先に前腕と膝で、脚を戻す空間を作る。" },
      { id: "squeeze", label: "両脚だけ強く締める", description: "上半身を動かさず脚だけで止めようとする。", blocked: "脚だけでは上半身の押さえ込みは解けない。支えと距離を見よう。" },
    ], nodes: {
      framed: { title: "片脚が残り、前腕を使える", situation: "青が下。赤の片脚を両脚の間に残しているが、上の膝は低い。", facts: ["青：ハーフの下", "前腕：使える", "膝の壁：低い"], focus: "elbow", hint: "前腕の支えを確認する。", progress: 0, visual: { family: "half", state: "framed" }, moves: { frame: { to: "space", observation: true, feedback: "前腕の配置を確認した。膝を置く場所を見る。" } } },
      flat: { title: "上半身を床へ向けられた", situation: "赤の圧で青の背中が床へ近づき、前腕の支えが外れた。", facts: ["片脚：残っている", "上半身：平ら", "前腕：戻す必要"], focus: "elbow", hint: "先に前腕の支えを戻す。", progress: 0, visual: { family: "half", state: "flat" }, moves: { frame: { to: "space", feedback: "横向きと前腕の支えを戻した局面へ。途中の体の向きの変更は省略している。" } } },
      space: { title: "上の膝を置く空間がある", situation: "前腕で距離を残しながら、上の膝を間へ戻す。", facts: ["前腕：支えあり", "膝：これから"], focus: "knee", hint: "膝で距離を作る。", progress: 1, visual: { family: "half", state: "framed" }, moves: { knee: { to: "shield", feedback: "膝の壁を戻した局面。相手の脚を捕らえることと、距離を保つことを分けて見る。" } } },
      shield: { title: "膝の壁が戻った", situation: "青の膝が二人の間に入り、赤の上半身との距離ができた。", facts: ["膝：間にある", "赤の片脚：まだ内側"], focus: "hip", hint: "ガードを回復した局面へ進む。", progress: 2, visual: { family: "half", state: "shield" }, moves: { recover: { to: "end", feedback: "両脚を戻したガードの局面へ。脚を抜き直す経路は省略。青は下のままで、スイープではない。" } } },
      end: { title: "下のままガードを回復", situation: "青の脚が赤の腰を囲った。", facts: ["青：ガードの下", "返し：まだ行っていない"], focus: "knee", hint: "", progress: 3, end: "goal", visual: { family: "guard", state: "stable", top: "red" }, moves: {} },
    },
  },
  {
    id: "half-top", title: "ハーフガードで残った脚を見る", position: "ハーフガードの上",
    goal: "相手の膝の壁を確認し、脚を抜いた後のサイドを安定させる。",
    principle: "上半身を越えても、片脚が残っていればパスは途中。", transfer: "下の課題と比べ、どの段階で膝の壁が戻るか見る。",
    source: { title: "Gracie University — Blue Belt Stripe 2", url: course, section: "Lesson 34: Reverse Half Guard Pass の公開概要（相手のアンダーフックと横向きへの対応）", checked, scope },
    steps: ["膝と上半身を見る", "膝の壁を越える", "脚を抜いた位置を保つ"], starts: ["shield", "flat"],
    actions: [
      { id: "read", label: "膝と上半身の向きを見る", description: "膝の壁が残るか、相手が平らになったかを見る。", blocked: "配置は見えた。残っている壁に対応する。" },
      { id: "clear", label: "膝の壁の外へ進む", description: "上半身との距離を減らした局面を確認する。", blocked: "先に相手の膝と向きを確認する。" },
      { id: "pass", label: "脚を抜き、サイドで止まる", description: "脚を抜いた後の位置まで段階表示で進む。", blocked: "膝の壁を無視して引き抜こうとしない。" },
      { id: "rush", label: "脚を残してマウントを急ぐ", description: "捕まった脚を見ずに胴をまたごうとする。", blocked: "片脚が残っている。先に膝と脚の攻防を終える。" },
    ], nodes: {
      shield: { title: "赤の膝が二人の間にある", situation: "青が上だが、赤の膝が上半身を遠ざけている。", facts: ["青：上", "膝の壁：あり", "片脚：捕まっている"], focus: "knee", hint: "まず配置を見る。", progress: 0, visual: { family: "half", state: "shield", top: "blue" }, moves: { read: { to: "read", observation: true, feedback: "壁が残っている。脚を抜く前に上半身の距離へ対応する。" } } },
      flat: { title: "膝の壁は低いが脚が残る", situation: "赤は床を向き、膝の壁が低くなっている。", facts: ["上半身：平ら", "片脚：まだ残る"], focus: "knee", hint: "膝の壁が戻っていないことを確認。", progress: 0, visual: { family: "half", state: "flat", top: "blue" }, moves: { read: { to: "clear", observation: true, feedback: "今回は膝の壁を越えたところから。残った脚の攻防へ進める。" } } },
      read: { title: "先に膝の壁へ対応する", situation: "足だけを引くと、赤が上半身の距離を保てる。", facts: ["膝の壁：未通過", "脚を抜く：後"], focus: "knee", hint: "膝の壁の外へ進む。", progress: 1, visual: { family: "half", state: "shield", top: "blue" }, moves: { clear: { to: "clear", feedback: "相手が平らになった局面へ切り替える。壁を越える細かな手足の置き換えは省略。" } } },
      clear: { title: "残った片脚を確認", situation: "赤の上半身を越えても、片脚が両脚の間に残っている。", facts: ["膝の壁：低い", "パス：まだ途中"], focus: "ankle", hint: "脚を抜いた後のサイドを見る。", progress: 2, visual: { family: "half", state: "flat", top: "blue" }, moves: { pass: { to: "end", feedback: "脚を抜き、サイドを保った局面へ。抜き方の経路は省略している。" } } },
      end: { title: "脚の外でサイドを保った", situation: "青の脚が赤の囲いから出た。", facts: ["青：サイドの上", "次：膝の回復を止める"], focus: "hip", hint: "", progress: 3, end: "goal", visual: { family: "side", state: "side-held", top: "blue" }, moves: {} },
    },
  },
  {
    id: "turtle-bottom", title: "タートルで止まらず、次を探す", position: "タートルの下",
    goal: "首への手を見て守り、相手の位置を確認してガードへ戻る。",
    principle: "タートルは通過点の一つ。首の防御と次の移動を分けて考える。", transfer: "回る前後に背中と首を取られないか、指導者と確認する。",
    source: { title: "Stephan Kesting — A Roadmap for Brazilian Jiu-jitsu", url: "https://www.grapplearts.com/wp-content/uploads/2018/03/Roadmap-for-BJJ-1.5-1.pdf#page=12", section: "p.12: A Sample Defensive Strategy Map", checked, scope },
    steps: ["首への手を守る", "相手の位置を見る", "ガードの局面へ"], starts: ["compact", "reach"],
    actions: [
      { id: "protect", label: "首の前へ手を戻す", description: "首へ向かう相手の腕を追う。", blocked: "手は戻った。次は相手がいる側を見る。" },
      { id: "read", label: "相手の腰の位置を見る", description: "相手が真後ろか横かを確認する。", blocked: "首への腕を先に守る。" },
      { id: "turn", label: "ガードへ向き直した局面を見る", description: "回り込みと脚の戻しは省略して前後を比較する。", blocked: "相手の腕と腰を見ずに回ると背中を渡すことがある。" },
      { id: "wait", label: "手を離して丸まり続ける", description: "姿勢だけで攻撃を止めようとする。", blocked: "丸まるだけでは首の攻撃は止まらない。手と相手の位置を見る。" },
    ], nodes: {
      compact: { title: "青が四つ這いで、赤は横にいる", situation: "青は手と膝で支えている。赤の手が背中へ伸びている。", facts: ["青：タートル", "赤：腰の横", "首への手：観察"], focus: "neck", hint: "首の前へ手を戻す。", progress: 0, visual: { family: "turtle", state: "compact" }, moves: { protect: { to: "protected", feedback: "首の前へ手を戻した。片手と両膝の支えを残して位置を見る。" } } },
      reach: { title: "赤の手が首へ近づいた", situation: "回ろうとする前に、首へ向かう手へ対応したい。", facts: ["赤の手：首へ接近", "移動：先に守る"], focus: "neck", hint: "先に首を守る。", progress: 0, visual: { family: "turtle", state: "reach" }, moves: { protect: { to: "protected", feedback: "手を戻して首への攻撃へ対応した局面。防御できないならタップしてよい。" } } },
      protected: { title: "首を守り、横の相手を見る", situation: "赤は青の腰の横。まだ背後に脚のフックは入っていない。", facts: ["首の前：手を戻した", "赤：横", "フック：なし"], focus: "hip", hint: "腰の位置を確認する。", progress: 1, visual: { family: "turtle", state: "protected" }, moves: { read: { to: "ready", observation: true, feedback: "相手の位置を確認した。向き直った後のガードと比較する。" } } },
      ready: { title: "ガードへ戻る前", situation: "首の守りを残したまま、脚を間に戻した局面へ進む。", facts: ["次：相手に向く", "回り方：段階表示"], focus: "hip", hint: "ガードの局面へ。", progress: 2, visual: { family: "turtle", state: "protected" }, moves: { turn: { to: "end", feedback: "ガードへ向き直した局面へ切り替えた。回転と脚の差し込みの経路は省略。" } } },
      end: { title: "相手を前に置くガードへ", situation: "青は下のまま赤と向き合っている。", facts: ["青：ガードの下", "背後：渡していない局面"], focus: "hip", hint: "", progress: 3, end: "goal", visual: { family: "guard", state: "stable", top: "red" }, moves: {} },
    },
  },
  {
    id: "turtle-top", title: "タートルの横から背後へつなぐ", position: "タートルを取った側",
    goal: "相手の腰の横で位置を保ち、バックを取った後と比較する。",
    principle: "首だけを追う前に、相手が動く腰の位置を見る。", transfer: "バックを取った後は、既存の手の攻防の課題へつなぐ。",
    source: { title: "VR Jiu-Jitsu — Top Position on Opponent's Turtle", url: "https://www.vrjiujitsu.online/packages/vr-jiu-jitsu-fundamentals/videos/position-turtle-top", section: "タートルの上での配置の公開説明", checked, scope },
    steps: ["腰の位置を見る", "横で保持する", "バックの局面と比較"], starts: ["compact", "protected"],
    actions: [
      { id: "read", label: "相手の腰と支えを見る", description: "相手の膝と、青との位置関係を見る。", blocked: "配置は見えた。次に保持を確認する。" },
      { id: "hold", label: "腰の横で位置を保つ", description: "頭へ飛びつかず、相手についていける位置を確認する。", blocked: "まず相手の腰の位置を見る。" },
      { id: "back", label: "バックを取った局面と比べる", description: "倒し方とフックの挿入は省略し、前後を比べる。", blocked: "先に横の位置を保つ。" },
      { id: "neck", label: "腰を離して首だけ追う", description: "相手の腰を見ず上半身だけを追う。", blocked: "相手が腰を動かす道が空く。横の位置を見直そう。" },
    ], nodes: {
      compact: { title: "青が赤の腰の横にいる", situation: "赤は両手と膝で支えている。", facts: ["青：横の上", "赤：タートル"], focus: "hip", hint: "腰と支えを見る。", progress: 0, visual: { family: "turtle", state: "compact", top: "blue" }, moves: { read: { to: "ready", observation: true, feedback: "腰の横の配置を確認した。相手の動きについていく保持を見る。" } } },
      protected: { title: "赤が首を守る手を戻した", situation: "赤が片手を首へ戻した。青は腰の横に残る。", facts: ["赤：首を守る", "青：腰を見る"], focus: "hip", hint: "腰と支えを見る。", progress: 0, visual: { family: "turtle", state: "protected", top: "blue" }, moves: { read: { to: "ready", feedback: "赤が手を床へ戻した局面へ。手の反応が変わっても腰との位置を見る。" } } },
      ready: { title: "横の位置を保つ前", situation: "赤が動けば、青も腰の位置に合わせる必要がある。", facts: ["腰：青の近く", "保持：次に確認"], focus: "hip", hint: "横で保持する。", progress: 1, visual: { family: "turtle", state: "compact", top: "blue" }, moves: { hold: { to: "control", feedback: "腰への手の位置を調整した。まだ脚のフックは入っていない。" } } },
      control: { title: "腰の横で保持した", situation: "青が横で位置を保った。背後を取った後と比較できる。", facts: ["青：横で保持", "フック：これから"], focus: "hip", hint: "次のバックの局面を見る。", progress: 2, visual: { family: "turtle", state: "control", top: "blue" }, moves: { back: { to: "end", feedback: "バックを取った局面へ切り替えた。倒し方と脚のフックの挿入は省略している。" } } },
      end: { title: "背後を取った後の位置", situation: "青が背後で脚を回し、赤は首を守ろうとする。", facts: ["青：バックを保持", "次：手の攻防"], focus: "wrist", hint: "", progress: 3, end: "goal", visual: { family: "back", state: "reach", swap: true }, moves: {} },
    },
  },
  {
    id: "open-retention", title: "開いたガードで相手を前に置く", position: "オープンガードの下",
    goal: "相手が膝の外へ回る場面を読み、再び正面に置く。",
    principle: "脚が開いていても、距離と相手の方向を見続ける。", transfer: "腰の向き、膝の内外、接点を指導者と確認しよう。",
    source: { title: "Gracie University — Blue Belt Stripe 2", url: course, section: "Lesson 19: Open Guard Connections の公開概要（接点と距離）", checked, scope },
    steps: ["相手の方向を見る", "再び正面に置く", "距離を確認する"], starts: ["angle", "square"],
    actions: [
      { id: "read", label: "相手と膝の内外を見る", description: "相手がどちらへ動こうとしているか確認する。", blocked: "方向は見えた。次に向き直る。" },
      { id: "align", label: "相手を正面に戻した局面へ", description: "腰と膝の向きを合わせた後と比較する。", blocked: "先に相手が回る側を読む。" },
      { id: "hold", label: "膝と距離を確認して止める", description: "正面へ戻っても攻防が続くことを見る。", blocked: "まず相手を再び正面に置く。" },
      { id: "reach", label: "脚を伸ばしたまま手で追う", description: "相手の移動に腕だけでついていこうとする。", blocked: "腰と膝の方向が残る。腕だけで距離を埋めない。" },
    ], nodes: {
      angle: { title: "赤が膝の外側へ動いた", situation: "青は下で脚を開いている。赤は横へ回り始めた。", facts: ["青：下", "赤：横へ移動", "足：交差していない"], focus: "knee", hint: "相手と膝の位置を見る。", progress: 0, visual: { family: "open", state: "angle" }, moves: { read: { to: "read", observation: true, feedback: "赤が膝の外へ向かう配置を確認した。正面へ戻す局面と比べる。" } } },
      square: { title: "正面から赤が動き出す", situation: "今は赤が正面にいる。次の横移動を見る。", facts: ["赤：正面", "次：横へ動く"], focus: "knee", hint: "相手の反応を見る。", progress: 0, visual: { family: "open", state: "square" }, moves: { read: { to: "read", feedback: "赤が横へ移動した。青の膝の外側を狙っている。" } } },
      read: { title: "相手を再び前に置きたい", situation: "赤に横を取られる前に、腰と膝の方向を合わせる。", facts: ["赤：横", "青：向きを合わせる場面"], focus: "hip", hint: "正面へ戻した後を見る。", progress: 1, visual: { family: "open", state: "angle" }, moves: { align: { to: "front", feedback: "二人が再び向き合う局面へ切り替えた。腰を回す経路は省略している。" } } },
      front: { title: "赤が正面に戻った", situation: "青は下のまま、赤が膝の内側にいる。", facts: ["向き：正面", "攻防：継続"], focus: "knee", hint: "距離を観察して区切る。", progress: 2, visual: { family: "open", state: "square" }, moves: { hold: { to: "end", observation: true, feedback: "相手の方向を読み直せた。必ずパスを防げるという判定ではない。" } } },
      end: { title: "開いたガードで向きを保った", situation: "足は閉じていない。距離と相手の次の動きを見続ける。", facts: ["青：オープンガードの下", "次：接点と距離"], focus: "hip", hint: "", progress: 3, end: "goal", visual: { family: "open", state: "square" }, moves: {} },
    },
  },
];
