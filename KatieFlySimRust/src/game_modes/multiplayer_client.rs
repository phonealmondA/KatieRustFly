// Multiplayer Client Mode - Connects to host and receives snapshots via UDP
// Runs local predicted simulation and syncs with host snapshots

use macroquad::prelude::*;
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::collections::{HashSet, HashMap};
use serde::{Deserialize, Serialize};

use crate::entities::{GameObject, Planet, Rocket, Satellite};
use crate::game_constants::GameConstants;
use crate::save_system::{GameSaveData, SavedCamera, SavedPlanet, SavedRocket, SavedSatellite, SavedBullet};
use crate::systems::{World, EntityId, VehicleManager, PlayerInput, PlayerInputState};
use crate::ui::{Camera, GameInfoDisplay};

const KEEPALIVE_INTERVAL: f32 = 5.0; // Send keepalive every 5 seconds

/// Client input packet - sent from client to host
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClientInputPacket {
    player_id: u32,
    rotation_delta: f32,  // degrees per frame
    thrust_level: f32,    // 0.0 to 1.0
    convert_to_satellite: bool,
    shoot_bullet: bool,   // true if client wants to shoot
    save_requested: bool, // true if client pressed F5 (quick save)
    refuel_from_planet: bool, // true if client wants to refuel from planet (R key)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MultiplayerClientResult {
    None,
    ReturnToMenu,
    Quit,
    ConnectionLost,
}

pub struct MultiplayerClient {
    // Core game systems
    world: World,
    camera: Camera,
    vehicle_manager: VehicleManager,
    game_info: GameInfoDisplay,

    // Client player state
    player_input: PlayerInput,
    player_state: PlayerInputState,
    player_id: u32, // Assigned by host
    player_name: String, // This client's player name
    active_rocket_id: Option<EntityId>,

    // Networking
    socket: Arc<UdpSocket>,
    host_addr: SocketAddr,
    keepalive_timer: f32,
    last_snapshot_time: f64,
    connected: bool,
    player_names: HashMap<u32, String>, // Map player IDs to player names

    // Game state
    window_size: Vec2,
    paused: bool,
    show_controls: bool,
    show_quit_confirmation: bool,

    // Network map view
    show_network_map: bool,
    marked_satellites: HashSet<EntityId>,

    // Save celebration (F5 quick save)
    save_celebration_player_id: Option<u32>, // Which player triggered the save
    save_celebration_timer: f32,              // Time remaining for "what a save!!" text
}

impl MultiplayerClient {
    /// Calculate spawn position for a player based on their player ID
    /// Same as host - each player at +5 degrees from previous
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

    /// Create a new multiplayer client and connect to host
    pub fn new(window_size: Vec2, player_name: String, host_ip: &str, host_port: u16) -> Result<Self, String> {
        // Bind to any available local port
        let socket = UdpSocket::bind("0.0.0.0:0")
            .map_err(|e| format!("Failed to bind UDP socket: {}", e))?;

        // Set non-blocking mode
        socket.set_nonblocking(true)
            .map_err(|e| format!("Failed to set non-blocking mode: {}", e))?;

        // Parse host address
        let host_addr: SocketAddr = format!("{}:{}", host_ip, host_port)
            .parse()
            .map_err(|e| format!("Invalid host address: {}", e))?;

        log::info!("Multiplayer client '{}' connecting to {}:{}", player_name, host_ip, host_port);

        // Send initial join packet
        let join_packet = b"JOIN";
        socket.send_to(join_packet, host_addr)
            .map_err(|e| format!("Failed to send join packet: {}", e))?;

        // Initialize player names map with this client's name (will be updated with actual player_id later)
        let mut player_names = HashMap::new();
        player_names.insert(1, player_name.clone()); // Temporary ID 1, will be updated

        Ok(Self {
            world: World::new(),
            camera: Camera::new(window_size),
            vehicle_manager: VehicleManager::new(),
            game_info: GameInfoDisplay::new(),

            player_input: PlayerInput::player1(), // Client uses standard controls
            player_state: PlayerInputState::new(1), // Temporary, will be updated when assigned
            player_id: 1, // Temporary, will be assigned by host from snapshot
            player_name,
            active_rocket_id: None,

            socket: Arc::new(socket),
            host_addr,
            keepalive_timer: 0.0,
            last_snapshot_time: get_time(),
            connected: false,
            player_names,

            window_size,
            paused: false,
            show_controls: false,
            show_quit_confirmation: false,

            show_network_map: false,
            marked_satellites: HashSet::new(),

            save_celebration_player_id: None,
            save_celebration_timer: 0.0,
        })
    }

    /// Handle input for the client player
    pub fn handle_input(&mut self) -> MultiplayerClientResult {
        // Handle quit confirmation popup buttons
        if self.show_quit_confirmation {
            if is_mouse_button_pressed(MouseButton::Left) {
                let mouse_pos = mouse_position();
                let screen_w = screen_width();
                let screen_h = screen_height();
                let popup_w = 400.0;
                let popup_h = 200.0;
                let popup_x = screen_w / 2.0 - popup_w / 2.0;
                let popup_y = screen_h / 2.0 - popup_h / 2.0;

                // Yes button
                let yes_button_x = popup_x + popup_w / 2.0 - 110.0;
                let yes_button_y = popup_y + popup_h - 60.0;
                let button_w = 80.0;
                let button_h = 40.0;

                if mouse_pos.0 >= yes_button_x && mouse_pos.0 <= yes_button_x + button_w &&
                   mouse_pos.1 >= yes_button_y && mouse_pos.1 <= yes_button_y + button_h {
                    log::info!("Quit confirmed - disconnecting from host");
                    return MultiplayerClientResult::ReturnToMenu;
                }

                // No button
                let no_button_x = popup_x + popup_w / 2.0 + 30.0;
                if mouse_pos.0 >= no_button_x && mouse_pos.0 <= no_button_x + button_w &&
                   mouse_pos.1 >= yes_button_y && mouse_pos.1 <= yes_button_y + button_h {
                    self.show_quit_confirmation = false;
                }
            }
        }

        // ESC - close popups or show quit confirmation
        if is_key_pressed(KeyCode::Escape) {
            if self.show_controls {
                self.show_controls = false;
            } else if self.show_quit_confirmation {
                self.show_quit_confirmation = false;
            } else {
                self.show_quit_confirmation = true;
            }
        }

        // Enter - toggle controls menu (game keeps running in background)
        if is_key_pressed(KeyCode::Enter) {
            self.show_controls = !self.show_controls;
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

        // P - pause/unpause (local only, doesn't affect host, only if controls not showing)
        if is_key_pressed(KeyCode::P) && !self.show_controls {
            self.paused = !self.paused;
            log::info!("Local view {}", if self.paused { "paused" } else { "unpaused" });
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

        // F5 - quick save (sends request to host)
        // Note: F5 will be sent to host via input packet in handle_player_controls()
        // Celebration will be triggered when host processes it

        // Only process game controls if not paused
        if !self.paused {
            self.handle_player_controls();
        }

        MultiplayerClientResult::None
    }

    fn handle_player_controls(&mut self) {
        if let Some(rocket_id) = self.active_rocket_id {
            // Build input packet from current controls
            let mut rotation_delta = 0.0;
            if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
                rotation_delta = 3.0; // degrees per frame
            }
            if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
                rotation_delta = -3.0;
            }

            // Apply rotation locally for prediction
            if rotation_delta != 0.0 {
                let rotation_radians = rotation_delta * std::f32::consts::PI / 180.0;
                if let Some(rocket) = self.world.get_rocket_mut(rocket_id) {
                    rocket.rotate(rotation_radians);
                }
            }

            // Thrust adjustment (comma to decrease, period to increase)
            if is_key_pressed(KeyCode::Comma) {
                self.player_state.adjust_thrust(-0.05);
                log::info!("Client thrust level: {}%", (self.player_state.thrust_level() * 100.0) as i32);
            }
            if is_key_pressed(KeyCode::Period) {
                self.player_state.adjust_thrust(0.05);
                log::info!("Client thrust level: {}%", (self.player_state.thrust_level() * 100.0) as i32);
            }

            // Apply thrust (SPACE key)
            let thrust_level = if is_key_down(KeyCode::Space) {
                self.player_state.thrust_level()
            } else {
                0.0
            };

            // Apply thrust locally for prediction
            if let Some(rocket) = self.world.get_rocket_mut(rocket_id) {
                rocket.set_thrust_level(thrust_level);
            }

            // Convert to satellite (C key)
            let convert_to_satellite = is_key_pressed(KeyCode::C);
            if convert_to_satellite {
                log::info!("Client requesting satellite conversion");
            }

            // Shoot bullet (W key for multiplayer, X for singleplayer)
            // Don't shoot locally - send to host and let it handle authoritative shooting
            let shoot_bullet = is_key_pressed(KeyCode::W);
            if shoot_bullet {
                log::info!("Client requesting bullet shot");
            }

            // Quick save (F5 key) - sends request to host
            let save_requested = is_key_pressed(KeyCode::F5);
            if save_requested {
                log::info!("Client requesting quick save (F5)");
                // Trigger celebration locally immediately
                self.save_celebration_player_id = Some(self.player_id);
                self.save_celebration_timer = 5.0;
            }

            // Refuel from planet (R key)
            let refuel_from_planet = is_key_down(KeyCode::R);

            // Send input packet to host
            let input_packet = ClientInputPacket {
                player_id: self.player_id,
                rotation_delta,
                thrust_level,
                convert_to_satellite,
                shoot_bullet,
                save_requested,
                refuel_from_planet,
            };

            if let Ok(bytes) = bincode::serialize(&input_packet) {
                if let Err(e) = self.socket.send_to(&bytes, self.host_addr) {
                    log::warn!("Failed to send input packet: {}", e);
                }
            }

            // Zoom controls (local only, doesn't affect game state)
            // Q removed - was causing crashes
            if is_key_down(KeyCode::E) {
                self.camera.adjust_zoom(0.02); // Zoom out
            }

            // Mouse wheel zoom (local only)
            let mouse_wheel = mouse_wheel().1;
            if mouse_wheel != 0.0 {
                self.camera.adjust_zoom(-mouse_wheel * 0.02);
            }
        }
    }

    /// Update game simulation and network sync
    pub fn update(&mut self, delta_time: f32) {
        // Send keepalive packets to host
        self.keepalive_timer += delta_time;
        if self.keepalive_timer >= KEEPALIVE_INTERVAL {
            self.send_keepalive();
            self.keepalive_timer = 0.0;
        }

        // Receive snapshots from host
        self.receive_snapshots();

        // Check for connection timeout (no snapshot for 30 seconds)
        let time_since_snapshot = get_time() - self.last_snapshot_time;
        if time_since_snapshot > 30.0 && self.connected {
            log::warn!("Connection to host lost (no snapshots for 30 seconds)");
            self.connected = false;
        }

        if self.paused {
            return;
        }

        // Run local predicted simulation
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

            // If this was our rocket (this client's player), update active_rocket_id
            if player_id == self.player_id {
                self.active_rocket_id = Some(new_rocket_id);
                self.world.set_active_rocket(Some(new_rocket_id));
            }

            log::info!("Respawned new rocket {} for player {}", new_rocket_id, player_id);
        }

        // Update save celebration timer
        if self.save_celebration_timer > 0.0 {
            self.save_celebration_timer -= delta_time;
            if self.save_celebration_timer <= 0.0 {
                self.save_celebration_player_id = None;
            }
        }

        // Update camera to follow client rocket
        if let Some(rocket_id) = self.active_rocket_id {
            if let Some(rocket) = self.world.get_rocket(rocket_id) {
                self.camera.set_center(rocket.position());
            }
        }
        self.camera.update(delta_time);
    }

    /// Send keepalive packet to host
    fn send_keepalive(&self) {
        let keepalive_packet = b"KEEPALIVE";
        if let Err(e) = self.socket.send_to(keepalive_packet, self.host_addr) {
            log::warn!("Failed to send keepalive: {}", e);
        }
    }

    /// Receive and apply snapshots from host
    fn receive_snapshots(&mut self) {
        let mut buf = vec![0u8; 4096]; // Larger buffer for snapshots

        // Process all available packets
        loop {
            match self.socket.recv_from(&mut buf) {
                Ok((size, _src_addr)) => {
                    // Received snapshot from host
                    match GameSaveData::from_bytes(&buf[..size]) {
                        Ok(snapshot) => {
                            self.apply_snapshot(snapshot);
                            self.last_snapshot_time = get_time();
                            self.connected = true;
                        }
                        Err(e) => {
                            log::warn!("Failed to deserialize snapshot: {}", e);
                        }
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

    /// Apply received snapshot to local world state
    fn apply_snapshot(&mut self, snapshot: GameSaveData) {
        log::debug!("Applying snapshot from host");

        // Clear existing world
        self.world.clear_all_entities();

        // Load planets with their original IDs
        for saved_planet in snapshot.planets {
            let (id, planet) = saved_planet.to_planet();
            self.world.add_planet_with_id(id, planet);
        }

        // Load rockets with their original IDs and find ours
        let mut my_rocket_id: Option<EntityId> = None;
        let mut highest_player_id: u32 = 0;

        for saved_rocket in snapshot.rockets {
            let (id, rocket) = saved_rocket.to_rocket();

            // Track highest player_id
            if let Some(pid) = rocket.player_id() {
                if pid > highest_player_id {
                    highest_player_id = pid;
                }

                // Check if this rocket belongs to us
                if pid == self.player_id {
                    my_rocket_id = Some(id);
                    log::debug!("Found my rocket (player {}): {:?}", self.player_id, id);
                }
            }

            self.world.add_rocket_with_id(id, rocket);
        }

        // If this is our first snapshot and we haven't found our rocket,
        // we're probably the newest client, so use the highest player_id
        if my_rocket_id.is_none() && self.active_rocket_id.is_none() && highest_player_id > 0 {
            self.player_id = highest_player_id;
            self.player_state = PlayerInputState::new(highest_player_id);
            log::info!("Assigned player ID from snapshot: {}", highest_player_id);

            // Find the rocket with this player_id
            for (id, rocket) in self.world.rockets_with_ids() {
                if rocket.player_id() == Some(highest_player_id) {
                    my_rocket_id = Some(id);
                    log::debug!("Found my rocket: {:?}", id);
                    break;
                }
            }
        }

        // Load satellites with their original IDs
        for saved_satellite in snapshot.satellites {
            let (id, satellite) = saved_satellite.to_satellite();
            self.world.add_satellite_with_id(id, satellite);
        }

        // Load bullets with their original IDs
        for saved_bullet in snapshot.bullets {
            let (id, bullet) = saved_bullet.to_bullet();
            self.world.add_bullet_with_id(id, bullet);
        }

        // Update our active rocket to the one that belongs to us
        if let Some(rocket_id) = my_rocket_id {
            self.active_rocket_id = Some(rocket_id);
            self.world.set_active_rocket(Some(rocket_id));
        } else if self.active_rocket_id.is_none() {
            // If we haven't found our rocket yet, this might be the first snapshot
            // before the host has spawned our rocket. Keep waiting.
            log::debug!("Haven't found my rocket yet (player {}), waiting for host to spawn it", self.player_id);
        }

        // Note: We keep our local camera instead of using snapshot camera
        // This gives the client freedom to look around independently
    }

    fn draw_network_map(&mut self) {
        use crate::utils::vector_helper;

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
        for (_rocket_id, rocket) in self.world.rockets_with_ids() {
            let map_pos = world_to_map(rocket.position());
            let rocket_size = 6.0;

            // Get player color - use get_trajectory_color for clients
            let player_color = if let Some(player_id) = rocket.player_id() {
                Self::get_trajectory_color(player_id)
            } else {
                WHITE
            };

            // Bright player color dot
            draw_circle(map_pos.x, map_pos.y, rocket_size, player_color);
            draw_circle_lines(map_pos.x, map_pos.y, rocket_size, 2.0, Color::new(0.0, 1.0, 0.0, 1.0));

            // Label (show player name if available, otherwise P{id})
            if let Some(player_id) = rocket.player_id() {
                let label = self.player_names.get(&player_id)
                    .map(|name| name.clone())
                    .unwrap_or_else(|| format!("P{}", player_id));
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
                for (_rocket_id, rocket) in self.world.rockets_with_ids() {
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
                        break;
                    }
                }

                // Check satellite collisions
                for (_sat_id, satellite) in &satellites {
                    let distance = (predicted_pos - satellite.position()).length();
                    let satellite_radius = 7.0;
                    if distance < satellite_radius + 3.0 {
                        // Collision predicted! Draw warning marker
                        let collision_map_pos = world_to_map(predicted_pos);
                        draw_circle(collision_map_pos.x, collision_map_pos.y, 8.0, Color::new(1.0, 0.5, 0.0, 0.8)); // Orange warning
                        draw_circle_lines(collision_map_pos.x, collision_map_pos.y, 8.0, 2.0, Color::new(1.0, 0.0, 0.0, 1.0)); // Red outline
                        draw_text("!", collision_map_pos.x - 3.0, collision_map_pos.y + 5.0, 20.0, RED);
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
        let title = "MULTIPLAYER CLIENT CONTROLS";
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
            ("W", "Fire bullet"),
            ("P", "Pause/Unpause (local)"),
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
            ("F5", "Request quick save"),
            ("ENTER", "Toggle this menu"),
            ("ESC", "Disconnect"),
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

        // Footer
        draw_text(
            "Press ESC or ENTER to close this menu",
            popup_x + popup_w / 2.0 - 160.0,
            popup_y + popup_h - 30.0,
            18.0,
            LIGHTGRAY,
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

        // Store celebration rocket position for screen-space rendering
        let celebration_screen_pos = if let Some(player_id) = self.save_celebration_player_id {
            // Find the rocket belonging to this player
            self.world.rockets_with_ids()
                .find(|(_id, rocket)| rocket.player_id() == Some(player_id))
                .map(|(_id, rocket)| self.camera.world_to_screen(rocket.position()))
        } else {
            None
        };

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
                    self.connected,  // network_connected
                    Some(self.player_id as usize),  // Client player ID
                    1,  // player_count (just the client for now)
                    Some(&satellite_stats),
                );
                self.game_info.draw_all_panels();
            }
        }

        // Show connection status at bottom
        let status_color = if self.connected { GREEN } else { RED };
        let status_text = if self.connected {
            format!("CLIENT | Connected to {} | Player {}", self.host_addr, self.player_id)
        } else {
            format!("CLIENT | Disconnected | Last seen: {:.1}s ago", get_time() - self.last_snapshot_time)
        };

        draw_text(
            &status_text,
            10.0,
            screen_height() - 20.0,
            20.0,
            status_color,
        );

        // Show "Press ENTER for controls" at top-right
        let help_text = "Press ENTER for controls";
        let help_w = measure_text(help_text, None, 18, 1.0).width;
        draw_text(help_text, screen_width() - help_w - 20.0, 30.0, 18.0, LIGHTGRAY);

        if self.paused {
            draw_text(
                "PAUSED (Local)",
                screen_width() / 2.0 - 80.0,
                50.0,
                40.0,
                YELLOW,
            );
        }

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

        // Draw quit confirmation popup if showing
        if self.show_quit_confirmation {
            self.draw_quit_confirmation();
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

    fn draw_quit_confirmation(&self) {
        let screen_w = screen_width();
        let screen_h = screen_height();
        let popup_w = 400.0;
        let popup_h = 200.0;
        let popup_x = screen_w / 2.0 - popup_w / 2.0;
        let popup_y = screen_h / 2.0 - popup_h / 2.0;

        // Semi-transparent overlay
        draw_rectangle(0.0, 0.0, screen_w, screen_h, Color::new(0.0, 0.0, 0.0, 0.6));

        // Popup background
        draw_rectangle(popup_x, popup_y, popup_w, popup_h, Color::new(0.15, 0.15, 0.2, 0.95));
        // Popup border
        draw_rectangle_lines(popup_x, popup_y, popup_w, popup_h, 3.0, Color::new(1.0, 0.3, 0.3, 1.0));

        // Title
        let title = "Quit Game?";
        let title_size = 36.0;
        let title_dims = measure_text(title, None, title_size as u16, 1.0);
        draw_text(
            title,
            popup_x + popup_w / 2.0 - title_dims.width / 2.0,
            popup_y + 60.0,
            title_size,
            WHITE,
        );

        // Message
        let message = "Do you want to quit the game?";
        let message_size = 20.0;
        let message_dims = measure_text(message, None, message_size as u16, 1.0);
        draw_text(
            message,
            popup_x + popup_w / 2.0 - message_dims.width / 2.0,
            popup_y + 105.0,
            message_size,
            LIGHTGRAY,
        );

        // Buttons
        let button_w = 80.0;
        let button_h = 40.0;
        let yes_button_x = popup_x + popup_w / 2.0 - 110.0;
        let no_button_x = popup_x + popup_w / 2.0 + 30.0;
        let button_y = popup_y + popup_h - 60.0;

        // Check if mouse is hovering over buttons
        let mouse_pos = mouse_position();
        let yes_hover = mouse_pos.0 >= yes_button_x && mouse_pos.0 <= yes_button_x + button_w &&
                        mouse_pos.1 >= button_y && mouse_pos.1 <= button_y + button_h;
        let no_hover = mouse_pos.0 >= no_button_x && mouse_pos.0 <= no_button_x + button_w &&
                       mouse_pos.1 >= button_y && mouse_pos.1 <= button_y + button_h;

        // Yes button
        let yes_color = if yes_hover {
            Color::from_rgba(180, 50, 50, 255)
        } else {
            Color::from_rgba(150, 40, 40, 255)
        };
        draw_rectangle(yes_button_x, button_y, button_w, button_h, yes_color);
        draw_rectangle_lines(yes_button_x, button_y, button_w, button_h, 2.0, WHITE);
        let yes_text = "Yes";
        let yes_dims = measure_text(yes_text, None, 24, 1.0);
        draw_text(
            yes_text,
            yes_button_x + button_w / 2.0 - yes_dims.width / 2.0,
            button_y + button_h / 2.0 + 8.0,
            24.0,
            WHITE,
        );

        // No button
        let no_color = if no_hover {
            Color::from_rgba(50, 150, 50, 255)
        } else {
            Color::from_rgba(40, 120, 40, 255)
        };
        draw_rectangle(no_button_x, button_y, button_w, button_h, no_color);
        draw_rectangle_lines(no_button_x, button_y, button_w, button_h, 2.0, WHITE);
        let no_text = "No";
        let no_dims = measure_text(no_text, None, 24, 1.0);
        draw_text(
            no_text,
            no_button_x + button_w / 2.0 - no_dims.width / 2.0,
            button_y + button_h / 2.0 + 8.0,
            24.0,
            WHITE,
        );

        // Footer hint
        draw_text(
            "Press ESC to cancel",
            popup_x + popup_w / 2.0 - 75.0,
            popup_y + popup_h - 15.0,
            16.0,
            LIGHTGRAY,
        );
    }

    /// Check if connected to host
    pub fn is_connected(&self) -> bool {
        self.connected
    }
}
