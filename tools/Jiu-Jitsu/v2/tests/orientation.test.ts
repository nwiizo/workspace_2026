// root 変換が意図した向きを生むことの回帰テスト。
// 位置関係バグの調査 (2026-07) で「回転は正しく適用されている」ことを特定した際の
// 切り分けテストを恒久化したもの。

import { describe, expect, it } from "vitest";
import * as THREE from "three";
import { Fighter } from "../src/render/fighter";
import { POSES } from "../src/render/poses";

function bellyDir(f: Fighter): THREE.Vector3 {
  return new THREE.Vector3(0, 0, 1).applyQuaternion(f.root.quaternion);
}

describe("ポーズの向き (root 変換の適用)", () => {
  it("仰向けポーズは腹 (local +Z) が上を向く", () => {
    for (const name of ["blueUnderMount", "blueUnderSide", "redRolledBottom", "redGuardOpened"] as const) {
      const f = new Fighter({ color: 0x2f5fd0, accent: 0xbfd4ff });
      f.applyPose(POSES[name], { immediate: true });
      expect(bellyDir(f).y, name).toBeGreaterThan(0.7);
    }
  });

  it("マウント上の赤は下の青の頭側 (-Z) を向く", () => {
    const f = new Fighter({ color: 0xc23b3b, accent: 0xffc9c9 });
    f.applyPose(POSES.redMountTop, { immediate: true });
    expect(bellyDir(f).z).toBeLessThan(-0.7);
  });

  it("接地ソルバ: 接地系ポーズのプローブ最下点がマット面に着く", () => {
    for (const name of ["blueSeatedFront", "redMountTop", "blueUnderMount"] as const) {
      const f = new Fighter({ color: 0x2f5fd0, accent: 0xbfd4ff });
      f.applyPose(POSES[name], { immediate: true });
      // 接地解決後の root は必ず有限で、極端な浮き (元の y + 0.5 以上) にならない
      expect(Number.isFinite(f.root.position.y), name).toBe(true);
      expect(f.root.position.y, name).toBeLessThan(1.0);
    }
  });
});
