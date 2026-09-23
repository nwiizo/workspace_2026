import { cloneRoutePose, routePose, type RouteFrame } from "../engine/route";
import { backPair, guardPair, mountPair, sidePair, type Actor } from "../render/practicePose";
import { rollPair } from "../render/rollPose";

export interface SuggestedRoute {
  id: string; name: string; fact: string; exercise: string;
  category?: string; top?: Actor;
  source?: string; sourceTitle?: string; published?: string; checked?: string;
  frames: () => RouteFrame[];
}
const worlds = "https://ibjjf.com/news/2026-world-championship-black-belt-recap";
const frame = (label: string, pair: ReturnType<typeof sidePair>): RouteFrame => ({ label, seconds: 2, pose: routePose(pair) });

/** 公開情報から作った観察用の例。選手の試合映像のトレースではない。 */
export const SUGGESTED_ROUTES: SuggestedRoute[] = [
  {
    id: "passing-pressure", name: "パス後の攻防：膝の回復を止める", category: "サイド", top: "blue",
    fact: "2026年世界選手権の公式総括は、タイナン・ダルプラが外側と近距離のパスを組み合わせ、背中を露出させる攻めを紹介しています。",
    exercise: "サイドへ到達した後の練習例。膝の回復を見て、手の位置と胸の重なりを調整します。パス全体や背中への移行は、中間姿勢を足して作れます。",
    source: worlds, sourceTitle: "IBJJF・2026世界選手権総括（Tainan Dalpra）", published: "2026-06-04", checked: "2026-09-07",
    frames: () => {
      const first = frame("膝の回復を見る", sidePair("blocked-path", "blue"));
      const second = { label: "腰側の手を戻す", seconds: 2, pose: cloneRoutePose(first.pose) };
      second.pose.blue.arms.L.end.set(0.13, 0.25, 0.25);
      const third = { label: "膝が外へ下がる", seconds: 2, pose: cloneRoutePose(second.pose) };
      third.pose.red.legs.L.end.set(0.44, 0.055, 0.74);
      third.pose.red.legs.L.pole.set(0.40, 0.40, 0.36);
      return [first, second, third];
    },
  },
  {
    id: "back-connection", name: "バックの攻防：手の位置を追う", category: "バック",
    fact: "同大会では、ジャンセン・ゴメスの外側からのパスとバックへの攻撃、ライダー・ズチの決勝でのバックテイクが報告されています。",
    exercise: "バックを取られた後の防御例。青の手を首の前へ戻し、赤の持ち替えに合わせます。到達点の手と肩の位置を編集して観察します。",
    source: worlds, sourceTitle: "IBJJF・2026世界選手権総括（Gomes / Zuchi）", published: "2026-06-04", checked: "2026-09-07",
    frames: () => [frame("首へ向かう手を見る", backPair("reach")), frame("持ち替えに対応", backPair("switch")), frame("手を首の前に保つ", backPair("defended"))],
  },
  {
    id: "guard-angle", name: "ガードの攻防：腕と腰の角度を変える", category: "ガード", top: "blue",
    fact: "マイサ・バストスは2026年世界選手権決勝で、ラッソーからのアームドラッグで背中を露出させたと公式総括に記されています。",
    exercise: "腕と身体の角度を合わせるためのクローズドガードの練習例です。ラッソーや試合のアームドラッグそのものを再現したものではありません。",
    source: worlds, sourceTitle: "IBJJF・2026世界選手権総括（Mayssa Bastos）", published: "2026-06-04", checked: "2026-09-07",
    frames: () => [frame("姿勢と腕の位置を見る", guardPair("stable", "blue")), frame("相手の腰が横へずれる", guardPair("angle", "blue")), frame("上の支えを広げる", guardPair("base-angle", "blue"))],
  },
  {
    id: "mount-defense", name: "マウントの下：肘を戻して返す準備", category: "マウント", top: "red",
    fact: "青はマウントの下。まず浮いた肘を戻し、赤の手足がどこで支えているかを見ます。",
    exercise: "次は返す側の支えを止める段階です。肘を戻しただけで上下が入れ替わるわけではありません。",
    frames: () => [frame("浮いた肘を確認", mountPair("low", "red")), frame("肘を体の近くへ", mountPair("read-low", "red")), frame("返す側の支えを見る", mountPair("trapped", "red"))],
  },
  {
    id: "mount-control", name: "マウントの上：姿勢を保つ", category: "マウント", top: "blue",
    fact: "青が胴をまたいだ局面。赤の腰の動きと、自分の両膝の支えを観察します。",
    exercise: "上を安定させてから腕の攻防へ。相手が動いたら、先に支えを整え直します。",
    frames: () => [frame("赤の腰の動きを見る", mountPair("mounted", "blue")), frame("腰と両膝を整える", mountPair("mount-held", "blue"))],
  },
  {
    id: "mount-arm", name: "マウントから極めへ：片腕を分ける", category: "極め技", top: "blue",
    fact: "青が赤の片腕を胴から離した局面。腕を確保しても、上の位置を失えば攻めを続けにくくなります。",
    exercise: "次は身体の向きと脚の位置を変え、腕十字の形へ移る段階です。ここでは肘を伸ばし切りません。",
    frames: () => {
      const a = frame("両手と片腕の位置を確認", mountPair("isolated", "blue"));
      const b = { label: "片腕を保って次の位置を見る", seconds: 2, pose: cloneRoutePose(a.pose) };
      b.pose.blue.arms.L.end.y += 0.025;
      return [a, b];
    },
  },
  {
    id: "side-defense", name: "サイドの下：前腕の支えを調整", category: "サイド", top: "red",
    fact: "青が横から抑えられ、前腕を二人の間に戻した局面です。肘の位置と残った空間を見ます。",
    exercise: "次は腰を離し、膝を戻す空間を作ります。このルートでは前腕の支えの位置を調整します。",
    frames: () => {
      const a = frame("前腕と相手の胸の位置を見る", sidePair("framed", "red"));
      const b = { label: "前腕の支えを少し外へ調整", seconds: 2, pose: cloneRoutePose(a.pose) };
      b.pose.blue.arms.L.end.set(0.38, 0.32, 0.06);
      return [a, b];
    },
  },
  {
    id: "guard-posture", name: "ガード内の上：崩れた姿勢を戻す", category: "ガード", top: "blue",
    fact: "青は上ですが、赤の脚の内側にいます。頭を引かれたときの腰と膝の位置を観察します。",
    exercise: "土台と姿勢を戻してからガードを開く攻防へ。上にいることと、脚を越えたことは別です。",
    frames: () => [frame("前へ流れた上体を見る", guardPair("pull", "blue")), frame("膝で土台を整える", guardPair("base-pull", "blue")), frame("上体を戻す", guardPair("stable", "blue"))],
  },
  {
    id: "open-guard", name: "オープンガード：膝の外側を探す", category: "ガード", top: "blue",
    fact: "赤の足の交差が開いています。青はまだ赤の膝の内側にいるため、パスは終わっていません。",
    exercise: "脚を押さえる手と、相手の膝が戻る道を観察します。次は膝の線を越えて横へ回る段階です。",
    frames: () => {
      const a = frame("開いた脚と膝の線を見る", rollPair({ red: "redGuardOpened", blue: "blueGuardPass", badge: "" }));
      const b = { label: "膝側の手の位置を調整", seconds: 2, pose: cloneRoutePose(a.pose) };
      b.pose.blue.arms.R.end.x += 0.04;
      return [a, b];
    },
  },
  {
    id: "armbar-shape", name: "腕十字：脚と手で腕を保つ形", category: "極め技",
    fact: "青が赤の片腕を保持した局面。マウントからは、身体の向きと脚を置き換えた後にこの形へ進みます。",
    exercise: "脚・相手の肩・手首の関係を観察して終了します。肘を伸ばし切る動きは含みません。",
    frames: () => {
      const p = rollPair({ red: "redGuardArmbarFinish", blue: "blueGuardArmbarCaught", badge: "" });
      const a: RouteFrame = { label: "腕十字の保持位置を見る", seconds: 2, pose: { blue: routePose(p).red, red: routePose(p).blue } };
      const b = { label: "手首を保ったまま位置を確認", seconds: 2, pose: cloneRoutePose(a.pose) };
      b.pose.blue.arms.L.end.y += 0.02;
      return [a, b];
    },
  },
  {
    id: "triangle-shape", name: "三角絞め：首と片腕を囲う形", category: "極め技", top: "red",
    fact: "青が下から赤の首と片腕を脚で囲った局面。相手の片腕が内、もう一方が外にある配置を見ます。",
    exercise: "脚を組むまでの移動は別段階です。この例では締め付けず、囲った位置を観察して終了します。",
    frames: () => {
      const p = routePose(rollPair({ red: "redTriangleFinish", blue: "blueCaughtInTriangle", badge: "" }));
      const a: RouteFrame = { label: "片腕が内側にある配置を見る", seconds: 2, pose: { blue: p.red, red: p.blue } };
      const b = { label: "両手の位置を確認して止める", seconds: 2, pose: cloneRoutePose(a.pose) };
      b.pose.blue.arms.L.end.y += 0.02;
      return [a, b];
    },
  },
];

export interface RouteCourse {
  id: string; name: string; description: string;
  steps: { route: string; title: string; why: string; transition: string; swap?: boolean }[];
}

/** 局面の間はカットで表示する。返し・パス・脚の組み替えを直線補間しない。 */
export const ROUTE_COURSES: RouteCourse[] = [
  { id: "position-armbar", name: "サイド → マウント → 腕十字", description: "脚の回復を止める、上を安定させる、片腕を分ける、という順番を観察します。", steps: [
    { route: "passing-pressure", title: "1. サイドで膝の回復を止める", why: "赤の脚が間に戻ると先へ進めません。まず腰と膝の位置を確認します。", transition: "開始局面" },
    { route: "mount-control", title: "2. 胴をまたいだら上を安定させる", why: "マウントに入った直後も返される可能性があります。腕を狙う前に土台を整えます。", transition: "胴をまたぐ途中は省略し、マウントに入った局面へ切り替えます。" },
    { route: "mount-arm", title: "3. 上を保って片腕を分ける", why: "赤の肘が胴から離れた局面を観察します。位置を失った場合は抑え直す段階へ戻ります。", transition: "腕を分ける攻防の途中は省略し、片腕を保持した局面へ切り替えます。" },
    { route: "armbar-shape", title: "4. 腕十字の保持位置で区切る", why: "手首だけでなく、脚と相手の肩の位置も確認します。極め切る前の形で終了します。", transition: "旋回と脚の置き換えは省略し、腕十字の形へ切り替えます。" },
  ] },
  { id: "guard-triangle", name: "ガードの角度 → 三角絞めの形", description: "下の人が腰の角度を作り、首と片腕を囲った局面までを段階で見ます。", steps: [
    { route: "guard-angle", swap: true, title: "1. 青が下から腰の角度を作る", why: "青の脚が赤の腰を囲んでいます。赤の姿勢と両腕の位置が次の攻防を左右します。", transition: "開始局面" },
    { route: "triangle-shape", title: "2. 首と片腕を囲った位置を見る", why: "腰の角度だけで技が完成するわけではありません。片腕を分け、脚を置き換えた後の形です。", transition: "片腕を分けて脚を組み替える途中は省略し、三角の形へ切り替えます。" },
  ] },
  { id: "top-exchange", name: "トップの取り合い：青 → 赤 → 青", description: "返しが成立した場合の攻防例。上が交代しても相手のガードは残ることを観察します。", steps: [
    { route: "mount-defense", title: "1. 赤が上、青が返す準備", why: "青は肘を戻し、赤が返す側へ手足をつけるかを見ます。", transition: "開始局面" },
    { route: "guard-angle", title: "2. 青が返して上になる", why: "片側の支えを止めた返しが成立した分岐です。赤はガードを作り、腰の角度で青を崩しにきます。", transition: "ブリッジで回る途中は省略し、青がガード内の上になった局面へ切り替えます。" },
    { route: "guard-angle", swap: true, title: "3. 赤が返し、青が下から反応", why: "青が支え直せず、赤の返しが成立した分岐です。今度は青が下から角度を作ります。", transition: "スイープの途中は省略し、赤がガード内の上になった局面へ切り替えます。" },
    { route: "guard-posture", title: "4. 青が上を取り返して姿勢を戻す", why: "青の返しが成立。上になっても赤の脚の内側なので、姿勢を戻す攻防が続きます。", transition: "再度のスイープは省略し、青が上を取り返した局面へ切り替えます。" },
  ] },
  { id: "top-retained", name: "トップの取り合い：上が支え直して残る", description: "同じ崩しに対して、上の人が土台を戻せた場合の別の展開です。", steps: [
    { route: "guard-angle", title: "1. 赤が下から角度を作る", why: "青はまだ上にいますが、赤が片側へ角度を作っています。", transition: "開始局面" },
    { route: "guard-posture", title: "2. 青が土台と上体を戻す", why: "今回は青が支え直せた分岐です。上の色は変わらず、次はガードを開く攻防へ進みます。", transition: "正面へ向き直る途中は省略し、姿勢を立て直す局面へ切り替えます。" },
    { route: "open-guard", title: "3. ガードが開いても膝が残る", why: "足の交差がほどけてもパスは未完了です。膝の線を越えるまで上の攻防が続きます。", transition: "ガードを開く途中は省略し、足の交差が開いた局面へ切り替えます。" },
  ] },
];

export function courseRoutes(course: RouteCourse): SuggestedRoute[] {
  return course.steps.map((step) => {
    const route = SUGGESTED_ROUTES.find((r) => r.id === step.route);
    if (!route) throw new Error(`Unknown course route: ${step.route}`);
    return { ...route, name: step.title, fact: step.why, exercise: step.transition,
      top: step.swap && route.top ? (route.top === "blue" ? "red" : "blue") : route.top,
      frames: () => route.frames().map((f) => ({ ...f, pose: step.swap ? { blue: f.pose.red, red: f.pose.blue } : f.pose })),
    };
  });
}
