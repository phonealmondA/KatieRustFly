// Multiplayer Host Mode - Authoritative server broadcasting snapshots via UDP
// Runs the simulation and broadcasts state every 10-15 seconds to all connected clients

use macroquad::prelude::*;
use std::net::{SocketAddr, UdpSocket};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::entities::{Planet, Rocket, Satellite};
use crate::game_constants::GameConstants;
use crate::save_system::{GameSaveData, SavedCamera, SavedPlanet, SavedRocket, SavedSatellite};
use crate::systems::{World, EntityId, VehicleManager, PlayerInput, PlayerInputState};
use crate::ui::{Camera, GameInfoDisplay};

const SNAPSHOT_INTERVAL: f32 = 12.0; // 12 seconds between broadcasts

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MultiplayerHostResult {
    None,
    ReturnToMenu,
    Quit,
}

struct ConnectedClient {
    addr: SocketAddr,
    player_id: u32,
    last_seen: f64, // Timestamp of last received packet
}

pub struct MultiplayerHost {
    // Core game systems
    world: World,
    camera: Camera,
    vehicle_manager: VehicleManager,
    game_info: GameInfoDisplay,

    // Host player state (player 0)
    player_input: PlayerInput,
    player_state: PlayerInputState,
    active_rocket_id: Option<EntityId>,

    // Networking
    socket: Arc<UdpSocket>,
    clients: Arc<Mutex<HashMap<SocketAddr, ConnectedClient>>>,
    snapshot_timer: f32,
    next_player_id: u32, // Next available player ID for new clients
    port: u16, // UDP port this host is listening on

    // Game state
    window_size: Vec2,
    paused: bool,
    show_controls: bool,
    current_save_name: Option<String>,
}

impl MultiplayerHost {
    /// Calculate spawn position for a player based on their player ID
    /// Host (player 0) spawns at 0 degrees, each subsequent player at +5 degrees
    fn calculate_spawn_position(player_id: u32) -> Vec2 {
        let angle_degrees = player_id as f32 * 5.0;
        let angle_radians = angle_degrees.to_radians();
        let spawn_distance = GameConstants::MAIN_PLANET_RADIUS + 200.0;

        Vec2::new(
            GameConstants::MAIN_PLANET_X + spawn_distance * angle_radians.cos(),
            GameConstants::MAIN_PLANET_Y + spawn_distance * angle_radians.sin(),
        )
    }

    /// Get rocket color for a player based on their player ID
    fn get_player_color(player_id: u32) -> Color {
        match player_id {
            0 => Color::from_rgba(255, 100, 100, 255), // Red for host
            1 => Color::from_rgba(100, 100, 255, 255), // Blue for player 1
            2 => Color::from_rgba(100, 255, 100, 255), // Green for player 2
            3 => Color::from_rgba(255, 255, 100, 255), // Yellow for player 3
            4 => Color::from_rgba(255, 100, 255, 255), // Magenta for player 4
            5 => Color::from_rgba(100, 255, 255, 255), // Cyan for player 5
            _ => Color::from_rgba(200, 200, 200, 255), // Gray for others
        }
    }

    /// Create a new multiplayer host
    pub fn new(window_size: Vec2, port: u16) -> Result<Self, String> {
        // Bind UDP socket
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", port))
            .map_err(|e| format!("Failed to bind UDP socket on port {}: {}", port, e))?;

        // Set non-blocking mode so we don't freeze waiting for packets
        socket.set_nonblocking(true)
            .map_err(|e| format!("Failed to set non-blocking mode: {}", e))?;

        log::info!("Multiplayer host listening on port {}", port);

        Ok(Self {
            world: World::new(),
            camera: Camera::new(window_size),
            vehicle_manager: VehicleManager::new(),
            game_info: GameInfoDisplay::new(),

            player_input: PlayerInput::player1(), // Host uses Player 1 controls
            player_state: PlayerInputState::new(0), // Host is player 0
            active_rocket_id: None,

            socket: Arc::new(socket),
            clients: Arc::new(Mutex::new(HashMap::new())),
            snapshot_timer: 0.0,
            next_player_id: 1, // Host is player 0, clients start at 1
            port,

            window_size,
            paused: false,
            show_controls: false,
            current_save_name: None,
        })
    }

    /// Initialize a new game with default starting conditions
    pub fn initialize_new_game(&mut self) {
        log::info!("Initializing new multiplayer host game");

        self.world.clear_all_entities();

        // Create main planet (Earth)
        let main_planet = Planet::new(
            Vec2::new(GameConstants::MAIN_PLANET_X, GameConstants::MAIN_PLANET_Y),
            GameConstants::MAIN_PLANET_RADIUS,
            GameConstants::MAIN_PLANET_MASS,
            BLUE,
        );
        self.world.add_planet(main_planet);

        // Create secondary planet (Moon) - match single player configuration
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
        secondary_planet.set_velocity(Vec2::new(0.0, -moon_velocity));
        self.world.add_planet(secondary_planet);

        // Spawn host's rocket (player 0) at 0 degrees
        let spawn_position = Self::calculate_spawn_position(0);
        let mut rocket = Rocket::new(
            spawn_position,
            Vec2::new(0.0, 0.0),
            Self::get_player_color(0),
            GameConstants::ROCKET_BASE_MASS,
        );
        rocket.set_player_id(Some(0)); // Host is player 0

        let rocket_id = self.world.add_rocket(rocket);
        self.active_rocket_id = Some(rocket_id);
        self.world.set_active_rocket(Some(rocket_id));

        // Initialize camera
        self.camera.set_center(spawn_position);

        log::info!("Multiplayer host game initialized - waiting for clients");
    }

    /// Load game from a save file
    pub fn load_from_save(&mut self, save_data: GameSaveData, save_name: String) {
        log::info!("Loading multiplayer host game from save: {}", save_name);

        // Clear existing world
        self.world.clear_all_entities();

        // Load planets with their original IDs
        for saved_planet in save_data.planets {
            let (id, planet) = saved_planet.to_planet();
            self.world.add_planet_with_id(id, planet);
        }

        // Load rockets with their original IDs
        for saved_rocket in save_data.rockets {
            let (id, rocket) = saved_rocket.to_rocket();
            self.world.add_rocket_with_id(id, rocket);
        }

        // Load satellites with their original IDs
        for saved_satellite in save_data.satellites {
            let (id, satellite) = saved_satellite.to_satellite();
            self.world.add_satellite_with_id(id, satellite);
        }

        // Restore active rocket
        self.active_rocket_id = save_data.active_rocket_id;
        self.world.set_active_rocket(save_data.active_rocket_id);

        // Restore camera state
        self.camera.set_center(save_data.camera.center.into());

        self.current_save_name = Some(save_name);
        log::info!("Multiplayer host save loaded successfully");
    }

    /// Handle input for the host player
    pub fn handle_input(&mut self) -> MultiplayerHostResult {
        // ESC - return to menu or close controls popup
        if is_key_pressed(KeyCode::Escape) {
            if self.show_controls {
                self.show_controls = false;
                self.paused = false;
            } else {
                log::info!("ESC pressed - returning to menu");
                return MultiplayerHostResult::ReturnToMenu;
            }
        }

        // Enter - toggle controls menu
        if is_key_pressed(KeyCode::Enter) {
            self.show_controls = !self.show_controls;
            self.paused = self.show_controls; // Pause when showing controls
        }

        // Panel visibility toggles (keys 1-5)
        if is_key_pressed(KeyCode::Key1) {
            self.game_info.toggle_rocket_panel();
        }
        if is_key_pressed(KeyCode::Key2) {
            self.game_info.toggle_planet_panel();
        }
        if is_key_pressed(KeyCode::Key3) {
            self.game_info.toggle_orbit_panel();
        }
        if is_key_pressed(KeyCode::Key4) {
            self.game_info.toggle_controls_panel();
        }
        if is_key_pressed(KeyCode::Key5) {
            self.game_info.toggle_network_panel();
        }
        // Key 0 to toggle all panels
        if is_key_pressed(KeyCode::Key0) {
            self.game_info.show_all_panels();
        }
        if is_key_pressed(KeyCode::Key9) {
            self.game_info.hide_all_panels();
        }

        // P - pause/unpause (only if controls not showing)
        if is_key_pressed(KeyCode::P) && !self.show_controls {
            self.paused = !self.paused;
            log::info!("Game {}", if self.paused { "paused" } else { "unpaused" });
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

        // F - save game
        if is_key_pressed(KeyCode::F) {
            self.save_game();
        }

        // Only process game controls if not paused
        if !self.paused {
            self.handle_player_controls();
        }

        MultiplayerHostResult::None
    }

    fn handle_player_controls(&mut self) {
        if let Some(rocket_id) = self.active_rocket_id {
            // Rotation (A/D or Left/Right, same as singleplayer)
            let mut rotation_delta = 0.0;
            if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
                rotation_delta = 3.0; // degrees per frame
            }
            if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
                rotation_delta = -3.0;
            }

            if rotation_delta != 0.0 {
                let rotation_radians = rotation_delta * std::f32::consts::PI / 180.0;
                if let Some(rocket) = self.world.get_rocket_mut(rocket_id) {
                    rocket.rotate(rotation_radians);
                }
            }

            // Thrust adjustment (comma to decrease, period to increase, same as singleplayer)
            if is_key_pressed(KeyCode::Comma) {
                self.player_state.adjust_thrust(-0.05);
                log::info!("Host thrust level: {}%", (self.player_state.thrust_level() * 100.0) as i32);
            }
            if is_key_pressed(KeyCode::Period) {
                self.player_state.adjust_thrust(0.05);
                log::info!("Host thrust level: {}%", (self.player_state.thrust_level() * 100.0) as i32);
            }

            // Apply thrust (SPACE key, same as singleplayer)
            let thrust_level = if is_key_down(KeyCode::Space) {
                self.player_state.thrust_level()
            } else {
                0.0
            };

            if let Some(rocket) = self.world.get_rocket_mut(rocket_id) {
                rocket.set_thrust_level(thrust_level);
            }

            // Convert to satellite (C key, same as singleplayer)
            if is_key_pressed(KeyCode::C) {
                if self.world.convert_rocket_to_satellite(rocket_id).is_some() {
                    log::info!("Host converted rocket to satellite");

                    // Spawn new rocket for host at 0 degrees
                    let spawn_position = Self::calculate_spawn_position(0);
                    let mut new_rocket = Rocket::new(
                        spawn_position,
                        Vec2::new(0.0, 0.0),
                        Self::get_player_color(0),
                        GameConstants::ROCKET_BASE_MASS,
                    );
                    new_rocket.set_player_id(Some(0)); // Host is player 0
                    let new_rocket_id = self.world.add_rocket(new_rocket);
                    self.active_rocket_id = Some(new_rocket_id);
                    self.world.set_active_rocket(Some(new_rocket_id));
                    log::info!("Host respawned new rocket");
                }
            }

            // Zoom controls (Q = zoom in, E = zoom out, same as singleplayer)
            if is_key_down(KeyCode::Q) {
                self.camera.adjust_zoom(-0.02); // Zoom in
            }
            if is_key_down(KeyCode::E) {
                self.camera.adjust_zoom(0.02); // Zoom out
            }

            // Mouse wheel zoom (same as singleplayer)
            let mouse_wheel = mouse_wheel().1;
            if mouse_wheel != 0.0 {
                self.camera.adjust_zoom(-mouse_wheel * 0.02);
            }
        }
    }

    /// Update game simulation and broadcast snapshots
    pub fn update(&mut self, delta_time: f32) {
        if self.paused {
            return;
        }

        // Receive any incoming packets from clients
        self.receive_client_packets();

        // Update physics
        self.world.update(delta_time);

        // Update camera to follow host rocket
        if let Some(rocket_id) = self.active_rocket_id {
            if let Some(rocket) = self.world.get_rocket(rocket_id) {
                self.camera.set_center(rocket.position());
            }
        }
        self.camera.update(delta_time);

        // Update snapshot broadcast timer
        self.snapshot_timer += delta_time;
        if self.snapshot_timer >= SNAPSHOT_INTERVAL {
            self.broadcast_snapshot();
            self.snapshot_timer = 0.0;
        }
    }

    /// Receive and process packets from clients
    fn receive_client_packets(&mut self) {
        let mut buf = [0u8; 1024];

        // Process all available packets (non-blocking)
        loop {
            match self.socket.recv_from(&mut buf) {
                Ok((size, src_addr)) => {
                    // Client sent a packet (likely a "join" request or keep-alive)
                    let mut clients = self.clients.lock().unwrap();

                    if !clients.contains_key(&src_addr) && self.next_player_id < 20 {
                        // New client joining
                        let player_id = self.next_player_id;
                        self.next_player_id += 1;

                        clients.insert(src_addr, ConnectedClient {
                            addr: src_addr,
                            player_id,
                            last_seen: get_time(),
                        });

                        log::info!("New client connected: {} assigned player_id {}", src_addr, player_id);

                        // Spawn a rocket for this player at their designated angle
                        drop(clients); // Drop the lock before spawning
                        let spawn_position = Self::calculate_spawn_position(player_id);
                        let mut client_rocket = Rocket::new(
                            spawn_position,
                            Vec2::new(0.0, 0.0),
                            Self::get_player_color(player_id),
                            GameConstants::ROCKET_BASE_MASS,
                        );
                        client_rocket.set_player_id(Some(player_id)); // Tag with player ID
                        let client_rocket_id = self.world.add_rocket(client_rocket);
                        log::info!("Spawned rocket {:?} for player {} at angle {} degrees",
                            client_rocket_id, player_id, player_id * 5);

                        return; // Exit early since we dropped the lock
                    } else if let Some(client) = clients.get_mut(&src_addr) {
                        // Existing client - update last seen time
                        client.last_seen = get_time();
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No more packets available
                    break;
                }
                Err(e) => {
                    log::warn!("Error receiving UDP packet: {}", e);
                    break;
                }
            }
        }
    }

    /// Create GameSaveData snapshot from current world state
    fn create_snapshot(&self) -> GameSaveData {
        let mut save_data = GameSaveData::new();

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

        // Save player state (host is player 0)
        save_data.player_id = Some(0);
        save_data.active_rocket_id = self.active_rocket_id;

        // Save camera state
        save_data.camera = SavedCamera {
            center: self.camera.camera().target.into(),
            zoom: self.camera.zoom_level(),
        };

        save_data
    }

    /// Broadcast current game state snapshot to all connected clients
    fn broadcast_snapshot(&self) {
        // Create snapshot from current world state
        let snapshot = self.create_snapshot();

        // Serialize to bytes
        match snapshot.to_bytes() {
            Ok(bytes) => {
                let clients = self.clients.lock().unwrap();
                let client_count = clients.len();

                if client_count > 0 {
                    log::debug!("Broadcasting snapshot ({} bytes) to {} clients", bytes.len(), client_count);

                    // Send to all connected clients
                    for client in clients.values() {
                        if let Err(e) = self.socket.send_to(&bytes, client.addr) {
                            log::warn!("Failed to send snapshot to {}: {}", client.addr, e);
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to serialize snapshot: {}", e);
            }
        }
    }

    /// Save current game state to disk
    fn save_game(&mut self) {
        let save_name = if let Some(ref name) = self.current_save_name {
            name.clone()
        } else {
            format!("multiplayer_host_{}", get_time() as u64)
        };

        let save_data = self.create_snapshot();

        match save_data.save_to_file(&save_name) {
            Ok(_) => {
                log::info!("Game saved: {}", save_name);
                self.current_save_name = Some(save_name);
            }
            Err(e) => {
                log::error!("Failed to save game: {}", e);
            }
        }
    }

    /// Render the game
    pub fn render(&mut self) {
        // Set camera
        set_camera(self.camera.camera());

        // Render world
        self.world.render();

        // Draw trajectory visualization for host's rocket
        if let Some(rocket_id) = self.active_rocket_id {
            if let Some(rocket) = self.world.get_rocket(rocket_id) {
                let all_planets: Vec<&Planet> = self.world.planets().collect();
                self.vehicle_manager.draw_visualizations(
                    rocket,
                    &all_planets,
                    self.camera.zoom_level(),
                    self.camera.camera(),
                );
            }
        }

        // Reset to default camera for UI
        set_default_camera();

        // Update and draw game info panels
        if let Some(rocket_id) = self.active_rocket_id {
            if let Some(rocket) = self.world.get_rocket(rocket_id) {
                let all_planets: Vec<&Planet> = self.world.planets().collect();
                let satellite_stats = self.world.get_satellite_network_stats();

                self.game_info.update_all_panels(
                    Some(rocket),
                    &all_planets,
                    self.player_state.thrust_level(),
                    true,  // network_connected (hosting)
                    Some(0),  // Host is player 0
                    1,  // player_count (just the host for now)
                    Some(&satellite_stats),
                );
                self.game_info.draw_all_panels();
            }
        }

        // Draw visualization HUD (shows T and G key status)
        self.vehicle_manager.draw_visualization_hud();

        // Show host status at bottom
        let clients = self.clients.lock().unwrap();
        draw_text(
            &format!("HOST | Port: {} | Clients: {}", self.port, clients.len()),
            10.0,
            screen_height() - 20.0,
            20.0,
            GREEN,
        );

        // Show "Press ENTER for controls" at top-right
        let help_text = "Press ENTER for controls";
        let help_w = measure_text(help_text, None, 18, 1.0).width;
        draw_text(help_text, screen_width() - help_w - 20.0, 30.0, 18.0, LIGHTGRAY);

        if self.paused && !self.show_controls {
            draw_text(
                "PAUSED",
                screen_width() / 2.0 - 50.0,
                50.0,
                40.0,
                YELLOW,
            );
        }

        // Draw controls popup if showing
        if self.show_controls {
            self.draw_controls_popup();
        }
    }

    fn draw_controls_popup(&self) {
        let screen_w = screen_width();
        let screen_h = screen_height();
        let popup_w = 800.0;
        let popup_h = 600.0;
        let popup_x = screen_w / 2.0 - popup_w / 2.0;
        let popup_y = screen_h / 2.0 - popup_h / 2.0;

        // Semi-transparent overlay
        draw_rectangle(0.0, 0.0, screen_w, screen_h, Color::new(0.0, 0.0, 0.0, 0.5));

        // Popup background
        draw_rectangle(popup_x, popup_y, popup_w, popup_h, Color::new(0.1, 0.1, 0.1, 0.95));
        // Popup border
        draw_rectangle_lines(popup_x, popup_y, popup_w, popup_h, 3.0, WHITE);

        // Title
        let title = "MULTIPLAYER HOST CONTROLS";
        let title_size = 32.0;
        let title_dims = measure_text(title, None, title_size as u16, 1.0);
        draw_text(
            title,
            popup_x + popup_w / 2.0 - title_dims.width / 2.0,
            popup_y + 40.0,
            title_size,
            WHITE,
        );

        // Controls list - Two columns
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
            ("F", "Save game"),
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

    /// Get connected client count
    pub fn client_count(&self) -> usize {
        self.clients.lock().unwrap().len()
    }
}
