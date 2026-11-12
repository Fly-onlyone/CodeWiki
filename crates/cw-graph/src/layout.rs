//! Force-directed layout.
//!
//! Follows the ForceAtlas2 model (Jacomy et al., "ForceAtlas2, a Continuous
//! Graph Layout Algorithm for Handy Network Visualization", 2014): degree-scaled
//! repulsion, linear attraction, gravity and swing-based adaptive speed, with
//! Barnes-Hut approximation of repulsion. Implemented from the paper.

#[derive(Debug, Clone, Copy)]
pub struct LayoutOptions {
    pub iterations: usize,
    pub seed: u64,
    pub scaling: f32,
    pub gravity: f32,
    /// Barnes-Hut opening angle; larger is faster and less exact.
    pub theta: f32,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            iterations: 120,
            seed: 1,
            scaling: 10.0,
            gravity: 1.0,
            theta: 1.2,
        }
    }
}

/// SplitMix64: small, fast, and identical on every platform.
struct Rng(u64);

impl Rng {
    fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 40) as f32 / (1u64 << 24) as f32
    }
}

const NO_CHILD: u32 = u32::MAX;

#[derive(Clone, Copy)]
struct Cell {
    cx: f32,
    cy: f32,
    half: f32,
    mass: f32,
    mx: f32,
    my: f32,
    children: [u32; 4],
    /// Body stored in a leaf; `NO_CHILD` when empty or internal.
    body: u32,
}

struct QuadTree {
    cells: Vec<Cell>,
}

impl QuadTree {
    fn build(pos: &[[f32; 2]], mass: &[f32]) -> Self {
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for p in pos {
            min_x = min_x.min(p[0]);
            min_y = min_y.min(p[1]);
            max_x = max_x.max(p[0]);
            max_y = max_y.max(p[1]);
        }
        let half = ((max_x - min_x).max(max_y - min_y) / 2.0).max(1e-3) * 1.01;
        let mut tree = Self {
            cells: Vec::with_capacity(pos.len() * 2),
        };
        tree.cells.push(Self::cell(
            (min_x + max_x) / 2.0,
            (min_y + max_y) / 2.0,
            half,
        ));
        for i in 0..pos.len() {
            tree.insert(0, i as u32, pos, mass, 0);
        }
        tree
    }

    fn cell(cx: f32, cy: f32, half: f32) -> Cell {
        Cell {
            cx,
            cy,
            half,
            mass: 0.0,
            mx: 0.0,
            my: 0.0,
            children: [NO_CHILD; 4],
            body: NO_CHILD,
        }
    }

    fn insert(&mut self, mut at: usize, body: u32, pos: &[[f32; 2]], mass: &[f32], mut depth: u32) {
        loop {
            let p = pos[body as usize];
            let m = mass[body as usize];
            let c = &mut self.cells[at];
            let is_leaf = c.children == [NO_CHILD; 4];
            c.mx += p[0] * m;
            c.my += p[1] * m;
            c.mass += m;
            if is_leaf && c.body == NO_CHILD && c.mass == m {
                c.body = body;
                return;
            }
            // Coincident points would split forever; keep them in one cell.
            if depth > 40 {
                return;
            }
            if is_leaf && c.body != NO_CHILD {
                let old = c.body;
                c.body = NO_CHILD;
                let child = self.child_for(at, pos[old as usize]);
                let om = mass[old as usize];
                let oc = &mut self.cells[child];
                oc.mx += pos[old as usize][0] * om;
                oc.my += pos[old as usize][1] * om;
                oc.mass += om;
                oc.body = old;
            }
            at = self.child_for(at, p);
            depth += 1;
        }
    }

    fn child_for(&mut self, at: usize, p: [f32; 2]) -> usize {
        let c = self.cells[at];
        let quadrant = usize::from(p[0] >= c.cx) | (usize::from(p[1] >= c.cy) << 1);
        if c.children[quadrant] == NO_CHILD {
            let h = c.half / 2.0;
            let cx = if quadrant & 1 == 1 {
                c.cx + h
            } else {
                c.cx - h
            };
            let cy = if quadrant & 2 == 2 {
                c.cy + h
            } else {
                c.cy - h
            };
            self.cells.push(Self::cell(cx, cy, h));
            self.cells[at].children[quadrant] = (self.cells.len() - 1) as u32;
        }
        self.cells[at].children[quadrant] as usize
    }

    /// Repulsive force on body `i`.
    fn repulsion(
        &self,
        i: usize,
        pos: &[[f32; 2]],
        mass: &[f32],
        scaling: f32,
        theta: f32,
    ) -> [f32; 2] {
        let mut force = [0.0f32; 2];
        let p = pos[i];
        let mut stack = vec![0u32];
        while let Some(at) = stack.pop() {
            let c = &self.cells[at as usize];
            if c.mass == 0.0 || c.body == i as u32 {
                continue;
            }
            let (gx, gy) = (c.mx / c.mass, c.my / c.mass);
            let (dx, dy) = (p[0] - gx, p[1] - gy);
            let d2 = dx * dx + dy * dy;
            let is_leaf = c.children == [NO_CHILD; 4];
            if is_leaf || (2.0 * c.half) * (2.0 * c.half) < theta * theta * d2 {
                if d2 > 1e-9 {
                    // F = k * m_i * m_cell / d, along the unit vector: divide by d twice.
                    let f = scaling * mass[i] * c.mass / d2;
                    force[0] += dx * f;
                    force[1] += dy * f;
                }
            } else {
                stack.extend(c.children.iter().filter(|&&ch| ch != NO_CHILD));
            }
        }
        force
    }
}

/// Compute 2D positions for a symmetric weighted adjacency list.
pub fn force_layout(adjacency: &[Vec<(usize, f64)>], options: LayoutOptions) -> Vec<[f32; 2]> {
    let n = adjacency.len();
    if n == 0 {
        return Vec::new();
    }
    let mut rng = Rng(options.seed);
    let spread = (n as f32).sqrt() * 10.0;
    let mut pos: Vec<[f32; 2]> = (0..n)
        .map(|_| {
            [
                (rng.next_f32() - 0.5) * spread,
                (rng.next_f32() - 0.5) * spread,
            ]
        })
        .collect();
    if n == 1 {
        return vec![[0.0, 0.0]];
    }
    let mass: Vec<f32> = adjacency.iter().map(|l| l.len() as f32 + 1.0).collect();
    let mut prev_force = vec![[0.0f32; 2]; n];
    let mut speed = 1.0f32;

    for _ in 0..options.iterations {
        let tree = QuadTree::build(&pos, &mass);
        let mut force: Vec<[f32; 2]> = (0..n)
            .map(|i| tree.repulsion(i, &pos, &mass, options.scaling, options.theta))
            .collect();
        for (i, f) in force.iter_mut().enumerate() {
            // Gravity pulls toward the origin so components do not drift apart.
            let d = (pos[i][0] * pos[i][0] + pos[i][1] * pos[i][1])
                .sqrt()
                .max(1e-6);
            let g = options.gravity * mass[i];
            f[0] -= pos[i][0] / d * g;
            f[1] -= pos[i][1] / d * g;
            for &(j, w) in &adjacency[i] {
                f[0] -= (pos[i][0] - pos[j][0]) * w as f32;
                f[1] -= (pos[i][1] - pos[j][1]) * w as f32;
            }
        }

        let (mut swing, mut traction) = (0.0f32, 0.0f32);
        for i in 0..n {
            let s = ((force[i][0] - prev_force[i][0]).powi(2)
                + (force[i][1] - prev_force[i][1]).powi(2))
            .sqrt();
            let t = ((force[i][0] + prev_force[i][0]).powi(2)
                + (force[i][1] + prev_force[i][1]).powi(2))
            .sqrt()
                / 2.0;
            swing += mass[i] * s;
            traction += mass[i] * t;
        }
        let target = if swing > 0.0 {
            1.0 * traction / swing
        } else {
            speed
        };
        // The paper limits how fast global speed may rise between steps.
        speed = (speed + (target - speed).min(0.5 * speed)).max(1e-4);

        for i in 0..n {
            let s = ((force[i][0] - prev_force[i][0]).powi(2)
                + (force[i][1] - prev_force[i][1]).powi(2))
            .sqrt();
            let magnitude = (force[i][0].powi(2) + force[i][1].powi(2)).sqrt();
            let mut factor = speed / (1.0 + (speed * mass[i] * s).sqrt());
            if magnitude > 0.0 {
                factor = factor.min(10.0 / magnitude);
            }
            pos[i][0] += force[i][0] * factor;
            pos[i][1] += force[i][1] * factor;
        }
        prev_force = force;
    }
    pos
}

#[cfg(test)]
mod tests {
    use super::*;

    fn symmetric(n: usize, edges: &[(usize, usize)]) -> Vec<Vec<(usize, f64)>> {
        let mut adj = vec![Vec::new(); n];
        for &(a, b) in edges {
            adj[a].push((b, 1.0));
            adj[b].push((a, 1.0));
        }
        adj
    }

    fn dist(a: [f32; 2], b: [f32; 2]) -> f32 {
        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
    }

    #[test]
    fn layout_is_deterministic_and_finite() {
        let adj = symmetric(6, &[(0, 1), (1, 2), (2, 0), (3, 4), (4, 5), (5, 3), (2, 3)]);
        let a = force_layout(&adj, LayoutOptions::default());
        let b = force_layout(&adj, LayoutOptions::default());
        assert_eq!(a, b);
        assert!(a.iter().all(|p| p[0].is_finite() && p[1].is_finite()));
        let other = force_layout(
            &adj,
            LayoutOptions {
                seed: 2,
                ..LayoutOptions::default()
            },
        );
        assert_ne!(a, other);
    }

    #[test]
    fn connected_nodes_end_closer_than_unconnected() {
        let adj = symmetric(6, &[(0, 1), (1, 2), (2, 0), (3, 4), (4, 5), (5, 3), (2, 3)]);
        let p = force_layout(
            &adj,
            LayoutOptions {
                iterations: 300,
                ..LayoutOptions::default()
            },
        );
        assert!(dist(p[0], p[1]) < dist(p[0], p[5]));
        assert!(dist(p[4], p[5]) < dist(p[1], p[4]));
    }

    #[test]
    fn handles_trivial_and_coincident_input() {
        assert!(force_layout(&[], LayoutOptions::default()).is_empty());
        assert_eq!(
            force_layout(&[vec![]], LayoutOptions::default()),
            vec![[0.0, 0.0]]
        );
        let many = vec![Vec::new(); 200];
        let p = force_layout(
            &many,
            LayoutOptions {
                iterations: 20,
                ..LayoutOptions::default()
            },
        );
        assert!(p.iter().all(|q| q[0].is_finite() && q[1].is_finite()));
    }
}
