// Split-Screen Multiplayer - Two players on same device
// Shared world simulation with separate controls and dynamic camera

use macroquad::prelude::*;

use crate::entities::{GameObject, Planet, Rocket};
use crate::game_constants::GameConstants;
use crate::save_system::GameSaveData;
use crate::systems::{World, VehicleManager, PlayerInput, PlayerInputState, EntityId};
use crate::ui::{Camera, GameInfoDisplay};

/// Camera mode for split-screen
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CameraMode {
    ShowBoth,               // Default: show both players
    FocusPlayer1(u32),      // Focus on P1, with remaining time in deciseconds
    FocusPlayer2(u32),      // Focus on P2, with remaining time in deciseconds
}

/// Split-screen game result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitScreenResult {
    Continue,
    ReturnToMenu,
    Quit,
}

/// Split-screen multiplayer game mode
pub struct SplitScreenGame {
    world: World,
    camera: Camera,
    player1_info_display: GameInfoDisplay,  // Player 1 UI (left side, red)
    player2_info_display: GameInfoDisplay,  // Player 2 UI (right side, blue)
    vehicle_manager: VehicleManager,
    game_time: f32,
    is_paused: bool,
    show_controls: bool,

    // Player 1 state
    player1_input: PlayerInput,
    player1_state: PlayerInputState,
    player1_rocket_id: Option<EntityId>,

    // Player 2 state
    player2_input: PlayerInput,
    player2_state: PlayerInputState,
    player2_rocket_id: Option<EntityId>,

    // Camera management
    camera_mode: CameraMode,
    camera_focus_duration: f32,  // 10 seconds
    manual_zoom_mode: bool,  // true when user manually zooms, disables auto-zoom

    // Save/load
    current_save_name: Option<String>,
    last_auto_save: f32,
    auto_save_interval: f32,

    // Starting positions
    rocket_spawn_position: Vec2,
    rocket_spawn_velocity: Vec2,

    // Save celebration (F5 quick save)
    save_celebration_timer: f32,  // Time remaining for "what a save!!" text
}

impl SplitScreenGame {
    pub fn new(window_size: Vec2) -> Self {
        // Create Player 1 info display (left side, red theme)
        let player1_info_display = GameInfoDisplay::new_for_player(0);

        // Create Player 2 info display (right side, blue theme)
        let player2_info_display = GameInfoDisplay::new_for_player(1);

        SplitScreenGame {
            world: World::new(),
            camera: Camera::new(window_size),
            player1_info_display,
            player2_info_display,
            vehicle_manager: VehicleManager::new(),
            game_time: 0.0,
            is_paused: false,
            show_controls: false,

            player1_input: PlayerInput::player1(),
            player1_state: PlayerInputState::new(0),
            player1_rocket_id: None,

            player2_input: PlayerInput::player2(),
            player2_state: PlayerInputState::new(1),
            player2_rocket_id: None,

            camera_mode: CameraMode::ShowBoth,
            camera_focus_duration: 10.0,
            manual_zoom_mode: false,

            current_save_name: None,
            last_auto_save: 0.0,
            auto_save_interval: 60.0,
            rocket_spawn_position: Vec2::ZERO,
            rocket_spawn_velocity: Vec2::ZERO,
            save_celebration_timer: 0.0,
        }
    }

    /// Initialize a new game with two players
    pub fn initialize_new_game(&mut self) {
        self.world.clear_all();
        self.game_time = 0.0;

        // Create main planet (Earth)
        let mut main_planet = Planet::new(
            Vec2::new(GameConstants::MAIN_PLANET_X, GameConstants::MAIN_PLANET_Y),
            GameConstants::MAIN_PLANET_RADIUS,
            GameConstants::MAIN_PLANET_MASS,
            BLUE,
        );
        // Calculate radius from mass to ensure consistency with mass-depletion system
        main_planet.update_radius_from_mass();
        self.world.add_planet(main_planet);

        // Create secondary planet (Moon)
        let moon_x = *crate::game_constants::SECONDARY_PLANET_X;
        let moon_y = *crate::game_constants::SECONDARY_PLANET_Y;
        let moon_radius = GameConstants::SECONDARY_PLANET_RADIUS;
        let moon_velocity = *crate::game_constants::SECONDARY_PLANET_ORBITAL_VELOCITY;

        let mut secondary_planet = Planet::new(
            Vec2::new(moon_x, moon_y),
            moon_radius,
            GameConstants::SECONDARY_PLANET_MASS,
            Color::from_rgba(150, 150, 150, 255),
        );
        // Calculate radius from mass to ensure consistency with mass-depletion system
        secondary_planet.update_radius_from_mass();
        secondary_planet.set_velocity(Vec2::new(0.0, -moon_velocity));
        self.world.add_planet(secondary_planet);

        // Spawn Player 1 rocket at default position
        let rocket_spawn_distance = GameConstants::MAIN_PLANET_RADIUS + 200.0;
        self.rocket_spawn_position = Vec2::new(
            GameConstants::MAIN_PLANET_X + rocket_spawn_distance,
            GameConstants::MAIN_PLANET_Y,
        );
        self.rocket_spawn_velocity = Vec2::new(0.0, 0.0);

        let rocket1 = Rocket::new(
            self.rocket_spawn_position,
            self.rocket_spawn_velocity,
            Color::from_rgba(255, 100, 100, 255),  // Red for Player 1
            GameConstants::ROCKET_BASE_MASS,
        );
        let rocket1_id = self.world.add_rocket(rocket1);
        self.player1_rocket_id = Some(rocket1_id);
        self.world.set_active_rocket(Some(rocket1_id));

        // Spawn Player 2 rocket with 5 degree offset from Player 1
        let offset_angle = 5.0_f32.to_radians();
        let center = Vec2::new(GameConstants::MAIN_PLANET_X, GameConstants::MAIN_PLANET_Y);

        // Rotate spawn position by 5 degrees around planet center
        let rel_pos = self.rocket_spawn_position - center;
        let rotated_x = rel_pos.x * offset_angle.cos() - rel_pos.y * offset_angle.sin();
        let rotated_y = rel_pos.x * offset_angle.sin() + rel_pos.y * offset_angle.cos();
        let p2_spawn_pos = center + Vec2::new(rotated_x, rotated_y);

        // Rotate velocity vector by 5 degrees
        let vel_x = self.rocket_spawn_velocity.x * offset_angle.cos() - self.rocket_spawn_velocity.y * offset_angle.sin();
        let vel_y = self.rocket_spawn_velocity.x * offset_angle.sin() + self.rocket_spawn_velocity.y * offset_angle.cos();
        let p2_spawn_vel = Vec2::new(vel_x, vel_y);

        let rocket2 = Rocket::new(
            p2_spawn_pos,
            p2_spawn_vel,
            Color::from_rgba(100, 100, 255, 255),  // Blue for Player 2
            GameConstants::ROCKET_BASE_MASS,
        );
        let rocket2_id = self.world.add_rocket(rocket2);
        self.player2_rocket_id = Some(rocket2_id);

        // Set camera to show both players
        self.update_camera_for_mode();

        log::info!("Split-screen game initialized with two players");
    }

    /// Load a single player save and add Player 2
    pub fn load_from_save_with_player2(&mut self, save_data: GameSaveData, save_name: String) {
        self.load_from_snapshot(save_data);

        // Player 1 gets the active rocket from the save
        self.player1_rocket_id = self.world.active_rocket_id();

        // Spawn Player 2 rocket with 5 degree offset
        if let Some(p1_rocket_id) = self.player1_rocket_id {
            if let Some(p1_rocket) = self.world.get_rocket(p1_rocket_id) {
                let p1_pos = p1_rocket.position();
                let p1_vel = p1_rocket.velocity();

                // Find nearest planet to use as center
                let center = Vec2::new(GameConstants::MAIN_PLANET_X, GameConstants::MAIN_PLANET_Y);

                // 5 degree offset
                let offset_angle = 5.0_f32.to_radians();
                let rel_pos = p1_pos - center;
                let rotated_x = rel_pos.x * offset_angle.cos() - rel_pos.y * offset_angle.sin();
                let rotated_y = rel_pos.x * offset_angle.sin() + rel_pos.y * offset_angle.cos();
                let p2_pos = center + Vec2::new(rotated_x, rotated_y);

                let vel_x = p1_vel.x * offset_angle.cos() - p1_vel.y * offset_angle.sin();
                let vel_y = p1_vel.x * offset_angle.sin() + p1_vel.y * offset_angle.cos();
                let p2_vel = Vec2::new(vel_x, vel_y);

                let rocket2 = Rocket::new(
                    p2_pos,
                    p2_vel,
                    Color::from_rgba(100, 100, 255, 255),  // Blue for Player 2
                    GameConstants::ROCKET_BASE_MASS,
                );
                let rocket2_id = self.world.add_rocket(rocket2);
                self.player2_rocket_id = Some(rocket2_id);

                log::info!("Loaded save '{}' with Player 2 added", save_name);
            }
        }

        self.current_save_name = Some(save_name);
        self.update_camera_for_mode();
    }

    fn load_from_snapshot(&mut self, snapshot: GameSaveData) {
        self.world.clear_all_entities();
        self.game_time = snapshot.game_time;

        let planet_count = snapshot.planets.len();
        let rocket_count = snapshot.rockets.len();
        let satellite_count = snapshot.satellites.len();

        for saved_planet in snapshot.planets {
            let (id, planet) = saved_planet.to_planet();
            self.world.add_planet_with_id(id, planet);
        }

        for saved_rocket in snapshot.rockets {
            let (id, rocket) = saved_rocket.to_rocket();
            self.world.add_rocket_with_id(id, rocket);
        }

        for saved_satellite in snapshot.satellites {
            let (id, satellite) = saved_satellite.to_satellite();
            self.world.add_satellite_with_id(id, satellite);
        }

        self.world.set_active_rocket(snapshot.active_rocket_id);

        log::info!(
            "Loaded snapshot: {} planets, {} rockets, {} satellites",
            planet_count, rocket_count, satellite_count
        );
    }

    /// Quick save triggered by F5 key - saves and shows "what a save!!" celebration
    fn quick_save(&mut self) {
        let save_data = self.create_save_data();

        match save_data.save_to_file("quicksave_splitscreen") {
            Ok(_) => {
                log::info!("Quick save successful (split screen)");
                self.current_save_name = Some("quicksave_splitscreen".to_string());

                // Trigger save celebration
                self.save_celebration_timer = 5.0; // Show for 5 seconds
            }
            Err(e) => {
                log::error!("Failed to quick save: {}", e);
            }
        }
    }

    /// Create save data snapshot from current game state
    fn create_save_data(&self) -> GameSaveData {
        let mut save_data = GameSaveData::new();
        save_data.game_time = self.game_time;

        // Save all planets with their IDs
        use crate::save_system::SavedPlanet;
        save_data.planets = self.world.planets_with_ids()
            .map(|(id, planet)| SavedPlanet::from_planet(id, planet))
            .collect();

        // Save all rockets with their IDs
        use crate::save_system::SavedRocket;
        save_data.rockets = self.world.rockets_with_ids()
            .map(|(id, rocket)| SavedRocket::from_rocket(id, rocket))
            .collect();

        // Save all satellites with their IDs
        use crate::save_system::SavedSatellite;
        save_data.satellites = self.world.satellites_with_ids()
            .map(|(id, satellite)| SavedSatellite::from_satellite(id, satellite))
            .collect();

        // Save Player 1's rocket as active (split screen uses Player 1 as primary)
        save_data.player_id = Some(0);
        save_data.active_rocket_id = self.player1_rocket_id;

        // Save camera state
        use crate::save_system::SavedCamera;
        save_data.camera = SavedCamera {
            center: self.camera.camera().target.into(),
            zoom: self.camera.zoom_level(),
        };

        save_data
    }

    /// Update camera based on current mode
    fn update_camera_for_mode(&mut self) {
        match self.camera_mode {
            CameraMode::ShowBoth => {
                // Calculate midpoint between both players
                if let (Some(r1_id), Some(r2_id)) = (self.player1_rocket_id, self.player2_rocket_id) {
                    if let (Some(r1), Some(r2)) = (self.world.get_rocket(r1_id), self.world.get_rocket(r2_id)) {
                        let midpoint = (r1.position() + r2.position()) / 2.0;
                        self.camera.set_center(midpoint);

                        // Only auto-calculate zoom if not in manual zoom mode
                        if !self.manual_zoom_mode {
                            let distance = r1.position().distance(r2.position());
                            let zoom = (distance / 300.0).max(1.0).min(10.0);
                            self.camera.set_target_zoom(zoom);
                        }
                    }
                }
            }
            CameraMode::FocusPlayer1(_) => {
                if let Some(r1_id) = self.player1_rocket_id {
                    if let Some(r1) = self.world.get_rocket(r1_id) {
                        self.camera.set_center(r1.position());
                        self.camera.set_target_zoom(1.0);
                    }
                }
            }
            CameraMode::FocusPlayer2(_) => {
                if let Some(r2_id) = self.player2_rocket_id {
                    if let Some(r2) = self.world.get_rocket(r2_id) {
                        self.camera.set_center(r2.position());
                        self.camera.set_target_zoom(1.0);
                    }
                }
            }
        }
    }

    /// Handle input for game controls
    pub fn handle_input(&mut self) -> SplitScreenResult {
        // Check for escape to return to menu or close controls popup
        if is_key_pressed(KeyCode::Escape) {
            if self.show_controls {
                self.show_controls = false;
                self.is_paused = false;
            } else {
                return SplitScreenResult::ReturnToMenu;
            }
        }

        // Toggle controls menu with Enter key
        if is_key_pressed(KeyCode::Enter) {
            self.show_controls = !self.show_controls;
            self.is_paused = self.show_controls;
        }

        // Toggle UI panels with 0-9 keys (work even when paused)
        if is_key_pressed(KeyCode::Key0) {
            self.player1_info_display.toggle_rocket_panel();
            self.player2_info_display.toggle_rocket_panel();
        }
        if is_key_pressed(KeyCode::Key1) {
            self.player1_info_display.toggle_planet_panel();
            self.player2_info_display.toggle_planet_panel();
        }
        if is_key_pressed(KeyCode::Key2) {
            self.player1_info_display.toggle_orbit_panel();
            self.player2_info_display.toggle_orbit_panel();
        }
        if is_key_pressed(KeyCode::Key3) {
            self.player1_info_display.toggle_network_panel();
            self.player2_info_display.toggle_network_panel();
        }
        if is_key_pressed(KeyCode::Key8) {
            self.player1_info_display.show_all_panels();
            self.player2_info_display.show_all_panels();
        }
        if is_key_pressed(KeyCode::Key9) {
            self.player1_info_display.hide_all_panels();
            self.player2_info_display.hide_all_panels();
        }

        // Visualization toggles (shared for both players)
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

        // Quick save (F5 key) - saves and shows "what a save!!" celebration
        if is_key_pressed(KeyCode::F5) {
            self.quick_save();
        }

        // Pause/unpause (P key, only if controls not showing)
        if is_key_pressed(KeyCode::P) && !self.show_controls {
            self.is_paused = !self.is_paused;
            log::info!("Game {}", if self.is_paused { "paused" } else { "unpaused" });
        }

        // Mouse wheel zoom - always works, switches to ShowBoth mode with manual zoom
        let mouse_wheel = mouse_wheel().1;
        if mouse_wheel != 0.0 {
            self.camera_mode = CameraMode::ShowBoth;
            self.manual_zoom_mode = true;
            self.camera.adjust_zoom(-mouse_wheel * 0.02);
        }

        // Keyboard zoom controls - only work when camera is focused on that player
        match self.camera_mode {
            CameraMode::FocusPlayer1(_) => {
                // Player 1 focused: Q = zoom in, E = zoom out
                if is_key_down(KeyCode::Q) {
                    self.camera.adjust_zoom(-0.02);
                }
                if is_key_down(KeyCode::E) {
                    self.camera.adjust_zoom(0.02);
                }
            }
            CameraMode::FocusPlayer2(_) => {
                // Player 2 focused: / = zoom in, ' = zoom out
                if is_key_down(KeyCode::Slash) {
                    self.camera.adjust_zoom(-0.02);
                }
                if is_key_down(KeyCode::Apostrophe) {
                    self.camera.adjust_zoom(0.02);
                }
            }
            CameraMode::ShowBoth => {
                // No keyboard zoom in ShowBoth mode
            }
        }

        if self.is_paused {
            return SplitScreenResult::Continue;
        }

        // Handle camera focus keys - disable manual zoom mode to re-enable auto-zoom
        if self.player1_input.just_focused_camera() {
            self.camera_mode = CameraMode::FocusPlayer1((self.camera_focus_duration * 10.0) as u32);
            self.manual_zoom_mode = false;
            log::info!("Camera focused on Player 1");
        }
        if self.player2_input.just_focused_camera() {
            self.camera_mode = CameraMode::FocusPlayer2((self.camera_focus_duration * 10.0) as u32);
            self.manual_zoom_mode = false;
            log::info!("Camera focused on Player 2");
        }

        // Player 1 controls
        let p1_input = self.player1_input.clone();
        self.handle_player_input(p1_input, 0, self.player1_rocket_id);

        // Player 2 controls
        let p2_input = self.player2_input.clone();
        self.handle_player_input(p2_input, 1, self.player2_rocket_id);

        SplitScreenResult::Continue
    }

    fn handle_player_input(&mut self, input: PlayerInput, player_index: usize, rocket_id: Option<EntityId>) {
        // Get the appropriate state
        let state = if player_index == 0 {
            &mut self.player1_state
        } else {
            &mut self.player2_state
        };
        // Thrust level adjustment
        if input.just_decreased_thrust() {
            state.adjust_thrust(-0.05);
            log::info!("Player {} thrust: {}%", input.player_id, (state.thrust_level() * 100.0) as i32);
        }
        if input.just_increased_thrust() {
            state.adjust_thrust(0.05);
            log::info!("Player {} thrust: {}%", input.player_id, (state.thrust_level() * 100.0) as i32);
        }

        // Apply controls to rocket
        if let Some(rid) = rocket_id {
            // Rotation
            let rotation_input = input.get_rotation_input();
            if rotation_input != 0.0 {
                let rotation_degrees = rotation_input * 3.0; // degrees per frame
                let rotation_radians = rotation_degrees.to_radians();
                if let Some(rocket) = self.world.get_rocket_mut(rid) {
                    rocket.rotate(rotation_radians);
                }
            }

            // Thrust
            let thrust_level = if input.is_thrusting() {
                state.thrust_level()
            } else {
                0.0
            };

            if let Some(rocket) = self.world.get_rocket_mut(rid) {
                rocket.set_thrust_level(thrust_level);
            }

            // Convert to satellite
            if input.just_converted_to_satellite() {
                if let Some(new_satellite_id) = self.world.convert_rocket_to_satellite(rid) {
                    log::info!("Player {} converted rocket to satellite", input.player_id);

                    // Spawn new rocket for this player
                    let new_rocket = Rocket::new(
                        self.rocket_spawn_position,
                        self.rocket_spawn_velocity,
                        if input.player_id == 0 {
                            Color::from_rgba(255, 100, 100, 255)
                        } else {
                            Color::from_rgba(100, 100, 255, 255)
                        },
                        GameConstants::ROCKET_BASE_MASS,
                    );
                    let new_rocket_id = self.world.add_rocket(new_rocket);

                    // Update player's rocket ID
                    if input.player_id == 0 {
                        self.player1_rocket_id = Some(new_rocket_id);
                    } else {
                        self.player2_rocket_id = Some(new_rocket_id);
                    }
                }
            }

            // Shoot bullet (S for Player 1, Down for Player 2)
            let shoot_pressed = if player_index == 0 {
                is_key_pressed(KeyCode::S)
            } else {
                is_key_pressed(KeyCode::Down)
            };

            if shoot_pressed {
                if let Some(bullet_id) = self.world.shoot_bullet_from_rocket(rid) {
                    log::info!("Player {} fired bullet {}", input.player_id, bullet_id);
                } else {
                    log::debug!("Player {} cannot shoot: not enough fuel", input.player_id);
                }
            }
        }
    }

    /// Update game state
    pub fn update(&mut self, delta_time: f32) -> SplitScreenResult {
        if self.is_paused {
            return SplitScreenResult::Continue;
        }

        // Update camera focus timer
        match self.camera_mode {
            CameraMode::FocusPlayer1(remaining) | CameraMode::FocusPlayer2(remaining) => {
                if remaining == 0 {
                    self.camera_mode = CameraMode::ShowBoth;
                    log::info!("Camera focus expired, showing both players");
                } else {
                    // Decrement timer (in deciseconds)
                    let new_remaining = remaining.saturating_sub((delta_time * 10.0) as u32);
                    self.camera_mode = match self.camera_mode {
                        CameraMode::FocusPlayer1(_) => CameraMode::FocusPlayer1(new_remaining),
                        CameraMode::FocusPlayer2(_) => CameraMode::FocusPlayer2(new_remaining),
                        _ => CameraMode::ShowBoth,
                    };
                }
            }
            _ => {}
        }

        // Update world physics
        self.world.update(delta_time);

        // Handle rockets destroyed by bullets (respawn like 'C' key, but without satellite)
        let destroyed_rockets = self.world.take_destroyed_rockets();
        for destroyed in destroyed_rockets {
            let player_id = destroyed.player_id.unwrap_or(0);
            log::info!("Player {} rocket destroyed by bullet, respawning", player_id);

            // Determine spawn position based on player (Player 2 at +5 degrees from Player 1)
            let (spawn_pos, spawn_vel, color) = if player_id == 0 {
                (self.rocket_spawn_position, self.rocket_spawn_velocity, Color::from_rgba(255, 100, 100, 255))
            } else {
                let offset_angle = 5.0_f32.to_radians();
                let center = Vec2::new(GameConstants::MAIN_PLANET_X, GameConstants::MAIN_PLANET_Y);

                let rel_pos = self.rocket_spawn_position - center;
                let rotated_x = rel_pos.x * offset_angle.cos() - rel_pos.y * offset_angle.sin();
                let rotated_y = rel_pos.x * offset_angle.sin() + rel_pos.y * offset_angle.cos();
                let p2_pos = center + Vec2::new(rotated_x, rotated_y);

                let vel_x = self.rocket_spawn_velocity.x * offset_angle.cos() - self.rocket_spawn_velocity.y * offset_angle.sin();
                let vel_y = self.rocket_spawn_velocity.x * offset_angle.sin() + self.rocket_spawn_velocity.y * offset_angle.cos();
                let p2_vel = Vec2::new(vel_x, vel_y);

                (p2_pos, p2_vel, Color::from_rgba(100, 100, 255, 255))
            };

            // Spawn new rocket for this player
            let new_rocket = Rocket::new(
                spawn_pos,
                spawn_vel,
                color,
                GameConstants::ROCKET_BASE_MASS,
            );

            let new_rocket_id = self.world.add_rocket(new_rocket);

            // Update the appropriate player's rocket ID
            if player_id == 0 {
                self.player1_rocket_id = Some(new_rocket_id);
                self.world.set_active_rocket(Some(new_rocket_id));
            } else {
                self.player2_rocket_id = Some(new_rocket_id);
            }

            log::info!("Respawned new rocket {} for player {}", new_rocket_id, player_id);
        }

        // Update save celebration timer
        if self.save_celebration_timer > 0.0 {
            self.save_celebration_timer -= delta_time;
        }

        // Handle manual planet refueling (R key for both players - shared key)
        if is_key_down(KeyCode::R) {
            if let Some(rocket_id) = self.player1_rocket_id {
                self.world.handle_manual_planet_refuel(rocket_id, delta_time);
            }
            if let Some(rocket_id) = self.player2_rocket_id {
                self.world.handle_manual_planet_refuel(rocket_id, delta_time);
            }
        }

        // Update game time
        self.game_time += delta_time;

        // Update camera position based on mode
        self.update_camera_for_mode();
        self.camera.update(delta_time);

        SplitScreenResult::Continue
    }

    /// Render the game
    pub fn render(&mut self) {
        // Set camera
        set_camera(self.camera.camera());

        // Draw world entities
        self.world.render();

        // Draw trajectories for both players with color-coded lines
        if let Some(r1_id) = self.player1_rocket_id {
            if let Some(r1) = self.world.get_rocket(r1_id) {
                let all_planets: Vec<&Planet> = self.world.planets().collect();
                // Player 1: red trajectory
                self.vehicle_manager.draw_visualizations_with_color(
                    r1,
                    &all_planets,
                    self.camera.zoom_level(),
                    self.camera.camera(),
                    Some(Color::new(1.0, 0.0, 0.0, 0.6)),
                );
            }
        }

        if let Some(r2_id) = self.player2_rocket_id {
            if let Some(r2) = self.world.get_rocket(r2_id) {
                let all_planets: Vec<&Planet> = self.world.planets().collect();
                // Player 2: blue trajectory
                self.vehicle_manager.draw_visualizations_with_color(
                    r2,
                    &all_planets,
                    self.camera.zoom_level(),
                    self.camera.camera(),
                    Some(Color::new(0.0, 0.5, 1.0, 0.6)),
                );
            }
        }

        // Store celebration rocket position for screen-space rendering (Player 1's rocket)
        let celebration_screen_pos = if self.save_celebration_timer > 0.0 {
            self.player1_rocket_id
                .and_then(|rid| self.world.get_rocket(rid))
                .map(|rocket| self.camera.world_to_screen(rocket.position()))
        } else {
            None
        };

        // Reset to default camera for UI
        set_default_camera();

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

        // Draw UI
        self.draw_ui();

        // Draw controls popup if showing
        if self.show_controls {
            self.draw_controls_popup();
        }
    }

    fn draw_ui(&mut self) {
        let screen_w = screen_width();
        let screen_h = screen_height();
        let all_planets: Vec<&Planet> = self.world.planets().collect();

        // Get satellite stats for network panel
        let satellite_stats = self.world.get_satellite_network_stats();

        // Update and draw Player 1 info display (left side, red theme)
        if let Some(r1_id) = self.player1_rocket_id {
            if let Some(r1) = self.world.get_rocket(r1_id) {
                // Default to Earth (first planet) for split-screen
                use crate::systems::ReferenceBody;
                let selected_planet = all_planets.get(0).copied();
                self.player1_info_display.update_all_panels(
                    Some(r1),
                    &all_planets,
                    selected_planet,
                    ReferenceBody::Earth,  // Default to Earth
                    self.player1_state.thrust_level(),
                    false,  // network_connected (not applicable for local split-screen)
                    Some(0),  // Player 1 ID
                    2,  // player_count (always 2 in split-screen)
                    Some(&satellite_stats),
                );
                self.player1_info_display.draw_all_panels();
            }
        }

        // Update and draw Player 2 info display (right side, blue theme)
        if let Some(r2_id) = self.player2_rocket_id {
            if let Some(r2) = self.world.get_rocket(r2_id) {
                // Default to Earth (first planet) for split-screen
                use crate::systems::ReferenceBody;
                let selected_planet = all_planets.get(0).copied();
                self.player2_info_display.update_all_panels(
                    Some(r2),
                    &all_planets,
                    selected_planet,
                    ReferenceBody::Earth,  // Default to Earth
                    self.player2_state.thrust_level(),
                    false,  // network_connected
                    Some(1),  // Player 2 ID
                    2,  // player_count
                    Some(&satellite_stats),
                );
                self.player2_info_display.draw_all_panels();
            }
        }

        // Camera mode indicator (center-top)
        let mode_text = match self.camera_mode {
            CameraMode::ShowBoth => "Camera: Both Players",
            CameraMode::FocusPlayer1(t) => &format!("Camera: Player 1 ({}s)", t / 10),
            CameraMode::FocusPlayer2(t) => &format!("Camera: Player 2 ({}s)", t / 10),
        };
        let text_w = measure_text(mode_text, None, 20, 1.0).width;
        draw_text(mode_text, screen_w / 2.0 - text_w / 2.0, 30.0, 20.0, YELLOW);

        // "Press ENTER for controls" text at top-right
        let help_text = "Press ENTER for controls";
        let help_w = measure_text(help_text, None, 18, 1.0).width;
        draw_text(help_text, screen_w - help_w - 20.0, 30.0, 18.0, LIGHTGRAY);
    }

    fn draw_controls_popup(&self) {
        let screen_w = screen_width();
        let screen_h = screen_height();

        // Draw backdrop
        draw_rectangle(0.0, 0.0, screen_w, screen_h, Color::from_rgba(0, 0, 0, 180));

        // Draw popup window
        let popup_w = 900.0;
        let popup_h = 700.0;
        let popup_x = screen_w / 2.0 - popup_w / 2.0;
        let popup_y = screen_h / 2.0 - popup_h / 2.0;

        draw_rectangle(popup_x, popup_y, popup_w, popup_h, Color::from_rgba(40, 40, 60, 255));
        draw_rectangle_lines(popup_x, popup_y, popup_w, popup_h, 2.0, WHITE);

        // Title
        let title = "SPLIT-SCREEN CONTROLS";
        let title_w = measure_text(title, None, 32, 1.0).width;
        draw_text(title, popup_x + popup_w / 2.0 - title_w / 2.0, popup_y + 40.0, 32.0, WHITE);

        // Three columns: Player 1, Action, Player 2
        let col1_x = popup_x + 50.0;
        let col2_x = popup_x + 350.0;
        let col3_x = popup_x + 650.0;
        let start_y = popup_y + 100.0;
        let line_height = 30.0;

        // Column headers
        draw_text("Player 1", col1_x, start_y, 24.0, RED);
        draw_text("Action", col2_x, start_y, 24.0, YELLOW);
        draw_text("Player 2", col3_x, start_y, 24.0, BLUE);

        let mut y = start_y + 40.0;

        let controls = [
            ("D", "Rotate Left", "RIGHT"),
            ("A", "Rotate Right", "LEFT"),
            ("W", "Thrust", "UP"),
            ("Z", "Decrease Thrust", "COMMA"),
            ("X", "Increase Thrust", "PERIOD"),
            ("C", "Convert to Satellite", "]"),
            ("S", "Shoot Bullet", "DOWN"),
            ("Q", "Zoom In", "/"),
            ("E", "Zoom Out", "'"),
            ("R", "Focus Camera (10s)", ";"),
        ];

        for (p1_key, action, p2_key) in controls.iter() {
            draw_text(p1_key, col1_x, y, 20.0, WHITE);
            draw_text(action, col2_x, y, 20.0, WHITE);
            draw_text(p2_key, col3_x, y, 20.0, WHITE);
            y += line_height;
        }

        y += 20.0;
        draw_text("ENTER - Toggle Controls Menu", col2_x, y, 20.0, GRAY);
        y += line_height;
        draw_text("ESC - Return to Menu", col2_x, y, 20.0, GRAY);
    }
}
