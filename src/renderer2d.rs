use std::cell::RefCell;
use std::rc::Rc;

use raylib::prelude::*;

use crate::graph::{Graph, Node};
use crate::physics;
use crate::renderer::{
    BG, Dim, EDGE_COLOR, HIGHLIGHT_COLOR, MISSING_COLOR, NODE_COLOR, Renderer, RendererProperties,
    TEXT_COLOR,
};

#[derive(Default)]
pub struct Renderer2D {
    properties: RendererProperties,
}

impl Renderer2D {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Renderer for Renderer2D {
    fn physics_step(&self, graph: &mut Graph) {
        physics::apply_physics::<2>(graph, &self.properties);
    }

    fn hit_node(&self, graph: &Graph) -> Option<Rc<RefCell<Node>>> {
        let threshold = self.properties.node_radius as f64 * self.properties.zoom + 5.0;
        crate::renderer::find_nearest_node(
            graph,
            self.properties.mouse_x,
            self.properties.mouse_y,
            threshold,
            |pos| {
                let sx = (pos.x as f64 + self.properties.pan.width) * self.properties.zoom;
                let sy = (pos.y as f64 + self.properties.pan.height) * self.properties.zoom;
                (sx, sy)
            },
        )
    }

    fn run(&mut self, graph: &mut Graph) {
        let (mut rl, thread) = raylib::init()
            .size(1200, 800)
            .resizable()
            .title("obsidian-graph")
            .build();

        self.properties.pan = Default::default();
        self.properties.zoom = 1.0;
        self.properties.dragged_node = None;
        self.properties.panning = false;

        while !rl.window_should_close() {
            self.properties.screen_width = rl.get_screen_width() as f64;
            self.properties.screen_height = rl.get_screen_height() as f64;
            self.properties.mouse_x = rl.get_mouse_x() as f64;
            self.properties.mouse_y = rl.get_mouse_y() as f64;

            let wheel = rl.get_mouse_wheel_move();
            if wheel > 0.0 {
                self.properties.zoom *= 1.1;
            } else if wheel < 0.0 {
                self.properties.zoom /= 1.1;
            }

            if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
                if let Some(node) = self.hit_node(graph) {
                    self.properties.dragged_node = Some(node);
                } else {
                    self.properties.panning = true;
                    self.properties.drag_start = Dim {
                        width: self.properties.mouse_x,
                        height: self.properties.mouse_y,
                    };
                    self.properties.pan_start = self.properties.pan;
                }
            }
            if rl.is_mouse_button_released(MouseButton::MOUSE_BUTTON_LEFT) {
                self.properties.dragged_node = None;
                self.properties.panning = false;
            }

            if let Some(ref node_rc) = self.properties.dragged_node {
                let mut node = node_rc.borrow_mut();
                node.position.x = (self.properties.mouse_x / self.properties.zoom
                    - self.properties.pan.width) as f32;
                node.position.y = (self.properties.mouse_y / self.properties.zoom
                    - self.properties.pan.height) as f32;
                node.velocity = Vector3::new(0.0, 0.0, 0.0);
            }

            if self.properties.panning {
                let dx = self.properties.mouse_x - self.properties.drag_start.width;
                let dy = self.properties.mouse_y - self.properties.drag_start.height;
                self.properties.pan = Dim {
                    width: self.properties.pan_start.width + dx / self.properties.zoom,
                    height: self.properties.pan_start.height + dy / self.properties.zoom,
                };
            }

            self.physics_step(graph);

            let mut d = rl.begin_drawing(&thread);
            d.clear_background(BG);

            let p0 = self.properties.pan.width;
            let p1 = self.properties.pan.height;
            for node_rc in &graph.nodes {
                let node = node_rc.borrow();
                let sx = (node.position.x as f64 + p0) * self.properties.zoom;
                let sy = (node.position.y as f64 + p1) * self.properties.zoom;
                for edge in &node.edges {
                    let target = edge.target.borrow();
                    let tx = (target.position.x as f64 + p0) * self.properties.zoom;
                    let ty = (target.position.y as f64 + p1) * self.properties.zoom;
                    d.draw_line_ex(
                        Vector2::new(sx as f32, sy as f32),
                        Vector2::new(tx as f32, ty as f32),
                        1.0,
                        EDGE_COLOR,
                    );
                }
            }

            for node_rc in &graph.nodes {
                let node = node_rc.borrow();
                let x = (node.position.x as f64 + p0) * self.properties.zoom;
                let y = (node.position.y as f64 + p1) * self.properties.zoom;
                let color = if self
                    .properties
                    .dragged_node
                    .as_ref()
                    .is_some_and(|d| Rc::ptr_eq(d, node_rc))
                {
                    HIGHLIGHT_COLOR
                } else if !node.exists {
                    MISSING_COLOR
                } else {
                    NODE_COLOR
                };
                d.draw_circle(
                    x as i32,
                    y as i32,
                    self.properties.node_radius * self.properties.zoom as f32,
                    color,
                );
                let font_size =
                    (self.properties.font_size as f64 * self.properties.zoom).max(6.0) as i32;
                d.draw_text(
                    &node.name,
                    x as i32 + self.properties.node_radius as i32 + 4,
                    y as i32 - font_size / 2,
                    font_size,
                    TEXT_COLOR,
                );
            }
        }
    }
}
