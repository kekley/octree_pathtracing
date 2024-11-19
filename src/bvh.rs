use std::{cmp::Ordering, f64::INFINITY, marker::PhantomData, mem::swap};

use fastrand::Rng;

use crate::{
    aabb::AABB,
    hittable::{HitList, HitRecord, Hittable},
    interval::Interval,
    ray::Ray,
};
#[derive(Debug, Clone)]
pub struct BVHTree<'a> {
    nodes: Vec<BVHNode>,
    objects: Vec<Hittable<'a>>,
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

impl<'a> BVHTree<'a> {
    pub fn bbox(&self) -> &AABB {
        &self.nodes[0].bbox
    }
    pub fn from_hit_list(list: &HitList<'a>) -> Self {
        Self::from_hittable_vec(list.objects.clone())
    }

    pub fn from_hittable_vec(objects: Vec<Hittable<'a>>) -> Self {
        let mut indices: Vec<u32> = (0..objects.len()).map(|i| i as u32).collect();
        let mut nodes: Vec<BVHNode> = vec![Default::default(); objects.len() * 2 - 1];
        fn subdivide(
            nodes: &mut Vec<BVHNode>,
            node_idx: usize,
            nodes_used: &mut u32,
            objects: &Vec<Hittable>,
            indices: &mut Vec<u32>,
        ) {
            let axis = nodes[node_idx].bbox.longest_axis();

            if nodes[node_idx].hittable_count <= 2 {
                return;
            }

            let mut i = nodes[node_idx].first_hittable_idx as usize;
            let mut j = (i as u32 + nodes[node_idx].hittable_count - 1) as usize;

            while i <= j {
                match box_compare(
                    &objects[indices[i] as usize],
                    &objects[indices[j] as usize],
                    axis,
                ) {
                    Ordering::Less => {
                        i += 1;
                    }
                    _ => {
                        indices.swap(i, j);
                        j -= 1;
                    }
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
                nodes[left_child_idx].bbox = AABB::from_boxes(
                    &nodes[left_child_idx].bbox,
                    objects[obj_index as usize].get_bbox(),
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
                nodes[right_child_idx].bbox = AABB::from_boxes(
                    &nodes[right_child_idx].bbox,
                    objects[obj_index as usize].get_bbox(),
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
            bbox = AABB::from_boxes(&bbox, obj.get_bbox())
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

    pub fn stack_hit(&self, ray: &Ray, ray_t: Interval) -> Option<HitRecord> {
        let mut node = &self.nodes[0];
        let mut stack_idx = 0 as usize;
        let mut stack = [node; 30];
        let mut closest_hit = ray_t.max;
        let mut return_val = None;
        loop {
            if node.is_leaf() {
                (node.first_hittable_idx..node.first_hittable_idx + node.hittable_count).for_each(
                    |i| {
                        let obj_idx = self.indices[i as usize];
                        let rec = self.objects[obj_idx as usize]
                            .hit(ray, Interval::new(ray_t.min, closest_hit));
                        match rec {
                            Some(rec) => {
                                closest_hit = rec.t;
                                return_val = Some(rec);
                            }
                            None => {}
                        }
                    },
                );
                if stack_idx == 0 {
                    break return_val;
                } else {
                    stack_idx -= 1;
                    node = stack[stack_idx];
                    continue;
                }
            }
            let mut child_1 = &self.nodes[node.left_node_idx as usize];
            let mut child_2 = &self.nodes[node.left_node_idx as usize + 1];

            let mut dist_1 = child_1.bbox.intersects(ray, ray_t);
            let mut dist_2 = child_2.bbox.intersects(ray, ray_t);

            if dist_1 > dist_2 {
                swap(&mut dist_1, &mut dist_2);
                swap(&mut child_1, &mut child_2);
            }
            if dist_1 == INFINITY {
                if stack_idx == 0 {
                    break return_val;
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

    /*     pub fn hit(&self, ray: &Ray, ray_t: Interval, node_idx: usize) -> HitRecord {
        todo!();
        let mut ret_val = HitRecord::MISS;
        let node = &self.nodes[node_idx];
        let mut closest_hit = ray_t.max;
        let dist = node.bbox.intersects(ray, ray_t);

        if dist < 0.0 {
            return ret_val;
        }

        if node.is_leaf() {
            for i in node.first_hittable_idx..node.first_hittable_idx + node.hittable_count {
                let obj_index = self.indices[i as usize];
                let object = &self.objects[obj_index as usize];
                let rec = object.hit(ray, Interval::new(ray_t.min, closest_hit));
                match rec.t >= 0.0 {
                    true => {
                        closest_hit = rec.t;
                        ret_val = rec;
                    }
                    false => {}
                }
            }
        } else {
            let left_hit = self.hit(ray, ray_t, node.left_node_idx as usize);
            let right_hit = self.hit(ray, ray_t, node.left_node_idx as usize + 1);

            return match (left_hit, right_hit) {
                (Some(l_hit), Some(r_hit)) => {
                    if l_hit.t <= r_hit.t {
                        Some(l_hit)
                    } else {
                        Some(r_hit)
                    }
                }
                (Some(l_hit), None) => Some(l_hit),
                (None, Some(r_hit)) => Some(r_hit),
                (None, None) => None,
            };
        }

        ret_val
    } */
}

fn box_compare(a: &Hittable, b: &Hittable, axis: u8) -> Ordering {
    let a_axis_interval = a.get_bbox().get_interval(axis);
    let b_axis_interval = b.get_bbox().get_interval(axis);

    match a_axis_interval.min.total_cmp(&b_axis_interval.min) {
        Ordering::Less => Ordering::Less,
        Ordering::Equal => Ordering::Equal,
        Ordering::Greater => Ordering::Greater,
    }
}

fn box_x_compare(objects: &Vec<Hittable>, a_idx: u32, b_idx: u32) -> Ordering {
    box_compare(&objects[a_idx as usize], &objects[b_idx as usize], 0)
}

fn box_y_compare(objects: &Vec<Hittable>, a_idx: u32, b_idx: u32) -> Ordering {
    box_compare(&objects[a_idx as usize], &objects[b_idx as usize], 1)
}

fn box_z_compare(objects: &Vec<Hittable>, a_idx: u32, b_idx: u32) -> Ordering {
    box_compare(&objects[a_idx as usize], &objects[b_idx as usize], 2)
}
