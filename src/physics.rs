use std::cell::RefCell;
use std::rc::Rc;

use crate::graph::Graph;
use crate::renderer::RendererProperties;

pub fn apply_physics<const N: usize>(graph: &mut Graph, params: &RendererProperties) {
    let active: Vec<usize> = (0..graph.nodes.len())
        .filter(|&i| graph.nodes[i].borrow().appeared)
        .collect();

    for (ai, &i) in active.iter().enumerate() {
        for &j in &active[(ai + 1)..] {
            let pa = graph.nodes[i].borrow().position;
            let pb = graph.nodes[j].borrow().position;
            let dx = pa.x - pb.x;
            let dy = pa.y - pb.y;
            let mut dist_sq = dx * dx + dy * dy;
            if N == 3 {
                let dz = pa.z - pb.z;
                dist_sq += dz * dz;
            }
            let mut dist = dist_sq.sqrt();
            if dist < params.min_dist as f32 {
                dist = params.min_dist as f32;
            }
            let force = params.repel_force as f32 / (dist * dist);
            let fx = force * dx / dist;
            let fy = force * dy / dist;
            {
                let mut va = graph.nodes[i].borrow_mut();
                va.velocity.x += fx;
                va.velocity.y += fy;
                if N == 3 {
                    let fz = force * (pa.z - pb.z) / dist;
                    va.velocity.z += fz;
                }
            }
            {
                let mut vb = graph.nodes[j].borrow_mut();
                vb.velocity.x -= fx;
                vb.velocity.y -= fy;
                if N == 3 {
                    let fz = force * (pa.z - pb.z) / dist;
                    vb.velocity.z -= fz;
                }
            }
        }
    }

    for &i in active.iter() {
        let edge_targets: Vec<Rc<RefCell<crate::graph::Node>>> = {
            let node = graph.nodes[i].borrow();
            node.edges
                .iter()
                .filter(|e| e.target.borrow().appeared)
                .map(|e| e.target.clone())
                .collect()
        };

        for target_rc in &edge_targets {
            if Rc::ptr_eq(&graph.nodes[i], target_rc) {
                continue;
            }

            let pa = graph.nodes[i].borrow().position;
            let pb = target_rc.borrow().position;
            let dx = pb.x - pa.x;
            let dy = pb.y - pa.y;
            let mut dist_sq = dx * dx + dy * dy;
            if N == 3 {
                let dz = pb.z - pa.z;
                dist_sq += dz * dz;
            }
            let mut dist = dist_sq.sqrt();
            if dist < params.min_dist as f32 {
                dist = params.min_dist as f32;
            }
            let force = params.link_force as f32 * (dist - params.link_distance as f32);
            let fx = force * dx / dist;
            let fy = force * dy / dist;
            {
                let mut sn = graph.nodes[i].borrow_mut();
                sn.velocity.x += fx;
                sn.velocity.y += fy;
                if N == 3 {
                    let fz = force * (pb.z - pa.z) / dist;
                    sn.velocity.z += fz;
                }
            }
            {
                let mut tn = target_rc.borrow_mut();
                tn.velocity.x -= fx;
                tn.velocity.y -= fy;
                if N == 3 {
                    let fz = force * (pb.z - pa.z) / dist;
                    tn.velocity.z -= fz;
                }
            }
        }
    }

    for &i in &active {
        let mut node = graph.nodes[i].borrow_mut();
        let p = node.position;
        node.velocity.x += -p.x * params.center_force as f32;
        node.velocity.y += -p.y * params.center_force as f32;
        if N == 3 {
            node.velocity.z += -p.z * params.center_force as f32;
        }

        node.velocity.x *= params.damping as f32;
        node.velocity.y *= params.damping as f32;
        let mut speed_sq = node.velocity.x * node.velocity.x + node.velocity.y * node.velocity.y;
        if N == 3 {
            node.velocity.z *= params.damping as f32;
            speed_sq += node.velocity.z * node.velocity.z;
        }
        let speed = speed_sq.sqrt();
        if speed > params.max_vel as f32 {
            let scale = params.max_vel as f32 / speed;
            node.velocity.x *= scale;
            node.velocity.y *= scale;
            if N == 3 {
                node.velocity.z *= scale;
            }
        }
        node.position.x += node.velocity.x;
        node.position.y += node.velocity.y;
        if N == 3 {
            node.position.z += node.velocity.z;
        }
    }
}

#[cfg(test)]
mod tests {
    use raylib::prelude::Vector3;
    use tempfile::tempdir;

    use super::apply_physics;
    use crate::renderer::RendererProperties;
    use crate::vault::Vault;

    fn graph_with_two_nodes() -> crate::graph::Graph {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "[[b]]").unwrap();
        std::fs::write(dir.path().join("b.md"), "").unwrap();
        let vault = Vault::scan(dir.path()).unwrap();
        vault.build_graph().unwrap()
    }

    #[test]
    fn unappeared_nodes_are_frozen() {
        let mut graph = graph_with_two_nodes();
        for node_rc in &graph.nodes {
            let name = node_rc.borrow().name.clone();
            let mut node = node_rc.borrow_mut();
            node.position = Vector3::new(10.0, 0.0, 0.0);
            node.velocity = Vector3::new(0.0, 0.0, 0.0);
            node.appeared = name == "b";
        }

        let a_before = graph
            .nodes
            .iter()
            .find(|n| n.borrow().name == "a")
            .unwrap()
            .borrow()
            .position;
        let b_before = graph
            .nodes
            .iter()
            .find(|n| n.borrow().name == "b")
            .unwrap()
            .borrow()
            .position;

        apply_physics::<2>(&mut graph, &RendererProperties::default());

        let a_after = graph
            .nodes
            .iter()
            .find(|n| n.borrow().name == "a")
            .unwrap()
            .borrow()
            .position;
        let b_after = graph
            .nodes
            .iter()
            .find(|n| n.borrow().name == "b")
            .unwrap()
            .borrow()
            .position;

        assert_eq!(a_before.x, a_after.x);
        assert_eq!(a_before.y, a_after.y);
        assert!(
            (b_after.x - b_before.x).abs() > 0.01,
            "active node b should move toward origin"
        );
        assert_eq!(b_before.y, b_after.y);
    }
}
