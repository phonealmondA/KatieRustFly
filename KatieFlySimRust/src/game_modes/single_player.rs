// Single Player Game - Main single player game mode
// Integrates all systems for playable game

use macroquad::prelude::*;

use crate::entities::{GameObject, Planet, Rocket};
use crate::game_constants::GameConstants;
use crate::save_system::{GameSaveData, SavedCamera};
use crate::systems::{World, VehicleManager};
use crate::ui::{Camera, GameInfoDisplay};

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
    info_display: GameInfoDisplay,
    vehicle_manager: VehicleManager,
    game_time: f32,
    is_paused: bool,
    show_controls: bool,

    // Input state
    selected_thrust_level: f32, // 0.0 to 1.0 (0% to 100%)
    rotation_input: f32,

    // Save/load
    current_save_name: Option<String>,
    last_auto_save: f32,
    auto_save_interval: f32,

    // Starting position for new rockets
    rocket_spawn_position: Vec2,
    rocket_spawn_velocity: Vec2,
}

impl SinglePlayerGame {
    pub fn new(window_size: Vec2) -> Self {
        let mut info_display = GameInfoDisplay::new();
        // Hide controls panel since we use the popup system instead (Enter key / "..." button)
        info_display.toggle_controls_panel();

        SinglePlayerGame {
            world: World::new(),
            camera: Camera::new(window_size),
            info_display,
            vehicle_manager: VehicleManager::new(),
            game_time: 0.0,
            is_paused: false,
            show_controls: false,
            selected_thrust_level: 0.0, // Start at 0% thrust
            rotation_input: 0.0,
            current_save_name: None,
            last_auto_save: 0.0,
            auto_save_interval: 60.0, // Auto-save every 60 seconds
            rocket_spawn_position: Vec2::ZERO,
            rocket_spawn_velocity: Vec2::ZERO,
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
        log::info!("Main planet: pos=({}, {}), radius={}, mass={}",
            GameConstants::MAIN_PLANET_X, GameConstants::MAIN_PLANET_Y,
            GameConstants::MAIN_PLANET_RADIUS, GameConstants::MAIN_PLANET_MASS);
        self.world.add_planet(main_planet);

        // Create secondary planet (like Moon)
        let moon_x = *crate::game_constants::SECONDARY_PLANET_X;
        let moon_y = *crate::game_constants::SECONDARY_PLANET_Y;
        let moon_radius = GameConstants::SECONDARY_PLANET_RADIUS;
        let moon_velocity = *crate::game_constants::SECONDARY_PLANET_ORBITAL_VELOCITY;

        log::info!("Moon: pos=({}, {}), radius={}, mass={}, velocity={}",
            moon_x, moon_y, moon_radius, GameConstants::SECONDARY_PLANET_MASS, moon_velocity);
        log::info!("Distance from main planet: {}",
            ((moon_x - GameConstants::MAIN_PLANET_X).powi(2) + (moon_y - GameConstants::MAIN_PLANET_Y).powi(2)).sqrt());

        let mut secondary_planet = Planet::new(
            Vec2::new(moon_x, moon_y),
            moon_radius,
            GameConstants::SECONDARY_PLANET_MASS,
            Color::from_rgba(150, 150, 150, 255),
        );

        // Set orbital velocity for secondary planet
        secondary_planet.set_velocity(Vec2::new(
            0.0,
            -moon_velocity,
        ));

        self.world.add_planet(secondary_planet);

        // Create starting rocket near main planet
        let rocket_spawn_distance = GameConstants::MAIN_PLANET_RADIUS + 200.0;
        self.rocket_spawn_position = Vec2::new(
            GameConstants::MAIN_PLANET_X + rocket_spawn_distance,
            GameConstants::MAIN_PLANET_Y,
        );
        self.rocket_spawn_velocity = Vec2::new(0.0, 0.0);

        let rocket = Rocket::new(
            self.rocket_spawn_position,
            self.rocket_spawn_velocity,
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

        // Panel visibility toggles (keys 1-5)
        if is_key_pressed(KeyCode::Key1) {
            self.info_display.toggle_rocket_panel();
            log::info!("Toggled rocket panel");
        }
        if is_key_pressed(KeyCode::Key2) {
            self.info_display.toggle_planet_panel();
            log::info!("Toggled planet panel");
        }
        if is_key_pressed(KeyCode::Key3) {
            self.info_display.toggle_orbit_panel();
            log::info!("Toggled orbit panel");
        }
        if is_key_pressed(KeyCode::Key4) {
            self.info_display.toggle_controls_panel();
            log::info!("Toggled controls panel");
        }
        if is_key_pressed(KeyCode::Key5) {
            self.info_display.toggle_network_panel();
            log::info!("Toggled network panel");
        }
        // Key 0 to toggle all panels
        if is_key_pressed(KeyCode::Key0) {
            self.info_display.show_all_panels();
            log::info!("Showed all panels");
        }
        if is_key_pressed(KeyCode::Key9) {
            self.info_display.hide_all_panels();
            log::info!("Hid all panels");
        }

        // Visualization toggles
        if is_key_pressed(KeyCode::T) {
            self.vehicle_manager.toggle_trajectory();
            log::info!("Toggled trajectory visualization: {}", self.vehicle_manager.visualization().show_trajectory);
        }
        if is_key_pressed(KeyCode::G) {
            self.vehicle_manager.toggle_gravity_forces();
            log::info!("Toggled gravity force visualization: {}", self.vehicle_manager.visualization().show_gravity_forces);
        }

        // Mouse wheel zoom (reduced delta for finer control)
        let mouse_wheel = mouse_wheel().1;
        if mouse_wheel != 0.0 {
            self.camera.adjust_zoom(-mouse_wheel * 0.02);
        }

        // Keyboard zoom controls (E = zoom out, Q = zoom in)
        // Note: zoom_scale = 1/zoom_level, so larger zoom_level = more zoomed out
        if is_key_down(KeyCode::Q) {
            self.camera.adjust_zoom(-0.02); // Gradual zoom in (decrease zoom_level)
        }
        if is_key_down(KeyCode::E) {
            self.camera.adjust_zoom(0.02); // Gradual zoom out (increase zoom_level)
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
        // Thrust level adjustment (comma to decrease, period to increase)
        if is_key_pressed(KeyCode::Comma) {
            self.selected_thrust_level = (self.selected_thrust_level - 0.05).max(0.0);
            log::info!("Thrust level decreased to {}%", (self.selected_thrust_level * 100.0) as i32);
        }
        if is_key_pressed(KeyCode::Period) {
            self.selected_thrust_level = (self.selected_thrust_level + 0.05).min(1.0);
            log::info!("Thrust level increased to {}%", (self.selected_thrust_level * 100.0) as i32);
        }

        // Get input state
        let mut thrust_level = 0.0;
        let mut rotation_delta = 0.0;

        // Thrust controls - space bar applies the selected thrust level
        if is_key_down(KeyCode::Space) {
            thrust_level = self.selected_thrust_level;
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

        // Convert to satellite (C key)
        if is_key_pressed(KeyCode::C) {
            if let Some(rocket_id) = self.world.active_rocket_id() {
                // Convert rocket to satellite
                if self.world.convert_rocket_to_satellite(rocket_id).is_some() {
                    log::info!("Rocket converted to satellite");

                    // Spawn new rocket at the starting position
                    let new_rocket = Rocket::new(
                        self.rocket_spawn_position,
                        self.rocket_spawn_velocity,
                        WHITE,
                        GameConstants::ROCKET_BASE_MASS,
                    );

                    let new_id = self.world.add_rocket(new_rocket);
                    self.world.set_active_rocket(Some(new_id));
                    log::info!("New rocket spawned at starting position");
                }
            }
        }
    }

    /// Render the game
    pub fn render(&mut self) {
        // Set camera view
        set_camera(self.camera.camera());

        // Render world
        self.world.render();

        // Get all planets for trajectory calculations
        let all_planets: Vec<&Planet> = self.world.planets().collect();

        // Get current zoom level for scaling trajectory line thickness
        let zoom_level = self.camera.zoom_level();

        // Draw vehicle visualizations (trajectory, gravity forces) using VehicleManager
        if let Some(rocket) = self.world.get_active_rocket() {
            self.vehicle_manager.draw_visualizations(rocket, &all_planets, zoom_level);
        }

        // Reset to default camera for HUD
        set_default_camera();

        // Update and render GameInfoDisplay
        let all_planets: Vec<&Planet> = self.world.planets().collect();
        let active_rocket = self.world.get_active_rocket();

        // Get satellite network statistics
        let satellite_stats = if self.world.satellite_count() > 0 {
            Some(self.world.get_satellite_network_stats())
        } else {
            None
        };

        self.info_display.update_all_panels(
            active_rocket,
            &all_planets,
            self.selected_thrust_level,
            false,          // network_connected (not used in single player)
            None,           // player_id (not used in single player)
            1,              // player_count (always 1 in single player)
            satellite_stats.as_ref(),
        );

        self.info_display.draw_all_panels();

        // Draw visualization HUD (shows T and G key status)
        self.vehicle_manager.draw_visualization_hud();

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
            let popup_w = 800.0;  // Wider for two columns
            let popup_h = 600.0;  // Taller to fit more controls
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

            // Controls list - Split into two columns
            let controls_left = [
                ("COMMA", "Decrease thrust -5%"),
                ("PERIOD", "Increase thrust +5%"),
                ("SPACE", "Apply thrust"),
                ("A / LEFT", "Rotate left"),
                ("D / RIGHT", "Rotate right"),
                ("Q", "Zoom in"),
                ("E", "Zoom out"),
                ("MOUSE WHEEL", "Zoom"),
                ("C", "Convert to satellite"),
                ("P", "Pause/Unpause"),
            ];

            let controls_right = [
                ("T", "Toggle trajectory"),
                ("G", "Toggle gravity forces"),
                ("1", "Toggle rocket panel"),
                ("2", "Toggle planet panel"),
                ("3", "Toggle orbit panel"),
                ("4", "Toggle controls panel"),
                ("5", "Toggle network panel"),
                ("9", "Hide all panels"),
                ("0", "Show all panels"),
                ("F5", "Quick save"),
                ("ENTER", "Toggle this menu"),
                ("ESC", "Return to menu"),
            ];

            let font_size = 17.0;
            let line_height = 32.0;
            let col_spacing = popup_w / 2.0;

            // Draw left column
            let mut y = popup_y + 85.0;
            for (key, action) in &controls_left {
                draw_text(
                    key,
                    popup_x + 30.0,
                    y,
                    font_size,
                    Color::new(0.8, 0.8, 1.0, 1.0), // Light blue
                );
                draw_text(
                    action,
                    popup_x + 160.0,
                    y,
                    font_size,
                    WHITE,
                );
                y += line_height;
            }

            // Draw right column
            y = popup_y + 85.0;
            for (key, action) in &controls_right {
                draw_text(
                    key,
                    popup_x + col_spacing + 30.0,
                    y,
                    font_size,
                    Color::new(0.8, 0.8, 1.0, 1.0), // Light blue
                );
                draw_text(
                    action,
                    popup_x + col_spacing + 160.0,
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
