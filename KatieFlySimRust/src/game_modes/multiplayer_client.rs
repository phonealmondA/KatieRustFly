// Multiplayer Client Mode - Connects to host and receives snapshots via UDP
// Runs local predicted simulation and syncs with host snapshots

use macroquad::prelude::*;
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;

use crate::entities::Rocket;
use crate::game_constants::GameConstants;
use crate::save_system::GameSaveData;
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
                self.player_state.decrease_thrust();
            }
            if is_key_pressed(self.player_input.increase_thrust) {
                self.player_state.increase_thrust();
            }

            // Apply thrust
            if is_key_down(self.player_input.thrust) {
                if let Some(rocket) = self.world.get_rocket_mut(rocket_id) {
                    rocket.apply_thrust(self.player_state.selected_thrust_level);
                }
            }

            // Convert to satellite
            if is_key_pressed(self.player_input.convert_to_satellite) {
                if let Some(rocket) = self.world.get_rocket(rocket_id) {
                    let position = rocket.position();
                    let velocity = rocket.velocity();
                    let mass = rocket.mass();

                    self.world.spawn_satellite(position, velocity, mass);
                    self.world.despawn_rocket(rocket_id);

                    // Spawn new rocket for client
                    let spawn_distance = GameConstants::MAIN_PLANET_RADIUS + 200.0;
                    let new_rocket = Rocket::new(
                        Vec2::new(spawn_distance, 0.0),
                        Vec2::new(0.0, 0.0),
                        Color::from_rgba(100, 100, 255, 255), // Blue for client
                        50.0,
                    );
                    self.active_rocket_id = Some(self.world.spawn_rocket(new_rocket));
                    log::info!("Client converted rocket to satellite and respawned");
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

        // Update UI
        if let Some(rocket_id) = self.active_rocket_id {
            if let Some(rocket) = self.world.get_rocket(rocket_id) {
                self.game_info.update_from_rocket(rocket, &self.world);
            }
        }
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
        self.world = World::new();

        // Load planets
        for saved_planet in snapshot.planets {
            self.world.spawn_planet(
                saved_planet.position,
                saved_planet.velocity,
                saved_planet.mass,
                saved_planet.radius,
                Color::from_rgba(
                    saved_planet.color[0],
                    saved_planet.color[1],
                    saved_planet.color[2],
                    saved_planet.color[3],
                ),
            );
        }

        // Load rockets
        for saved_rocket in snapshot.rockets {
            let rocket = Rocket::new(
                saved_rocket.position,
                saved_rocket.velocity,
                Color::from_rgba(
                    saved_rocket.color[0],
                    saved_rocket.color[1],
                    saved_rocket.color[2],
                    saved_rocket.color[3],
                ),
                saved_rocket.mass,
            );
            let rocket_id = self.world.spawn_rocket(rocket);

            // Find our rocket (matching player_id)
            if saved_rocket.player_id == self.player_id {
                self.active_rocket_id = Some(rocket_id);
            }
        }

        // Load satellites
        for saved_satellite in snapshot.satellites {
            self.world.spawn_satellite(
                saved_satellite.position,
                saved_satellite.velocity,
                saved_satellite.mass,
            );
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

        // Render vehicle trajectories
        if let Some(rocket_id) = self.active_rocket_id {
            self.vehicle_manager.render_trajectory(
                &self.world,
                rocket_id,
                &self.camera,
            );
        }

        // Reset to default camera for UI
        set_default_camera();

        // Render UI
        if let Some(rocket_id) = self.active_rocket_id {
            if let Some(rocket) = self.world.get_rocket(rocket_id) {
                self.game_info.draw(rocket, &self.player_state);
            }
        }

        // Show connection status
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
