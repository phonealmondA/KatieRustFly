// Single Player Game - Main single player game mode
// Integrates all systems for playable game

use macroquad::prelude::*;
use std::collections::HashSet;

use crate::entities::{GameObject, Planet, Rocket};
use crate::game_constants::GameConstants;
use crate::save_system::{GameSaveData, SavedCamera, SavedPlanet, SavedRocket, SavedSatellite};
use crate::systems::{World, VehicleManager, EntityId};
use crate::ui::{Camera, GameInfoDisplay};
use crate::utils::vector_helper;

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

    // Network map view
    show_network_map: bool,
    marked_satellites: HashSet<EntityId>,

    // Save celebration (F5 quick save)
    save_celebration_timer: f32,  // Time remaining for "what a save!!" text
}

impl SinglePlayerGame {
    pub fn new(window_size: Vec2) -> Self {
        let info_display = GameInfoDisplay::new();

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
            show_network_map: false,
            marked_satellites: HashSet::new(),
            save_celebration_timer: 0.0,
        }
    }

    /// Initialize a new game with default setup
    pub fn initialize_new_game(&mut self) {
        self.world.clear_all();
        self.game_time = 0.0;

        // Create main planet (like Earth)
        let mut main_planet = Planet::new(
            Vec2::new(GameConstants::MAIN_PLANET_X, GameConstants::MAIN_PLANET_Y),
            GameConstants::MAIN_PLANET_RADIUS,
            GameConstants::MAIN_PLANET_MASS,
            BLUE,
        );
        // Calculate radius from mass to ensure consistency with mass-depletion system
        main_planet.update_radius_from_mass();
        log::info!("Main planet: pos=({}, {}), radius={}, mass={}",
            GameConstants::MAIN_PLANET_X, GameConstants::MAIN_PLANET_Y,
            main_planet.radius(), GameConstants::MAIN_PLANET_MASS);
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

        // Calculate radius from mass to ensure consistency with mass-depletion system
        secondary_planet.update_radius_from_mass();

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

    /// Load game from save data (used by saves menu)
    pub fn load_from_save(&mut self, save_data: GameSaveData, save_name: String) {
        self.load_from_snapshot(save_data);
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

    /// Load game state from save file
    pub fn load_game(&mut self, save_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let save_data = GameSaveData::load_from_file(save_name)?;
        self.load_from_snapshot(save_data);
        self.current_save_name = Some(save_name.to_string());
        log::info!("Game loaded: {}", save_name);
        Ok(())
    }

    /// Load game state from a snapshot (works for both save files and network packets)
    fn load_from_snapshot(&mut self, snapshot: GameSaveData) {
        // Clear existing world
        self.world.clear_all_entities();

        // Restore game time
        self.game_time = snapshot.game_time;

        // Save counts for logging before consuming vectors
        let planet_count = snapshot.planets.len();
        let rocket_count = snapshot.rockets.len();
        let satellite_count = snapshot.satellites.len();

        // Restore all planets with their original IDs
        for saved_planet in snapshot.planets {
            let (id, planet) = saved_planet.to_planet();
            self.world.add_planet_with_id(id, planet);
        }

        // Restore all rockets with their original IDs
        for saved_rocket in snapshot.rockets {
            let (id, rocket) = saved_rocket.to_rocket();
            self.world.add_rocket_with_id(id, rocket);
        }

        // Restore all satellites with their original IDs
        for saved_satellite in snapshot.satellites {
            let (id, satellite) = saved_satellite.to_satellite();
            self.world.add_satellite_with_id(id, satellite);
        }

        // Restore active rocket
        self.world.set_active_rocket(snapshot.active_rocket_id);

        // Restore camera
        self.camera.set_center(snapshot.camera.center.into());
        self.camera.set_target_zoom(snapshot.camera.zoom);

        log::info!(
            "Loaded snapshot: {} planets, {} rockets, {} satellites at time {:.1}s",
            planet_count,
            rocket_count,
            satellite_count,
            snapshot.game_time
        );
    }

    /// Create complete snapshot from current game state
    /// This snapshot can be used for both save files and network packets (multiplayer)
    fn create_save_data(&self) -> GameSaveData {
        let mut save_data = GameSaveData::new();
        save_data.game_time = self.game_time;

        // Save all planets with their IDs
        save_data.planets = self.world.planets_with_ids()
            .map(|(id, planet)| SavedPlanet::from_planet(id, planet))
            .collect();

        // Save all rockets with their IDs
        save_data.rockets = self.world.rockets_with_ids()
            .map(|(id, rocket)| SavedRocket::from_rocket(id, rocket))
            .collect();

        // Save all satellites with their IDs
        save_data.satellites = self.world.satellites_with_ids()
            .map(|(id, satellite)| SavedSatellite::from_satellite(id, satellite))
            .collect();

        // Save player state
        save_data.player_id = None;  // Single player
        save_data.active_rocket_id = self.world.active_rocket_id();

        // Save camera state
        save_data.camera = SavedCamera {
            center: self.camera.camera().target.into(),
            zoom: self.camera.zoom_level(),
        };

        log::info!(
            "Created snapshot: {} planets, {} rockets, {} satellites",
            save_data.planets.len(),
            save_data.rockets.len(),
            save_data.satellites.len()
        );

        save_data
    }

    /// Quick save triggered by F5 key - saves and shows "what a save!!" celebration
    fn quick_save(&mut self) {
        match self.save_game("quicksave") {
            Ok(_) => {
                log::info!("Quick save successful");
                self.current_save_name = Some("quicksave".to_string());

                // Trigger save celebration
                self.save_celebration_timer = 5.0; // Show for 5 seconds
            }
            Err(e) => {
                log::error!("Failed to quick save: {}", e);
            }
        }
    }

    /// Handle input for game controls
    pub fn handle_input(&mut self) -> SinglePlayerResult {
        // Check for escape to return to menu or close popups
        if is_key_pressed(KeyCode::Escape) {
            if self.show_controls {
                self.show_controls = false;
                self.is_paused = false;
            } else if self.show_network_map {
                self.show_network_map = false;
            } else {
                return SinglePlayerResult::ReturnToMenu;
            }
        }

        // Toggle controls menu with Enter key
        if is_key_pressed(KeyCode::Enter) {
            self.show_controls = !self.show_controls;
            self.is_paused = self.show_controls; // Pause when showing controls
        }

        // Handle mouse click for controls button, popup, and network map
        if is_mouse_button_pressed(MouseButton::Left) {
            let mouse_pos = mouse_position();
            let screen_w = screen_width();
            let screen_h = screen_height();

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
                let popup_y = screen_h / 2.0 - 250.0;
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
            } else if self.show_network_map {
                // Handle clicks on satellites in the network map
                let map_size = 700.0;
                let map_x = screen_w / 2.0 - map_size / 2.0;
                let map_y = screen_h / 2.0 - map_size / 2.0;

                // Check if click is in satellite list
                let list_x = map_x + map_size + 20.0;
                let list_y = map_y + 60.0;
                let list_width = 200.0;

                if mouse_pos.0 >= list_x && mouse_pos.0 <= list_x + list_width {
                    // Calculate which satellite was clicked based on Y position
                    let y_offset = mouse_pos.1 - (list_y + 40.0);
                    let sat_index = (y_offset / 20.0).floor() as usize;

                    // Get satellite at this index
                    let satellites: Vec<_> = self.world.satellites_with_ids().collect();
                    if sat_index < satellites.len() {
                        let (sat_id, _) = satellites[sat_index];

                        // Toggle marked status
                        if self.marked_satellites.contains(&sat_id) {
                            self.marked_satellites.remove(&sat_id);
                            log::info!("Unmarked satellite {}", sat_id);
                        } else {
                            self.marked_satellites.insert(sat_id);
                            log::info!("Marked satellite {}", sat_id);
                        }
                    }
                }

                // Check if click is on a satellite in the map itself
                let mut earth_pos = Vec2::new(GameConstants::MAIN_PLANET_X, GameConstants::MAIN_PLANET_Y);
                let mut earth_mass = 0.0f32;
                for planet in self.world.planets() {
                    if planet.mass() > earth_mass {
                        earth_mass = planet.mass();
                        earth_pos = planet.position();
                    }
                }

                let map_world_radius = 50000.0;
                let map_center = Vec2::new(map_x + map_size / 2.0, map_y + map_size / 2.0);
                let map_scale = (map_size * 0.45) / map_world_radius;

                let world_to_map = |world_pos: Vec2| -> Vec2 {
                    let relative = world_pos - earth_pos;
                    let scaled = relative * map_scale;
                    // Flip Y coordinate to fix inverted display
                    Vec2::new(map_center.x + scaled.x, map_center.y - scaled.y)
                };

                // Check each satellite
                let satellites: Vec<_> = self.world.satellites_with_ids().collect();
                for (sat_id, satellite) in satellites {
                    let map_pos = world_to_map(satellite.position());
                    let click_distance = ((mouse_pos.0 - map_pos.x).powi(2) + (mouse_pos.1 - map_pos.y).powi(2)).sqrt();

                    if click_distance < 10.0 { // Click radius
                        // Toggle marked status
                        if self.marked_satellites.contains(&sat_id) {
                            self.marked_satellites.remove(&sat_id);
                            log::info!("Unmarked satellite {}", sat_id);
                        } else {
                            self.marked_satellites.insert(sat_id);
                            log::info!("Marked satellite {}", sat_id);
                        }
                        break;
                    }
                }

                // Check if click is outside map to close
                if mouse_pos.0 < map_x || mouse_pos.0 > map_x + map_size + list_width + 20.0 ||
                   mouse_pos.1 < map_y || mouse_pos.1 > map_y + map_size {
                    self.show_network_map = false;
                    log::info!("Clicked outside network map, closing");
                }
            }
        }

        // Toggle pause (only if controls not showing)
        if is_key_pressed(KeyCode::P) && !self.show_controls {
            self.is_paused = !self.is_paused;
        }

        // Quick save (F5 key) - saves and shows "what a save!!" celebration
        if is_key_pressed(KeyCode::F5) {
            self.quick_save();
        }

        // Panel visibility toggles (keys 1-3, 5)
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
        // Key 4 removed - controls panel deleted
        if is_key_pressed(KeyCode::Key5) {
            self.show_network_map = !self.show_network_map;
            log::info!("Toggled network map: {}", self.show_network_map);
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
        if is_key_pressed(KeyCode::Tab) {
            self.vehicle_manager.toggle_reference_body();
            log::info!("Toggled reference body: {:?}", self.vehicle_manager.visualization().reference_body);
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

        // Handle rockets destroyed by bullets (respawn like 'C' key, but without satellite)
        let destroyed_rockets = self.world.take_destroyed_rockets();
        for destroyed in destroyed_rockets {
            log::info!("Rocket destroyed by bullet, respawning");

            // Spawn new rocket at the starting position
            let new_rocket = Rocket::new(
                self.rocket_spawn_position,
                self.rocket_spawn_velocity,
                WHITE,
                GameConstants::ROCKET_BASE_MASS,
            );

            let new_rocket_id = self.world.add_rocket(new_rocket);
            self.world.set_active_rocket(Some(new_rocket_id));
            log::info!("New rocket {} spawned at starting position", new_rocket_id);
        }

        // Update save celebration timer
        if self.save_celebration_timer > 0.0 {
            self.save_celebration_timer -= delta_time;
        }

        // Handle manual planet refueling (R key)
        if let Some(rocket_id) = self.world.active_rocket_id() {
            if is_key_down(KeyCode::R) {
                self.world.handle_manual_planet_refuel(rocket_id, delta_time);
            }
        }

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

        // Shoot bullet (W key, same as multiplayer)
        if is_key_pressed(KeyCode::W) {
            if let Some(rocket_id) = self.world.active_rocket_id() {
                if let Some(bullet_id) = self.world.shoot_bullet_from_rocket(rocket_id) {
                    log::info!("Bullet {} fired from rocket {}", bullet_id, rocket_id);
                } else {
                    log::info!("Cannot shoot: not enough fuel (need 1 unit)");
                }
            }
        }
    }

    /// Draw network map popup
    fn draw_network_map(&mut self) {
        let screen_w = screen_width();
        let screen_h = screen_height();
        let map_size = 700.0;
        let map_x = screen_w / 2.0 - map_size / 2.0;
        let map_y = screen_h / 2.0 - map_size / 2.0;

        // Semi-transparent overlay
        draw_rectangle(0.0, 0.0, screen_w, screen_h, Color::new(0.0, 0.0, 0.0, 0.6));

        // Map background (green-tinted)
        draw_rectangle(map_x, map_y, map_size, map_size, Color::new(0.05, 0.15, 0.05, 0.95));
        // Map border
        draw_rectangle_lines(map_x, map_y, map_size, map_size, 3.0, Color::new(0.0, 1.0, 0.0, 0.8));

        // Title
        let title = "SATELLITE NETWORK MAP";
        let title_size = 24.0;
        let title_dims = measure_text(title, None, title_size as u16, 1.0);
        draw_text(
            title,
            map_x + map_size / 2.0 - title_dims.width / 2.0,
            map_y + 30.0,
            title_size,
            Color::new(0.0, 1.0, 0.0, 1.0),
        );

        // Calculate map scale - center on Earth
        // Find Earth (the most massive planet)
        let mut earth_pos = Vec2::new(GameConstants::MAIN_PLANET_X, GameConstants::MAIN_PLANET_Y);
        let mut earth_mass = 0.0f32;
        for planet in self.world.planets() {
            if planet.mass() > earth_mass {
                earth_mass = planet.mass();
                earth_pos = planet.position();
            }
        }

        // Map viewport (what area of the game world to show)
        // Show from 0 to ~50000 units from center
        let map_world_radius = 50000.0;
        let map_center = Vec2::new(map_x + map_size / 2.0, map_y + map_size / 2.0);
        let map_scale = (map_size * 0.45) / map_world_radius;

        // Helper function to convert world position to map position
        let world_to_map = |world_pos: Vec2| -> Vec2 {
            let relative = world_pos - earth_pos;
            let scaled = relative * map_scale;
            // Flip Y coordinate to fix inverted display
            Vec2::new(map_center.x + scaled.x, map_center.y - scaled.y)
        };

        // Draw range rings at 1500 unit intervals from Earth's surface
        let earth_radius = 10000.0; // From constants
        for i in 1..=40 {
            let ring_distance = earth_radius + (i as f32 * 1500.0);
            let ring_radius_on_map = ring_distance * map_scale;

            if ring_radius_on_map < map_size / 2.0 {
                draw_circle_lines(
                    map_center.x,
                    map_center.y,
                    ring_radius_on_map,
                    1.0,
                    Color::new(0.0, 0.5, 0.0, 0.3),
                );
            }
        }

        // Find Moon position for Moon-centered rings
        let mut moon_pos = None;
        for planet in self.world.planets() {
            if planet.mass() != earth_mass {
                // This is the Moon (not Earth)
                moon_pos = Some(planet.position());
                break;
            }
        }

        // Draw Moon-centered rings (7 rings at 1500 unit intervals)
        if let Some(moon_world_pos) = moon_pos {
            let moon_map_pos = world_to_map(moon_world_pos);
            let moon_radius = 1737.0; // Moon's radius from constants

            for i in 1..=7 {
                let ring_distance = moon_radius + (i as f32 * 1500.0);
                let ring_radius_on_map = ring_distance * map_scale;

                // Draw rings that move with the Moon
                draw_circle_lines(
                    moon_map_pos.x,
                    moon_map_pos.y,
                    ring_radius_on_map,
                    1.0,
                    Color::new(0.4, 0.4, 0.6, 0.4), // Slightly blue-tinted for Moon
                );
            }
        }

        // Draw planets
        for planet in self.world.planets() {
            let map_pos = world_to_map(planet.position());
            let planet_radius = planet.radius() * map_scale;
            let planet_radius_clamped = planet_radius.max(8.0);

            draw_circle(map_pos.x, map_pos.y, planet_radius_clamped, planet.color());
            draw_circle_lines(map_pos.x, map_pos.y, planet_radius_clamped, 2.0, WHITE);

            // Label
            let label = if planet.mass() == earth_mass { "Earth" } else { "Moon" };
            draw_text(label, map_pos.x - 15.0, map_pos.y - planet_radius_clamped - 5.0, 14.0, WHITE);
        }

        // Draw player rocket
        if let Some(rocket) = self.world.get_active_rocket() {
            let map_pos = world_to_map(rocket.position());
            let rocket_size = 6.0;

            // Bright player color dot
            draw_circle(map_pos.x, map_pos.y, rocket_size, Color::new(1.0, 1.0, 1.0, 1.0));
            draw_circle_lines(map_pos.x, map_pos.y, rocket_size, 2.0, Color::new(0.0, 1.0, 0.0, 1.0));

            // Label
            draw_text("YOU", map_pos.x - 12.0, map_pos.y - 10.0, 12.0, WHITE);
        }

        // Draw connection lines between satellites in range
        let satellite_transfer_range = GameConstants::SATELLITE_TRANSFER_RANGE;
        let satellites: Vec<_> = self.world.satellites_with_ids().collect();

        for i in 0..satellites.len() {
            for j in (i + 1)..satellites.len() {
                let (id1, sat1) = satellites[i];
                let (id2, sat2) = satellites[j];

                let distance = vector_helper::distance(sat1.position(), sat2.position());

                if distance <= satellite_transfer_range {
                    let map_pos1 = world_to_map(sat1.position());
                    let map_pos2 = world_to_map(sat2.position());

                    // Green connection line
                    draw_line(
                        map_pos1.x,
                        map_pos1.y,
                        map_pos2.x,
                        map_pos2.y,
                        2.0,
                        Color::new(0.0, 1.0, 0.0, 0.4),
                    );
                }
            }
        }

        // Draw satellites
        let satellites: Vec<_> = self.world.satellites_with_ids().collect();
        for (sat_id, satellite) in &satellites {
            let map_pos = world_to_map(satellite.position());
            let is_marked = self.marked_satellites.contains(sat_id);

            // Satellite dot
            let sat_color = if is_marked {
                Color::new(1.0, 1.0, 0.0, 1.0) // Yellow for marked
            } else {
                satellite.status_color()
            };

            let sat_size = if is_marked { 5.0 } else { 4.0 };

            draw_circle(map_pos.x, map_pos.y, sat_size, sat_color);
            draw_circle_lines(map_pos.x, map_pos.y, sat_size, 1.0, WHITE);

            // Satellite ID label
            let id_text = format!("{}", sat_id);
            draw_text(&id_text, map_pos.x + 7.0, map_pos.y + 4.0, 12.0, WHITE);
        }

        // Draw bullet trajectories (red lines showing curved path, 3x longer than default)
        let bullets: Vec<_> = self.world.bullets_with_ids().collect();
        for (_bullet_id, bullet) in &bullets {
            let bullet_pos = bullet.position();
            let bullet_vel = bullet.velocity();

            // Predict bullet trajectory accounting for moving planets (especially Moon)
            // Use 600 steps (6x normal) to show longer trajectory in map
            let prediction_steps = 600;
            let dt = 0.1; // Time step for prediction
            let mut predicted_positions = Vec::new();
            let mut current_pos = bullet_pos;
            let mut current_vel = bullet_vel;

            // Create mutable copies of planet states (position, velocity, mass, radius)
            // This allows us to simulate their motion during trajectory prediction
            let mut planet_states: Vec<(Vec2, Vec2, f32, f32)> = self.world.planets()
                .map(|p| (p.position(), p.velocity(), p.mass(), p.radius()))
                .collect();

            // Identify Earth (largest planet) for pinning - it doesn't move
            let earth_index = planet_states
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.3.partial_cmp(&b.3).unwrap())
                .map(|(i, _)| i)
                .unwrap_or(0);

            for _ in 0..prediction_steps {
                predicted_positions.push(current_pos);

                // Step 1: Apply planet-to-planet gravity (e.g., Moon orbiting Earth)
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
                            let distance = direction.length();

                            if distance > radius_i {
                                // Use same gravity constant as actual physics (GameConstants::G = 100.0)
                                let force_magnitude = (GameConstants::G * mass_i * mass_j) / (distance * distance);
                                let force_direction = direction / distance;
                                planet_acceleration += force_direction * (force_magnitude / mass_i);
                            }
                        }

                        // Update planet velocity
                        planet_states[i].1 += planet_acceleration * dt;
                    }
                }

                // Step 2: Update planet positions based on their velocities
                for i in 0..planet_states.len() {
                    if i != earth_index {
                        let vel = planet_states[i].1;
                        planet_states[i].0 += vel * dt;
                    }
                }

                // Step 3: Apply gravity from updated planet positions to bullet
                // Use same gravity constant as actual physics
                let mut total_accel = Vec2::ZERO;
                for &(planet_pos, _, planet_mass, _) in &planet_states {
                    let diff = planet_pos - current_pos;
                    let distance = diff.length();
                    if distance > 0.0 {
                        let direction = diff / distance;
                        let bullet_mass = 1.0; // Bullet mass
                        let force_magnitude = (GameConstants::G * planet_mass * bullet_mass) / (distance * distance);
                        let acceleration = direction * (force_magnitude / bullet_mass);
                        total_accel += acceleration;
                    }
                }

                // Step 4: Update bullet velocity and position
                current_vel += total_accel * dt;
                current_pos += current_vel * dt;

                // Step 5: Check for collision with planets at their PREDICTED positions
                // Only stop if bullet will actually hit planet when it arrives at this time
                let mut hit_planet = false;
                for &(planet_pos, _, _, planet_radius) in &planet_states {
                    let distance = (planet_pos - current_pos).length();
                    if distance < planet_radius + 5.0 {
                        hit_planet = true;
                        break;
                    }
                }
                if hit_planet {
                    break;
                }
            }

            // Draw red trajectory line
            for i in 0..(predicted_positions.len() - 1) {
                let pos1 = predicted_positions[i];
                let pos2 = predicted_positions[i + 1];
                let map_pos1 = world_to_map(pos1);
                let map_pos2 = world_to_map(pos2);

                draw_line(
                    map_pos1.x,
                    map_pos1.y,
                    map_pos2.x,
                    map_pos2.y,
                    2.0,
                    Color::new(1.0, 0.0, 0.0, 0.6), // Red trajectory
                );
            }

            // Draw bullet current position as small red dot
            let bullet_map_pos = world_to_map(bullet_pos);
            draw_circle(bullet_map_pos.x, bullet_map_pos.y, 3.0, Color::new(1.0, 0.0, 0.0, 1.0));
        }

        // Satellite list on the right side of the map
        let list_x = map_x + map_size + 20.0;
        let list_y = map_y + 60.0;
        let list_width = 200.0;

        // List background
        draw_rectangle(list_x, list_y - 10.0, list_width, map_size - 50.0, Color::new(0.0, 0.0, 0.0, 0.8));
        draw_rectangle_lines(list_x, list_y - 10.0, list_width, map_size - 50.0, 2.0, Color::new(0.0, 1.0, 0.0, 0.6));

        draw_text("SATELLITES", list_x + 10.0, list_y + 10.0, 16.0, Color::new(0.0, 1.0, 0.0, 1.0));

        let mut y_offset = 40.0;
        for (sat_id, satellite) in &satellites {
            let is_marked = self.marked_satellites.contains(sat_id);
            let mark_indicator = if is_marked { "[X]" } else { "[ ]" };

            let fuel_pct = satellite.fuel_percentage();
            let sat_text = format!("{} ID:{} F:{:.0}%", mark_indicator, sat_id, fuel_pct);

            let text_color = if is_marked {
                Color::new(1.0, 1.0, 0.0, 1.0)
            } else {
                WHITE
            };

            draw_text(&sat_text, list_x + 10.0, list_y + y_offset, 14.0, text_color);
            y_offset += 20.0;

            if y_offset > map_size - 100.0 {
                break; // Don't overflow the list
            }
        }

        // Instructions
        let instructions = "Click satellite ID to toggle mark | ESC to close | 5 to toggle";
        let inst_dims = measure_text(instructions, None, 12, 1.0);
        draw_text(
            instructions,
            map_x + map_size / 2.0 - inst_dims.width / 2.0,
            map_y + map_size - 15.0,
            12.0,
            Color::new(0.7, 0.7, 0.7, 1.0),
        );
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
            self.vehicle_manager.draw_visualizations(rocket, &all_planets, zoom_level, self.camera.camera());
        }

        // Draw overlay dots for marked satellites
        for sat_id in &self.marked_satellites {
            if let Some(satellite) = self.world.get_satellite(*sat_id) {
                let sat_pos = satellite.position();

                // Calculate overlay size based on satellite transfer range
                let overlay_radius = GameConstants::SATELLITE_TRANSFER_RANGE;

                // Draw bright overlay circle at satellite position
                // Use a very bright player color (white with yellow tint)
                draw_circle(
                    sat_pos.x,
                    sat_pos.y,
                    overlay_radius,
                    Color::new(1.0, 1.0, 0.0, 0.2), // Yellow, semi-transparent
                );

                // Draw outline for better visibility
                draw_circle_lines(
                    sat_pos.x,
                    sat_pos.y,
                    overlay_radius,
                    8.0 / zoom_level, // Scale line thickness with zoom
                    Color::new(1.0, 1.0, 0.0, 0.8), // Bright yellow
                );

                // Draw a smaller bright center dot
                draw_circle(
                    sat_pos.x,
                    sat_pos.y,
                    20.0,
                    Color::new(1.0, 1.0, 0.0, 1.0), // Fully opaque yellow center
                );
            }
        }

        // Store celebration rocket position for screen-space rendering
        let celebration_screen_pos = if self.save_celebration_timer > 0.0 {
            self.world.get_active_rocket()
                .map(|rocket| self.camera.world_to_screen(rocket.position()))
        } else {
            None
        };

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

        // Get selected planet for panels 2 and 3 based on reference body
        use crate::systems::ReferenceBody;
        let reference_body = self.vehicle_manager.visualization().reference_body;

        // Use direct index: planets[0] = Moon, planets[1] = Earth
        let selected_planet = match reference_body {
            ReferenceBody::Earth => all_planets.get(1).copied(),
            ReferenceBody::Moon => all_planets.get(0).copied(),
        };

        self.info_display.update_all_panels(
            active_rocket,
            &all_planets,
            selected_planet,
            reference_body,  // Pass reference body so UI knows which planet
            self.selected_thrust_level,
            false,          // network_connected (not used in single player)
            None,           // player_id (not used in single player)
            1,              // player_count (always 1 in single player)
            satellite_stats.as_ref(),
        );

        self.info_display.draw_all_panels();

        // Draw visualization HUD (shows T and G key status)
        self.vehicle_manager.draw_visualization_hud();

        // Draw "what a save!!" celebration text in screen space
        if let Some(screen_pos) = celebration_screen_pos {
            let text = "what a save!!";
            let text_size = 30.0;
            let text_offset_y = -80.0; // Above rocket (in screen space, negative is up)

            // Calculate text dimensions for centering
            let text_dims = measure_text(text, None, text_size as u16, 1.0);
            let text_x = screen_pos.x - text_dims.width / 2.0;
            let text_y = screen_pos.y + text_offset_y;

            // Draw shadow/outline
            for dx in &[-2.0, 0.0, 2.0] {
                for dy in &[-2.0, 0.0, 2.0] {
                    if *dx != 0.0 || *dy != 0.0 {
                        draw_text(text, text_x + dx, text_y + dy, text_size, BLACK);
                    }
                }
            }

            // Draw main text (yellow/gold color)
            draw_text(text, text_x, text_y, text_size, Color::new(1.0, 0.9, 0.0, 1.0));
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
                ("W", "Shoot bullet"),
                ("R", "Refuel from planet"),
                ("P", "Pause/Unpause"),
            ];

            let controls_right = [
                ("T", "Toggle trajectory"),
                ("G", "Toggle gravity forces"),
                ("TAB", "Switch planet (panels 2/3)"),
                ("1", "Toggle rocket panel"),
                ("2", "Toggle planet panel"),
                ("3", "Toggle orbit panel"),
                ("5", "Toggle network map"),
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

        // Draw network map popup if showing
        if self.show_network_map {
            self.draw_network_map();
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
