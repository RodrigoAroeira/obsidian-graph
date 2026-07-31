use std::cell::RefCell;
use std::rc::Rc;

use crate::graph::Graph;
use raylib::prelude::{Color, Vector3};

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
    pub dragged_node: Option<Rc<RefCell<crate::graph::Node>>>,
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
) -> Option<Rc<RefCell<crate::graph::Node>>>
where
    F: Fn(&Vector3) -> (f64, f64),
{
    let mut closest: Option<(Rc<RefCell<crate::graph::Node>>, f64)> = None;
    for node_rc in &graph.nodes {
        let pos = node_rc.borrow().position;
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
    fn physics_step(&self, graph: &mut Graph);
    fn hit_node(&self, graph: &Graph) -> Option<Rc<RefCell<crate::graph::Node>>>;
    fn run(&mut self, graph: &mut Graph);
}
