// Multiplayer Client Mode - Connects to host and receives snapshots via UDP
// Runs local predicted simulation and syncs with host snapshots

use macroquad::prelude::*;
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;

use crate::entities::{Planet, Rocket, Satellite};
use crate::game_constants::GameConstants;
use crate::save_system::{GameSaveData, SavedCamera, SavedPlanet, SavedRocket, SavedSatellite};
use crate::systems::{World, EntityId, VehicleManager, PlayerInput, PlayerInputState};
use crate::ui::{Camera, GameInfoDisplay};

const KEEPALIVE_INTERVAL: f32 = 5.0; // Send keepalive every 5 seconds

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
            player_state: PlayerInputState::new(0), // Will be updated when assigned
            player_id: 0, // Will be assigned by host
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
            // Rotation
            if is_key_down(self.player_input.rotate_left) {
                if let Some(rocket) = self.world.get_rocket_mut(rocket_id) {
                    let rotation_speed = 3.0_f32.to_radians();
                    rocket.rotate(-rotation_speed);
                }
            }
            if is_key_down(self.player_input.rotate_right) {
                if let Some(rocket) = self.world.get_rocket_mut(rocket_id) {
                    let rotation_speed = 3.0_f32.to_radians();
                    rocket.rotate(rotation_speed);
                }
            }

            // Thrust adjustment
            if is_key_pressed(self.player_input.decrease_thrust) {
                self.player_state.adjust_thrust(-0.05);
            }
            if is_key_pressed(self.player_input.increase_thrust) {
                self.player_state.adjust_thrust(0.05);
            }

            // Apply thrust
            if is_key_down(self.player_input.thrust) {
                if let Some(rocket) = self.world.get_rocket_mut(rocket_id) {
                    rocket.apply_thrust(self.player_state.selected_thrust_level);
                }
            }

            // Convert to satellite
            if is_key_pressed(self.player_input.convert_to_satellite) {
                if self.world.convert_rocket_to_satellite(rocket_id).is_some() {
                    log::info!("Client converted rocket to satellite");

                    // Spawn new rocket for client
                    let spawn_distance = GameConstants::MAIN_PLANET_RADIUS + 200.0;
                    let spawn_position = Vec2::new(
                        GameConstants::MAIN_PLANET_X + spawn_distance,
                        GameConstants::MAIN_PLANET_Y,
                    );
                    let new_rocket = Rocket::new(
                        spawn_position,
                        Vec2::new(0.0, 0.0),
                        Color::from_rgba(100, 100, 255, 255), // Blue for client
                        GameConstants::ROCKET_BASE_MASS,
                    );
                    let new_rocket_id = self.world.add_rocket(new_rocket);
                    self.active_rocket_id = Some(new_rocket_id);
                    self.world.set_active_rocket(Some(new_rocket_id));
                    log::info!("Client respawned new rocket");
                }
            }

            // Zoom controls
            if is_key_down(self.player_input.zoom_out) {
                self.camera.adjust_zoom(1.02);
            }
            if is_key_down(self.player_input.zoom_in) {
                self.camera.adjust_zoom(0.98);
            }

            // Mouse wheel zoom
            let (_mouse_wheel_x, mouse_wheel_y) = mouse_wheel();
            if mouse_wheel_y != 0.0 {
                let zoom_factor = if mouse_wheel_y > 0.0 { 0.9 } else { 1.1 };
                self.camera.adjust_zoom(zoom_factor);
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

        // Load rockets with their original IDs
        for saved_rocket in snapshot.rockets {
            let (id, rocket) = saved_rocket.to_rocket();
            self.world.add_rocket_with_id(id, rocket);
        }

        // Load satellites with their original IDs
        for saved_satellite in snapshot.satellites {
            let (id, satellite) = saved_satellite.to_satellite();
            self.world.add_satellite_with_id(id, satellite);
        }

        // Update our active rocket from snapshot
        self.active_rocket_id = snapshot.active_rocket_id;
        self.world.set_active_rocket(snapshot.active_rocket_id);

        // Note: We keep our local camera instead of using snapshot camera
        // This gives the client freedom to look around independently
    }

    /// Render the game
    pub fn render(&mut self) {
        // Set camera
        set_camera(self.camera.camera());

        // Render world
        self.world.render();

        // Draw trajectory visualization for client's rocket
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
