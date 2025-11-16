// Vehicle Manager - Manages active vehicle with visualization features
// Extended from original C++ VehicleManager with trajectory and force display

use macroquad::prelude::*;
use crate::entities::{Rocket, Planet, GameObject};
use crate::physics::TrajectoryPredictor;
use crate::systems::EntityId;

/// Reference body index for trajectory calculations
/// This is an index into the planets array
pub type ReferenceBody = usize;

/// Vehicle visualization options
#[derive(Debug, Clone)]
pub struct VisualizationOptions {
    pub show_trajectory: bool,
    pub show_gravity_forces: bool,
    pub show_planet_trajectories: bool,
    pub trajectory_steps: usize,
    pub trajectory_time_step: f32,
    pub force_vector_scale: f32,
    pub reference_body: ReferenceBody,
}

impl Default for VisualizationOptions {
    fn default() -> Self {
        VisualizationOptions {
            show_trajectory: true,
            show_gravity_forces: false,
            show_planet_trajectories: false,
            trajectory_steps: 200,
            trajectory_time_step: 0.5,
            force_vector_scale: 15.0,
            reference_body: 0, // Default to first celestial body
        }
    }
}

/// Vehicle Manager - Manages the active vehicle and visualizations
pub struct VehicleManager {
    active_vehicle_id: Option<EntityId>,
    trajectory_predictor: TrajectoryPredictor,
    visualization: VisualizationOptions,
}

impl VehicleManager {
    pub fn new() -> Self {
        VehicleManager {
            active_vehicle_id: None,
            trajectory_predictor: TrajectoryPredictor::new(),
            visualization: VisualizationOptions::default(),
        }
    }

    /// Set the active vehicle
    pub fn set_active_vehicle(&mut self, vehicle_id: Option<EntityId>) {
        self.active_vehicle_id = vehicle_id;
    }

    /// Get the active vehicle ID
    pub fn active_vehicle_id(&self) -> Option<EntityId> {
        self.active_vehicle_id
    }

    /// Toggle trajectory visualization
    pub fn toggle_trajectory(&mut self) {
        self.visualization.show_trajectory = !self.visualization.show_trajectory;
    }

    /// Toggle gravity force visualization
    pub fn toggle_gravity_forces(&mut self) {
        self.visualization.show_gravity_forces = !self.visualization.show_gravity_forces;
    }

    /// Toggle planet trajectory visualization
    pub fn toggle_planet_trajectories(&mut self) {
        self.visualization.show_planet_trajectories = !self.visualization.show_planet_trajectories;
    }

    /// Cycle to next reference body
    pub fn toggle_reference_body(&mut self, num_bodies: usize) {
        if num_bodies == 0 {
            return;
        }
        self.visualization.reference_body = (self.visualization.reference_body + 1) % num_bodies;
    }

    /// Set visualization options
    pub fn set_visualization(&mut self, options: VisualizationOptions) {
        self.visualization = options;
    }

    /// Get visualization options
    pub fn visualization(&self) -> &VisualizationOptions {
        &self.visualization
    }

    /// Draw vehicle visualizations (trajectory, forces, etc.)
    pub fn draw_visualizations(
        &mut self,
        rocket: &Rocket,
        planets: &[&Planet],
        zoom_level: f32,
        camera: &Camera2D,
    ) {
        self.draw_visualizations_with_color(rocket, planets, zoom_level, camera, None);
    }

    /// Draw vehicle visualizations with custom trajectory color
    pub fn draw_visualizations_with_color(
        &mut self,
        rocket: &Rocket,
        planets: &[&Planet],
        zoom_level: f32,
        camera: &Camera2D,
        trajectory_color: Option<Color>,
    ) {
        // Draw reference body indicator (white circle)
        self.draw_reference_body_indicator(planets);

        // Draw trajectory prediction
        if self.visualization.show_trajectory {
            let (trajectory_points, self_intersects) = self.trajectory_predictor.predict_trajectory_with_reference(
                rocket,
                planets,
                self.visualization.trajectory_time_step,
                self.visualization.trajectory_steps,
                true, // detect self-intersection
                self.visualization.reference_body,
            );

            // Use custom color if provided, otherwise default to cyan
            let color = trajectory_color.unwrap_or(Color::new(0.0, 1.0, 1.0, 0.6));

            self.trajectory_predictor.draw_trajectory(
                &trajectory_points,
                color,
                self_intersects,
                zoom_level,
            );

            // Draw intersection warning
            if self_intersects {
                let warning = "⚠ Orbit Intersects!";
                let text_size = 24.0;
                let text_dims = measure_text(warning, None, text_size as u16, 1.0);

                draw_text(
                    warning,
                    screen_width() / 2.0 - text_dims.width / 2.0,
                    50.0,
                    text_size,
                    Color::new(1.0, 0.5, 0.0, 1.0), // Orange warning
                );
            }
        }

        // Draw gravity force vectors
        if self.visualization.show_gravity_forces {
            self.trajectory_predictor.draw_gravity_force_vectors(
                rocket,
                planets,
                self.visualization.force_vector_scale,
                camera,
            );
        }
    }

    /// Draw white circle indicator at the center of the selected reference body
    fn draw_reference_body_indicator(&self, planets: &[&Planet]) {
        if planets.is_empty() {
            return;
        }

        // Get the selected body index (ensure it's within bounds)
        let selected_idx = if self.visualization.reference_body < planets.len() {
            self.visualization.reference_body
        } else {
            0
        };

        let selected_planet = planets[selected_idx];
        let pos = selected_planet.position();

        // Draw white circle at center (size scales with zoom, min 5, max 15)
        let circle_radius = 8.0;
        draw_circle(pos.x, pos.y, circle_radius, WHITE);
        draw_circle_lines(pos.x, pos.y, circle_radius, 2.0, Color::new(0.0, 0.0, 0.0, 0.8));
    }

    /// Draw trajectory predictions for all planets (IMPROVED with better visibility)
    pub fn draw_planet_trajectories(&mut self, planets: &[&Planet], zoom_level: f32) {
        if !self.visualization.show_planet_trajectories || planets.is_empty() {
            return;
        }

        // Draw trajectory for each planet
        for planet in planets.iter() {
            let planet_name = planet.name().unwrap_or("Unknown");

            // Skip pinned planets (they don't move)
            if planet.is_pinned() {
                continue;
            }

            let vel = planet.velocity();
            let vel_mag = (vel.x * vel.x + vel.y * vel.y).sqrt();

            // Skip stationary planets
            if vel_mag < 0.1 {
                continue;
            }

            // Use longer prediction for planets (they move slower)
            let planet_traj_steps = (self.visualization.trajectory_steps * 3).min(1000);

            // Predict trajectory using the planet's current position, velocity, and mass
            let (trajectory_points, _self_intersects) = self.trajectory_predictor.predict_trajectory_from_state(
                planet.position(),
                planet.velocity(),
                planet.mass(),
                planets,
                self.visualization.trajectory_time_step,
                planet_traj_steps,
                false, // Don't detect self-intersection for planets
            );

            // Use a brighter, more visible color scheme for planet orbits
            // Make the trajectories glow with higher alpha for better visibility
            let base_color = planet.color();
            let enhanced_color = Color::new(
                base_color.r.max(0.3), // Ensure minimum brightness
                base_color.g.max(0.3),
                base_color.b.max(0.3),
                0.8, // Higher alpha for better visibility
            );

            // Draw trajectory with enhanced visibility
            self.trajectory_predictor.draw_trajectory(
                &trajectory_points,
                enhanced_color,
                false, // No self-intersection warning for planets
                zoom_level,
            );
        }
    }

    /// Draw HUD overlay for visualization status
    pub fn draw_visualization_hud(&self, planets: &[&Planet]) {
        let x = 10.0;
        let mut y = screen_height() - 170.0;
        let line_height = 25.0;
        let font_size = 18.0;

        // Title
        draw_text("Visualizations:", x, y, font_size, WHITE);
        y += line_height;

        // Trajectory status
        let traj_status = if self.visualization.show_trajectory {
            "✓ Trajectory (T)"
        } else {
            "  Trajectory (T)"
        };
        let traj_color = if self.visualization.show_trajectory {
            Color::new(0.0, 1.0, 0.0, 1.0)
        } else {
            Color::new(0.5, 0.5, 0.5, 1.0)
        };
        draw_text(traj_status, x, y, font_size, traj_color);
        y += line_height;

        // Gravity forces status
        let forces_status = if self.visualization.show_gravity_forces {
            "✓ Forces (G)"
        } else {
            "  Forces (G)"
        };
        let forces_color = if self.visualization.show_gravity_forces {
            Color::new(0.0, 1.0, 0.0, 1.0)
        } else {
            Color::new(0.5, 0.5, 0.5, 1.0)
        };
        draw_text(forces_status, x, y, font_size, forces_color);
        y += line_height;

        // Planet trajectories status
        let planet_traj_status = if self.visualization.show_planet_trajectories {
            "✓ Planet Orbits (O)"
        } else {
            "  Planet Orbits (O)"
        };
        let planet_traj_color = if self.visualization.show_planet_trajectories {
            Color::new(0.0, 1.0, 0.0, 1.0)
        } else {
            Color::new(0.5, 0.5, 0.5, 1.0)
        };
        draw_text(planet_traj_status, x, y, font_size, planet_traj_color);
        y += line_height;

        // Reference body status - show actual planet name
        let ref_body_name = if planets.is_empty() {
            "Unknown"
        } else {
            let idx = if self.visualization.reference_body < planets.len() {
                self.visualization.reference_body
            } else {
                0
            };
            planets[idx].name().unwrap_or("Unknown")
        };
        let ref_body_text = format!("  Ref: {} (Tab)", ref_body_name);
        draw_text(&ref_body_text, x, y, font_size, Color::new(0.7, 0.9, 1.0, 1.0));
    }

    /// Check if vehicle can convert to satellite
    pub fn can_convert_to_satellite(&self, rocket: &Rocket) -> bool {
        // Require sufficient fuel and velocity for stable orbit
        rocket.current_fuel() > 50.0
    }
}

impl Default for VehicleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vehicle_manager_creation() {
        let manager = VehicleManager::new();
        assert!(manager.active_vehicle_id().is_none());
        assert!(manager.visualization().show_trajectory);
        assert!(!manager.visualization().show_gravity_forces);
    }

    #[test]
    fn test_set_active_vehicle() {
        let mut manager = VehicleManager::new();
        manager.set_active_vehicle(Some(42));
        assert_eq!(manager.active_vehicle_id(), Some(42));
    }

    #[test]
    fn test_toggle_visualizations() {
        let mut manager = VehicleManager::new();

        let initial_traj = manager.visualization().show_trajectory;
        manager.toggle_trajectory();
        assert_eq!(manager.visualization().show_trajectory, !initial_traj);

        let initial_forces = manager.visualization().show_gravity_forces;
        manager.toggle_gravity_forces();
        assert_eq!(manager.visualization().show_gravity_forces, !initial_forces);
    }

    #[test]
    fn test_can_convert_to_satellite() {
        let manager = VehicleManager::new();

        let mut rocket = Rocket::new(
            Vec2::new(100.0, 0.0),
            Vec2::new(0.0, 50.0),
            WHITE,
            1.0,
        );

        // Add enough fuel for conversion (requires > 50.0)
        rocket.add_fuel(100.0);

        // Should be able to convert with sufficient fuel
        assert!(manager.can_convert_to_satellite(&rocket));
    }
}
