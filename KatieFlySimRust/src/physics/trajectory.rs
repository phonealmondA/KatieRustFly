// Trajectory prediction system for visualizing orbital paths

use macroquad::prelude::*;
use crate::entities::{Planet, Rocket};
use crate::physics::GravitySimulator;
use crate::systems::vehicle_manager::ReferenceBody;
use crate::utils::vector_helper;

/// Predicted trajectory point
#[derive(Clone, Debug)]
pub struct TrajectoryPoint {
    pub position: Vec2,
    pub velocity: Vec2,
    pub time: f32,
}

/// Trajectory predictor
pub struct TrajectoryPredictor {
    gravity_simulator: GravitySimulator,
}

impl TrajectoryPredictor {
    pub fn new() -> Self {
        TrajectoryPredictor {
            gravity_simulator: GravitySimulator::new(),
        }
    }

    /// Predict trajectory for a rocket, accounting for planet motion
    ///
    /// # Arguments
    /// * `rocket` - The rocket to predict trajectory for
    /// * `planets` - All planets affecting gravity
    /// * `time_step` - Time step for simulation (default: 0.5 seconds)
    /// * `steps` - Number of steps to predict (default: 200)
    /// * `detect_self_intersection` - Check if trajectory intersects itself
    ///
    /// # Returns
    /// Vector of trajectory points and whether it self-intersects
    ///
    /// Note: This simulation accounts for planet motion (e.g., Moon orbiting Earth)
    /// during the trajectory prediction, providing accurate future positions.
    pub fn predict_trajectory(
        &mut self,
        rocket: &Rocket,
        planets: &[&Planet],
        time_step: f32,
        steps: usize,
        detect_self_intersection: bool,
    ) -> (Vec<TrajectoryPoint>, bool) {
        let mut points = Vec::with_capacity(steps);
        let mut self_intersects = false;

        // Start with current rocket state
        let mut position = rocket.position();
        let mut velocity = rocket.velocity();
        let mut time = 0.0;

        // Create mutable copies of planet states (position, velocity, mass, radius)
        // This allows us to simulate their motion during trajectory prediction
        let mut planet_states: Vec<(Vec2, Vec2, f32, f32)> = planets
            .iter()
            .map(|p| (p.position(), p.velocity(), p.mass(), p.radius()))
            .collect();

        // Identify Earth (largest planet) for pinning - it doesn't move
        let earth_index = planet_states
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.3.partial_cmp(&b.3).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Simulate forward in time
        for _ in 0..steps {
            points.push(TrajectoryPoint {
                position,
                velocity,
                time,
            });

            // Step 1: Apply planet-to-planet gravity (e.g., Moon orbiting Earth)
            // This updates planet velocities based on gravitational interactions
            if planet_states.len() >= 2 {
                for i in 0..planet_states.len() {
                    if i == earth_index {
                        continue; // Earth is pinned, doesn't move
                    }

                    let mut planet_acceleration = Vec2::ZERO;

                    // Calculate gravity from all other planets
                    for j in 0..planet_states.len() {
                        if i == j {
                            continue;
                        }

                        let (pos_i, _, mass_i, radius_i) = planet_states[i];
                        let (pos_j, _, mass_j, _) = planet_states[j];

                        let direction = pos_j - pos_i;
                        let distance = vector_helper::magnitude(direction);

                        if distance > radius_i {
                            let force = self.gravity_simulator.calculate_gravitational_force(
                                pos_i, mass_i, pos_j, mass_j,
                            );
                            planet_acceleration += force / mass_i;
                        }
                    }

                    // Update planet velocity
                    planet_states[i].1 += planet_acceleration * time_step;
                }
            }

            // Step 2: Update planet positions based on their velocities
            for i in 0..planet_states.len() {
                if i != earth_index {
                    let vel = planet_states[i].1; // Extract velocity first to avoid borrow checker issues
                    planet_states[i].0 += vel * time_step;
                }
            }

            // Step 3: Calculate rocket's acceleration from updated planet positions
            let mut acceleration = Vec2::ZERO;
            for &(planet_pos, _, planet_mass, planet_radius) in &planet_states {
                let direction = planet_pos - position;
                let distance = vector_helper::magnitude(direction);

                if distance > planet_radius {
                    let force_vec = self.gravity_simulator.calculate_gravitational_force(
                        position,
                        rocket.current_mass(),
                        planet_pos,
                        planet_mass,
                    );
                    acceleration += force_vec / rocket.current_mass();
                }
            }

            // Step 4: Update rocket velocity and position
            velocity += acceleration * time_step;
            position += velocity * time_step;
            time += time_step;

            // Check for collision with planets at their predicted positions
            let mut hit_planet = false;
            for &(planet_pos, _, _, planet_radius) in &planet_states {
                let distance = vector_helper::magnitude(planet_pos - position);
                // Add small buffer for collision detection
                if distance < planet_radius + 5.0 {
                    hit_planet = true;
                    break;
                }
            }
            if hit_planet {
                break;
            }

            // Check for self-intersection if requested
            if detect_self_intersection && points.len() > 20 {
                if self.check_intersection(&points, position) {
                    self_intersects = true;
                    break;
                }
            }
        }

        (points, self_intersects)
    }

    /// Predict trajectory with selectable reference body (Earth or Moon)
    ///
    /// # Arguments
    /// * `rocket` - The rocket to predict trajectory for
    /// * `planets` - All planets affecting gravity
    /// * `time_step` - Time step for simulation (default: 0.5 seconds)
    /// * `steps` - Number of steps to predict (default: 200)
    /// * `detect_self_intersection` - Check if trajectory intersects itself
    /// * `reference_body` - Which body to use as reference (Earth or Moon)
    ///
    /// # Returns
    /// Vector of trajectory points (relative to reference body) and whether it self-intersects
    ///
    /// When Moon is selected, trajectory is calculated in Moon's non-inertial reference frame,
    /// accounting for the Moon's acceleration from Earth's gravity as a fictitious force.
    pub fn predict_trajectory_with_reference(
        &mut self,
        rocket: &Rocket,
        planets: &[&Planet],
        time_step: f32,
        steps: usize,
        detect_self_intersection: bool,
        reference_body: ReferenceBody,
    ) -> (Vec<TrajectoryPoint>, bool) {
        // If Earth is selected, use standard Earth-centered prediction
        if reference_body == ReferenceBody::Earth {
            return self.predict_trajectory(
                rocket,
                planets,
                time_step,
                steps,
                detect_self_intersection,
            );
        }

        // For Moon reference, we need to simulate in the Moon's accelerating reference frame
        // Find Earth and Moon indices
        let mut earth_idx = 0;
        let mut max_radius = 0.0f32;
        for (i, planet) in planets.iter().enumerate() {
            if planet.radius() > max_radius {
                max_radius = planet.radius();
                earth_idx = i;
            }
        }

        let moon_idx = if planets.len() < 2 {
            earth_idx // Fallback if no Moon
        } else {
            if earth_idx == 0 { 1 } else { 0 }
        };

        let mut points = Vec::with_capacity(steps);
        let mut self_intersects = false;

        // Start with current rocket state in absolute coordinates
        let mut rocket_pos_abs = rocket.position();
        let mut rocket_vel_abs = rocket.velocity();
        let mut time = 0.0;

        // Create mutable copies of planet states (position, velocity, mass, radius)
        let mut planet_states: Vec<(Vec2, Vec2, f32, f32)> = planets
            .iter()
            .map(|p| (p.position(), p.velocity(), p.mass(), p.radius()))
            .collect();

        // Store Moon's initial position for drawing offset
        let moon_initial_pos = planet_states[moon_idx].0;

        // Simulate forward in time
        for step in 0..steps {
            // Get current Moon state
            let (moon_pos, moon_vel, moon_mass, moon_radius) = planet_states[moon_idx];

            // Transform rocket to Moon-relative coordinates for storage
            let rocket_pos_moon_rel = rocket_pos_abs - moon_pos;
            let rocket_pos_draw = rocket_pos_moon_rel + moon_initial_pos;

            points.push(TrajectoryPoint {
                position: rocket_pos_draw,
                velocity: rocket_vel_abs - moon_vel, // Velocity relative to Moon
                time,
            });

            // === PHYSICS STEP ===

            // Step 1: Calculate Moon's acceleration from Earth
            let moon_accel = if planet_states.len() >= 2 {
                let (earth_pos, _, earth_mass, _) = planet_states[earth_idx];
                let direction = earth_pos - moon_pos;
                let distance = vector_helper::magnitude(direction);

                if distance > moon_radius {
                    let force = self.gravity_simulator.calculate_gravitational_force(
                        moon_pos,
                        moon_mass,
                        earth_pos,
                        earth_mass,
                    );
                    force / moon_mass
                } else {
                    Vec2::ZERO
                }
            } else {
                Vec2::ZERO
            };

            // Step 2: Calculate rocket's acceleration from all planets in absolute frame
            let mut rocket_accel_abs = Vec2::ZERO;
            for &(planet_pos, _, planet_mass, planet_radius) in &planet_states {
                let direction = planet_pos - rocket_pos_abs;
                let distance = vector_helper::magnitude(direction);

                if distance > planet_radius {
                    let force = self.gravity_simulator.calculate_gravitational_force(
                        rocket_pos_abs,
                        rocket.current_mass(),
                        planet_pos,
                        planet_mass,
                    );
                    rocket_accel_abs += force / rocket.current_mass();
                }
            }

            // Step 3: Update rocket velocity and position (absolute frame)
            rocket_vel_abs += rocket_accel_abs * time_step;
            rocket_pos_abs += rocket_vel_abs * time_step;

            // Step 4: Update Moon's velocity and position (using same physics)
            planet_states[moon_idx].1 += moon_accel * time_step;
            let moon_updated_vel = planet_states[moon_idx].1; // Extract to avoid borrow checker issue
            planet_states[moon_idx].0 += moon_updated_vel * time_step;

            time += time_step;

            // Check for collision with planets at their current positions
            let mut hit_planet = false;
            for &(planet_pos, _, _, planet_radius) in &planet_states {
                let distance = vector_helper::magnitude(planet_pos - rocket_pos_abs);
                if distance < planet_radius + 5.0 {
                    hit_planet = true;
                    break;
                }
            }
            if hit_planet {
                break;
            }

            // Check for self-intersection if requested
            if detect_self_intersection && points.len() > 20 {
                // Check intersection in Moon-relative coordinates (more accurate for Moon orbits)
                if self.check_intersection(&points, rocket_pos_draw) {
                    self_intersects = true;
                    break;
                }
            }
        }

        (points, self_intersects)
    }

    /// Predict trajectory from a specific position and velocity
    ///
    /// # Arguments
    /// * `start_pos` - Starting position
    /// * `start_vel` - Starting velocity
    /// * `mass` - Object mass (affects acceleration from gravity)
    /// * `planets` - All planets affecting gravity
    /// * `time_step` - Time step for simulation
    /// * `steps` - Number of steps to predict
    /// * `detect_self_intersection` - Check if trajectory intersects itself
    ///
    /// # Returns
    /// Vector of trajectory points and whether it self-intersects
    pub fn predict_trajectory_from_state(
        &mut self,
        start_pos: Vec2,
        start_vel: Vec2,
        mass: f32,
        planets: &[&Planet],
        time_step: f32,
        steps: usize,
        detect_self_intersection: bool,
    ) -> (Vec<TrajectoryPoint>, bool) {
        let mut points = Vec::with_capacity(steps);
        let mut self_intersects = false;

        // Start with provided state
        let mut position = start_pos;
        let mut velocity = start_vel;
        let mut time = 0.0;

        // Create mutable copies of planet states (position, velocity, mass, radius)
        // This allows us to simulate their motion during trajectory prediction
        let mut planet_states: Vec<(Vec2, Vec2, f32, f32)> = planets
            .iter()
            .map(|p| (p.position(), p.velocity(), p.mass(), p.radius()))
            .collect();

        // Identify Earth (largest planet) for pinning - it doesn't move
        let earth_index = planet_states
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.3.partial_cmp(&b.3).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Simulate forward in time
        for _ in 0..steps {
            points.push(TrajectoryPoint {
                position,
                velocity,
                time,
            });

            // Step 1: Apply planet-to-planet gravity (e.g., Moon orbiting Earth)
            // This updates planet velocities based on gravitational interactions
            if planet_states.len() >= 2 {
                for i in 0..planet_states.len() {
                    if i == earth_index {
                        continue; // Earth is pinned, doesn't move
                    }

                    let mut planet_acceleration = Vec2::ZERO;

                    // Calculate gravity from all other planets
                    for j in 0..planet_states.len() {
                        if i == j {
                            continue;
                        }

                        let (pos_i, _, mass_i, radius_i) = planet_states[i];
                        let (pos_j, _, mass_j, _) = planet_states[j];

                        let direction = pos_j - pos_i;
                        let distance = vector_helper::magnitude(direction);

                        if distance > radius_i {
                            let force = self.gravity_simulator.calculate_gravitational_force(
                                pos_i, mass_i, pos_j, mass_j,
                            );
                            planet_acceleration += force / mass_i;
                        }
                    }

                    // Update planet velocity
                    planet_states[i].1 += planet_acceleration * time_step;
                }
            }

            // Step 2: Update planet positions based on their velocities
            for i in 0..planet_states.len() {
                if i != earth_index {
                    let vel = planet_states[i].1;
                    planet_states[i].0 += vel * time_step;
                }
            }

            // Step 3: Calculate object's acceleration from updated planet positions
            let mut acceleration = Vec2::ZERO;
            for &(planet_pos, _, planet_mass, planet_radius) in &planet_states {
                let direction = planet_pos - position;
                let distance = vector_helper::magnitude(direction);

                if distance > planet_radius {
                    let force_vec = self.gravity_simulator.calculate_gravitational_force(
                        position,
                        mass,
                        planet_pos,
                        planet_mass,
                    );
                    acceleration += force_vec / mass;
                }
            }

            // Step 4: Update velocity and position
            velocity += acceleration * time_step;
            position += velocity * time_step;
            time += time_step;

            // Check for collision with planets at their predicted positions
            let mut hit_planet = false;
            for &(planet_pos, _, _, planet_radius) in &planet_states {
                let distance = vector_helper::magnitude(planet_pos - position);
                // Add small buffer for collision detection
                if distance < planet_radius + 5.0 {
                    hit_planet = true;
                    break;
                }
            }
            if hit_planet {
                break;
            }

            // Check for self-intersection if requested
            if detect_self_intersection && points.len() > 20 {
                if self.check_intersection(&points, position) {
                    self_intersects = true;
                    break;
                }
            }
        }

        (points, self_intersects)
    }

    /// Predict trajectory for a planet (e.g., moon orbiting Earth)
    ///
    /// # Arguments
    /// * `planet` - The planet to predict trajectory for
    /// * `other_planets` - All other planets affecting gravity (excluding the planet itself)
    /// * `time_step` - Time step for simulation (default: 0.5 seconds)
    /// * `steps` - Number of steps to predict (default: 840 for full orbit)
    /// * `detect_self_intersection` - Check if trajectory intersects itself (orbit closes)
    ///
    /// # Returns
    /// Vector of trajectory points and whether it self-intersects (completes orbit)
    pub fn predict_planet_trajectory(
        &mut self,
        planet: &Planet,
        other_planets: &[&Planet],
        time_step: f32,
        steps: usize,
        detect_self_intersection: bool,
    ) -> (Vec<TrajectoryPoint>, bool) {
        let mut points = Vec::with_capacity(steps);
        let mut self_intersects = false;

        // Start with current planet state
        let mut position = planet.position();
        let mut velocity = planet.velocity();
        let mut time = 0.0;

        // Simulate forward in time
        for _ in 0..steps {
            points.push(TrajectoryPoint {
                position,
                velocity,
                time,
            });

            // Apply gravity forces from other planets
            let mut acceleration = Vec2::ZERO;
            for other_planet in other_planets {
                let direction = other_planet.position() - position;
                let distance = vector_helper::magnitude(direction);

                if distance > other_planet.radius() {
                    let force_vec = self.gravity_simulator.calculate_gravitational_force(
                        position,
                        planet.mass(),
                        other_planet.position(),
                        other_planet.mass(),
                    );
                    acceleration += force_vec / planet.mass();
                }
            }

            // Update velocity and position
            velocity += acceleration * time_step;
            position += velocity * time_step;
            time += time_step;

            // Check for self-intersection if requested (orbit completion)
            if detect_self_intersection && points.len() > 20 {
                if self.check_intersection(&points, position) {
                    self_intersects = true;
                    break;
                }
            }
        }

        (points, self_intersects)
    }

    /// Check if a position intersects with previous trajectory
    fn check_intersection(&self, points: &[TrajectoryPoint], position: Vec2) -> bool {
        // Only check against points that are far enough back in time
        let check_start = points.len().saturating_sub(20);

        for i in 0..check_start {
            let distance = vector_helper::distance(points[i].position, position);
            // Consider intersection if within 50 units
            if distance < 50.0 {
                return true;
            }
        }
        false
    }

    /// Draw trajectory path with zoom-scaled line thickness
    ///
    /// # Arguments
    /// * `points` - Trajectory points to draw
    /// * `color` - Base color for the trajectory
    /// * `self_intersects` - Whether the trajectory completes an orbit
    /// * `zoom_level` - Current camera zoom level (for scaling line thickness)
    pub fn draw_trajectory(
        &self,
        points: &[TrajectoryPoint],
        color: Color,
        self_intersects: bool,
        zoom_level: f32,
    ) {
        if points.len() < 2 {
            return;
        }

        // Scale line thickness with zoom level, but less aggressively
        // Using power function to gradually increase thickness without constant size
        // Base thickness is 8.0, scaled by zoom_level^0.8
        // When zoom_level = 1.0 (zoomed in), thickness = 8.0
        // When zoom_level = 10.0 (medium zoom), thickness ≈ 50.0
        // When zoom_level = 100.0 (zoomed out), thickness ≈ 318.0
        let base_line_thickness = 8.0;
        let scaled_line_thickness = base_line_thickness * zoom_level.powf(0.8);

        let base_marker_radius = 12.0;
        let scaled_marker_radius = base_marker_radius * zoom_level.powf(0.8);

        let base_completion_radius = 60.0;
        let scaled_completion_radius = base_completion_radius * zoom_level.powf(0.8);

        // Draw lines between points with zoom-scaled thickness
        for i in 0..points.len() - 1 {
            let p1 = points[i].position;
            let p2 = points[i + 1].position;

            // Fade out color along trajectory
            let alpha = 1.0 - (i as f32 / points.len() as f32) * 0.7;
            let fade_color = Color::new(color.r, color.g, color.b, color.a * alpha);

            draw_line(p1.x, p1.y, p2.x, p2.y, scaled_line_thickness, fade_color);
        }

        // Draw intersection warning if detected
        if self_intersects {
            if let Some(last_point) = points.last() {
                draw_circle(
                    last_point.position.x,
                    last_point.position.y,
                    scaled_completion_radius,
                    Color::new(1.0, 0.0, 0.0, 0.5),
                );
            }
        }

        // Draw markers at intervals
        for (i, point) in points.iter().enumerate() {
            if i % 20 == 0 {
                draw_circle(point.position.x, point.position.y, scaled_marker_radius, color);
            }
        }
    }

    /// Draw gravity force vectors acting on a rocket
    pub fn draw_gravity_force_vectors(
        &self,
        rocket: &Rocket,
        planets: &[&Planet],
        scale: f32,
        camera: &Camera2D,
    ) {
        let rocket_pos = rocket.position();

        for planet in planets {
            let direction = planet.position() - rocket_pos;
            let distance = vector_helper::magnitude(direction);

            if distance > planet.radius() {
                let force_vec = self.gravity_simulator.calculate_gravitational_force(
                    rocket_pos,
                    rocket.current_mass(),
                    planet.position(),
                    planet.mass(),
                );

                let force_magnitude = vector_helper::magnitude(force_vec);
                let force_vector = force_vec * scale;

                // Draw force vector as arrow
                let end_pos = rocket_pos + force_vector;

                // Main line
                draw_line(
                    rocket_pos.x,
                    rocket_pos.y,
                    end_pos.x,
                    end_pos.y,
                    2.0,
                    GREEN,
                );

                // Arrow head
                let arrow_size = 10.0;
                let force_direction = if force_magnitude > 0.0 {
                    vector_helper::normalize(force_vec)
                } else {
                    Vec2::new(1.0, 0.0)
                };
                let perpendicular = Vec2::new(-force_direction.y, force_direction.x);
                let arrow_base = end_pos - force_direction * arrow_size;
                let arrow_left = arrow_base + perpendicular * arrow_size * 0.5;
                let arrow_right = arrow_base - perpendicular * arrow_size * 0.5;

                draw_triangle(
                    end_pos,
                    arrow_left,
                    arrow_right,
                    GREEN,
                );

                // Draw label with force magnitude
                // Convert world position to screen position so text appears right-side up
                let label_pos = rocket_pos + force_vector * 0.5;
                let force_text = format!("{:.1}N", force_magnitude);

                // Convert world coordinates to screen coordinates
                let screen_pos = camera.world_to_screen(label_pos);

                // Temporarily switch to default camera to draw text right-side up
                set_default_camera();
                draw_text(
                    &force_text,
                    screen_pos.x + 5.0,
                    screen_pos.y - 5.0,
                    16.0,
                    WHITE,
                );
                // Restore the world camera
                set_camera(camera);
            }
        }
    }
}

impl Default for TrajectoryPredictor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trajectory_prediction() {
        let mut predictor = TrajectoryPredictor::new();

        let rocket = Rocket::new(
            Vec2::new(200.0, 0.0),
            Vec2::new(0.0, 50.0),
            WHITE,
            1.0,
        );

        let planet = Planet::new(
            Vec2::new(0.0, 0.0),
            50.0,
            10000.0,
            BLUE,
        );

        let planets = vec![&planet];
        let (points, _self_intersects) = predictor.predict_trajectory(
            &rocket,
            &planets,
            0.1,
            100,
            false,
        );

        assert_eq!(points.len(), 100);
        assert!(points[0].position == rocket.position());
    }

    #[test]
    fn test_trajectory_with_intersection_detection() {
        let mut predictor = TrajectoryPredictor::new();

        // Create a rocket that might orbit back on itself
        let rocket = Rocket::new(
            Vec2::new(300.0, 0.0),
            Vec2::new(0.0, 80.0),
            WHITE,
            1.0,
        );

        let planet = Planet::new(
            Vec2::new(0.0, 0.0),
            50.0,
            50000.0,
            BLUE,
        );

        let planets = vec![&planet];
        let (_points, _self_intersects) = predictor.predict_trajectory(
            &rocket,
            &planets,
            0.5,
            500,
            true,
        );

        // Just verify it doesn't crash
        assert!(true);
    }

    #[test]
    fn test_trajectory_empty() {
        let predictor = TrajectoryPredictor::new();
        let points: Vec<TrajectoryPoint> = vec![];

        // Should not crash when drawing empty trajectory
        predictor.draw_trajectory(&points, YELLOW, false, 1.0);
        assert!(true);
    }
}
