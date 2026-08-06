use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;
use raylib::ffi;
use raylib::prelude::*;

use crate::graph::{Graph, Node};
use crate::physics;
use crate::renderer::{
    BG, EDGE_COLOR, HIGHLIGHT_COLOR, MISSING_COLOR, NODE_COLOR, Renderer, RendererProperties,
    Spawn, TEXT_COLOR, find_nearest_node,
};
use crate::vault::Vault;

#[derive(Default)]
pub struct Renderer3D {
    properties: RendererProperties,
    cam: Option<Camera3D>,
    spawn: Spawn,
}

impl Renderer3D {
    pub fn new() -> Self {
        Default::default()
    }
    fn prepare_for_hit_node(&mut self, cam: Camera3D) {
        self.cam = Some(cam);
    }
    pub fn physics_step(&self, graph: &mut Graph) {
        physics::apply_physics::<3>(graph, &self.properties);
    }
    // self.cam is refreshed each frame by prepare_for_hit_node
    // projection uses ffi::GetWorldToScreen because &self has no RaylibHandle.
    pub fn hit_node(&self, graph: &Graph) -> Option<Rc<RefCell<Node>>> {
        let cam = self.cam?;
        let threshold = self.properties.node_radius as f64 * self.properties.zoom * 3.0;
        find_nearest_node(
            graph,
            self.properties.mouse_x,
            self.properties.mouse_y,
            threshold,
            |pos| {
                let screen = unsafe { ffi::GetWorldToScreen(*pos, cam.into()) };
                (screen.x as f64, screen.y as f64)
            },
        )
    }
}

#[derive(Clone)]
struct Camera {
    dir: Vector3,
    up: Vector3,
    radius: f32,
    target: Vector3,
}

impl Camera {
    pub fn pos(&self) -> Vector3 {
        self.dir.scale(self.radius) + self.target
    }

    pub fn orbit(&mut self, dyaw: f32, dpitch: f32) {
        self.dir = self.dir.rotate_by_axis_angle(self.up, dyaw).normalize();
        let right = self.dir.cross(self.up).normalize();
        self.dir = self.dir.rotate_by_axis_angle(right, dpitch).normalize();
        self.up = self.up.rotate_by_axis_angle(right, dpitch).normalize();
    }

    pub fn zoom(&mut self, factor: f32) {
        self.radius *= factor;
    }

    pub fn move_screen(&mut self, dx: f32, dy: f32) {
        let right = self.up.cross(self.dir).normalize();
        self.target += right.scale(dx) + self.up.scale(dy);
    }
}

fn ray_plane_intersection(ray: Ray, origin: Vector3, normal: Vector3) -> Option<Vector3> {
    let denom = ray.direction.dot(normal);
    if denom.abs() <= 1e-6 {
        return None;
    }
    let t = (origin - ray.position).dot(normal) / denom;
    Some(ray.position + ray.direction.scale(t))
}

impl Renderer for Renderer3D {
    fn run(&mut self, vault: &Vault) -> Result<()> {
        let (mut rl, thread) = raylib::init()
            .size(1200, 800)
            .resizable()
            .title("obsidian-graph (3D)")
            .build();

        let mut graph = vault.build_graph()?;

        self.properties.zoom = 1.0;
        self.properties.dragged_node = None;
        self.spawn.reveal_all(&graph);

        let mut camera = Camera {
            dir: Vector3::new(0.0, -0.5, 0.866),
            up: Vector3::new(0.0, 0.866, 0.5),
            radius: 1200.0,
            target: Vector3::new(0.0, 0.0, 0.0),
        };

        let default = camera.clone();

        while !rl.window_should_close() {
            self.properties.screen_width = rl.get_screen_width() as f64;
            self.properties.screen_height = rl.get_screen_height() as f64;
            self.properties.mouse_x = rl.get_mouse_x() as f64;
            self.properties.mouse_y = rl.get_mouse_y() as f64;

            let wheel = rl.get_mouse_wheel_move();
            if wheel != 0.0 {
                camera.zoom(1.0 - wheel * 0.1);
            }

            if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_RIGHT) {
                let delta = rl.get_mouse_delta();
                camera.orbit(-delta.x * 0.005, delta.y * 0.005);
            }

            if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT)
                && self.properties.dragged_node.is_none()
            {
                let speed = camera.pos().length() * 0.001;
                let delta = rl.get_mouse_delta() * speed;
                camera.move_screen(-delta.x, delta.y);
            }

            if matches!(rl.get_key_pressed(), Some(KeyboardKey::KEY_R)) {
                camera = default.clone();
            }

            if matches!(rl.get_key_pressed(), Some(KeyboardKey::KEY_L)) {
                match vault.rebuild_graph() {
                    Ok(fresh) => {
                        graph = fresh;
                        self.spawn.reveal_all(&graph);
                        self.properties.dragged_node = None;
                    }
                    Err(e) => eprintln!("reload failed: {e}"),
                }
            }

            if rl.is_key_pressed(KeyboardKey::KEY_A) {
                self.spawn.reset_from(&graph);
                self.properties.dragged_node = None;
            }

            let cam_pos = camera.pos();
            let cam = Camera3D::perspective(cam_pos, camera.target, camera.up, 45.0);

            self.prepare_for_hit_node(cam);

            if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
                self.properties.dragged_node = self.hit_node(&graph);
            }

            if rl.is_mouse_button_released(MouseButton::MOUSE_BUTTON_LEFT) {
                self.properties.dragged_node = None;
            }

            if let Some(ref node_rc) = self.properties.dragged_node {
                let mouse = Vector2::new(
                    self.properties.mouse_x as f32,
                    self.properties.mouse_y as f32,
                );
                let ray = rl.get_screen_to_world_ray(mouse, cam);
                let forward = (camera.target - cam_pos).normalize();
                let node_pos = node_rc.borrow().position;
                if let Some(world) = ray_plane_intersection(ray, node_pos, forward) {
                    let mut node = node_rc.borrow_mut();
                    node.position = world;
                    node.velocity = Vector3::new(0.0, 0.0, 0.0);
                }
            }

            let dt = rl.get_frame_time();
            self.spawn.advance(dt as f64);

            self.physics_step(&mut graph);

            let label_threshold = cam_pos.length() * 0.7;
            let mut screen_pos: Vec<(Rc<RefCell<Node>>, Vector2, f32)> = Vec::new();
            for node_rc in &graph.nodes {
                let node = node_rc.borrow();
                if !node.appeared {
                    continue;
                }
                let p = node.position;
                let world = p;
                let screen = rl.get_world_to_screen(world, cam);
                let dist = world.distance(cam_pos);
                screen_pos.push((node_rc.clone(), screen, dist));
            }

            let mut d = rl.begin_drawing(&thread);
            d.clear_background(BG);
            let mut d3 = d.begin_mode3D(cam);

            for node_rc in &graph.nodes {
                let node = node_rc.borrow();
                if !node.appeared {
                    continue;
                }
                let p = node.position;
                for edge in &node.edges {
                    let target = edge.target.borrow();
                    if !target.appeared {
                        continue;
                    }
                    let tp = target.position;
                    d3.draw_line3D(p, tp, EDGE_COLOR);
                }
            }

            for node_rc in &graph.nodes {
                let node = node_rc.borrow();
                if !node.appeared {
                    continue;
                }
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

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_camera() -> Camera {
        Camera {
            dir: Vector3::new(0.0, -0.5, 0.866),
            up: Vector3::new(0.0, 0.866, 0.5),
            radius: 1200.0,
            target: Vector3::new(0.0, 0.0, 0.0),
        }
    }

    fn assert_close(a: f32, b: f32) {
        let tol = 1e-3 * b.abs().max(1.0);
        assert!((a - b).abs() <= tol, "expected {a} ~= {b}");
    }

    fn assert_vec_close(a: Vector3, b: Vector3) {
        let tol = 1e-3 * b.length().max(1.0);
        let d = a - b;
        assert!(d.length() <= tol, "expected {a:?} ~= {b:?}");
    }

    #[test]
    fn initial_pos_matches_expected() {
        let cam = default_camera();
        assert_vec_close(cam.pos(), Vector3::new(0.0, -600.0, 1039.2305));
        assert_close(cam.pos().distance(cam.target), cam.radius);
    }

    #[test]
    fn orbit_keeps_radius() {
        let mut cam = default_camera();
        for _ in 0..50 {
            cam.orbit(3.0_f32.to_radians(), 1.5_f32.to_radians());
        }
        assert_close(cam.pos().distance(cam.target), cam.radius);
    }

    #[test]
    fn pitch_works_after_full_yaw() {
        let mut cam = default_camera();
        for _ in 0..90 {
            cam.orbit(1.0_f32.to_radians(), 0.0);
        }
        for _ in 0..90 {
            cam.orbit(0.0, 1.0_f32.to_radians());
        }
        let height = (cam.pos().y - cam.target.y).abs();
        assert!(
            height > cam.radius * 0.5,
            "pitch dead after yaw: height {height}"
        );
        assert_close(cam.pos().distance(cam.target), cam.radius);
    }

    #[test]
    fn orbit_crosses_pole_smoothly() {
        let mut cam = default_camera();
        let step = 2.0_f32.to_radians();
        let mut prev_pos = cam.pos();
        let mut prev_up = cam.up;
        for _ in 0..70 {
            cam.orbit(0.0, step);
            let pos = cam.pos();
            assert!(prev_pos.distance(pos) < cam.radius * 0.1, "position jump");
            assert!(prev_up.distance(cam.up) < 0.1, "up flip");
            assert!(
                cam.up.dot(cam.dir).abs() < 1e-3,
                "up no longer perpendicular"
            );
            prev_pos = pos;
            prev_up = cam.up;
        }
        assert_close(cam.pos().distance(cam.target), cam.radius);
    }

    #[test]
    fn up_stays_perpendicular_to_dir() {
        let mut cam = default_camera();
        for _ in 0..100 {
            cam.orbit(2.0_f32.to_radians(), 1.0_f32.to_radians());
        }
        assert!(cam.up.dot(cam.dir).abs() < 1e-3);
    }

    #[test]
    fn zoom_scales_radius_only() {
        let mut cam = default_camera();
        cam.target = Vector3::new(10.0, -20.0, 30.0);
        let before = cam.pos();
        cam.zoom(0.5);
        assert_close(cam.radius, 600.0);
        let expected = Vector3::new(
            cam.target.x + (before.x - cam.target.x) * 0.5,
            cam.target.y + (before.y - cam.target.y) * 0.5,
            cam.target.z + (before.z - cam.target.z) * 0.5,
        );
        assert_vec_close(cam.pos(), expected);
        assert_close(cam.pos().distance(cam.target), cam.radius);
    }

    #[test]
    fn move_screen_moves_along_screen_axes() {
        let mut cam = default_camera();
        let right = cam.up.cross(cam.dir).normalize();
        let before_target = cam.target;
        let before_pos = cam.pos();
        cam.move_screen(3.0, -2.0);
        let expected = before_target + right.scale(3.0) + cam.up.scale(-2.0);
        assert_vec_close(cam.target, expected);
        assert_close(cam.pos().distance(cam.target), cam.radius);
        assert_vec_close(cam.pos() - before_pos, cam.target - before_target);
    }

    #[test]
    fn orbit_after_pan_keeps_radius() {
        let mut cam = default_camera();
        cam.move_screen(50.0, -30.0);
        for _ in 0..60 {
            cam.orbit(2.0_f32.to_radians(), 1.0_f32.to_radians());
        }
        assert_close(cam.pos().distance(cam.target), cam.radius);
    }

    #[test]
    fn ray_plane_intersection_hits_known_point() {
        let ray = Ray {
            position: Vector3::new(0.0, 0.0, 10.0),
            direction: Vector3::new(0.0, 0.0, -1.0),
        };
        let hit = ray_plane_intersection(
            ray,
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        assert_vec_close(hit.unwrap(), Vector3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn ray_plane_intersection_handles_offset_plane() {
        let ray = Ray {
            position: Vector3::new(5.0, 0.0, 10.0),
            direction: Vector3::new(-1.0, 0.0, 0.0),
        };
        let hit = ray_plane_intersection(
            ray,
            Vector3::new(2.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
        );
        assert_vec_close(hit.unwrap(), Vector3::new(2.0, 0.0, 10.0));
    }

    #[test]
    fn ray_plane_intersection_parallel_returns_none() {
        let ray = Ray {
            position: Vector3::new(0.0, 0.0, 10.0),
            direction: Vector3::new(1.0, 0.0, 0.0),
        };
        let hit = ray_plane_intersection(
            ray,
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        assert!(hit.is_none());
    }
}
