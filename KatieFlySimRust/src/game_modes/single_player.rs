// Single Player Game - Main single player game mode
// Integrates all systems for playable game

use macroquad::prelude::*;

use crate::entities::{GameObject, Planet, Rocket};
use crate::game_constants::GameConstants;
use crate::physics::TrajectoryPredictor;
use crate::save_system::{GameSaveData, SavedCamera};
use crate::systems::World;
use crate::ui::{Camera, Hud};

/// Single player game result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinglePlayerResult {
    Continue,
    ReturnToMenu,
    Quit,
}

/// Single player game mode
pub struct SinglePlayerGame {
    world: World,
    camera: Camera,
    hud: Hud,
    trajectory_predictor: TrajectoryPredictor,
    game_time: f32,
    is_paused: bool,
    show_controls: bool,

    // Input state
    thrust_input: f32,
    rotation_input: f32,

    // Save/load
    current_save_name: Option<String>,
    last_auto_save: f32,
    auto_save_interval: f32,
}

impl SinglePlayerGame {
    pub fn new(window_size: Vec2) -> Self {
        SinglePlayerGame {
            world: World::new(),
            camera: Camera::new(window_size),
            hud: Hud::new(Vec2::new(10.0, 10.0)),
            trajectory_predictor: TrajectoryPredictor::new(),
            game_time: 0.0,
            is_paused: false,
            show_controls: false,
            thrust_input: 0.0,
            rotation_input: 0.0,
            current_save_name: None,
            last_auto_save: 0.0,
            auto_save_interval: 60.0, // Auto-save every 60 seconds
        }
    }

    /// Initialize a new game with default setup
    pub fn initialize_new_game(&mut self) {
        self.world.clear_all();
        self.game_time = 0.0;

        // Create main planet (like Earth)
        let main_planet = Planet::new(
            Vec2::new(GameConstants::MAIN_PLANET_X, GameConstants::MAIN_PLANET_Y),
            GameConstants::MAIN_PLANET_RADIUS,
            GameConstants::MAIN_PLANET_MASS,
            BLUE,
        );
        self.world.add_planet(main_planet);

        // Create secondary planet (like Moon)
        let mut secondary_planet = Planet::new(
            Vec2::new(*crate::game_constants::SECONDARY_PLANET_X, *crate::game_constants::SECONDARY_PLANET_Y),
            GameConstants::SECONDARY_PLANET_RADIUS,
            GameConstants::SECONDARY_PLANET_MASS,
            Color::from_rgba(150, 150, 150, 255),
        );

        // Set orbital velocity for secondary planet
        secondary_planet.set_velocity(Vec2::new(
            0.0,
            -*crate::game_constants::SECONDARY_PLANET_ORBITAL_VELOCITY,
        ));

        self.world.add_planet(secondary_planet);

        // Create starting rocket near main planet
        let rocket_spawn_distance = GameConstants::MAIN_PLANET_RADIUS + 200.0;
        let rocket = Rocket::new(
            Vec2::new(
                GameConstants::MAIN_PLANET_X + rocket_spawn_distance,
                GameConstants::MAIN_PLANET_Y,
            ),
            Vec2::new(0.0, 0.0),
            WHITE,
            GameConstants::ROCKET_BASE_MASS,
        );

        let rocket_id = self.world.add_rocket(rocket);

        // Set camera to follow rocket
        if let Some(rocket) = self.world.get_rocket(rocket_id) {
            self.camera.set_center(rocket.position());
        }

        log::info!("New game initialized");
    }

    /// Load game from save data
    pub fn load_from_save(&mut self, save_data: GameSaveData, save_name: String) {
        self.world.clear_all();
        self.game_time = save_data.game_time;

        // Load planets
        for saved_planet in save_data.planets {
            let (_id, planet) = saved_planet.to_planet();
            self.world.add_planet(planet);
        }

        // Load rockets
        for saved_rocket in save_data.rockets {
            let (_id, rocket) = saved_rocket.to_rocket();
            self.world.add_rocket(rocket);
        }

        // Load satellites
        for saved_satellite in save_data.satellites {
            let (_id, satellite) = saved_satellite.to_satellite();
            self.world.add_satellite(satellite);
        }

        // Set active rocket
        self.world.set_active_rocket(save_data.active_rocket_id);

        // Restore camera
        self.camera.set_center(save_data.camera.center.into());
        self.camera.set_target_zoom(save_data.camera.zoom);

        self.current_save_name = Some(save_name.clone());

        log::info!("Game loaded from save: {}", save_name);
    }

    /// Save current game state
    pub fn save_game(&self, save_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let save_data = self.create_save_data();
        save_data.save_to_file(save_name)?;
        log::info!("Game saved: {}", save_name);
        Ok(())
    }

    /// Create save data from current game state
    fn create_save_data(&self) -> GameSaveData {
        let mut save_data = GameSaveData::new();
        save_data.game_time = self.game_time;

        // Save planets (we need to iterate with IDs - simplified for now)
        // In a real implementation, World would provide an iterator with IDs

        // Save camera
        save_data.camera = SavedCamera {
            center: self.camera.camera().target.into(),
            zoom: self.camera.zoom_level(),
        };

        save_data.active_rocket_id = self.world.active_rocket_id();

        save_data
    }

    /// Handle input for game controls
    pub fn handle_input(&mut self) -> SinglePlayerResult {
        // Check for escape to return to menu or close controls popup
        if is_key_pressed(KeyCode::Escape) {
            if self.show_controls {
                self.show_controls = false;
                self.is_paused = false;
            } else {
                return SinglePlayerResult::ReturnToMenu;
            }
        }

        // Toggle controls menu with Enter key
        if is_key_pressed(KeyCode::Enter) {
            self.show_controls = !self.show_controls;
            self.is_paused = self.show_controls; // Pause when showing controls
        }

        // Handle mouse click for controls button and popup
        if is_mouse_button_pressed(MouseButton::Left) {
            let mouse_pos = mouse_position();
            let screen_w = screen_width();

            // Controls button in top-right corner (40x30 button with 10px margin)
            let button_x = screen_w - 50.0;
            let button_y = 10.0;
            let button_w = 40.0;
            let button_h = 30.0;

            // Check if click is on the button first
            let clicked_button = mouse_pos.0 >= button_x && mouse_pos.0 <= button_x + button_w &&
                                 mouse_pos.1 >= button_y && mouse_pos.1 <= button_y + button_h;

            if clicked_button {
                self.show_controls = !self.show_controls;
                self.is_paused = self.show_controls; // Pause when showing controls
                log::info!("Controls button clicked, show_controls: {}", self.show_controls);
            } else if self.show_controls {
                // Only check "click outside to close" if we didn't click the button
                let popup_x = screen_w / 2.0 - 200.0;
                let popup_y = screen_height() / 2.0 - 250.0;
                let popup_w = 400.0;
                let popup_h = 500.0;

                // Check if click is inside the popup
                let clicked_inside = mouse_pos.0 >= popup_x && mouse_pos.0 <= popup_x + popup_w &&
                                     mouse_pos.1 >= popup_y && mouse_pos.1 <= popup_y + popup_h;

                // Close if clicking outside the popup
                if !clicked_inside {
                    self.show_controls = false;
                    self.is_paused = false;
                    log::info!("Clicked outside popup, closing controls");
                }
            }
        }

        // Toggle pause (only if controls not showing)
        if is_key_pressed(KeyCode::P) && !self.show_controls {
            self.is_paused = !self.is_paused;
        }

        // Quick save
        if is_key_pressed(KeyCode::F5) {
            if let Err(e) = self.save_game("quicksave") {
                log::error!("Failed to quick save: {}", e);
            }
        }

        // Mouse wheel zoom (reduced delta for finer control)
        let mouse_wheel = mouse_wheel().1;
        if mouse_wheel != 0.0 {
            self.camera.adjust_zoom(-mouse_wheel * 0.02);
        }

        // Keyboard zoom controls (Q = zoom out, E = zoom in)
        if is_key_down(KeyCode::Q) {
            self.camera.adjust_zoom(-0.02); // Gradual zoom out
        }
        if is_key_down(KeyCode::E) {
            self.camera.adjust_zoom(0.02); // Gradual zoom in
        }

        SinglePlayerResult::Continue
    }

    /// Update game state
    pub fn update(&mut self, delta_time: f32) {
        if self.is_paused {
            return;
        }

        self.game_time += delta_time;

        // Handle input for active rocket
        self.update_rocket_input();

        // Update world (physics, entities)
        self.world.update(delta_time);

        // Update camera to follow active rocket
        if let Some(rocket) = self.world.get_active_rocket() {
            self.camera.follow(rocket.position());
        }

        self.camera.update(delta_time);

        // Auto-save
        if self.game_time - self.last_auto_save > self.auto_save_interval {
            if let Err(e) = self.save_game("autosave") {
                log::error!("Auto-save failed: {}", e);
            }
            self.last_auto_save = self.game_time;
        }
    }

    /// Update rocket based on keyboard input
    fn update_rocket_input(&mut self) {
        // Get input state
        let mut thrust_level = 0.0;
        let mut rotation_delta = 0.0;

        // Thrust controls
        if is_key_down(KeyCode::Space) {
            thrust_level = 1.0;
        }

        // Rotation controls
        if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            rotation_delta = 3.0; // degrees per frame
        }
        if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            rotation_delta = -3.0;
        }

        // Convert degrees to radians
        let rotation_radians = rotation_delta * std::f32::consts::PI / 180.0;

        // Apply to active rocket
        if let Some(rocket) = self.world.get_active_rocket_mut() {
            rocket.set_thrust_level(thrust_level);
            if rotation_delta != 0.0 {
                rocket.rotate(rotation_radians);
            }
        }

        // Launch new rocket (L key)
        if is_key_pressed(KeyCode::L) {
            self.launch_new_rocket();
        }

        // Convert to satellite (T key)
        if is_key_pressed(KeyCode::T) {
            if let Some(rocket_id) = self.world.active_rocket_id() {
                if self.world.convert_rocket_to_satellite(rocket_id).is_some() {
                    log::info!("Rocket converted to satellite");
                }
            }
        }
    }

    /// Launch a new rocket from the active rocket's position
    fn launch_new_rocket(&mut self) {
        if let Some(current_rocket) = self.world.get_active_rocket() {
            let new_rocket = Rocket::new(
                current_rocket.position(),
                current_rocket.velocity(),
                Color::from_rgba(200, 200, 255, 255),
                GameConstants::ROCKET_BASE_MASS,
            );

            let new_id = self.world.add_rocket(new_rocket);
            self.world.set_active_rocket(Some(new_id));

            log::info!("New rocket launched");
        }
    }

    /// Render the game
    pub fn render(&mut self) {
        // Set camera view
        set_camera(self.camera.camera());

        // Render world
        self.world.render();

        // Draw trajectory prediction for active rocket
        if let Some(rocket) = self.world.get_active_rocket() {
            // Get all planets for gravity calculation
            let planets: Vec<&Planet> = self.world.planets().collect();

            // Predict trajectory (0.5 second steps, 200 steps = 100 seconds)
            let (trajectory_points, self_intersects) = self.trajectory_predictor.predict_trajectory(
                rocket,
                &planets,
                0.5,
                200,
                true,
            );

            // Draw trajectory with color based on whether it self-intersects (completes orbit)
            let trajectory_color = if self_intersects {
                Color::new(0.0, 1.0, 0.0, 0.7) // Green if orbit closes
            } else {
                Color::new(1.0, 1.0, 0.0, 0.7) // Yellow if orbit is open
            };

            self.trajectory_predictor.draw_trajectory(&trajectory_points, trajectory_color, self_intersects);
        }

        // Reset to default camera for HUD
        set_default_camera();

        // Render HUD
        if let Some(rocket) = self.world.get_active_rocket() {
            self.hud.draw_rocket_stats(rocket);
        } else {
            self.hud.draw_message("No active rocket");
        }

        // Draw pause indicator if paused (but not if showing controls)
        if self.is_paused && !self.show_controls {
            let screen_width = screen_width();
            let screen_height = screen_height();
            let pause_text = "PAUSED";
            let font_size = 48.0;
            let text_dims = measure_text(pause_text, None, font_size as u16, 1.0);
            draw_text(
                pause_text,
                screen_width / 2.0 - text_dims.width / 2.0,
                screen_height / 2.0,
                font_size,
                WHITE,
            );
        }

        // Draw controls button in top-right corner
        let screen_w = screen_width();
        let button_x = screen_w - 50.0;
        let button_y = 10.0;
        let button_w = 40.0;
        let button_h = 30.0;

        // Button background
        draw_rectangle(button_x, button_y, button_w, button_h, Color::new(0.2, 0.2, 0.2, 0.8));
        // Button border
        draw_rectangle_lines(button_x, button_y, button_w, button_h, 2.0, WHITE);
        // Button text "..."
        let button_text = "...";
        let button_text_dims = measure_text(button_text, None, 20, 1.0);
        draw_text(
            button_text,
            button_x + button_w / 2.0 - button_text_dims.width / 2.0,
            button_y + button_h / 2.0 + button_text_dims.height / 2.0,
            20.0,
            WHITE,
        );

        // Draw controls popup if showing
        if self.show_controls {
            let screen_h = screen_height();
            let popup_w = 400.0;
            let popup_h = 500.0;
            let popup_x = screen_w / 2.0 - popup_w / 2.0;
            let popup_y = screen_h / 2.0 - popup_h / 2.0;

            // Semi-transparent overlay
            draw_rectangle(0.0, 0.0, screen_w, screen_h, Color::new(0.0, 0.0, 0.0, 0.5));

            // Popup background
            draw_rectangle(popup_x, popup_y, popup_w, popup_h, Color::new(0.1, 0.1, 0.1, 0.95));
            // Popup border
            draw_rectangle_lines(popup_x, popup_y, popup_w, popup_h, 3.0, WHITE);

            // Title
            let title = "CONTROLS";
            let title_size = 32.0;
            let title_dims = measure_text(title, None, title_size as u16, 1.0);
            draw_text(
                title,
                popup_x + popup_w / 2.0 - title_dims.width / 2.0,
                popup_y + 40.0,
                title_size,
                WHITE,
            );

            // Controls list
            let controls = [
                ("SPACE", "Thrust"),
                ("A / LEFT", "Rotate right"),
                ("D / RIGHT", "Rotate left"),
                ("Q", "Zoom out"),
                ("E", "Zoom in"),
                ("MOUSE WHEEL", "Zoom"),
                ("L", "Convert to satellite"),
                ("T", "Launch new rocket"),
                ("P", "Pause/Unpause"),
                ("F5", "Quick save"),
                ("ENTER", "Toggle this menu"),
                ("ESC", "Return to menu / Close this"),
            ];

            let mut y = popup_y + 80.0;
            let font_size = 18.0;
            let line_height = 35.0;

            for (key, action) in &controls {
                // Draw key
                draw_text(
                    key,
                    popup_x + 30.0,
                    y,
                    font_size,
                    Color::new(0.8, 0.8, 1.0, 1.0), // Light blue
                );

                // Draw action
                draw_text(
                    action,
                    popup_x + 200.0,
                    y,
                    font_size,
                    WHITE,
                );

                y += line_height;
            }

            // Footer text
            let footer = "Click outside or press ESC to close";
            let footer_dims = measure_text(footer, None, 14, 1.0);
            draw_text(
                footer,
                popup_x + popup_w / 2.0 - footer_dims.width / 2.0,
                popup_y + popup_h - 20.0,
                14.0,
                Color::new(0.7, 0.7, 0.7, 1.0),
            );
        }
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused
    }

    pub fn game_time(&self) -> f32 {
        self.game_time
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Tests requiring SFML resources (Font) are limited
    // Would need mock or test fixtures

    #[test]
    fn test_game_time_tracking() {
        // This test would require proper setup with Font
        // Simplified test structure shown
    }
}
