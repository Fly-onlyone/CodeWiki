//! The constellation behind the glass: a made-up graph of notes and the links
//! between them, which the backdrop (`backdrop.rs`) draws over its soft
//! color.
//!
//! It is the same everywhere. A fixed seed lays it out, so every start and
//! every repository shows the same sky, and nothing is asked of the backend.
//! Nodes are spread over the whole window as shares of its width and height:
//! the dense reading column hides what falls behind it, and the clear margins
//! beside it show the rest.
//!
//! The nodes form three clusters, one for each of the theme's brand colors.
//! Inside a cluster the links make a tree around a hub, the way a map of
//! content links its notes, plus a few links that close loops; one bridge
//! joins each pair of clusters where they come closest.
//!
//! Light travels along the links. A pulse runs from one end of a link to the
//! other, and may go on along another link from the node it reaches. Pulses
//! come now and then by themselves ([`Traffic`]), and often while the wiki is
//! being written.
//!
//! No UI types, so all of it is unit-tested.

use std::f32::consts::{PI, TAU};

/// Nodes in the constellation: a grid of `COLUMNS` by `ROWS`, each moved off
/// its cell's middle.
const COLUMNS: usize = 7;
const ROWS: usize = 6;
pub const NODES: usize = COLUMNS * ROWS;
/// How far a node may stray from its cell's middle, as a share of the cell.
const JITTER: f32 = 0.38;

/// Clusters, one per brand color.
pub const CLUSTERS: usize = 3;
/// Where each cluster gathers, as shares of the window's width and height:
/// one behind the sidebar and the left margin, one up and right, one low and
/// right, so that each color shows beside the page.
const CENTRES: [(f32, f32); CLUSTERS] = [(0.16, 0.38), (0.78, 0.18), (0.66, 0.82)];

/// Width over height of the window the distances are measured for, so that
/// "nearest" looks nearest on a wide window.
const ASPECT: f32 = 1.6;

/// Share of the nodes that get one more link, to their second nearest
/// neighbour in the cluster, which closes a loop.
const EXTRA_LINKS: f32 = 0.3;

/// The seed the sky is laid out from. Any value makes a sky; this one was
/// looked at.
const SEED: u64 = 0xC0DE_517E_2026_1124;

/// How far a node wanders from its place, as a share of the window's shorter
/// side.
const REACH: f32 = 0.012;

/// A small, fast generator of numbers that look random: SplitMix64. The sky
/// and the pulses need nothing better, and no crate is brought in for it.
#[derive(Clone, Debug)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number from 0 up to, not including, 1.
    fn unit(&mut self) -> f32 {
        // The top 24 bits: exactly what an f32 holds.
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// A number from 0 up to, not including, `n`, which must not be 0.
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    /// `middle`, moved by up to `spread` of itself either way.
    fn around(&mut self, middle: f32, spread: f32) -> f32 {
        middle * (1.0 + spread * (self.unit() * 2.0 - 1.0))
    }
}

/// One note of the sky.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// Resting place, as shares of the window's width and height.
    pub at: (f32, f32),
    /// Which brand color it takes.
    pub cluster: usize,
    /// Links that end here. A hub has the most and is drawn largest.
    pub degree: usize,
    /// Whole turns per loop of the drift, across and down.
    turns: (f32, f32),
    /// Where in its turn it starts, from 0 to 1.
    offset: f32,
}

impl Node {
    /// Where the node is at `phase` of the drift (0 to 1), in a window
    /// `width` by `height`.
    pub fn place(&self, phase: f32, width: f32, height: f32) -> (f32, f32) {
        let unit = width.min(height);
        let angle = |turns: f32| TAU * (phase * turns + self.offset);
        (
            self.at.0 * width + REACH * unit * angle(self.turns.0).sin(),
            self.at.1 * height + REACH * unit * angle(self.turns.1).cos(),
        )
    }
}

/// The whole sky: its nodes and the links between them.
#[derive(Clone, Debug, PartialEq)]
pub struct Constellation {
    pub nodes: Vec<Node>,
    /// Index pairs into `nodes`, the smaller first.
    pub links: Vec<[usize; 2]>,
    /// The links that end at each node.
    touching: Vec<Vec<usize>>,
}

/// Distance between two resting places, measured as on a wide window.
fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0) * ASPECT).hypot(a.1 - b.1)
}

/// Link `a` and `b`, unless they are one node or linked already.
fn add_link(links: &mut Vec<[usize; 2]>, a: usize, b: usize) {
    let pair = [a.min(b), a.max(b)];
    if a != b && !links.contains(&pair) {
        links.push(pair);
    }
}

impl Constellation {
    /// The sky every window shows.
    pub fn new() -> Self {
        Self::from_seed(SEED)
    }

    fn from_seed(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let mut nodes: Vec<Node> = (0..NODES)
            .map(|index| {
                let (column, row) = (index % COLUMNS, index / COLUMNS);
                let jitter = |rng: &mut Rng| (rng.unit() * 2.0 - 1.0) * JITTER;
                let at = (
                    (column as f32 + 0.5 + jitter(&mut rng)) / COLUMNS as f32,
                    (row as f32 + 0.5 + jitter(&mut rng)) / ROWS as f32,
                );
                let cluster = (0..CLUSTERS)
                    .min_by(|&a, &b| distance(at, CENTRES[a]).total_cmp(&distance(at, CENTRES[b])))
                    .unwrap_or(0);
                Node {
                    at,
                    cluster,
                    degree: 0,
                    turns: (1.0 + rng.below(3) as f32, 1.0 + rng.below(3) as f32),
                    offset: rng.unit(),
                }
            })
            .collect();

        let mut links: Vec<[usize; 2]> = Vec::new();
        let members: Vec<Vec<usize>> = (0..CLUSTERS)
            .map(|cluster| {
                (0..NODES)
                    .filter(|&n| nodes[n].cluster == cluster)
                    .collect()
            })
            .collect();
        for (cluster, members) in members.iter().enumerate() {
            // A tree from the hub, the node nearest the cluster's centre:
            // Prim's, so each node joins by the shortest link it can.
            let Some(&hub) = members.iter().min_by(|&&a, &&b| {
                distance(nodes[a].at, CENTRES[cluster])
                    .total_cmp(&distance(nodes[b].at, CENTRES[cluster]))
            }) else {
                continue;
            };
            let mut joined = vec![hub];
            while joined.len() < members.len() {
                let nearest = members
                    .iter()
                    .filter(|&&n| !joined.contains(&n))
                    .flat_map(|&n| joined.iter().map(move |&j| (n, j)))
                    .min_by(|&(a, b), &(c, d)| {
                        distance(nodes[a].at, nodes[b].at)
                            .total_cmp(&distance(nodes[c].at, nodes[d].at))
                    });
                let Some((new, to)) = nearest else { break };
                add_link(&mut links, new, to);
                joined.push(new);
            }
            // A few loops.
            for &node in members {
                if rng.unit() >= EXTRA_LINKS {
                    continue;
                }
                let mut others: Vec<usize> =
                    members.iter().copied().filter(|&n| n != node).collect();
                others.sort_by(|&a, &b| {
                    distance(nodes[node].at, nodes[a].at)
                        .total_cmp(&distance(nodes[node].at, nodes[b].at))
                });
                if let Some(&second) = others.get(1) {
                    add_link(&mut links, node, second);
                }
            }
        }
        // A bridge between each pair of clusters, where they come closest.
        for a in 0..CLUSTERS {
            for b in a + 1..CLUSTERS {
                let closest = members[a]
                    .iter()
                    .flat_map(|&x| members[b].iter().map(move |&y| (x, y)))
                    .min_by(|&(p, q), &(r, s)| {
                        distance(nodes[p].at, nodes[q].at)
                            .total_cmp(&distance(nodes[r].at, nodes[s].at))
                    });
                if let Some((x, y)) = closest {
                    add_link(&mut links, x, y);
                }
            }
        }

        let mut touching = vec![Vec::new(); NODES];
        for (index, &[a, b]) in links.iter().enumerate() {
            touching[a].push(index);
            touching[b].push(index);
        }
        for (node, links) in nodes.iter_mut().zip(&touching) {
            node.degree = links.len();
        }
        Self {
            nodes,
            links,
            touching,
        }
    }

    /// The links that end at `node`.
    pub fn touching(&self, node: usize) -> &[usize] {
        &self.touching[node]
    }

    /// The other end of `link` from `node`.
    fn across(&self, link: usize, node: usize) -> usize {
        let [a, b] = self.links[link];
        if a == node { b } else { a }
    }

    /// How long a link is, measured as on a wide window.
    fn length(&self, link: usize) -> f32 {
        let [a, b] = self.links[link];
        distance(self.nodes[a].at, self.nodes[b].at)
    }
}

impl Default for Constellation {
    fn default() -> Self {
        Self::new()
    }
}

/// Light running along one link.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pulse {
    pub link: usize,
    /// The node it set out from; it runs to the link's other end.
    pub from: usize,
    /// When it set out, in seconds of the backdrop's clock.
    pub start: f32,
    /// Seconds it takes to run the link.
    pub duration: f32,
    /// Links it may still go on along after this one.
    hops: u8,
}

impl Pulse {
    /// How far along the link it is at `clock`, from 0 to 1.
    pub fn progress(&self, clock: f32) -> f32 {
        ((clock - self.start) / self.duration).clamp(0.0, 1.0)
    }

    /// The node it runs to.
    pub fn to(&self, sky: &Constellation) -> usize {
        sky.across(self.link, self.from)
    }

    /// How bright it is at `progress`: it swells from nothing as it leaves a
    /// node and fades as it reaches the next.
    pub fn brightness(progress: f32) -> f32 {
        // At 1 the sine comes out a hair below zero.
        (PI * progress).sin().max(0.0).sqrt()
    }

    /// How strongly the node it runs to flashes at `progress`: from nothing
    /// until `FLASH_FROM` of the way, up and down again by its arrival.
    pub fn flash(progress: f32) -> f32 {
        let t = ((progress - FLASH_FROM) / (1.0 - FLASH_FROM)).clamp(0.0, 1.0);
        (PI * t).sin().max(0.0)
    }
}

/// How far along its link a pulse is when the node ahead starts to flash.
const FLASH_FROM: f32 = 0.6;

/// Speed of a pulse, in the units of [`distance`] per second.
const SPEED: f32 = 0.22;
/// The shortest and longest a pulse takes for one link, in seconds.
const RUN: (f32, f32) = (0.7, 2.6);

/// Seconds between two pulses, on average, when nothing is going on, and
/// while the wiki is being written.
const QUIET_EVERY: f32 = 3.5;
const BUSY_EVERY: f32 = 0.35;
/// The most pulses running at once, in each case.
const QUIET_CAP: usize = 4;
const BUSY_CAP: usize = 18;
/// The most links a pulse runs, in each case.
const QUIET_HOPS: usize = 3;
const BUSY_HOPS: usize = 6;

/// Seconds a pulse takes to run a link `length` long.
fn run_time(length: f32) -> f32 {
    (length / SPEED).clamp(RUN.0, RUN.1)
}

/// The pulses on their way, and when the next one sets out.
#[derive(Clone, Debug)]
pub struct Traffic {
    pub pulses: Vec<Pulse>,
    /// When the next pulse sets out, in seconds of the backdrop's clock.
    next: f32,
    rng: Rng,
}

impl Traffic {
    pub fn new(seed: u64) -> Self {
        Self {
            pulses: Vec::new(),
            // The first pulse comes soon after the window opens.
            next: 1.0,
            rng: Rng::new(seed),
        }
    }

    /// Bring the pulses up to `clock`. A pulse that has reached its node goes
    /// on along another link while it has hops left, or ends; new pulses set
    /// out, often while `busy`.
    pub fn advance(&mut self, sky: &Constellation, clock: f32, busy: bool) {
        if sky.links.is_empty() {
            return;
        }
        let (every, cap, hops) = if busy {
            (BUSY_EVERY, BUSY_CAP, BUSY_HOPS)
        } else {
            (QUIET_EVERY, QUIET_CAP, QUIET_HOPS)
        };
        // Writing just began: start at once, not after a quiet wait.
        if busy && self.next - clock > every * 1.5 {
            self.next = clock;
        }

        let mut going = Vec::with_capacity(self.pulses.len());
        for pulse in std::mem::take(&mut self.pulses) {
            let arrives = pulse.start + pulse.duration;
            if clock < arrives {
                going.push(pulse);
                continue;
            }
            if pulse.hops == 0 {
                continue;
            }
            let at = pulse.to(sky);
            let onward: Vec<usize> = sky
                .touching(at)
                .iter()
                .copied()
                .filter(|&l| l != pulse.link)
                .collect();
            if onward.is_empty() {
                continue;
            }
            let link = onward[self.rng.below(onward.len())];
            going.push(Pulse {
                link,
                from: at,
                start: arrives,
                duration: run_time(sky.length(link)),
                hops: pulse.hops - 1,
            });
        }
        self.pulses = going;

        while self.next <= clock {
            if self.pulses.len() < cap {
                let link = self.rng.below(sky.links.len());
                let [a, b] = sky.links[link];
                let from = if self.rng.unit() < 0.5 { a } else { b };
                self.pulses.push(Pulse {
                    link,
                    from,
                    start: self.next,
                    duration: run_time(sky.length(link)),
                    hops: self.rng.below(hops + 1) as u8,
                });
            }
            self.next += self.rng.around(every, 0.5);
        }
    }
}

/// How far `p` is from the nearest point of the segment from `a` to `b`.
pub fn segment_distance(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = dx * dx + dy * dy;
    let t = if length > 0.0 {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p.0 - (a.0 + t * dx)).hypot(p.1 - (a.1 + t * dy))
}

/// How strongly something at `distance` pixels from the pointer is lit, from
/// 1 under the pointer to 0 at `radius` and beyond, easing out at the rim.
pub fn lit(distance: f32, radius: f32) -> f32 {
    if radius <= 0.0 {
        return 0.0;
    }
    let t = (1.0 - distance / radius).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sky_is_the_same_every_time() {
        assert_eq!(Constellation::new(), Constellation::new());
        assert_ne!(Constellation::new(), Constellation::from_seed(7));
    }

    #[test]
    fn every_node_rests_inside_the_window() {
        let sky = Constellation::new();
        assert_eq!(sky.nodes.len(), NODES);
        for node in &sky.nodes {
            let (x, y) = node.at;
            assert!(
                (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y),
                "{x}, {y}"
            );
        }
    }

    #[test]
    fn every_cluster_has_nodes_enough_to_read_as_a_graph() {
        let sky = Constellation::new();
        for cluster in 0..CLUSTERS {
            let count = sky.nodes.iter().filter(|n| n.cluster == cluster).count();
            assert!(count >= 6, "cluster {cluster} has {count} nodes");
        }
    }

    #[test]
    fn links_are_real_and_unique_and_every_node_has_one() {
        let sky = Constellation::new();
        for &[a, b] in &sky.links {
            assert!(a < b && b < NODES, "{a}-{b}");
        }
        let mut sorted = sky.links.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), sky.links.len());
        for (index, node) in sky.nodes.iter().enumerate() {
            assert!(node.degree >= 1, "node {index} has no link");
            assert_eq!(node.degree, sky.touching(index).len());
        }
        let ends: usize = sky.nodes.iter().map(|n| n.degree).sum();
        assert_eq!(ends, sky.links.len() * 2);
    }

    #[test]
    fn the_whole_sky_is_one_graph() {
        let sky = Constellation::new();
        let mut seen = vec![false; NODES];
        let mut stack = vec![0];
        while let Some(node) = stack.pop() {
            if std::mem::replace(&mut seen[node], true) {
                continue;
            }
            stack.extend(sky.touching(node).iter().map(|&l| sky.across(l, node)));
        }
        assert!(seen.iter().all(|&s| s), "{seen:?}");
    }

    #[test]
    fn hubs_stand_out() {
        let sky = Constellation::new();
        let most = sky.nodes.iter().map(|n| n.degree).max().unwrap_or(0);
        assert!(most >= 3, "{most}");
    }

    #[test]
    fn a_node_closes_its_loop_and_stays_within_reach() {
        let sky = Constellation::new();
        let (width, height) = (1600.0, 900.0);
        let reach = REACH * height;
        for node in &sky.nodes {
            let rest = (node.at.0 * width, node.at.1 * height);
            let (x0, y0) = node.place(0.0, width, height);
            let (x1, y1) = node.place(1.0, width, height);
            assert!((x0 - x1).abs() < 1e-2 && (y0 - y1).abs() < 1e-2);
            for step in 0..=100 {
                let (x, y) = node.place(step as f32 / 100.0, width, height);
                assert!((x - rest.0).abs() <= reach + 1e-3);
                assert!((y - rest.1).abs() <= reach + 1e-3);
            }
        }
    }

    #[test]
    fn the_generator_stays_in_its_range() {
        let mut rng = Rng::new(1);
        for _ in 0..10_000 {
            let u = rng.unit();
            assert!((0.0..1.0).contains(&u));
            assert!(rng.below(5) < 5);
            let a = rng.around(2.0, 0.5);
            assert!((1.0..=3.0).contains(&a));
        }
    }

    #[test]
    fn a_pulse_runs_from_nothing_to_its_node() {
        let pulse = Pulse {
            link: 0,
            from: 0,
            start: 10.0,
            duration: 2.0,
            hops: 0,
        };
        assert_eq!(pulse.progress(9.0), 0.0);
        assert_eq!(pulse.progress(11.0), 0.5);
        assert_eq!(pulse.progress(20.0), 1.0);
        assert_eq!(Pulse::brightness(0.0), 0.0);
        assert!((Pulse::brightness(0.5) - 1.0).abs() < 1e-6);
        assert!(Pulse::brightness(1.0) < 1e-3);
        // The node ahead flashes as the pulse comes in, and is dark again
        // once it has arrived.
        assert_eq!(Pulse::flash(0.3), 0.0);
        assert!(Pulse::flash(0.8) > 0.9);
        assert!(Pulse::flash(1.0) < 1e-3);
    }

    #[test]
    fn a_segment_is_as_far_as_its_nearest_point() {
        let (a, b) = ((0.0, 0.0), (10.0, 0.0));
        assert_eq!(segment_distance((5.0, 3.0), a, b), 3.0);
        assert_eq!(segment_distance((-4.0, 3.0), a, b), 5.0);
        assert_eq!(segment_distance((13.0, 4.0), a, b), 5.0);
        // A segment of no length is its one point.
        assert_eq!(segment_distance((3.0, 4.0), a, a), 5.0);
    }

    #[test]
    fn a_pulse_takes_a_while_on_any_link() {
        assert_eq!(run_time(0.0), RUN.0);
        assert_eq!(run_time(10.0), RUN.1);
        let mid = run_time(0.3);
        assert!(RUN.0 < mid && mid < RUN.1);
    }

    /// Pulses that set out over `seconds`, advanced at about 24 frames a
    /// second.
    fn departures(busy: bool, seconds: f32) -> usize {
        let sky = Constellation::new();
        let mut traffic = Traffic::new(3);
        let mut seen = std::collections::HashSet::new();
        let mut clock = 0.0;
        while clock < seconds {
            clock += 0.042;
            traffic.advance(&sky, clock, busy);
            for pulse in &traffic.pulses {
                seen.insert((pulse.start.to_bits(), pulse.link));
            }
        }
        seen.len()
    }

    #[test]
    fn pulses_come_now_and_then_and_often_while_busy() {
        let quiet = departures(false, 60.0);
        let busy = departures(true, 60.0);
        assert!((8..=90).contains(&quiet), "{quiet} quiet pulses a minute");
        assert!(busy > quiet * 4, "{busy} busy against {quiet} quiet");
    }

    #[test]
    fn no_more_pulses_run_than_the_cap() {
        let sky = Constellation::new();
        for (busy, cap) in [(false, QUIET_CAP), (true, BUSY_CAP)] {
            let mut traffic = Traffic::new(5);
            let mut clock = 0.0;
            while clock < 120.0 {
                clock += 0.042;
                traffic.advance(&sky, clock, busy);
                // A pulse that goes on takes the place of the one that
                // arrived, so only new ones can add to the count.
                assert!(traffic.pulses.len() <= cap, "{}", traffic.pulses.len());
            }
        }
    }

    #[test]
    fn a_pulse_goes_on_from_the_node_it_reached() {
        let sky = Constellation::new();
        let hub = (0..NODES).max_by_key(|&n| sky.nodes[n].degree).unwrap();
        let link = sky.touching(hub)[0];
        let from = sky.across(link, hub);
        let mut traffic = Traffic::new(9);
        traffic.next = f32::MAX;
        traffic.pulses.push(Pulse {
            link,
            from,
            start: 0.0,
            duration: 1.0,
            hops: 1,
        });
        traffic.advance(&sky, 1.01, false);
        let [next] = traffic.pulses[..] else {
            panic!("{:?}", traffic.pulses);
        };
        assert_eq!(next.from, hub);
        assert_ne!(next.link, link);
        assert_eq!(next.start, 1.0);
        assert_eq!(next.hops, 0);
        // Once it arrives it ends.
        traffic.advance(&sky, next.start + next.duration + 0.01, false);
        assert!(traffic.pulses.is_empty());
    }

    #[test]
    fn writing_starts_pulses_at_once() {
        let sky = Constellation::new();
        let mut traffic = Traffic::new(11);
        traffic.next = 100.0;
        traffic.advance(&sky, 10.0, true);
        assert!(!traffic.pulses.is_empty());
    }

    #[test]
    fn the_pointer_lights_what_is_near_it() {
        assert_eq!(lit(0.0, 100.0), 1.0);
        assert_eq!(lit(100.0, 100.0), 0.0);
        assert_eq!(lit(500.0, 100.0), 0.0);
        let (near, far) = (lit(20.0, 100.0), lit(70.0, 100.0));
        assert!(near > far && far > 0.0);
        assert_eq!(lit(0.0, 0.0), 0.0);
    }
}
