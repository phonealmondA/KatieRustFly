// Rocket - Player-controlled spacecraft with fuel and thrust
// Ported from C++ Rocket class

use macroquad::prelude::*;

use super::game_object::{GameObject, GameObjectData};
use super::rocket_part::RocketPart;
use super::engine::Engine;
use crate::game_constants::GameConstants;
use crate::utils::vector_helper;

/// Rocket with dynamic mass, fuel system, and thrust control
pub struct Rocket {
    data: GameObjectData,
    rotation: f32,
    angular_velocity: f32,
    thrust_level: f32, // 0.0 to 1.0

    // Dynamic mass system
    mass: f32,
    base_mass: f32,
    max_mass: f32,

    // Fuel system
    current_fuel: f32,
    max_fuel: f32,
    is_currently_thrusting: bool,

    // Manual fuel transfer
    is_transferring_fuel_in: bool,
    is_transferring_fuel_out: bool,
    fuel_transfer_rate: f32,

    // Landing state
    landed: bool,
    landed_on_planet_id: Option<usize>,

    // Rocket parts (engines, etc.)
    parts: Vec<Box<dyn RocketPart>>,

    // Visualization settings
    pub show_velocity_vector: bool,
    pub show_trajectory: bool,
    pub velocity_vector_scale: f32,
    pub trajectory_steps: usize,
    pub trajectory_time_step: f32,
}

impl Rocket {
    pub fn new(position: Vec2, velocity: Vec2, color: Color, base_mass: f32) -> Self {
        let max_fuel = GameConstants::ROCKET_MAX_FUEL;
        let starting_fuel = GameConstants::ROCKET_STARTING_FUEL;
        let mass = base_mass + starting_fuel;

        // Create default rocket parts (single engine)
        let mut parts: Vec<Box<dyn RocketPart>> = Vec::new();
        parts.push(Box::new(Engine::new(
            Vec2::new(0.0, GameConstants::ROCKET_SIZE * 0.8),
            GameConstants::ENGINE_THRUST_POWER,
            Color::new(0.8, 0.2, 0.2, 1.0), // Red engine
        )));

        Rocket {
            data: GameObjectData::new(position, velocity, color),
            rotation: 0.0,
            angular_velocity: 0.0,
            thrust_level: 0.0,
            mass,
            base_mass,
            max_mass: GameConstants::ROCKET_MAX_MASS,
            current_fuel: starting_fuel,
            max_fuel,
            is_currently_thrusting: false,
            is_transferring_fuel_in: false,
            is_transferring_fuel_out: false,
            fuel_transfer_rate: 0.0,
            landed: false,
            landed_on_planet_id: None,
            parts,
            show_velocity_vector: false,
            show_trajectory: false,
            velocity_vector_scale: 5.0,
            trajectory_steps: 50,
            trajectory_time_step: 0.1,
        }
    }

    // === Mass System ===
    pub fn mass(&self) -> f32 {
        self.mass
    }

    pub fn base_mass(&self) -> f32 {
        self.base_mass
    }

    pub fn max_mass(&self) -> f32 {
        self.max_mass
    }

    pub fn mass_capacity_remaining(&self) -> f32 {
        self.max_mass - self.mass
    }

    fn update_mass_from_fuel(&mut self) {
        let old_mass = self.mass;
        self.mass = self.base_mass + self.current_fuel;

        // Preserve momentum when mass changes
        if old_mass > 0.0 && self.mass > 0.0 {
            self.preserve_momentum_during_mass_change(old_mass, self.mass);
        }
    }

    fn preserve_momentum_during_mass_change(&mut self, old_mass: f32, new_mass: f32) {
        // p = m * v  =>  v_new = (m_old * v_old) / m_new
        if new_mass > 0.0 {
            self.data.velocity = self.data.velocity * (old_mass / new_mass);
        }
    }

    // === Fuel System ===
    pub fn current_fuel(&self) -> f32 {
        self.current_fuel
    }

    pub fn max_fuel(&self) -> f32 {
        self.max_fuel
    }

    pub fn fuel_percentage(&self) -> f32 {
        if self.max_fuel > 0.0 {
            (self.current_fuel / self.max_fuel) * 100.0
        } else {
            0.0
        }
    }

    pub fn can_thrust(&self) -> bool {
        self.current_fuel > 0.0
    }

    pub fn set_fuel(&mut self, fuel: f32) {
        self.current_fuel = fuel.clamp(0.0, self.max_fuel);
        self.update_mass_from_fuel();
    }

    pub fn add_fuel(&mut self, fuel: f32) {
        self.set_fuel(self.current_fuel + fuel);
    }

    fn consume_fuel(&mut self, delta_time: f32) {
        if self.thrust_level < GameConstants::FUEL_CONSUMPTION_MIN_THRESHOLD {
            return;
        }

        let consumption = self.calculate_fuel_consumption() * delta_time;
        self.current_fuel = (self.current_fuel - consumption).max(0.0);
        self.update_mass_from_fuel();
    }

    fn calculate_fuel_consumption(&self) -> f32 {
        if self.thrust_level < GameConstants::FUEL_CONSUMPTION_MIN_THRESHOLD {
            return 0.0;
        }

        GameConstants::FUEL_CONSUMPTION_BASE +
            (self.thrust_level * GameConstants::FUEL_CONSUMPTION_MULTIPLIER)
    }

    // === Thrust and Control ===
    pub fn thrust_level(&self) -> f32 {
        self.thrust_level
    }

    pub fn set_thrust_level(&mut self, level: f32) {
        self.thrust_level = level.clamp(0.0, 1.0);
    }

    pub fn apply_thrust(&mut self, amount: f32) {
        if !self.can_thrust() {
            self.is_currently_thrusting = false;
            return;
        }

        self.is_currently_thrusting = true;

        // Calculate thrust direction (in the direction the rocket nose points)
        // Rocket nose points in direction (sin(rotation), -cos(rotation))
        // Thrust points in same direction: (sin(rotation), -cos(rotation))
        let thrust_direction = Vec2::new(
            self.rotation.sin(),
            -self.rotation.cos(),
        );

        // Apply thrust force
        let thrust_force = thrust_direction * amount * GameConstants::ENGINE_THRUST_POWER;
        let acceleration = thrust_force / self.mass;

        self.data.velocity += acceleration;
    }

    pub fn rotate(&mut self, amount: f32) {
        self.rotation += amount;
        // Normalize angle to [0, 2*PI]
        self.rotation = self.rotation.rem_euclid(2.0 * std::f32::consts::PI);
    }

    pub fn rotation(&self) -> f32 {
        self.rotation
    }

    // === Fuel Transfer ===
    pub fn start_fuel_transfer_in(&mut self, transfer_rate: f32) {
        self.is_transferring_fuel_in = true;
        self.is_transferring_fuel_out = false;
        self.fuel_transfer_rate = transfer_rate;
    }

    pub fn start_fuel_transfer_out(&mut self, transfer_rate: f32) {
        self.is_transferring_fuel_out = true;
        self.is_transferring_fuel_in = false;
        self.fuel_transfer_rate = transfer_rate;
    }

    pub fn stop_fuel_transfer(&mut self) {
        self.is_transferring_fuel_in = false;
        self.is_transferring_fuel_out = false;
        self.fuel_transfer_rate = 0.0;
    }

    pub fn is_transferring_fuel(&self) -> bool {
        self.is_transferring_fuel_in || self.is_transferring_fuel_out
    }

    pub fn current_transfer_rate(&self) -> f32 {
        self.fuel_transfer_rate
    }

    // === Position and Velocity Accessors ===

    pub fn position(&self) -> Vec2 {
        self.data.position
    }

    pub fn velocity(&self) -> Vec2 {
        self.data.velocity
    }

    pub fn set_velocity(&mut self, velocity: Vec2) {
        self.data.velocity = velocity;
    }

    pub fn set_position(&mut self, position: Vec2) {
        self.data.position = position;
    }

    /// Current mass including fuel
    pub fn current_mass(&self) -> f32 {
        self.mass
    }

    // === Rocket Parts Management ===

    pub fn add_part(&mut self, part: Box<dyn RocketPart>) {
        self.parts.push(part);
    }

    pub fn parts(&self) -> &[Box<dyn RocketPart>] {
        &self.parts
    }

    // === Visualization ===

    /// Draw velocity vector
    fn draw_velocity_vector(&self) {
        if !self.show_velocity_vector {
            return;
        }

        let velocity_magnitude = vector_helper::magnitude(self.data.velocity);
        if velocity_magnitude < 0.1 {
            return; // Don't draw tiny velocities
        }

        let scaled_velocity = self.data.velocity * self.velocity_vector_scale;
        let end_pos = self.data.position + scaled_velocity;

        // Draw velocity line
        draw_line(
            self.data.position.x,
            self.data.position.y,
            end_pos.x,
            end_pos.y,
            2.0,
            Color::new(0.0, 1.0, 0.0, 0.8), // Green
        );

        // Draw arrowhead
        let arrow_size = 5.0;
        let velocity_normalized = vector_helper::normalize(self.data.velocity);
        let perpendicular = Vec2::new(-velocity_normalized.y, velocity_normalized.x);

        let arrow_base = end_pos - velocity_normalized * arrow_size * 2.0;
        let arrow_left = arrow_base + perpendicular * arrow_size;
        let arrow_right = arrow_base - perpendicular * arrow_size;

        draw_triangle(end_pos, arrow_left, arrow_right, Color::new(0.0, 1.0, 0.0, 0.8));
    }

    /// Draw trajectory prediction
    fn draw_trajectory(&self, planets: &[(&Vec2, f32, f32)]) {
        if !self.show_trajectory {
            return;
        }

        let mut predicted_pos = self.data.position;
        let mut predicted_vel = self.data.velocity;
        let dt = self.trajectory_time_step;

        let mut prev_pos = predicted_pos;

        for _ in 0..self.trajectory_steps {
            // Simple trajectory prediction with gravity
            let mut acceleration = Vec2::ZERO;

            // Calculate gravity from all planets
            for (planet_pos, planet_mass, _) in planets {
                let direction = **planet_pos - predicted_pos;
                let distance_sq = vector_helper::distance_squared(predicted_pos, **planet_pos);

                if distance_sq > 1.0 {
                    let force_magnitude = GameConstants::G * planet_mass / distance_sq;
                    acceleration += vector_helper::normalize(direction) * force_magnitude;
                }
            }

            // Update predicted velocity and position
            predicted_vel += acceleration * dt;
            predicted_pos += predicted_vel * dt;

            // Draw trajectory line segment
            draw_line(
                prev_pos.x,
                prev_pos.y,
                predicted_pos.x,
                predicted_pos.y,
                1.0,
                Color::new(1.0, 1.0, 0.0, 0.3), // Yellow, semi-transparent
            );

            prev_pos = predicted_pos;
        }

        // Draw small dots at regular intervals along trajectory
        predicted_pos = self.data.position;
        predicted_vel = self.data.velocity;

        for i in 0..self.trajectory_steps {
            if i % 10 == 0 {
                let mut acceleration = Vec2::ZERO;
                for (planet_pos, planet_mass, _) in planets {
                    let direction = **planet_pos - predicted_pos;
                    let distance_sq = vector_helper::distance_squared(predicted_pos, **planet_pos);

                    if distance_sq > 1.0 {
                        let force_magnitude = GameConstants::G * planet_mass / distance_sq;
                        acceleration += vector_helper::normalize(direction) * force_magnitude;
                    }
                }

                predicted_vel += acceleration * dt;
                predicted_pos += predicted_vel * dt;

                draw_circle(
                    predicted_pos.x,
                    predicted_pos.y,
                    2.0,
                    Color::new(1.0, 1.0, 0.0, 0.6),
                );
            } else {
                let mut acceleration = Vec2::ZERO;
                for (planet_pos, planet_mass, _) in planets {
                    let direction = **planet_pos - predicted_pos;
                    let distance_sq = vector_helper::distance_squared(predicted_pos, **planet_pos);

                    if distance_sq > 1.0 {
                        let force_magnitude = GameConstants::G * planet_mass / distance_sq;
                        acceleration += vector_helper::normalize(direction) * force_magnitude;
                    }
                }

                predicted_vel += acceleration * dt;
                predicted_pos += predicted_vel * dt;
            }
        }
    }

    /// Draw with optional planet data for trajectory prediction
    pub fn draw_with_planets(&self, planets: &[(&Vec2, f32, f32)]) {
        self.draw();
        self.draw_trajectory(planets);
    }

    // === Landing State ===

    /// Check if rocket is landed on a planet
    pub fn is_landed(&self) -> bool {
        self.landed
    }

    /// Get the planet ID this rocket is landed on
    pub fn landed_on_planet_id(&self) -> Option<usize> {
        self.landed_on_planet_id
    }

    /// Land the rocket on a planet
    pub fn land_on_planet(&mut self, planet_id: usize, surface_position: Vec2) {
        self.landed = true;
        self.landed_on_planet_id = Some(planet_id);
        self.data.position = surface_position;
        self.data.velocity = Vec2::ZERO;
        self.thrust_level = 0.0;
        self.is_currently_thrusting = false;
        log::info!("Rocket landed on planet {} at position ({:.1}, {:.1})",
            planet_id, surface_position.x, surface_position.y);
    }

    /// Take off from a planet
    pub fn take_off(&mut self) {
        if self.landed {
            self.landed = false;
            self.landed_on_planet_id = None;
            log::info!("Rocket taking off!");
        }
    }

    /// Draw thrust flame when rocket is firing engines
    fn draw_thrust_flame(&self) {
        let flame_length = GameConstants::ROCKET_SIZE * 1.5 * self.thrust_level;
        let flame_width = GameConstants::ROCKET_SIZE * 0.4;

        // Calculate flame position (at the back of rocket)
        let flame_offset = Vec2::new(0.0, GameConstants::ROCKET_SIZE);
        let rotated_offset = vector_helper::rotate(flame_offset, self.rotation);
        let flame_base = self.data.position + rotated_offset;

        // Calculate flame tip
        let flame_direction = Vec2::new(0.0, flame_length);
        let rotated_flame = vector_helper::rotate(flame_direction, self.rotation);
        let flame_tip = flame_base + rotated_flame;

        // Flame side points
        let perpendicular = vector_helper::rotate(Vec2::new(flame_width / 2.0, 0.0), self.rotation);
        let flame_left = flame_base + perpendicular;
        let flame_right = flame_base - perpendicular;

        // Draw flame (orange/yellow triangle)
        draw_triangle(
            flame_tip,
            flame_left,
            flame_right,
            Color::new(1.0, 0.6, 0.0, 0.8), // Orange flame
        );

        // Draw inner flame (brighter)
        let inner_flame_tip = flame_base + rotated_flame * 0.7;
        let inner_perpendicular = perpendicular * 0.6;
        let inner_left = flame_base + inner_perpendicular;
        let inner_right = flame_base - inner_perpendicular;

        draw_triangle(
            inner_flame_tip,
            inner_left,
            inner_right,
            Color::new(1.0, 1.0, 0.3, 0.9), // Bright yellow center
        );
    }
}

impl GameObject for Rocket {
    fn update(&mut self, delta_time: f32) {
        // Track if we just took off this frame to avoid double-applying thrust
        let mut just_took_off = false;

        // If landed, check for thrust to take off
        if self.landed {
            if self.thrust_level > 0.0 && self.can_thrust() {
                self.take_off();
                // Apply initial thrust for takeoff
                self.apply_thrust(self.thrust_level * delta_time);
                self.consume_fuel(delta_time);
                just_took_off = true;
                // Continue to update position after takeoff (don't return)
            } else {
                // Don't update position or physics while landed
                return;
            }
        }

        // Apply thrust if thrust level is set and didn't just take off
        if self.thrust_level > 0.0 && !just_took_off {
            self.apply_thrust(self.thrust_level * delta_time);
            self.consume_fuel(delta_time);
        } else if !just_took_off {
            self.is_currently_thrusting = false;
        }

        // Update position
        self.data.position += self.data.velocity * delta_time;
    }

    fn draw(&self) {
        // Rocket body is a triangle
        // Local coordinates (relative to rocket center)
        let local_points = [
            Vec2::new(0.0, -GameConstants::ROCKET_SIZE),
            Vec2::new(-GameConstants::ROCKET_SIZE / 2.0, GameConstants::ROCKET_SIZE),
            Vec2::new(GameConstants::ROCKET_SIZE / 2.0, GameConstants::ROCKET_SIZE),
        ];

        // Rotate and translate points to world space
        let cos_r = self.rotation.cos();
        let sin_r = self.rotation.sin();

        let world_points: Vec<Vec2> = local_points.iter().map(|p| {
            let rotated_x = p.x * cos_r - p.y * sin_r;
            let rotated_y = p.x * sin_r + p.y * cos_r;
            self.data.position + Vec2::new(rotated_x, rotated_y)
        }).collect();

        // Draw the rocket body
        draw_triangle(
            world_points[0],
            world_points[1],
            world_points[2],
            self.data.color,
        );

        // Draw rocket parts (engines, fuel tanks, etc.)
        for part in &self.parts {
            part.draw(self.data.position, self.rotation, 1.0);
        }

        // Draw thrust flame if thrusting
        if self.is_currently_thrusting && self.thrust_level > 0.0 {
            self.draw_thrust_flame();
        }

        // Draw velocity vector
        self.draw_velocity_vector();
    }

    fn position(&self) -> Vec2 {
        self.data.position
    }

    fn velocity(&self) -> Vec2 {
        self.data.velocity
    }

    fn set_velocity(&mut self, velocity: Vec2) {
        self.data.velocity = velocity;
    }

    fn color(&self) -> Color {
        self.data.color
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_rocket_creation() {
        let rocket = Rocket::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 0.0),
            WHITE,
            GameConstants::ROCKET_BASE_MASS,
        );

        assert_eq!(rocket.base_mass(), GameConstants::ROCKET_BASE_MASS);
        assert_eq!(rocket.current_fuel(), GameConstants::ROCKET_STARTING_FUEL);
    }

    #[test]
    fn test_fuel_addition() {
        let mut rocket = Rocket::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 0.0),
            WHITE,
            GameConstants::ROCKET_BASE_MASS,
        );

        // Rocket starts with ROCKET_STARTING_FUEL, already at max (100)
        // Adding 50 more should clamp to max
        rocket.add_fuel(50.0);
        assert_relative_eq!(rocket.current_fuel(), GameConstants::ROCKET_MAX_FUEL, epsilon = 0.01);
    }

    #[test]
    fn test_fuel_consumption() {
        let mut rocket = Rocket::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 0.0),
            WHITE,
            GameConstants::ROCKET_BASE_MASS,
        );

        rocket.set_fuel(100.0);
        let initial_fuel = rocket.current_fuel();

        rocket.set_thrust_level(0.5);
        rocket.update(1.0);

        assert!(rocket.current_fuel() < initial_fuel);
    }

    #[test]
    fn test_mass_updates_with_fuel() {
        let mut rocket = Rocket::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 0.0),
            WHITE,
            GameConstants::ROCKET_BASE_MASS,
        );

        // Rocket starts with full fuel (100), set it lower first
        rocket.set_fuel(80.0);
        let initial_mass = rocket.mass();

        // Now add 10 fuel
        rocket.add_fuel(10.0);
        assert_relative_eq!(rocket.mass(), initial_mass + 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_rotation() {
        let mut rocket = Rocket::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 0.0),
            WHITE,
            GameConstants::ROCKET_BASE_MASS,
        );

        rocket.rotate(std::f32::consts::PI / 2.0);
        assert_relative_eq!(rocket.rotation(), std::f32::consts::PI / 2.0, epsilon = 0.01);
    }
}
