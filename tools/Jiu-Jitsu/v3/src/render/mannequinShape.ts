import { BufferGeometry, CatmullRomCurve3, Float32BufferAttribute, LatheGeometry, SphereGeometry, Vector2, Vector3 } from "three";

/** 高さ、横半径、前側の厚み、後ろ側の厚み。単位はm。 */
type Section = readonly [number, number, number, number];
export const HIP_RADII = [0.088, 0.09, 0.09] as const;

function profile(sections: readonly Section[], sides = 24): BufferGeometry {
  const positions: number[] = [], indices: number[] = [];
  const frontCurve = new CatmullRomCurve3(sections.map(([y, x, z]) => new Vector3(x, y, z)));
  const backCurve = new CatmullRomCurve3(sections.map(([y, x, , z]) => new Vector3(x, y, z)));
  const rows = (sections.length - 1) * 3 + 1;
  const frontPoints = frontCurve.getPoints(rows - 1), backPoints = backCurve.getPoints(rows - 1);
  for (let row = 0; row < rows; row++) {
    const { y: height, x: width, z: front } = frontPoints[row]!, back = backPoints[row]!.z;
    for (let i = 0; i < sides; i++) {
      const angle = i / sides * Math.PI * 2, depth = Math.cos(angle);
      positions.push(Math.sin(angle) * width, height, depth * (depth >= 0 ? front : back));
    }
  }
  for (let row = 0; row < rows - 1; row++) for (let i = 0; i < sides; i++) {
    const a = row * sides + i, b = row * sides + (i + 1) % sides;
    indices.push(a, b, a + sides, b, b + sides, a + sides);
  }
  const bottom = positions.length / 3;
  positions.push(0, sections[0]![0], 0, 0, sections[sections.length - 1]![0], 0);
  for (let i = 0; i < sides; i++) {
    const next = (i + 1) % sides, top = (rows - 1) * sides;
    indices.push(bottom, next, i, bottom + 1, top + i, top + next);
  }
  const geometry = new BufferGeometry();
  geometry.setAttribute("position", new Float32BufferAttribute(positions, 3)); geometry.setIndex(indices);
  geometry.computeVertexNormals();
  return geometry;
}

export function torsoGeometry(): BufferGeometry {
  return profile([
    [-0.108, 0.022, 0.020, 0.024], [-0.08, 0.095, 0.062, 0.072], [-0.035, 0.145, 0.092, 0.10],
    [0.015, 0.156, 0.104, 0.105], [0.075, 0.138, 0.085, 0.083], [0.135, 0.122, 0.085, 0.080],
    [0.20, 0.150, 0.094, 0.092], [0.27, 0.187, 0.111, 0.113], [0.33, 0.201, 0.116, 0.119],
    [0.39, 0.187, 0.103, 0.112], [0.435, 0.153, 0.085, 0.093],
    [0.475, 0.087, 0.064, 0.068], [0.51, 0.048, 0.042, 0.046],
  ]);
}

export function headGeometry(): BufferGeometry {
  return profile([
    [-0.105, 0.028, 0.042, 0.031], [-0.088, 0.044, 0.064, 0.046],
    [-0.055, 0.074, 0.080, 0.074], [-0.015, 0.091, 0.089, 0.097],
    [0.025, 0.094, 0.090, 0.103], [0.066, 0.081, 0.072, 0.088],
    [0.095, 0.052, 0.049, 0.057], [0.111, 0.013, 0.014, 0.016],
  ]);
}

export function limbGeometry(startRadius: number, endRadius: number): BufferGeometry {
  const sections = [[0, 0.84], [0.12, 0.99], [0.32, 1], [0.62, 0.99], [0.85, 0.96], [1, 0.94]] as const;
  return new LatheGeometry(sections.map(([along, fullness]) => new Vector2((startRadius * (1 - along) + endRadius * along) * fullness, along - 0.5)), 16);
}

/** 単位球の範囲内で踵を絞り、下面に平らな足裏を作る。 */
export function footGeometry(): BufferGeometry {
  const geometry = new SphereGeometry(1, 16, 12), positions = geometry.getAttribute("position");
  for (let i = 0; i < positions.count; i++) {
    const x = positions.getX(i), y = positions.getY(i), z = positions.getZ(i);
    positions.setXYZ(i, x * (z < 0 ? 0.7 + 0.3 * (z + 1) : 1), y < -0.55 ? -1 : y, z);
  }
  geometry.computeVertexNormals();
  return geometry;
}
