// Vehicle Manager - Manages active vehicle with visualization features
// Extended from original C++ VehicleManager with trajectory and force display

use macroquad::prelude::*;
use crate::entities::{Rocket, Planet};
use crate::physics::TrajectoryPredictor;
use crate::systems::EntityId;

/// Vehicle visualization options
#[derive(Debug, Clone)]
pub struct VisualizationOptions {
    pub show_trajectory: bool,
    pub show_gravity_forces: bool,
    pub trajectory_steps: usize,
    pub trajectory_time_step: f32,
    pub force_vector_scale: f32,
}

impl Default for VisualizationOptions {
    fn default() -> Self {
        VisualizationOptions {
            show_trajectory: true,
            show_gravity_forces: false,
            trajectory_steps: 200,
            trajectory_time_step: 0.5,
            force_vector_scale: 15.0,
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
    ) {
        // Draw trajectory prediction
        if self.visualization.show_trajectory {
            let (trajectory_points, self_intersects) = self.trajectory_predictor.predict_trajectory(
                rocket,
                planets,
                self.visualization.trajectory_time_step,
                self.visualization.trajectory_steps,
                true, // detect self-intersection
            );

            self.trajectory_predictor.draw_trajectory(
                &trajectory_points,
                Color::new(0.0, 1.0, 1.0, 0.6), // Cyan
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
            );
        }
    }

    /// Draw HUD overlay for visualization status
    pub fn draw_visualization_hud(&self) {
        let x = 10.0;
        let mut y = screen_height() - 120.0;
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
