import { describe, expect, it } from "vitest";
import { practicePair } from "../src/render/practicePose";
import { PRACTICE_LESSONS } from "../src/content/practice";
import { BODY, Mannequin, solveLimb } from "../src/render/mannequin";
import { Mesh, Vector3 } from "three";

const cases = PRACTICE_LESSONS.flatMap((lesson) => Object.keys(lesson.nodes).map((node) => [lesson.id, node] as const));

describe("ポジション練習の支持点", () => {
  it.each(["low", "high"])("マウント %s で上の人の両膝がマットに接する", (node) => {
    const { red } = practicePair("mount-base", node);
    // 膝の厚みを含む接地。root の高さだけの検査では浮きを見逃す。
    expect(red.kneeL.y).toBeCloseTo(0.073, 2);
    expect(red.kneeR.y).toBeCloseTo(0.073, 2);
  });
});

describe("人形全体の幾何検証", () => {
  it.each(cases)("%s / %s の支持点・骨長・床との交差", (lesson, node) => {
    const pair = practicePair(lesson, node);
    for (const support of pair.supports) expect(pair[support.actor][support.joint].y, support.joint).toBeCloseTo(support.height, 6);
    for (const actor of ["blue", "red"] as const) {
      const p = pair[actor];
      for (const side of ["L", "R"] as const) {
        expect(p[`shoulder${side}`].distanceTo(p[`elbow${side}`])).toBeCloseTo(BODY.upperArm, 6);
        expect(p[`elbow${side}`].distanceTo(p[`wrist${side}`])).toBeCloseTo(BODY.forearm, 6);
        expect(p[`hip${side}`].distanceTo(p[`knee${side}`])).toBeCloseTo(BODY.thigh, 6);
        expect(p[`knee${side}`].distanceTo(p[`ankle${side}`])).toBeCloseTo(BODY.shin, 6);
      }
      const mannequin = new Mannequin(p, 0);
      mannequin.root.updateMatrixWorld(true);
      let minimum = Infinity;
      let lowestMesh = "";
      const point = new Vector3();
      mannequin.root.traverse((obj) => {
        if (!(obj instanceof Mesh)) return;
        const positions = obj.geometry.getAttribute("position");
        for (let i = 0; i < positions.count; i++) {
          const y = point.fromBufferAttribute(positions, i).applyMatrix4(obj.matrixWorld).y;
          if (y < minimum) { minimum = y; lowestMesh = `${obj.geometry.type} at ${obj.position.toArray()}`; }
        }
      });
      mannequin.dispose();
      // メッシュ頂点そのものを検査。プローブ点だけの接地による貫通を防ぐ。
      expect(minimum, `${actor}: ${lowestMesh}`).toBeGreaterThanOrEqual(-0.002);
    }
    expect(pair.blue.head.distanceTo(pair.red.head), "頭同士が重ならない").toBeGreaterThan(0.20);
  });

  it("低い／高いマウントの膝の位置と、支えを作る前後の手が変わる", () => {
    const low = practicePair("mount-base", "low").red;
    const high = practicePair("mount-base", "high").red;
    expect(low.kneeL.z - high.kneeL.z).toBeGreaterThan(0.20);
    const pinned = practicePair("side-space", "pinned").blue;
    const framed = practicePair("side-space", "framed").blue;
    expect(framed.wristL.y - pinned.wristL.y).toBeGreaterThan(0.15);
    expect(practicePair("side-space", "space").blue.pelvis.x).toBeLessThan(framed.pelvis.x - 0.15);
  });

  it("タップ後は二人を離す", () => {
    const p = practicePair("back-safety", "stopped");
    expect(p.blue.pelvis.distanceTo(p.red.pelvis)).toBeGreaterThan(1);
  });

  it.each(cases)("%s / %s で相手の胸郭・骨盤の中心を手足が貫通しない", (lesson, node) => {
    const pair = practicePair(lesson, node);
    for (const actor of ["blue", "red"] as const) {
      const self = pair[actor], other = pair[actor === "blue" ? "red" : "blue"];
      const inverse = other.rotation.clone().invert();
      for (const side of ["L", "R"] as const) {
        for (const [a, b] of [[self[`shoulder${side}`], self[`elbow${side}`]], [self[`elbow${side}`], self[`wrist${side}`]], [self[`hip${side}`], self[`knee${side}`]], [self[`knee${side}`], self[`ankle${side}`]]]) {
          for (const t of [0.1, 0.3, 0.5, 0.7, 0.9]) {
            const at = a!.clone().lerp(b!, t);
            for (const [center, radii] of [[other.chest, new Vector3(0.16, 0.15, 0.075)], [other.pelvis, new Vector3(0.12, 0.075, 0.07)]] as const) {
              const distance = at.clone().sub(center).applyQuaternion(inverse).divide(radii).length();
              expect(distance, `${actor}/${side}: segment ${a!.toArray()} → ${b!.toArray()}, t=${t}, inside ${center === other.chest ? "chest" : "pelvis"}`).toBeGreaterThanOrEqual(1);
            }
          }
        }
      }
    }
  });

  it("状態が進んだときは身体の配置も変わる", () => {
    for (const lesson of PRACTICE_LESSONS) {
      for (const [id, node] of Object.entries(lesson.nodes)) {
        for (const move of Object.values(node.moves)) {
          const before = practicePair(lesson.id, id), after = practicePair(lesson.id, move.to);
          const points = (pair: typeof before) => [pair.blue, pair.red].flatMap((body) => Object.values(body).filter((item): item is Vector3 => item instanceof Vector3));
          const a = points(before), b = points(after);
          if (!move.observation) expect(a.some((point, index) => point.distanceTo(b[index]!) > 0.025), `${lesson.id}: ${id} → ${move.to}`).toBe(true);
        }
      }
    }
  });

  it("膝の壁は上の人の近くへ戻り、ガードでは上の人の腰の左右を囲む", () => {
    const shield = practicePair("side-control", "blocked-path");
    const open = practicePair("side-control", "clear-path");
    expect(shield.red.kneeL.distanceTo(shield.blue.pelvis)).toBeLessThan(open.red.kneeL.distanceTo(open.blue.pelvis));
    const guard = practicePair("guard-posture", "stable");
    expect(guard.red.kneeL.x).toBeGreaterThan(guard.blue.pelvis.x + 0.20);
    expect(guard.red.kneeR.x).toBeLessThan(guard.blue.pelvis.x - 0.20);
  });

  it("頭を引く場面では相手の手が首の付け根へ届く", () => {
    for (const node of ["pull", "base-pull"]) {
      const pair = practicePair("guard-posture", node);
      expect(pair.red.wristL.distanceTo(pair.blue.neck), node).toBeLessThan(0.11);
    }
  });
});

describe("二節の計算", () => {
  it.each([new Vector3(4, 0, 0), new Vector3(), new Vector3(0.3, 0, 0)])("届かない目標や一直線のpoleでも骨長と有限値を保つ %s", (end) => {
    const start = new Vector3();
    const solved = solveLimb(start, { end, pole: new Vector3(1, 0, 0) }, 0.28, 0.26);
    expect(solved.joint.length()).toBeCloseTo(0.28, 6);
    expect(solved.joint.distanceTo(solved.end)).toBeCloseTo(0.26, 6);
    expect(solved.end.length()).toBeLessThanOrEqual(0.54);
  });
  it("手先の位置を保ち、肘を指定した側へ曲げる", () => {
    const solved = solveLimb(new Vector3(), { end: new Vector3(0.4, 0, 0), pole: new Vector3(0, 1, 0) }, 0.3, 0.3);
    expect(solved.end.toArray()).toEqual([0.4, 0, 0]);
    expect(solved.joint.x).toBeCloseTo(0.2);
    expect(solved.joint.y).toBeCloseTo(Math.sqrt(0.05));
  });
});
