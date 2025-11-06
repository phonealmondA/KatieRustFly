// Trajectory prediction system for visualizing orbital paths

use macroquad::prelude::*;
use crate::entities::{Planet, Rocket};
use crate::physics::GravitySimulator;
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

    /// Predict trajectory for a rocket
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

        // Simulate forward in time
        for _ in 0..steps {
            points.push(TrajectoryPoint {
                position,
                velocity,
                time,
            });

            // Apply gravity forces
            let mut acceleration = Vec2::ZERO;
            for planet in planets {
                let direction = planet.position() - position;
                let distance = vector_helper::magnitude(direction);

                if distance > planet.radius() {
                    let force_vec = self.gravity_simulator.calculate_gravitational_force(
                        position,
                        rocket.current_mass(),
                        planet.position(),
                        planet.mass(),
                    );
                    acceleration += force_vec / rocket.current_mass();
                }
            }

            // Update velocity and position
            velocity += acceleration * time_step;
            position += velocity * time_step;
            time += time_step;

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

    /// Draw trajectory path
    pub fn draw_trajectory(
        &self,
        points: &[TrajectoryPoint],
        color: Color,
        self_intersects: bool,
    ) {
        if points.len() < 2 {
            return;
        }

        // Draw lines between points
        for i in 0..points.len() - 1 {
            let p1 = points[i].position;
            let p2 = points[i + 1].position;

            // Fade out color along trajectory
            let alpha = 1.0 - (i as f32 / points.len() as f32) * 0.7;
            let fade_color = Color::new(color.r, color.g, color.b, color.a * alpha);

            draw_line(p1.x, p1.y, p2.x, p2.y, 2.0, fade_color);
        }

        // Draw intersection warning if detected
        if self_intersects {
            if let Some(last_point) = points.last() {
                draw_circle(
                    last_point.position.x,
                    last_point.position.y,
                    15.0,
                    Color::new(1.0, 0.0, 0.0, 0.5),
                );
            }
        }

        // Draw markers at intervals
        for (i, point) in points.iter().enumerate() {
            if i % 20 == 0 {
                draw_circle(point.position.x, point.position.y, 3.0, color);
            }
        }
    }

    /// Draw gravity force vectors acting on a rocket
    pub fn draw_gravity_force_vectors(
        &self,
        rocket: &Rocket,
        planets: &[&Planet],
        scale: f32,
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
                let label_pos = rocket_pos + force_vector * 0.5;
                let force_text = format!("{:.1}N", force_magnitude);
                draw_text(
                    &force_text,
                    label_pos.x + 5.0,
                    label_pos.y - 5.0,
                    16.0,
                    WHITE,
                );
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
        predictor.draw_trajectory(&points, YELLOW, false);
        assert!(true);
    }
}
