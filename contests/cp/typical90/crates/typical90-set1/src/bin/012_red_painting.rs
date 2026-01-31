// 012 - Red Painting (★4)
// https://atcoder.jp/contests/typical90/tasks/typical90_l
//
// 問題: H×Wのグリッドで、クエリごとにマスを赤く塗るか、
//       2マスが赤色で連結しているか判定する。
//
// 解法: Union-Find
//       赤く塗るときに隣接する赤マスと連結

use proconio::input;

fn main() {
    input! {
        h: usize,
        w: usize,
        q: usize,
    }

    let mut uf = UnionFind::new(h * w);
    let mut is_red = vec![vec![false; w]; h];

    for _ in 0..q {
        input! { query_type: usize }

        match query_type {
            1 => {
                input! { r: usize, c: usize }
                let (r, c) = (r - 1, c - 1);
                is_red[r][c] = true;

                // 隣接する赤マスと連結
                let dirs = [(0, 1), (1, 0), (0, -1), (-1, 0)];
                for (dr, dc) in dirs {
                    let nr = r as i32 + dr;
                    let nc = c as i32 + dc;
                    if nr >= 0 && nr < h as i32 && nc >= 0 && nc < w as i32 {
                        let (nr, nc) = (nr as usize, nc as usize);
                        if is_red[nr][nc] {
                            uf.unite(r * w + c, nr * w + nc);
                        }
                    }
                }
            }
            2 => {
                input! { ra: usize, ca: usize, rb: usize, cb: usize }
                let (ra, ca, rb, cb) = (ra - 1, ca - 1, rb - 1, cb - 1);

                if is_red[ra][ca] && is_red[rb][cb] && uf.same(ra * w + ca, rb * w + cb) {
                    println!("Yes");
                } else {
                    println!("No");
                }
            }
            _ => unreachable!(),
        }
    }
}

struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            rank: vec![0; n],
        }
    }

    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    fn unite(&mut self, x: usize, y: usize) {
        let (rx, ry) = (self.find(x), self.find(y));
        if rx == ry {
            return;
        }
        if self.rank[rx] < self.rank[ry] {
            self.parent[rx] = ry;
        } else {
            self.parent[ry] = rx;
            if self.rank[rx] == self.rank[ry] {
                self.rank[rx] += 1;
            }
        }
    }

    fn same(&mut self, x: usize, y: usize) -> bool {
        self.find(x) == self.find(y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_union_find() {
        let mut uf = UnionFind::new(9);
        // 3x3 グリッド
        // (0,0)=0, (0,1)=1, (0,2)=2
        // (1,0)=3, (1,1)=4, (1,2)=5
        // (2,0)=6, (2,1)=7, (2,2)=8

        uf.unite(0, 1);
        uf.unite(1, 4);
        assert!(uf.same(0, 4));
        assert!(!uf.same(0, 8));
    }
}
