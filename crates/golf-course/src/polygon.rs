//! Even-odd regions (OB rings, penalty areas, water bodies) with an O(1) point test.
//!
//! The region is covered by a grid of square cells. Each cell knows whether its centre is inside (from one scanline
//! pass at build time) and which edges touch it. A point is inside when its cell centre's parity, flipped once per
//! edge the segment centre → point crosses, is odd. That segment stays inside the cell, so only the cell's own edges
//! can cross it.

use golf_core::Vec2;

pub struct Region {
    edges: Vec<(Vec2, Vec2)>,
    x0: f64,
    y0: f64,
    cell_m: f64,
    nx: usize,
    ny: usize,
    centre_inside: Vec<bool>,
    /// edge indices per cell, flattened: cell k's edges are cell_edges[start[k]..start[k + 1]]
    start: Vec<u32>,
    cell_edges: Vec<u32>,
    pub area_bbox: (Vec2, Vec2),
}

#[inline]
fn orient(a: Vec2, b: Vec2, c: Vec2) -> f64 {
    (b - a).cross(c - a)
}

/// do segments p0-p1 and q0-q1 cross (general position)?
#[inline]
fn crosses(p0: Vec2, p1: Vec2, q0: Vec2, q1: Vec2) -> bool {
    (orient(q0, q1, p0) > 0.0) != (orient(q0, q1, p1) > 0.0) && (orient(p0, p1, q0) > 0.0) != (orient(p0, p1, q1) > 0.0)
}

/// distance from p to segment a-b
#[inline]
pub fn seg_dist(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let ab = b - a;
    let l2 = ab.dot(ab);
    let t = if l2 > 0.0 { ((p - a).dot(ab) / l2).clamp(0.0, 1.0) } else { 0.0 };
    (p - (a + ab * t)).len()
}

impl Region {
    /// Rings are closed implicitly (a repeated last point is dropped). `cell_m` around 8 m suits C0's rings.
    pub fn new(rings: &[Vec<[f64; 2]>], cell_m: f64) -> Region {
        let mut edges = Vec::new();
        for r in rings {
            let mut pts: Vec<Vec2> = r.iter().map(|p| Vec2::new(p[0], p[1])).collect();
            if pts.len() > 1 && pts.first() == pts.last() {
                pts.pop();
            }
            if pts.len() < 3 {
                continue;
            }
            for i in 0..pts.len() {
                edges.push((pts[i], pts[(i + 1) % pts.len()]));
            }
        }
        let (mut lo, mut hi) = (Vec2::new(f64::INFINITY, f64::INFINITY), Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY));
        for (a, _) in &edges {
            lo = Vec2::new(lo.x.min(a.x), lo.y.min(a.y));
            hi = Vec2::new(hi.x.max(a.x), hi.y.max(a.y));
        }
        if edges.is_empty() {
            return Region { edges, x0: 0.0, y0: 0.0, cell_m, nx: 0, ny: 0, centre_inside: vec![], start: vec![0], cell_edges: vec![], area_bbox: (Vec2::ZERO, Vec2::ZERO) };
        }
        let (x0, y0) = (lo.x - cell_m, lo.y - cell_m);
        let nx = ((hi.x + cell_m - x0) / cell_m).ceil() as usize;
        let ny = ((hi.y + cell_m - y0) / cell_m).ceil() as usize;

        // edges per cell (by each edge's bounding box: a superset, which is all the test needs)
        let mut per: Vec<Vec<u32>> = vec![Vec::new(); nx * ny];
        let cell = |v: f64, o: f64, n: usize| (((v - o) / cell_m).floor().max(0.0) as usize).min(n - 1);
        for (k, (a, b)) in edges.iter().enumerate() {
            for j in cell(a.y.min(b.y), y0, ny)..=cell(a.y.max(b.y), y0, ny) {
                for i in cell(a.x.min(b.x), x0, nx)..=cell(a.x.max(b.x), x0, nx) {
                    per[j * nx + i].push(k as u32);
                }
            }
        }
        let mut start = Vec::with_capacity(nx * ny + 1);
        let mut cell_edges = Vec::new();
        start.push(0);
        for v in per {
            cell_edges.extend(v);
            start.push(cell_edges.len() as u32);
        }

        // centre parity, one scanline per row of centres
        let mut centre_inside = vec![false; nx * ny];
        let mut xs = Vec::new();
        for j in 0..ny {
            let yc = y0 + (j as f64 + 0.5) * cell_m;
            xs.clear();
            for (a, b) in &edges {
                if (a.y > yc) != (b.y > yc) {
                    xs.push(a.x + (yc - a.y) * (b.x - a.x) / (b.y - a.y));
                }
            }
            xs.sort_by(|p, q| p.total_cmp(q));
            let mut n_left = 0;
            for i in 0..nx {
                let xc = x0 + (i as f64 + 0.5) * cell_m;
                while n_left < xs.len() && xs[n_left] < xc {
                    n_left += 1;
                }
                centre_inside[j * nx + i] = n_left % 2 == 1;
            }
        }
        Region { edges, x0, y0, cell_m, nx, ny, centre_inside, start, cell_edges, area_bbox: (lo, hi) }
    }

    #[inline]
    fn cell_of(&self, p: Vec2) -> Option<(usize, usize)> {
        let (fi, fj) = ((p.x - self.x0) / self.cell_m, (p.y - self.y0) / self.cell_m);
        if !(fi >= 0.0 && fj >= 0.0) || fi >= self.nx as f64 || fj >= self.ny as f64 {
            return None;
        }
        Some((fi as usize, fj as usize))
    }

    #[inline]
    fn edges_in(&self, i: usize, j: usize) -> impl Iterator<Item = &(Vec2, Vec2)> {
        let k = j * self.nx + i;
        self.cell_edges[self.start[k] as usize..self.start[k + 1] as usize].iter().map(|&e| &self.edges[e as usize])
    }

    /// is p inside (even-odd over every ring)?
    pub fn contains(&self, p: Vec2) -> bool {
        let Some((i, j)) = self.cell_of(p) else { return false };
        let c = Vec2::new(self.x0 + (i as f64 + 0.5) * self.cell_m, self.y0 + (j as f64 + 0.5) * self.cell_m);
        let mut inside = self.centre_inside[j * self.nx + i];
        for &(a, b) in self.edges_in(i, j) {
            if crosses(c, p, a, b) {
                inside = !inside;
            }
        }
        inside
    }

    /// is any part of the boundary within r of p?
    pub fn near_boundary(&self, p: Vec2, r: f64) -> bool {
        self.boundary_dist(p, r) <= r
    }

    /// the distance from p to the boundary if it is at most `max`, else +inf
    pub fn boundary_dist(&self, p: Vec2, max: f64) -> f64 {
        let range = |v: f64, o: f64, n: usize| -> Option<(usize, usize)> {
            let lo = ((v - max - o) / self.cell_m).floor() as i64;
            let hi = ((v + max - o) / self.cell_m).floor() as i64;
            let (lo, hi) = (lo.max(0), hi.min(n as i64 - 1));
            (lo <= hi).then_some((lo as usize, hi as usize))
        };
        let (Some((i0, i1)), Some((j0, j1))) = (range(p.x, self.x0, self.nx), range(p.y, self.y0, self.ny)) else {
            return f64::INFINITY;
        };
        let mut best = f64::INFINITY;
        for j in j0..=j1 {
            for i in i0..=i1 {
                for &(a, b) in self.edges_in(i, j) {
                    best = best.min(seg_dist(p, a, b));
                }
            }
        }
        if best <= max { best } else { f64::INFINITY }
    }

    pub fn n_edges(&self) -> usize {
        self.edges.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brute(rings: &[Vec<[f64; 2]>], p: Vec2) -> bool {
        let mut inside = false;
        for r in rings {
            let n = r.len();
            for i in 0..n {
                let (a, b) = (r[i], r[(i + 1) % n]);
                if (a[1] > p.y) != (b[1] > p.y) && p.x < a[0] + (p.y - a[1]) * (b[0] - a[0]) / (b[1] - a[1]) {
                    inside = !inside;
                }
            }
        }
        inside
    }

    #[test]
    fn grid_test_matches_brute_force() {
        // a wobbly ring with a hole, small cells so most points need the edge correction
        let outer: Vec<[f64; 2]> = (0..200)
            .map(|k| {
                let a = k as f64 / 200.0 * std::f64::consts::TAU;
                let r = 100.0 + 25.0 * (5.0 * a).sin() + 7.0 * (17.0 * a).cos();
                [500.0 + r * a.cos(), 300.0 + r * a.sin()]
            })
            .collect();
        let hole: Vec<[f64; 2]> = (0..40)
            .map(|k| {
                let a = -(k as f64) / 40.0 * std::f64::consts::TAU;
                [510.0 + 30.0 * a.cos(), 290.0 + 20.0 * a.sin()]
            })
            .collect();
        let rings = vec![outer, hole];
        let reg = Region::new(&rings, 8.0);
        let mut s: u64 = 12345;
        let mut rnd = || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (s >> 11) as f64 / (1u64 << 53) as f64
        };
        for _ in 0..50_000 {
            let p = Vec2::new(350.0 + 300.0 * rnd(), 150.0 + 300.0 * rnd());
            assert_eq!(reg.contains(p), brute(&rings, p), "{p:?}");
        }
    }

    #[test]
    fn boundary_distance() {
        let sq = vec![vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0], [0.0, 0.0]]];
        let reg = Region::new(&sq, 4.0);
        assert!((reg.boundary_dist(Vec2::new(5.0, 9.0), 2.0) - 1.0).abs() < 1e-12);
        assert!(reg.boundary_dist(Vec2::new(5.0, 5.0), 2.0).is_infinite());
        assert!(reg.near_boundary(Vec2::new(10.01, 5.0), 0.02));
        assert!(!reg.contains(Vec2::new(-50.0, 5.0)));
    }
}
