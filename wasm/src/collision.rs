use crate::types::*;
use anyhow::Result;
use ordered_float::OrderedFloat;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

/// Compute the Axis-Aligned Bounding Box (AABB) for a collection of nodes.
/// Returns (x_min, x_max, y_min, y_max) or None if the collection is empty.
fn aabb(nodes: &[Node]) -> Option<(GraphLength, GraphLength, GraphLength, GraphLength)> {
    if nodes.is_empty() {
        return None;
    }

    // More efficient min/max calculation starting with first element
    let mut nodes_iter = nodes.iter();
    let first_node = nodes_iter.next().unwrap(); // Safe because we checked is_empty

    Some(nodes_iter.fold(
        (first_node.0, first_node.0, first_node.1, first_node.1),
        |(x_min, x_max, y_min, y_max), node| {
            (
                if node.0 < x_min { node.0 } else { x_min },
                if node.0 > x_max { node.0 } else { x_max },
                if node.1 < y_min { node.1 } else { y_min },
                if node.1 > y_max { node.1 } else { y_max },
            )
        },
    ))
}

/// A collision manager based on the SAT theorem.
#[derive(Serialize)]
pub struct CollisionManager {
    #[serde(skip)]
    indices: HashMap<(u32, u32), Vec<usize>>,
    collisions: Vec<Vec<Node>>,
    #[serde(skip)]
    unit_size: GraphLength,
    x_min: GraphLength,
    x_max: GraphLength,
    y_min: GraphLength,
    y_max: GraphLength,
}

impl CollisionManager {
    pub fn new(unit_size: GraphLength) -> Self {
        CollisionManager {
            indices: HashMap::new(),
            collisions: Vec::new(),
            unit_size,
            x_min: 0.0.into(),
            x_max: 0.0.into(),
            y_min: 0.0.into(),
            y_max: 0.0.into(),
        }
    }
    #[inline]
    pub fn update_x_min(&mut self, x_min: GraphLength) {
        if x_min < self.x_min {
            self.x_min = x_min;
        }
    }
    #[inline]
    pub fn update_x_max(&mut self, x_max: GraphLength) {
        if x_max > self.x_max {
            self.x_max = x_max;
        }
    }
    #[inline]
    pub fn update_y_min(&mut self, y_min: GraphLength) {
        if y_min < self.y_min {
            self.y_min = y_min;
        }
    }
    #[inline]
    pub fn update_y_max(&mut self, y_max: GraphLength) {
        if y_max > self.y_max {
            self.y_max = y_max;
        }
    }
    pub fn add_collision(&mut self, collision: Vec<Node>) -> Result<&[Node]> {
        let Some((x_min, x_max, y_min, y_max)) = aabb(&collision) else {
            return Err(anyhow::anyhow!("Collision must have at least 3 vertices"));
        };

        self.update_x_min(x_min);
        self.update_x_max(x_max);
        self.update_y_min(y_min);
        self.update_y_max(y_max);

        // Get index before pushing to avoid len() call after modification
        let idx = self.collisions.len();
        self.collisions.push(collision);

        // Convert to f64 once to avoid repeated conversions
        let unit_size_f64: f64 = self.unit_size.into();
        let x_min_f64: f64 = x_min.into();
        let x_max_f64: f64 = x_max.into();
        let y_min_f64: f64 = y_min.into();
        let y_max_f64: f64 = y_max.into();

        let x_idx_min = (x_min_f64 / unit_size_f64).floor() as u32;
        let x_idx_max = (x_max_f64 / unit_size_f64).ceil() as u32;
        let y_idx_min = (y_min_f64 / unit_size_f64).floor() as u32;
        let y_idx_max = (y_max_f64 / unit_size_f64).ceil() as u32;

        // Use entry API to reduce HashMap lookups
        for i in x_idx_min..=x_idx_max {
            for j in y_idx_min..=y_idx_max {
                self.indices.entry((i, j)).or_default().push(idx);
            }
        }

        Ok(self.collisions.last().unwrap().as_slice())
    }
    /// reads a collision, compares it with the existing collisions, and provide a length that will remove the collision
    /// Ok(None) -> No collisions
    /// Err(any) -> input is not valid, or the iteration took too long
    /// Ok(Some(f64)) -> The length to move given the angle to remove the collision
    fn check_collision(&self, collision: &[Node], angle: f64) -> Result<Option<GraphLength>> {
        // Validate input
        if collision.len() < 3 {
            return Err(anyhow::anyhow!("Polygon must have at least 3 vertices"));
        }

        let Some((x_min, x_max, y_min, y_max)) = aabb(collision) else {
            return Ok(None); // Empty polygon can't collide
        };

        // Get candidate collisions from spatial index
        let unit_size_f64: f64 = self.unit_size.into();
        let x_idx_min = (f64::from(x_min) / unit_size_f64).floor() as u32;
        let x_idx_max = (f64::from(x_max) / unit_size_f64).ceil() as u32;
        let y_idx_min = (f64::from(y_min) / unit_size_f64).floor() as u32;
        let y_idx_max = (f64::from(y_max) / unit_size_f64).ceil() as u32;

        let mut candidates: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for i in x_idx_min..=x_idx_max {
            for j in y_idx_min..=y_idx_max {
                if let Some(indices) = self.indices.get(&(i, j)) {
                    candidates.extend(indices);
                }
            }
        }

        // If no candidates, no collision
        if candidates.is_empty() {
            return Ok(None);
        }

        // Movement direction vector
        let movement_x = angle.cos();
        let movement_y = angle.sin();

        let mut max_required_distance: f64 = 0.0;

        // Check SAT collision with each candidate
        for &idx in &candidates {
            if let Some(existing_polygon) = self.collisions.get(idx) {
                if let Some((mtv_x, mtv_y, overlap)) =
                    sat_collision_test_with_mtv(collision, existing_polygon)
                {
                    // We found a collision, calculate required movement distance

                    // Calculate the dot product of MTV with movement direction
                    let mtv_dot_movement = mtv_x * movement_x + mtv_y * movement_y;

                    if mtv_dot_movement.abs() < f64::EPSILON {
                        // Movement direction is perpendicular to MTV
                        // This can happen when:
                        // 1. Label tries to move vertically but collision requires horizontal movement
                        // 2. Two rectangles are side-by-side but we're trying to move up/down
                        // 3. Polygon shapes create complex collision normals

                        // Instead of erroring, try alternative strategies:
                        // Option 1: Move along MTV direction instead
                        let required_distance = overlap;
                        max_required_distance = max_required_distance.max(required_distance);

                        // Option 2: Could also try a slight angle adjustment
                        // But for now, we'll allow MTV-based movement
                        continue;
                    }

                    // Calculate required distance to resolve collision (always positive)
                    let required_distance = overlap / mtv_dot_movement.abs();

                    max_required_distance = max_required_distance.max(required_distance);
                }
            }
        }

        if max_required_distance > 0.0 {
            Ok(Some(GraphLength::from(max_required_distance)))
        } else {
            Ok(None)
        }
    }

    /// Recursively resolve collisions by moving along the specified angle until no more collisions occur.
    /// Returns the total distance needed to move to resolve all collisions.
    ///
    /// # Arguments
    /// * `collision` - The polygon to test and move
    /// * `angle` - The angle in radians to move along
    /// * `max_iterations` - Maximum number of recursive iterations to prevent infinite loops
    ///
    /// # Returns
    /// * `Ok(GraphLength)` - Total distance needed to resolve all collisions
    /// * `Err(_)` - Input invalid, too many iterations, or unresolvable collision
    pub fn resolve_collisions_recursive(
        &mut self,
        collision: Vec<Node>,
        angle: f64,
        max_iterations: u32,
    ) -> Result<(&[Node], GraphLength)> {
        self.resolve_collisions_recursive_impl(
            collision,
            angle,
            max_iterations,
            0,
            GraphLength::from(0.0),
        )
    }

    /// Convenience function to resolve collisions with a default maximum of 100 iterations.
    /// This is the recommended function to use in most cases.
    pub fn resolve_collisions(
        &mut self,
        collision: Vec<Node>,
        angle: f64,
    ) -> Result<(&[Node], GraphLength)> {
        self.resolve_collisions_recursive(collision, angle, 255)
    }

    /// Internal recursive implementation
    fn resolve_collisions_recursive_impl(
        &mut self,
        collision: Vec<Node>,
        angle: f64,
        max_iterations: u32,
        current_iteration: u32,
        accumulated_distance: GraphLength,
    ) -> Result<(&[Node], GraphLength)> {
        // Check iteration limit
        if current_iteration >= max_iterations {
            return Err(anyhow::anyhow!(
                "Maximum iterations ({}) reached while resolving collisions",
                max_iterations
            ));
        }

        // Check for collision at current position
        match self.check_collision(&collision, angle)? {
            None => {
                // No collision, we're done
                Ok((self.add_collision(collision).unwrap(), accumulated_distance))
            }
            Some(required_distance) => {
                if required_distance.value() <= 0.5 {
                    // No movement needed, but still a collision
                    return Ok((self.add_collision(collision).unwrap(), accumulated_distance));
                }
                // Found collision, move and check again
                let new_total_distance = accumulated_distance + required_distance;

                // Calculate new polygon position after movement
                let movement_x = angle.cos() * f64::from(required_distance);
                let movement_y = angle.sin() * f64::from(required_distance);

                let moved_collision: Vec<Node> = collision
                    .iter()
                    .map(|node| {
                        Node(
                            node.0 + GraphLength::from(movement_x),
                            node.1 + GraphLength::from(movement_y),
                        )
                    })
                    .collect();

                // Recursively check the moved polygon
                self.resolve_collisions_recursive_impl(
                    moved_collision,
                    angle,
                    max_iterations,
                    current_iteration + 1,
                    new_total_distance,
                )
            }
        }
    }
}

/// Get the normal vector for an edge defined by two points.
/// Returns a normalized perpendicular vector (outward normal).
fn get_edge_normal(p1: &Node, p2: &Node) -> (f64, f64) {
    let dx = f64::from(p2.0) - f64::from(p1.0);
    let dy = f64::from(p2.1) - f64::from(p1.1);

    // Perpendicular vector (normal)
    let nx = -dy;
    let ny = dx;

    // Normalize
    let length = (nx * nx + ny * ny).sqrt();
    if length > f64::EPSILON {
        (nx / length, ny / length)
    } else {
        (0.0, 0.0) // Degenerate edge
    }
}

/// Project a polygon onto an axis and return the min and max projection values.
fn project_polygon(polygon: &[Node], axis_x: f64, axis_y: f64) -> (f64, f64) {
    if polygon.is_empty() {
        return (0.0, 0.0);
    }

    let first_projection = f64::from(polygon[0].0) * axis_x + f64::from(polygon[0].1) * axis_y;
    let mut min_proj = first_projection;
    let mut max_proj = first_projection;

    for node in polygon.iter().skip(1) {
        let projection = f64::from(node.0) * axis_x + f64::from(node.1) * axis_y;
        min_proj = min_proj.min(projection);
        max_proj = max_proj.max(projection);
    }

    (min_proj, max_proj)
}

/// Calculate the minimum translation vector (MTV) between two projections.
/// Returns the overlap distance if they overlap, 0.0 if they don't.
fn calculate_overlap(min1: f64, max1: f64, min2: f64, max2: f64) -> f64 {
    if max1 < min2 || max2 < min1 {
        0.0 // No overlap
    } else {
        // Calculate overlap amount
        let overlap_direction1 = max1 - min2;
        let overlap_direction2 = max2 - min1;
        overlap_direction1.min(overlap_direction2)
    }
}

/// SAT collision test that returns the minimum translation vector (MTV).
/// Returns None if no collision, Some((axis_x, axis_y, overlap)) if collision detected.
fn sat_collision_test_with_mtv(polygon1: &[Node], polygon2: &[Node]) -> Option<(f64, f64, f64)> {
    if polygon1.len() < 3 || polygon2.len() < 3 {
        return None; // Not valid polygons
    }

    let mut min_overlap = f64::INFINITY;
    let mut mtv_axis = (0.0, 0.0);

    // Test all normals from polygon1
    for i in 0..polygon1.len() {
        let p1 = &polygon1[i];
        let p2 = &polygon1[(i + 1) % polygon1.len()];

        let (axis_x, axis_y) = get_edge_normal(p1, p2);
        if axis_x == 0.0 && axis_y == 0.0 {
            continue; // Skip degenerate edges
        }

        let (min1, max1) = project_polygon(polygon1, axis_x, axis_y);
        let (min2, max2) = project_polygon(polygon2, axis_x, axis_y);

        let overlap = calculate_overlap(min1, max1, min2, max2);
        if overlap == 0.0 {
            return None; // Separating axis found
        }

        if overlap < min_overlap {
            min_overlap = overlap;
            mtv_axis = (axis_x, axis_y);
        }
    }

    // Test all normals from polygon2
    for i in 0..polygon2.len() {
        let p1 = &polygon2[i];
        let p2 = &polygon2[(i + 1) % polygon2.len()];

        let (axis_x, axis_y) = get_edge_normal(p1, p2);
        if axis_x == 0.0 && axis_y == 0.0 {
            continue; // Skip degenerate edges
        }

        let (min1, max1) = project_polygon(polygon1, axis_x, axis_y);
        let (min2, max2) = project_polygon(polygon2, axis_x, axis_y);

        let overlap = calculate_overlap(min1, max1, min2, max2);
        if overlap == 0.0 {
            return None; // Separating axis found
        }

        if overlap < min_overlap {
            min_overlap = overlap;
            mtv_axis = (axis_x, axis_y);
        }
    }

    Some((mtv_axis.0, mtv_axis.1, min_overlap))
}

/// Rotate a polygon around a center point by the specified angle.
///
/// # Arguments
/// * `polygon` - The polygon vertices to rotate (consumed)
/// * `center_point` - The center point to rotate around
/// * `angle` - The rotation angle in radians (positive = counterclockwise)
///
/// # Returns
/// A new vector containing the rotated vertices
pub fn rotate_polygon(polygon: Vec<Node>, center_point: Node, angle: f64) -> Vec<Node> {
    let cos_angle = angle.cos();
    let sin_angle = angle.sin();

    let center_x = f64::from(center_point.0);
    let center_y = f64::from(center_point.1);

    polygon
        .into_iter()
        .map(|node| {
            // Translate to origin
            let x = f64::from(node.0) - center_x;
            let y = f64::from(node.1) - center_y;

            // Apply rotation matrix
            let rotated_x = x * cos_angle - y * sin_angle;
            let rotated_y = x * sin_angle + y * cos_angle;

            // Translate back
            Node(
                GraphLength::from(rotated_x + center_x),
                GraphLength::from(rotated_y + center_y),
            )
        })
        .collect()
}

pub struct LineCollisionManager {
    // Store actual line segments (start, end) instead of individual points
    collisions: Vec<BTreeMap<OrderedFloat<f64>, OrderedFloat<f64>>>,
}

impl LineCollisionManager {
    pub fn new() -> Self {
        Self {
            collisions: Vec::new(),
        }
    }

    #[inline]
    pub fn add_collision(&mut self, start: GraphLength, end: GraphLength, idx: usize) {
        let start_f64 = OrderedFloat(f64::from(start));
        let end_f64 = OrderedFloat(f64::from(end));

        // Ensure start <= end for consistent storage
        let (segment_start, segment_end) = if start_f64 <= end_f64 {
            (start_f64, end_f64)
        } else {
            (end_f64, start_f64)
        };

        // Ensure the collisions[idx] BTreeMap exists
        if self.collisions.len() <= idx {
            self.collisions.resize_with(idx + 1, BTreeMap::new);
        }

        // Insert the interval into the map
        self.collisions[idx].insert(segment_start, segment_end);
    }

    #[inline]
    pub fn check_collision(&self, start: GraphLength, end: GraphLength, idx: usize) -> bool {
        let start_f64 = OrderedFloat(f64::from(start));
        let end_f64 = OrderedFloat(f64::from(end));

        // Ensure start <= end for consistent comparison
        let (test_start, test_end) = if start_f64 <= end_f64 {
            (start_f64, end_f64)
        } else {
            (end_f64, start_f64)
        };
        if let Some(c) = self.collisions.get(idx) {
            // Check if the segment overlaps with any existing segments
            for (&existing_start, &existing_end) in c.range(..=test_end) {
                // Check if the segments overlap
                if existing_start <= test_end && existing_end >= test_start {
                    return true; // Collision detected
                }
            }
            false // No collision found
        } else {
            false
        }
    }

    pub fn resolve_collisions(&mut self, start: GraphLength, end: GraphLength) -> Result<i16> {
        self.resolve_collisions_impl(start, end, 255)
    }
    pub fn resolve_collisions_impl(
        &mut self,
        start: GraphLength,
        end: GraphLength,
        max_depth: i16,
    ) -> Result<i16> {
        // Remove the epsilon check since we're now storing actual line segments
        // Empty or point segments are valid in some cases
        for idx in 0..=max_depth {
            let level = if idx == 0 {
                0
            } else if idx % 2 == 1 {
                (idx + 1) / 2
            } else {
                -(idx / 2)
            };
            if self.check_collision(start, end, idx as usize) {
                continue;
            }
            self.add_collision(start, end, idx as usize);
            return Ok(level);
        }
        Err(
            anyhow::anyhow! {"Reached maximum recursion depth ({max_depth}) when processing station line collisions"},
        )
    }
}
