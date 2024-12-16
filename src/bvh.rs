use std::{cmp::Ordering, f32::INFINITY, mem::swap};

use crate::{
    aabb::AABB,
    hittable::{HitList, Hittable},
    interval::Interval,
    ray::Ray,
    vec3::Axis,
};

#[derive(Debug, Clone)]
pub struct BVHTree {
    nodes: Vec<BVHNode>,
    objects: Vec<Hittable>,
    indices: Vec<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct BVHNode {
    left_node_idx: u32,
    first_hittable_idx: u32,
    hittable_count: u32,
    pub bbox: AABB,
}

impl BVHNode {
    pub fn is_leaf(&self) -> bool {
        self.hittable_count > 0
    }
}

impl BVHTree {
    pub fn evaluate_sah(
        objects: &Vec<Hittable>,
        indices: &Vec<u32>,
        node: &BVHNode,
        axis: Axis,
        pos: f32,
    ) -> f32 {
        let mut left_box = AABB::EMPTY;
        let mut right_box = AABB::EMPTY;

        let mut left_count = 0;
        let mut right_count = 0;
        (0..node.hittable_count).for_each(|i| {
            let obj = &objects[indices[node.left_node_idx as usize + i as usize] as usize];
            if obj.get_bbox().centroid(axis) < pos {
                left_count += 1;
                left_box = AABB::from_aabb(&left_box, &obj.get_bbox());
            } else {
                right_count += 1;
                right_box = AABB::from_aabb(&right_box, &obj.get_bbox());
            }
        });

        let cost = left_count as f32 * left_box.area() + right_count as f32 * right_box.area();
        if cost > 0.0 {
            cost
        } else {
            INFINITY
        }
    }
    pub fn bbox(&self) -> &AABB {
        &self.nodes[0].bbox
    }
    pub fn from_hit_list(list: &HitList) -> Self {
        Self::from_hittable_vec(list.objects.clone())
    }

    pub fn from_hittable_vec(objects: Vec<Hittable>) -> Self {
        let mut indices: Vec<u32> = (0..objects.len()).map(|i| i as u32).collect();
        let mut nodes: Vec<BVHNode> = vec![Default::default(); objects.len() * 2 - 1];

        fn subdivide(
            nodes: &mut Vec<BVHNode>,
            node_idx: usize,
            nodes_used: &mut u32,
            objects: &Vec<Hittable>,
            indices: &mut Vec<u32>,
        ) {
            let mut best_axis = Axis::X;
            let mut best_cost = INFINITY;
            let mut best_pos = 0.0;
            for axis in Axis::iter() {
                (0..nodes[node_idx].hittable_count).for_each(|i| {
                    let obj = &objects[indices
                        [nodes[node_idx as usize].first_hittable_idx as usize + i as usize]
                        as usize];

                    let candidate_pos = obj.get_bbox().centroid(*axis);

                    let cost = BVHTree::evaluate_sah(
                        objects,
                        &indices,
                        &nodes[indices[node_idx as usize] as usize],
                        *axis,
                        candidate_pos,
                    );
                    if cost < best_cost {
                        best_axis = *axis;
                        best_cost = cost;
                        best_pos = candidate_pos;
                    }
                });
            }
            let axis = best_axis;
            let split_pos = best_pos;

            let e = nodes[node_idx].bbox.extent();
            let parent_area = e.x * e.y * e.y * e.z + e.z * e.x;
            let parent_cost = nodes[node_idx].hittable_count as f32 * parent_area;

            if best_cost >= parent_cost {
                return;
            }

            let mut i = nodes[node_idx].first_hittable_idx as usize;
            let mut j = (i + nodes[node_idx].hittable_count as usize - 1).saturating_sub(1);

            while i <= j {
                if objects[i].get_bbox().centroid(axis) < split_pos {
                    i += 1;
                } else {
                    indices.swap(i, j);
                    if j == 0 {
                        break;
                    }
                    j -= 1;
                }
            }

            let left_count = i - nodes[node_idx].first_hittable_idx as usize;

            if left_count == 0 || left_count == nodes[node_idx].hittable_count as usize {
                return;
            }

            let left_child_idx: usize = *nodes_used as usize;
            *nodes_used += 1;

            let right_child_idx: usize = *nodes_used as usize;
            *nodes_used += 1;

            nodes[left_child_idx].first_hittable_idx = nodes[node_idx].first_hittable_idx;
            nodes[left_child_idx].hittable_count = left_count as u32;
            for i in nodes[left_child_idx].first_hittable_idx
                ..nodes[left_child_idx].first_hittable_idx + nodes[left_child_idx].hittable_count
            {
                let obj_index = indices[i as usize];
                nodes[left_child_idx].bbox = AABB::from_aabb(
                    &nodes[left_child_idx].bbox,
                    &objects[obj_index as usize].get_bbox(),
                );
            }

            nodes[right_child_idx].first_hittable_idx = i as u32;
            nodes[right_child_idx].hittable_count =
                nodes[node_idx].hittable_count - left_count as u32;

            nodes[node_idx].left_node_idx = left_child_idx as u32;
            nodes[node_idx].hittable_count = 0;
            for i in nodes[right_child_idx].first_hittable_idx
                ..nodes[right_child_idx].first_hittable_idx + nodes[right_child_idx].hittable_count
            {
                let obj_index = indices[i as usize];
                nodes[right_child_idx].bbox = AABB::from_aabb(
                    &nodes[right_child_idx].bbox,
                    &objects[obj_index as usize].get_bbox(),
                );
            }

            subdivide(nodes, left_child_idx, nodes_used, objects, indices);
            subdivide(nodes, right_child_idx, nodes_used, objects, indices);
        }

        let root_node_idx: usize = 0;
        let mut nodes_used: u32 = 1;
        let root_node = &mut nodes[root_node_idx as usize];
        root_node.left_node_idx = 0;
        root_node.first_hittable_idx = 0;
        root_node.hittable_count = objects.len() as u32;
        let mut bbox = AABB::EMPTY;
        for obj in &objects {
            bbox = AABB::from_aabb(&bbox, &obj.get_bbox())
        }

        root_node.bbox = bbox;

        subdivide(
            &mut nodes,
            root_node_idx,
            &mut nodes_used,
            &objects,
            &mut indices,
        );
        Self {
            nodes: nodes,
            objects: objects,
            indices: indices,
        }
    }

    pub fn hit(&self, ray: &mut Ray, ray_t: Interval) {
        let mut node = &self.nodes[0];
        let mut stack_idx = 0 as usize;
        let mut stack = [node; 64];
        let mut closest_hit = ray_t.max;
        loop {
            if node.is_leaf() {
                (node.first_hittable_idx..node.first_hittable_idx + node.hittable_count).for_each(
                    |i| {
                        let obj_idx = self.indices[i as usize];
                        self.objects[obj_idx as usize]
                            .hit(ray, Interval::new(ray_t.min, closest_hit));

                        if ray.hit.t < closest_hit {
                            closest_hit = ray.hit.t;
                        }
                    },
                );
                if stack_idx == 0 {
                    break;
                } else {
                    stack_idx -= 1;
                    node = stack[stack_idx];
                    continue;
                }
            }
            let mut child_1 = &self.nodes[node.left_node_idx as usize];
            let mut child_2 = &self.nodes[node.left_node_idx as usize + 1];

            let mut dist_1 = child_1.bbox.intersects_sse(ray, ray_t);
            let mut dist_2 = child_2.bbox.intersects_sse(ray, ray_t);

            if dist_1 > dist_2 {
                swap(&mut dist_1, &mut dist_2);
                swap(&mut child_1, &mut child_2);
            }
            if dist_1 == INFINITY {
                if stack_idx == 0 {
                    break;
                } else {
                    stack_idx -= 1;
                    node = stack[stack_idx];
                }
            } else {
                node = child_1;
                if dist_2 != INFINITY {
                    stack[stack_idx] = child_2;
                    stack_idx += 1;
                }
            }
        }
    }
}

fn box_compare(a: &Hittable, b: &Hittable, axis: Axis) -> Ordering {
    let binding = a.get_bbox();
    let a_axis_interval = binding.get_interval(axis);
    let binding = b.get_bbox();
    let b_axis_interval = binding.get_interval(axis);

    match a_axis_interval.min.total_cmp(&b_axis_interval.min) {
        Ordering::Less => Ordering::Less,
        Ordering::Equal => Ordering::Equal,
        Ordering::Greater => Ordering::Greater,
    }
}
