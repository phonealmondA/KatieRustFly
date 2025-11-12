// Multiplayer Host Mode - Authoritative server broadcasting snapshots via UDP
// Runs the simulation and broadcasts state at ~60Hz (every frame) for real-time sync

use macroquad::prelude::*;
use std::net::{SocketAddr, UdpSocket};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

use crate::entities::{GameObject, Planet, Rocket, Satellite};
use crate::game_constants::GameConstants;
use crate::save_system::{GameSaveData, SavedCamera, SavedPlanet, SavedRocket, SavedSatellite, SavedBullet};
use crate::systems::{World, EntityId, VehicleManager, PlayerInput, PlayerInputState};
use crate::ui::{Camera, GameInfoDisplay};
use crate::utils::vector_helper;

const SNAPSHOT_INTERVAL: f32 = 1.0 / 60.0; // ~16.67ms between broadcasts (~60 Hz) for real-time sync

/// Client input packet - sent from client to host
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClientInputPacket {
    player_id: u32,
    rotation_delta: f32,  // degrees per frame
    thrust_level: f32,    // 0.0 to 1.0
    convert_to_satellite: bool,
    shoot_bullet: bool,   // true if client wants to shoot
    save_requested: bool, // true if client pressed F5 (quick save)
}

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

    // Network map view
    show_network_map: bool,
    marked_satellites: HashSet<EntityId>,

    // Save celebration (F5 quick save)
    save_celebration_player_id: Option<u32>, // Which player triggered the save
    save_celebration_timer: f32,              // Time remaining for "what a save!!" text
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

    /// Get trajectory color for a player (semi-transparent version)
    fn get_trajectory_color(player_id: u32) -> Color {
        match player_id {
            0 => Color::new(1.0, 0.4, 0.4, 0.6), // Red with transparency
            1 => Color::new(0.4, 0.4, 1.0, 0.6), // Blue with transparency
            2 => Color::new(0.4, 1.0, 0.4, 0.6), // Green with transparency
            3 => Color::new(1.0, 1.0, 0.4, 0.6), // Yellow with transparency
            4 => Color::new(1.0, 0.4, 1.0, 0.6), // Magenta with transparency
            5 => Color::new(0.4, 1.0, 1.0, 0.6), // Cyan with transparency
            _ => Color::new(0.8, 0.8, 0.8, 0.6), // Gray with transparency
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

            show_network_map: false,
            marked_satellites: HashSet::new(),

            save_celebration_player_id: None,
            save_celebration_timer: 0.0,
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

        // Load bullets with their original IDs
        for saved_bullet in save_data.bullets {
            let (id, bullet) = saved_bullet.to_bullet();
            self.world.add_bullet_with_id(id, bullet);
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
            self.show_network_map = !self.show_network_map;
            log::info!("Toggled network map: {}", self.show_network_map);
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

        // F5 - quick save (triggers "what a save!!" celebration)
        if is_key_pressed(KeyCode::F5) {
            self.quick_save(0); // Host is player 0
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

            // Shoot bullet (W key for multiplayer, X for singleplayer)
            if is_key_pressed(KeyCode::W) {
                if let Some(bullet_id) = self.world.shoot_bullet_from_rocket(rocket_id) {
                    log::info!("Bullet {} fired from rocket {}", bullet_id, rocket_id);
                } else {
                    log::info!("Cannot shoot: not enough fuel (need 1 unit)");
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

    /// Apply client input to their rocket
    fn apply_client_input(&mut self, input: ClientInputPacket) {
        // Find the rocket that belongs to this player
        let mut rocket_id: Option<EntityId> = None;
        for (id, rocket) in self.world.rockets_with_ids() {
            if rocket.player_id() == Some(input.player_id) {
                rocket_id = Some(id);
                break;
            }
        }

        if let Some(rid) = rocket_id {
            // Apply rotation
            if input.rotation_delta != 0.0 {
                let rotation_radians = input.rotation_delta * std::f32::consts::PI / 180.0;
                if let Some(rocket) = self.world.get_rocket_mut(rid) {
                    rocket.rotate(rotation_radians);
                }
            }

            // Apply thrust
            if let Some(rocket) = self.world.get_rocket_mut(rid) {
                rocket.set_thrust_level(input.thrust_level);
            }

            // Convert to satellite if requested
            if input.convert_to_satellite {
                if self.world.convert_rocket_to_satellite(rid).is_some() {
                    log::info!("Player {} converted rocket to satellite", input.player_id);

                    // Spawn new rocket for this player
                    let spawn_position = Self::calculate_spawn_position(input.player_id);
                    let mut new_rocket = Rocket::new(
                        spawn_position,
                        Vec2::new(0.0, 0.0),
                        Self::get_player_color(input.player_id),
                        GameConstants::ROCKET_BASE_MASS,
                    );
                    new_rocket.set_player_id(Some(input.player_id));
                    self.world.add_rocket(new_rocket);
                    log::info!("Respawned new rocket for player {}", input.player_id);
                }
            }

            // Shoot bullet if requested
            if input.shoot_bullet {
                if let Some(bullet_id) = self.world.shoot_bullet_from_rocket(rid) {
                    log::info!("Player {} fired bullet {}", input.player_id, bullet_id);
                } else {
                    log::debug!("Player {} cannot shoot: not enough fuel", input.player_id);
                }
            }

            // Quick save if requested (F5 key)
            if input.save_requested {
                self.quick_save(input.player_id);
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

        // Handle rockets destroyed by bullets (respawn like 'C' key, but without satellite)
        let destroyed_rockets = self.world.take_destroyed_rockets();
        for destroyed in destroyed_rockets {
            let player_id = destroyed.player_id.unwrap_or(0);
            log::info!("Player {} rocket destroyed by bullet, respawning", player_id);

            // Spawn new rocket for this player (same as 'C' key respawn logic)
            let spawn_position = Self::calculate_spawn_position(player_id);
            let mut new_rocket = Rocket::new(
                spawn_position,
                Vec2::new(0.0, 0.0),
                Self::get_player_color(player_id),
                GameConstants::ROCKET_BASE_MASS,
            );
            new_rocket.set_player_id(Some(player_id));
            let new_rocket_id = self.world.add_rocket(new_rocket);

            // If this was the host's rocket (player 0), update active_rocket_id
            if player_id == 0 {
                self.active_rocket_id = Some(new_rocket_id);
                self.world.set_active_rocket(Some(new_rocket_id));
            }

            log::info!("Respawned new rocket {} for player {}", new_rocket_id, player_id);
        }

        // Update camera to follow host rocket
        if let Some(rocket_id) = self.active_rocket_id {
            if let Some(rocket) = self.world.get_rocket(rocket_id) {
                self.camera.set_center(rocket.position());
            }
        }
        self.camera.update(delta_time);

        // Update save celebration timer
        if self.save_celebration_timer > 0.0 {
            self.save_celebration_timer -= delta_time;
            if self.save_celebration_timer <= 0.0 {
                self.save_celebration_player_id = None;
            }
        }

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
                    // Try to parse as input packet first
                    if let Ok(input_packet) = bincode::deserialize::<ClientInputPacket>(&buf[..size]) {
                        // This is a client input packet - apply it to their rocket
                        self.apply_client_input(input_packet);

                        // Update last seen time
                        let mut clients = self.clients.lock().unwrap();
                        if let Some(client) = clients.get_mut(&src_addr) {
                            client.last_seen = get_time();
                        }
                        continue;
                    }

                    // Not an input packet, check if it's a join/keepalive packet
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

        // Save all bullets with their IDs
        save_data.bullets = self.world.bullets_with_ids()
            .map(|(id, bullet)| SavedBullet::from_bullet(id, bullet))
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

        match save_data.save_to_multi_file(&save_name) {
            Ok(_) => {
                log::info!("Multiplayer game saved: {}", save_name);
                self.current_save_name = Some(save_name);
            }
            Err(e) => {
                log::error!("Failed to save multiplayer game: {}", e);
            }
        }
    }

    /// Quick save triggered by F5 key - saves and shows "what a save!!" celebration
    fn quick_save(&mut self, player_id: u32) {
        let save_data = self.create_snapshot();

        match save_data.save_to_multi_file("quicksave") {
            Ok(_) => {
                log::info!("Quick save successful (triggered by player {})", player_id);
                self.current_save_name = Some("quicksave".to_string());

                // Trigger save celebration
                self.save_celebration_player_id = Some(player_id);
                self.save_celebration_timer = 5.0; // Show for 5 seconds
            }
            Err(e) => {
                log::error!("Failed to quick save: {}", e);
            }
        }
    }

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

        // Draw all player rockets (different colors per player)
        for (rocket_id, rocket) in self.world.rockets_with_ids() {
            let map_pos = world_to_map(rocket.position());
            let rocket_size = 6.0;

            // Get player color
            let player_color = if let Some(player_id) = rocket.player_id() {
                Self::get_player_color(player_id)
            } else {
                WHITE
            };

            // Bright player color dot
            draw_circle(map_pos.x, map_pos.y, rocket_size, player_color);
            draw_circle_lines(map_pos.x, map_pos.y, rocket_size, 2.0, Color::new(0.0, 1.0, 0.0, 1.0));

            // Label
            if let Some(player_id) = rocket.player_id() {
                let label = format!("P{}", player_id);
                draw_text(&label, map_pos.x - 10.0, map_pos.y - 10.0, 12.0, WHITE);
            }
        }

        // Draw connection lines between satellites in range
        let satellite_transfer_range = GameConstants::SATELLITE_TRANSFER_RANGE;
        let satellites: Vec<_> = self.world.satellites_with_ids().collect();

        for i in 0..satellites.len() {
            for j in (i + 1)..satellites.len() {
                let (_id1, sat1) = satellites[i];
                let (_id2, sat2) = satellites[j];

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

        // Draw bullet trajectories (red lines showing curved path) - same red color for all players
        let bullets: Vec<_> = self.world.bullets_with_ids().collect();
        let rockets: Vec<_> = self.world.rockets_with_ids().collect();
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

            // Check for predicted collisions with rockets and satellites
            for &predicted_pos in &predicted_positions {
                // Check rocket collisions
                for (rocket_id, rocket) in &rockets {
                    if rocket.is_landed() {
                        continue; // Skip landed rockets
                    }
                    let distance = (predicted_pos - rocket.position()).length();
                    let rocket_radius = 12.0;
                    if distance < rocket_radius + 3.0 {
                        // Collision predicted! Draw warning marker
                        let collision_map_pos = world_to_map(predicted_pos);
                        draw_circle(collision_map_pos.x, collision_map_pos.y, 8.0, Color::new(1.0, 0.5, 0.0, 0.8)); // Orange warning
                        draw_circle_lines(collision_map_pos.x, collision_map_pos.y, 8.0, 2.0, Color::new(1.0, 0.0, 0.0, 1.0)); // Red outline
                        draw_text("!", collision_map_pos.x - 3.0, collision_map_pos.y + 5.0, 20.0, RED);
                        log::debug!("Collision predicted: bullet will hit rocket {}", rocket_id);
                        break;
                    }
                }

                // Check satellite collisions
                for (sat_id, satellite) in &satellites {
                    let distance = (predicted_pos - satellite.position()).length();
                    let satellite_radius = 7.0;
                    if distance < satellite_radius + 3.0 {
                        // Collision predicted! Draw warning marker
                        let collision_map_pos = world_to_map(predicted_pos);
                        draw_circle(collision_map_pos.x, collision_map_pos.y, 8.0, Color::new(1.0, 0.5, 0.0, 0.8)); // Orange warning
                        draw_circle_lines(collision_map_pos.x, collision_map_pos.y, 8.0, 2.0, Color::new(1.0, 0.0, 0.0, 1.0)); // Red outline
                        draw_text("!", collision_map_pos.x - 3.0, collision_map_pos.y + 5.0, 20.0, RED);
                        log::debug!("Collision predicted: bullet will hit satellite {}", sat_id);
                        break;
                    }
                }
            }
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
        // Set camera
        set_camera(self.camera.camera());

        // Render world
        self.world.render();

        // Draw trajectory visualizations for all players' rockets with their colors
        let all_planets: Vec<&Planet> = self.world.planets().collect();
        for (id, rocket) in self.world.rockets_with_ids() {
            if let Some(player_id) = rocket.player_id() {
                let trajectory_color = Self::get_trajectory_color(player_id);
                self.vehicle_manager.draw_visualizations_with_color(
                    rocket,
                    &all_planets,
                    self.camera.zoom_level(),
                    self.camera.camera(),
                    Some(trajectory_color),
                );
            }
        }

        // Draw "what a save!!" celebration text above player who triggered save
        if let Some(player_id) = self.save_celebration_player_id {
            // Find the rocket belonging to this player
            for (_id, rocket) in self.world.rockets_with_ids() {
                if rocket.player_id() == Some(player_id) {
                    let rocket_pos = rocket.position();
                    let text = "what a save!!";
                    let text_size = 30.0;
                    let text_offset_y = -80.0; // Above rocket

                    // Calculate text dimensions for centering
                    let text_dims = measure_text(text, None, text_size as u16, 1.0);

                    // Draw text with outline for visibility
                    let text_x = rocket_pos.x - text_dims.width / 2.0;
                    let text_y = rocket_pos.y + text_offset_y;

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

                    break; // Only draw for one rocket
                }
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
        {
            let clients = self.clients.lock().unwrap();
            draw_text(
                &format!("HOST | Port: {} | Clients: {}", self.port, clients.len()),
                10.0,
                screen_height() - 20.0,
                20.0,
                GREEN,
            );
        }

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

        // Draw network map popup if showing
        if self.show_network_map {
            self.draw_network_map();
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
