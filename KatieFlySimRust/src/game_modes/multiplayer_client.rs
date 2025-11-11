// Multiplayer Client Mode - Connects to host and receives snapshots via UDP
// Runs local predicted simulation and syncs with host snapshots

use macroquad::prelude::*;
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

use crate::entities::{Planet, Rocket, Satellite};
use crate::game_constants::GameConstants;
use crate::save_system::{GameSaveData, SavedCamera, SavedPlanet, SavedRocket, SavedSatellite};
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
    active_rocket_id: Option<EntityId>,

    // Networking
    socket: Arc<UdpSocket>,
    host_addr: SocketAddr,
    keepalive_timer: f32,
    last_snapshot_time: f64,
    connected: bool,

    // Game state
    window_size: Vec2,
    paused: bool,
}

impl MultiplayerClient {
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
    pub fn new(window_size: Vec2, host_ip: &str, host_port: u16) -> Result<Self, String> {
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

        log::info!("Multiplayer client connecting to {}:{}", host_ip, host_port);

        // Send initial join packet
        let join_packet = b"JOIN";
        socket.send_to(join_packet, host_addr)
            .map_err(|e| format!("Failed to send join packet: {}", e))?;

        Ok(Self {
            world: World::new(),
            camera: Camera::new(window_size),
            vehicle_manager: VehicleManager::new(),
            game_info: GameInfoDisplay::new(),

            player_input: PlayerInput::player1(), // Client uses standard controls
            player_state: PlayerInputState::new(1), // Temporary, will be updated when assigned
            player_id: 1, // Temporary, will be assigned by host from snapshot
            active_rocket_id: None,

            socket: Arc::new(socket),
            host_addr,
            keepalive_timer: 0.0,
            last_snapshot_time: get_time(),
            connected: false,

            window_size,
            paused: false,
        })
    }

    /// Handle input for the client player
    pub fn handle_input(&mut self) -> MultiplayerClientResult {
        // ESC - return to menu
        if is_key_pressed(KeyCode::Escape) {
            log::info!("ESC pressed - disconnecting from host");
            return MultiplayerClientResult::ReturnToMenu;
        }

        // Q - quit
        if is_key_pressed(KeyCode::Q) {
            return MultiplayerClientResult::Quit;
        }

        // P - pause/unpause (local only, doesn't affect host)
        if is_key_pressed(KeyCode::P) {
            self.paused = !self.paused;
            log::info!("Local view {}", if self.paused { "paused" } else { "unpaused" });
        }

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

            // Send input packet to host
            let input_packet = ClientInputPacket {
                player_id: self.player_id,
                rotation_delta,
                thrust_level,
                convert_to_satellite,
            };

            if let Ok(bytes) = bincode::serialize(&input_packet) {
                if let Err(e) = self.socket.send_to(&bytes, self.host_addr) {
                    log::warn!("Failed to send input packet: {}", e);
                }
            }

            // Zoom controls (local only, doesn't affect game state)
            if is_key_down(KeyCode::Q) {
                self.camera.adjust_zoom(-0.02); // Zoom in
            }
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
    }

    /// Check if connected to host
    pub fn is_connected(&self) -> bool {
        self.connected
    }
}
