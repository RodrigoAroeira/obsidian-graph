use std::cell::RefCell;
use std::rc::Rc;

use raylib::prelude::*;

use crate::graph::{Graph, Node};
use crate::physics;
use crate::renderer::{
    BG, EDGE_COLOR, HIGHLIGHT_COLOR, MISSING_COLOR, NODE_COLOR, Renderer, RendererProperties,
    TEXT_COLOR, find_nearest_node,
};

#[derive(Default)]
pub struct Renderer3D {
    properties: RendererProperties,
}

impl Renderer3D {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Renderer for Renderer3D {
    fn physics_step(&self, graph: &mut Graph) {
        physics::apply_physics::<3>(graph, &self.properties);
    }

    fn hit_node(&self, _graph: &Graph) -> Option<Rc<RefCell<Node>>> {
        None
    }

    fn run(&mut self, graph: &mut Graph) {
        let (mut rl, thread) = raylib::init()
            .size(1200, 800)
            .resizable()
            .title("obsidian-graph (3D)")
            .build();

        self.properties.zoom = 1.0;
        self.properties.dragged_node = None;

        let mut cam_yaw: f32 = 0.0;
        let mut cam_pitch: f32 = -30.0_f32.to_radians();
        let mut cam_dist: f32 = 1200.0;
        let mut pan_x: f32 = 0.0;
        let mut pan_y: f32 = 0.0;

        while !rl.window_should_close() {
            self.properties.screen_width = rl.get_screen_width() as f64;
            self.properties.screen_height = rl.get_screen_height() as f64;
            self.properties.mouse_x = rl.get_mouse_x() as f64;
            self.properties.mouse_y = rl.get_mouse_y() as f64;

            let wheel = rl.get_mouse_wheel_move();
            cam_dist *= 1.0 - wheel * 0.1;

            if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_RIGHT) {
                let delta = rl.get_mouse_delta();
                cam_yaw -= delta.x * 0.005;
                cam_pitch = (cam_pitch + delta.y * 0.005)
                    .clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians());
            }

            if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT)
                && self.properties.dragged_node.is_none()
            {
                let delta = rl.get_mouse_delta();
                let speed = cam_dist * 0.001;
                pan_x -= delta.x * speed;
                pan_y += delta.y * speed;
            }

            let cx = cam_dist * cam_pitch.cos() * cam_yaw.sin();
            let cy = cam_dist * cam_pitch.sin();
            let cz = cam_dist * cam_pitch.cos() * cam_yaw.cos();

            let cam_pos = Vector3::new(cx + pan_x, cy.max(10.0) + pan_y, cz);

            let camera = Camera3D::perspective(
                cam_pos,
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(0.0, 1.0, 0.0),
                45.0,
            );

            if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
                let threshold = self.properties.node_radius as f64 * self.properties.zoom * 3.0;
                self.properties.dragged_node = find_nearest_node(
                    graph,
                    self.properties.mouse_x,
                    self.properties.mouse_y,
                    threshold,
                    |pos| {
                        let screen = rl.get_world_to_screen(*pos, camera);
                        (screen.x as f64, screen.y as f64)
                    },
                );
            }
            if rl.is_mouse_button_released(MouseButton::MOUSE_BUTTON_LEFT) {
                self.properties.dragged_node = None;
            }

            self.physics_step(graph);

            let label_threshold = cam_dist * 0.7;
            let mut screen_pos: Vec<(Rc<RefCell<Node>>, Vector2, f32)> = Vec::new();
            for node_rc in &graph.nodes {
                let p = node_rc.borrow().position;
                let world = p;
                let screen = rl.get_world_to_screen(world, camera);
                let dist = world.distance(cam_pos);
                screen_pos.push((node_rc.clone(), screen, dist));
            }

            let mut d = rl.begin_drawing(&thread);
            d.clear_background(BG);
            let mut d3 = d.begin_mode3D(camera);

            for node_rc in &graph.nodes {
                let node = node_rc.borrow();
                let p = node.position;
                for edge in &node.edges {
                    let target = edge.target.borrow();
                    let tp = target.position;
                    d3.draw_line3D(p, tp, EDGE_COLOR);
                }
            }

            for node_rc in &graph.nodes {
                let node = node_rc.borrow();
                let p = node.position;
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
                d3.draw_sphere(p, self.properties.node_radius, color);
                d3.draw_sphere_wires(
                    p,
                    self.properties.node_radius,
                    16,
                    16,
                    Color::new(255, 255, 255, 30),
                );
            }

            drop(d3);

            let font_size =
                (self.properties.font_size as f64 * self.properties.zoom).max(6.0) as i32;
            for (node_rc, screen, dist) in &screen_pos {
                if *dist < label_threshold
                    || self
                        .properties
                        .dragged_node
                        .as_ref()
                        .is_some_and(|d| Rc::ptr_eq(d, node_rc))
                {
                    let node = node_rc.borrow();
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
                        screen.x as i32,
                        screen.y as i32,
                        self.properties.node_radius,
                        color,
                    );
                    d.draw_text(
                        &node.name,
                        screen.x as i32 + self.properties.node_radius as i32 + 4,
                        screen.y as i32 - font_size / 2,
                        font_size,
                        TEXT_COLOR,
                    );
                }
            }
        }
    }
}
