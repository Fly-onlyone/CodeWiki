//! Geometry of the dependency graph view. No UI types, so it is unit-tested.
//!
//! Node positions come from the backend's layout. This module maps them to the
//! screen, decides what is worth drawing at the current zoom, and finds the
//! node under the cursor.

use serde_json::Value;

pub struct GraphNode {
    pub id: String,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub rank: f32,
    pub community: usize,
}

pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    /// Index pairs into `nodes`.
    pub edges: Vec<[u32; 2]>,
    pub max_rank: f32,
}

impl GraphData {
    pub fn from_json(value: &Value) -> Self {
        let nodes: Vec<GraphNode> = value["nodes"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|n| GraphNode {
                id: n["id"].as_str().unwrap_or("").to_string(),
                name: n["name"].as_str().unwrap_or("").to_string(),
                x: n["x"].as_f64().unwrap_or(0.0) as f32,
                y: n["y"].as_f64().unwrap_or(0.0) as f32,
                rank: n["rank"].as_f64().unwrap_or(0.0) as f32,
                community: n["community"].as_u64().unwrap_or(0) as usize,
            })
            .collect();
        let count = nodes.len() as u64;
        let edges = value["edges"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|e| {
                let (a, b) = (e[0].as_u64()?, e[1].as_u64()?);
                // A self-edge draws nothing but still counts against the
                // edge budget and in the edge count shown.
                (a < count && b < count && a != b).then_some([a as u32, b as u32])
            })
            .collect();
        Self::new(nodes, edges)
    }

    fn new(nodes: Vec<GraphNode>, edges: Vec<[u32; 2]>) -> Self {
        let max_rank = nodes
            .iter()
            .map(|n| n.rank)
            .fold(0.0f32, f32::max)
            .max(1e-9);
        Self {
            nodes,
            edges,
            max_rank,
        }
    }

    /// A synthetic clustered graph for measuring drawing speed.
    pub fn stress(node_count: usize) -> Self {
        let mut seed = 0x9E37_79B9_7F4A_7C15u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 40) as f32 / (1u64 << 24) as f32
        };
        let clusters = (node_count / 400).max(1);
        let centres: Vec<(f32, f32)> = (0..clusters)
            .map(|_| ((next() - 0.5) * 4000.0, (next() - 0.5) * 4000.0))
            .collect();
        let nodes: Vec<GraphNode> = (0..node_count)
            .map(|i| {
                let cluster = i % clusters;
                let (cx, cy) = centres[cluster];
                GraphNode {
                    id: format!("n{i}"),
                    name: format!("node_{i}"),
                    x: cx + (next() - 0.5) * 300.0,
                    y: cy + (next() - 0.5) * 300.0,
                    rank: next().powi(6),
                    community: cluster,
                }
            })
            .collect();
        let edges = (0..node_count * 2)
            .map(|i| {
                let a = i % node_count;
                // Mostly inside a cluster, sometimes across.
                let step = if i % 9 == 0 {
                    1 + (next() * node_count as f32) as usize
                } else {
                    clusters * (1 + (next() * 20.0) as usize)
                };
                [a as u32, ((a + step) % node_count) as u32]
            })
            .collect();
        Self::new(nodes, edges)
    }
}

/// Maps world coordinates to the view. `(cx, cy)` is the world point shown at
/// the centre of the view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub zoom: f32,
    pub cx: f32,
    pub cy: f32,
}

pub const MIN_ZOOM: f32 = 0.005;
pub const MAX_ZOOM: f32 = 40.0;

impl Camera {
    /// Show the whole graph with a small margin.
    pub fn fit(data: &GraphData, width: f32, height: f32) -> Self {
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for n in &data.nodes {
            min_x = min_x.min(n.x);
            min_y = min_y.min(n.y);
            max_x = max_x.max(n.x);
            max_y = max_y.max(n.y);
        }
        if data.nodes.is_empty() {
            return Self {
                zoom: 1.0,
                cx: 0.0,
                cy: 0.0,
            };
        }
        let span_x = (max_x - min_x).max(1.0);
        let span_y = (max_y - min_y).max(1.0);
        let zoom = (width / span_x).min(height / span_y) * 0.9;
        Self {
            zoom: zoom.clamp(MIN_ZOOM, MAX_ZOOM),
            cx: (min_x + max_x) / 2.0,
            cy: (min_y + max_y) / 2.0,
        }
    }

    pub fn to_screen(self, x: f32, y: f32, width: f32, height: f32) -> (f32, f32) {
        (
            width / 2.0 + (x - self.cx) * self.zoom,
            height / 2.0 + (y - self.cy) * self.zoom,
        )
    }

    pub fn to_world(self, sx: f32, sy: f32, width: f32, height: f32) -> (f32, f32) {
        (
            self.cx + (sx - width / 2.0) / self.zoom,
            self.cy + (sy - height / 2.0) / self.zoom,
        )
    }

    /// Move the view by a screen-space distance.
    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.cx -= dx / self.zoom;
        self.cy -= dy / self.zoom;
    }

    /// Zoom by `factor`, keeping the world point under `(sx, sy)` in place.
    pub fn zoom_at(&mut self, sx: f32, sy: f32, factor: f32, width: f32, height: f32) {
        let (wx, wy) = self.to_world(sx, sy, width, height);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        self.cx = wx - (sx - width / 2.0) / self.zoom;
        self.cy = wy - (sy - height / 2.0) / self.zoom;
    }
}

/// Drawn radius of a node: more depended-upon nodes are larger, and nodes grow
/// a little when zoomed in.
pub fn radius(rank: f32, max_rank: f32, zoom: f32) -> f32 {
    let importance = (rank / max_rank).clamp(0.0, 1.0).sqrt();
    (1.5 + importance * 7.0) * zoom.clamp(0.6, 2.5)
}

/// Index of the node under the cursor, preferring the nearest.
pub fn pick(
    data: &GraphData,
    camera: &Camera,
    width: f32,
    height: f32,
    sx: f32,
    sy: f32,
) -> Option<usize> {
    let mut best: Option<(usize, f32)> = None;
    for (index, node) in data.nodes.iter().enumerate() {
        let (x, y) = camera.to_screen(node.x, node.y, width, height);
        let distance = ((x - sx).powi(2) + (y - sy).powi(2)).sqrt();
        // A few extra pixels make small nodes clickable.
        let reach = radius(node.rank, data.max_rank, camera.zoom) + 4.0;
        if distance <= reach && best.is_none_or(|(_, d)| distance < d) {
            best = Some((index, distance));
        }
    }
    best.map(|(index, _)| index)
}

/// Screen positions of the nodes inside the view, with their index.
pub fn visible_nodes(
    data: &GraphData,
    camera: &Camera,
    width: f32,
    height: f32,
) -> Vec<(usize, f32, f32)> {
    let margin = 12.0;
    data.nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            let (x, y) = camera.to_screen(node.x, node.y, width, height);
            (x >= -margin && y >= -margin && x <= width + margin && y <= height + margin)
                .then_some((index, x, y))
        })
        .collect()
}

/// Edges worth drawing: at least one end in view, capped at `limit` by even
/// sampling so a dense view stays fast and still shows overall structure.
pub fn visible_edges(
    data: &GraphData,
    camera: &Camera,
    width: f32,
    height: f32,
    limit: usize,
) -> Vec<(f32, f32, f32, f32)> {
    let inside = |x: f32, y: f32| x >= 0.0 && y >= 0.0 && x <= width && y <= height;
    let all: Vec<(f32, f32, f32, f32)> = data
        .edges
        .iter()
        .filter_map(|[a, b]| {
            let (na, nb) = (&data.nodes[*a as usize], &data.nodes[*b as usize]);
            let (ax, ay) = camera.to_screen(na.x, na.y, width, height);
            let (bx, by) = camera.to_screen(nb.x, nb.y, width, height);
            (inside(ax, ay) || inside(bx, by)).then_some((ax, ay, bx, by))
        })
        .collect();
    if all.len() <= limit || limit == 0 {
        return if limit == 0 { Vec::new() } else { all };
    }
    let step = all.len() as f32 / limit as f32;
    (0..limit)
        .map(|i| all[(i as f32 * step) as usize])
        .collect()
}

/// Above this many nodes in view, the view draws density cells instead of
/// every node. Drawing cost then depends on the view size, not the graph size.
pub const DETAIL_LIMIT: usize = 2000;

/// Cells drawn in the overview; chosen so a frame stays well under 16 ms.
const TARGET_CELLS: f32 = 2500.0;

/// One overview cell: screen position of its corner, the cluster most of its
/// nodes belong to, and how many nodes it holds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    pub x: f32,
    pub y: f32,
    pub community: usize,
    pub count: u32,
}

/// Side of an overview cell for a view of this size.
pub fn cell_size(width: f32, height: f32) -> f32 {
    (width * height / TARGET_CELLS).sqrt().max(6.0)
}

/// Bin the nodes in view into square screen cells. Each cell takes the
/// cluster of its highest-ranked node, so important symbols keep their color.
pub fn density_cells(data: &GraphData, camera: &Camera, width: f32, height: f32) -> Vec<Cell> {
    let side = cell_size(width, height);
    let columns = (width / side).ceil().max(1.0) as usize;
    let rows = (height / side).ceil().max(1.0) as usize;
    // Per cell: node count, best rank seen, cluster of that node.
    let mut grid: Vec<(u32, f32, usize)> = vec![(0, -1.0, 0); columns * rows];
    for node in &data.nodes {
        let (x, y) = camera.to_screen(node.x, node.y, width, height);
        if x < 0.0 || y < 0.0 || x >= width || y >= height {
            continue;
        }
        let slot = &mut grid[(y / side) as usize * columns + (x / side) as usize];
        slot.0 += 1;
        if node.rank > slot.1 {
            slot.1 = node.rank;
            slot.2 = node.community;
        }
    }
    grid.iter()
        .enumerate()
        .filter(|(_, (count, _, _))| *count > 0)
        .map(|(index, (count, _, community))| Cell {
            x: (index % columns) as f32 * side,
            y: (index / columns) as f32 * side,
            community: *community,
            count: *count,
        })
        .collect()
}

/// Indexes of the highest-ranked visible nodes, for labels.
pub fn label_candidates(
    data: &GraphData,
    visible: &[(usize, f32, f32)],
    limit: usize,
) -> Vec<(usize, f32, f32)> {
    let mut ranked: Vec<(usize, f32, f32)> = visible.to_vec();
    ranked.sort_by(|a, b| {
        data.nodes[b.0]
            .rank
            .total_cmp(&data.nodes[a.0].rank)
            .then(a.0.cmp(&b.0))
    });
    ranked.truncate(limit);
    ranked
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn data() -> GraphData {
        GraphData::from_json(&json!({
            "nodes": [
                {"id": "a", "name": "a", "x": 0.0, "y": 0.0, "rank": 0.5, "community": 0},
                {"id": "b", "name": "b", "x": 100.0, "y": 0.0, "rank": 0.1, "community": 1},
                {"id": "c", "name": "c", "x": 0.0, "y": 50.0, "rank": 0.01, "community": 1},
            ],
            "edges": [[0, 1], [1, 2], [0, 9]],
        }))
    }

    #[test]
    fn parses_and_drops_dangling_edges() {
        let d = data();
        assert_eq!(d.nodes.len(), 3);
        assert_eq!(d.edges, [[0, 1], [1, 2]]);
        assert_eq!(d.max_rank, 0.5);
    }

    #[test]
    fn drops_self_edges() {
        let d = GraphData::from_json(&json!({
            "nodes": [{"id": "a"}, {"id": "b"}],
            "edges": [[0, 0], [0, 1], [1, 1]],
        }));
        assert_eq!(d.edges, [[0, 1]]);
    }

    #[test]
    fn fit_shows_everything() {
        let d = data();
        let camera = Camera::fit(&d, 800.0, 600.0);
        assert_eq!((camera.cx, camera.cy), (50.0, 25.0));
        for node in &d.nodes {
            let (x, y) = camera.to_screen(node.x, node.y, 800.0, 600.0);
            assert!(
                (0.0..=800.0).contains(&x) && (0.0..=600.0).contains(&y),
                "{x},{y}"
            );
        }
        assert_eq!(visible_nodes(&d, &camera, 800.0, 600.0).len(), 3);
    }

    #[test]
    fn screen_and_world_round_trip() {
        let camera = Camera {
            zoom: 2.5,
            cx: 10.0,
            cy: -4.0,
        };
        let (sx, sy) = camera.to_screen(33.0, 7.0, 640.0, 480.0);
        let (wx, wy) = camera.to_world(sx, sy, 640.0, 480.0);
        assert!((wx - 33.0).abs() < 1e-3 && (wy - 7.0).abs() < 1e-3);
    }

    #[test]
    fn zooming_keeps_the_point_under_the_cursor() {
        let mut camera = Camera {
            zoom: 1.0,
            cx: 0.0,
            cy: 0.0,
        };
        let before = camera.to_world(200.0, 150.0, 800.0, 600.0);
        camera.zoom_at(200.0, 150.0, 3.0, 800.0, 600.0);
        let after = camera.to_world(200.0, 150.0, 800.0, 600.0);
        assert!((before.0 - after.0).abs() < 1e-3 && (before.1 - after.1).abs() < 1e-3);
        assert_eq!(camera.zoom, 3.0);
        camera.zoom_at(0.0, 0.0, 1e9, 800.0, 600.0);
        assert_eq!(camera.zoom, MAX_ZOOM);
    }

    #[test]
    fn panning_moves_by_screen_distance() {
        let mut camera = Camera {
            zoom: 2.0,
            cx: 0.0,
            cy: 0.0,
        };
        let before = camera.to_screen(10.0, 10.0, 800.0, 600.0);
        camera.pan(40.0, -20.0);
        let after = camera.to_screen(10.0, 10.0, 800.0, 600.0);
        assert_eq!((after.0 - before.0, after.1 - before.1), (40.0, -20.0));
    }

    #[test]
    fn picks_the_nearest_node_and_misses_empty_space() {
        let d = data();
        let camera = Camera::fit(&d, 800.0, 600.0);
        let (x, y) = camera.to_screen(100.0, 0.0, 800.0, 600.0);
        assert_eq!(pick(&d, &camera, 800.0, 600.0, x + 2.0, y - 1.0), Some(1));
        assert_eq!(pick(&d, &camera, 800.0, 600.0, 5.0, 5.0), None);
    }

    #[test]
    fn caps_edges_and_culls_offscreen() {
        let d = GraphData::stress(5000);
        assert_eq!((d.nodes.len(), d.edges.len()), (5000, 10000));
        let camera = Camera::fit(&d, 1000.0, 800.0);
        assert_eq!(visible_edges(&d, &camera, 1000.0, 800.0, 3000).len(), 3000);
        assert!(visible_edges(&d, &camera, 1000.0, 800.0, 0).is_empty());
        // Zoomed far into one corner, most of the graph is outside the view.
        let close = Camera {
            zoom: 20.0,
            cx: d.nodes[0].x,
            cy: d.nodes[0].y,
        };
        assert!(visible_nodes(&d, &close, 1000.0, 800.0).len() < 500);
        let labels = label_candidates(&d, &visible_nodes(&d, &camera, 1000.0, 800.0), 30);
        assert_eq!(labels.len(), 30);
        assert!(d.nodes[labels[0].0].rank >= d.nodes[labels[29].0].rank);
    }

    #[test]
    fn overview_cells_are_bounded_by_the_view_not_the_graph() {
        let d = GraphData::stress(50_000);
        let camera = Camera::fit(&d, 940.0, 880.0);
        let cells = density_cells(&d, &camera, 940.0, 880.0);
        assert!(cells.len() <= 2700, "{}", cells.len());
        let in_view = visible_nodes(&d, &camera, 940.0, 880.0).len();
        assert!(in_view > DETAIL_LIMIT);
        // Every node inside the view is counted in exactly one cell.
        let counted: u32 = cells.iter().map(|c| c.count).sum();
        let strictly_inside = d
            .nodes
            .iter()
            .filter(|n| {
                let (x, y) = camera.to_screen(n.x, n.y, 940.0, 880.0);
                x >= 0.0 && y >= 0.0 && x < 940.0 && y < 880.0
            })
            .count() as u32;
        assert_eq!(counted, strictly_inside);
        assert!(cells.iter().all(|c| c.x < 940.0 && c.y < 880.0));
    }

    #[test]
    fn important_nodes_are_drawn_larger() {
        assert!(radius(0.5, 0.5, 1.0) > radius(0.01, 0.5, 1.0));
        assert!(radius(0.1, 0.5, 2.0) > radius(0.1, 0.5, 1.0));
    }
}
