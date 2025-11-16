use super::MapConfiguration;
use macroquad::prelude::*;

#[derive(Clone, Debug)]
pub struct InitialState {
    pub position: Vec2,
    pub velocity: Vec2,
}

/// Calculate initial position/velocity for all bodies in a map
pub fn calculate_initial_states(
    map: &MapConfiguration,
    gravity_constant: f32,
) -> Vec<InitialState> {
    let mut states: Vec<InitialState> = Vec::new();

    // First pass: calculate positions for all bodies
    for (i, body) in map.celestial_bodies.iter().enumerate() {
        let position = if let Some(parent_index) = body.orbital_parent_index {
            // Orbiting body - position relative to parent
            let parent_pos = if parent_index < states.len() {
                states[parent_index].position
            } else {
                Vec2::ZERO // Parent not yet calculated, use origin
            };

            if let Some(distance) = body.orbital_distance {
                polar_to_cartesian(parent_pos, distance, body.initial_angle)
            } else {
                parent_pos // No distance specified, spawn at parent
            }
        } else {
            // Stationary/pinned body at origin
            Vec2::ZERO
        };

        // Velocity will be calculated in second pass
        states.push(InitialState {
            position,
            velocity: Vec2::ZERO,
        });
    }

    // Second pass: calculate velocities
    for (i, body) in map.celestial_bodies.iter().enumerate() {
        if body.is_pinned {
            // Pinned bodies don't move
            states[i].velocity = Vec2::ZERO;
        } else if let Some(parent_index) = body.orbital_parent_index {
            // Calculate orbital velocity around parent
            if let Some(distance) = body.orbital_distance {
                let parent_pos = states[parent_index].position;
                let parent_mass = map.celestial_bodies[parent_index].mass;
                let body_pos = states[i].position;

                // Calculate orbital velocity perpendicular to radius
                let orbital_velocity = calculate_orbital_velocity(
                    parent_pos,
                    body_pos,
                    parent_mass,
                    gravity_constant,
                );

                // Add parent's velocity for hierarchical orbits (e.g., Moon inherits Earth's velocity)
                let parent_velocity = states[parent_index].velocity;
                states[i].velocity = parent_velocity + orbital_velocity;
            }
        }
    }

    states
}

/// Convert polar coordinates (distance, angle) to Cartesian (x, y)
fn polar_to_cartesian(parent_pos: Vec2, distance: f32, angle: f32) -> Vec2 {
    Vec2::new(
        parent_pos.x + distance * angle.cos(),
        parent_pos.y + distance * angle.sin(),
    )
}

/// Calculate orbital velocity perpendicular to radius vector
fn calculate_orbital_velocity(
    parent_pos: Vec2,
    body_pos: Vec2,
    parent_mass: f32,
    gravity_constant: f32,
) -> Vec2 {
    let radius_vector = body_pos - parent_pos;
    let distance = radius_vector.length();

    if distance < 0.001 {
        return Vec2::ZERO; // Avoid division by zero
    }

    // v = sqrt(G * M / r)
    let speed = (gravity_constant * parent_mass / distance).sqrt();

    // Perpendicular to radius (rotate 90 degrees counterclockwise)
    let direction = Vec2::new(-radius_vector.y, radius_vector.x).normalize();

    direction * speed
}
