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
    info_display: GameInfoDisplay,
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

    // Save/load
    current_save_name: Option<String>,
    last_auto_save: f32,
    auto_save_interval: f32,

    // Starting positions
    rocket_spawn_position: Vec2,
    rocket_spawn_velocity: Vec2,
}

impl SplitScreenGame {
    pub fn new(window_size: Vec2) -> Self {
        let mut info_display = GameInfoDisplay::new();
        info_display.toggle_controls_panel();

        SplitScreenGame {
            world: World::new(),
            camera: Camera::new(window_size),
            info_display,
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

            current_save_name: None,
            last_auto_save: 0.0,
            auto_save_interval: 60.0,
            rocket_spawn_position: Vec2::ZERO,
            rocket_spawn_velocity: Vec2::ZERO,
        }
    }

    /// Initialize a new game with two players
    pub fn initialize_new_game(&mut self) {
        self.world.clear_all();
        self.game_time = 0.0;

        // Create main planet (Earth)
        let main_planet = Planet::new(
            Vec2::new(GameConstants::MAIN_PLANET_X, GameConstants::MAIN_PLANET_Y),
            GameConstants::MAIN_PLANET_RADIUS,
            GameConstants::MAIN_PLANET_MASS,
            BLUE,
        );
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
        secondary_planet.set_velocity(Vec2::new(0.0, moon_velocity));
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

    /// Update camera based on current mode
    fn update_camera_for_mode(&mut self) {
        match self.camera_mode {
            CameraMode::ShowBoth => {
                // Calculate midpoint between both players
                if let (Some(r1_id), Some(r2_id)) = (self.player1_rocket_id, self.player2_rocket_id) {
                    if let (Some(r1), Some(r2)) = (self.world.get_rocket(r1_id), self.world.get_rocket(r2_id)) {
                        let midpoint = (r1.position() + r2.position()) / 2.0;
                        self.camera.set_center(midpoint);

                        // Calculate distance and set zoom to fit both
                        let distance = r1.position().distance(r2.position());
                        let zoom = (distance / 300.0).max(1.0).min(10.0);
                        self.camera.set_target_zoom(zoom);
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

        if self.is_paused {
            return SplitScreenResult::Continue;
        }

        // Handle camera focus keys
        if self.player1_input.just_focused_camera() {
            self.camera_mode = CameraMode::FocusPlayer1((self.camera_focus_duration * 10.0) as u32);
            log::info!("Camera focused on Player 1");
        }
        if self.player2_input.just_focused_camera() {
            self.camera_mode = CameraMode::FocusPlayer2((self.camera_focus_duration * 10.0) as u32);
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

        // Draw trajectories for both players
        if let Some(r1_id) = self.player1_rocket_id {
            if let Some(r1) = self.world.get_rocket(r1_id) {
                let all_planets: Vec<&Planet> = self.world.planets().collect();
                self.vehicle_manager.draw_visualizations(r1, &all_planets, self.camera.zoom_level(), self.camera.camera());
            }
        }

        if let Some(r2_id) = self.player2_rocket_id {
            if let Some(r2) = self.world.get_rocket(r2_id) {
                let all_planets: Vec<&Planet> = self.world.planets().collect();
                self.vehicle_manager.draw_visualizations(r2, &all_planets, self.camera.zoom_level(), self.camera.camera());
            }
        }

        // Reset to default camera for UI
        set_default_camera();

        // Draw UI
        self.draw_ui();

        // Draw controls popup if showing
        if self.show_controls {
            self.draw_controls_popup();
        }
    }

    fn draw_ui(&mut self) {
        // Update and draw info displays for both players
        // For now, just show basic info
        let screen_w = screen_width();
        let screen_h = screen_height();

        // Player 1 info (top-left)
        draw_text(&format!("Player 1 (Red)"), 10.0, 30.0, 20.0, RED);
        draw_text(&format!("Thrust: {}%", (self.player1_state.thrust_level() * 100.0) as i32), 10.0, 50.0, 20.0, WHITE);

        // Player 2 info (top-right)
        draw_text(&format!("Player 2 (Blue)"), screen_w - 200.0, 30.0, 20.0, BLUE);
        draw_text(&format!("Thrust: {}%", (self.player2_state.thrust_level() * 100.0) as i32), screen_w - 200.0, 50.0, 20.0, WHITE);

        // Camera mode indicator (center-top)
        let mode_text = match self.camera_mode {
            CameraMode::ShowBoth => "Camera: Both Players",
            CameraMode::FocusPlayer1(t) => &format!("Camera: Player 1 ({}s)", t / 10),
            CameraMode::FocusPlayer2(t) => &format!("Camera: Player 2 ({}s)", t / 10),
        };
        let text_w = measure_text(mode_text, None, 20, 1.0).width;
        draw_text(mode_text, screen_w / 2.0 - text_w / 2.0, 30.0, 20.0, YELLOW);

        // Controls button (top-right corner)
        draw_rectangle(screen_w - 50.0, 10.0, 40.0, 30.0, Color::from_rgba(100, 100, 100, 255));
        draw_text("...", screen_w - 40.0, 32.0, 20.0, WHITE);
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
            ("A", "Rotate Left", "LEFT"),
            ("D", "Rotate Right", "RIGHT"),
            ("W", "Thrust", "UP"),
            ("Z", "Decrease Thrust", "COMMA"),
            ("X", "Increase Thrust", "PERIOD"),
            ("C", "Convert to Satellite", "]"),
            ("Q", "Zoom Out", "/"),
            ("E", "Zoom In", "'"),
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
