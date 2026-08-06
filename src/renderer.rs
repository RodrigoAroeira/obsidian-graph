use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use anyhow::Result;
use raylib::prelude::{Color, Vector3};

use crate::graph::{Graph, Node};
use crate::vault::Vault;

pub const BG: Color = Color::new(30, 30, 30, 255);
pub const NODE_COLOR: Color = Color::new(100, 150, 255, 255);
pub const EDGE_COLOR: Color = Color::new(60, 60, 60, 255);
pub const TEXT_COLOR: Color = Color::new(200, 200, 200, 255);
pub const HIGHLIGHT_COLOR: Color = Color::new(255, 200, 60, 255);
pub const MISSING_COLOR: Color = Color::new(150, 80, 80, 255);

#[derive(Clone, Copy, Default)]
pub struct Dim {
    pub width: f64,
    pub height: f64,
}

#[expect(dead_code)]
pub struct RendererProperties {
    pub repel_force: f64,
    pub link_force: f64,
    pub link_distance: f64,
    pub center_force: f64,
    pub damping: f64,
    pub max_vel: f64,
    pub min_dist: f64,

    pub node_radius: f32,
    pub font_size: i32,
    pub zoom: f64,
    pub dragged_node: Option<Rc<RefCell<Node>>>,
    pub panning: bool,
    pub pan: Dim,
    pub pan_start: Dim,
    pub drag_start: Dim,

    pub screen_width: f64,
    pub screen_height: f64,
    pub mouse_x: f64,
    pub mouse_y: f64,
    pub depth: f64,
}

impl Default for RendererProperties {
    fn default() -> Self {
        Self {
            repel_force: 10000.0,
            link_force: 0.005,
            link_distance: 100.0,
            center_force: 0.01,
            damping: 0.85,
            max_vel: 50.0,
            min_dist: 10.0,
            node_radius: 6.0,
            font_size: 14,
            zoom: 1.0,
            dragged_node: None,
            panning: false,
            pan: Default::default(),
            pan_start: Default::default(),
            drag_start: Default::default(),
            screen_width: 1200.0,
            screen_height: 800.0,
            mouse_x: 0.0,
            mouse_y: 0.0,
            depth: 800.0,
        }
    }
}

pub fn find_nearest_node<F>(
    graph: &Graph,
    mx: f64,
    my: f64,
    threshold: f64,
    to_screen: F,
) -> Option<Rc<RefCell<Node>>>
where
    F: Fn(&Vector3) -> (f64, f64),
{
    let mut closest: Option<(Rc<RefCell<Node>>, f64)> = None;
    for node_rc in &graph.nodes {
        let node = node_rc.borrow();
        if !node.appeared {
            continue;
        }
        let pos = node.position;
        let (sx, sy) = to_screen(&pos);
        let dx = mx - sx;
        let dy = my - sy;
        let dist = dx.hypot(dy);
        if dist < threshold && closest.as_ref().is_none_or(|(_, d)| dist < *d) {
            closest = Some((node_rc.clone(), dist));
        }
    }
    closest.map(|(n, _)| n)
}

pub trait Renderer {
    fn run(&mut self, vault: &Vault) -> Result<()>;
}

#[derive(Default)]
pub struct Spawn {
    queue: VecDeque<Rc<RefCell<Node>>>,
    rate: f64,
    accumulator: f64,
}

impl Spawn {
    pub fn from_graph(graph: &Graph) -> Self {
        let mut nodes: Vec<Rc<RefCell<Node>>> = graph.nodes.to_vec();
        for node_rc in &nodes {
            node_rc.borrow_mut().appeared = false;
        }
        nodes.sort_by(|a, b| {
            let a = a.borrow();
            let b = b.borrow();
            match (a.date_created, b.date_created) {
                (Some(x), Some(y)) => x.cmp(&y).then_with(|| a.name.cmp(&b.name)),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.name.cmp(&b.name),
            }
        });
        let rate = nodes.len() as f64 / 3.0;
        Self {
            queue: nodes.into(),
            rate,
            accumulator: 0.0,
        }
    }

    pub fn reveal_all(&mut self, graph: &Graph) {
        for node_rc in &graph.nodes {
            node_rc.borrow_mut().appeared = true;
        }
        self.queue.clear();
        self.rate = 0.0;
        self.accumulator = 0.0;
    }

    pub fn reset_from(&mut self, graph: &Graph) {
        *self = Self::from_graph(graph);
    }

    pub fn advance(&mut self, dt: f64) {
        self.accumulator += dt * self.rate;
        while self.accumulator >= 1.0 {
            match self.queue.pop_front() {
                Some(node) => node.borrow_mut().appeared = true,
                None => break,
            }
            self.accumulator -= 1.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::Spawn;
    use crate::vault::Vault;

    #[test]
    fn missing_notes_spawn_last() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "[[Missing]]").unwrap();
        let vault = Vault::scan(dir.path()).unwrap();
        let graph = vault.build_graph().unwrap();

        let mut spawn = Spawn::from_graph(&graph);
        for _ in 0..1000 {
            spawn.advance(0.1);
            if graph
                .nodes
                .iter()
                .any(|n| n.borrow().name == "a" && n.borrow().appeared)
            {
                break;
            }
        }
        assert!(
            graph
                .nodes
                .iter()
                .any(|n| n.borrow().name == "a" && n.borrow().appeared),
            "real note should appear"
        );
        assert!(
            !graph
                .nodes
                .iter()
                .any(|n| n.borrow().name == "Missing" && n.borrow().appeared),
            "missing note should appear after real notes"
        );

        spawn.advance(10.0);
        assert!(graph.nodes.iter().all(|n| n.borrow().appeared));
    }

    #[test]
    fn reveal_all_shows_everything_immediately() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "[[Missing]]").unwrap();
        let vault = Vault::scan(dir.path()).unwrap();
        let graph = vault.build_graph().unwrap();

        let mut spawn = Spawn::default();
        spawn.reveal_all(&graph);
        assert!(graph.nodes.iter().all(|n| n.borrow().appeared));

        spawn.advance(10.0);
        assert!(graph.nodes.iter().all(|n| n.borrow().appeared));
    }
}
