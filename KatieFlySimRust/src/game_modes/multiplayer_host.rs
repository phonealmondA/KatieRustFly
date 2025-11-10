// Multiplayer Host Mode - Authoritative server broadcasting snapshots via UDP
// Runs the simulation and broadcasts state every 10-15 seconds to all connected clients

use macroquad::prelude::*;
use std::net::{SocketAddr, UdpSocket};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::entities::Rocket;
use crate::game_constants::GameConstants;
use crate::save_system::GameSaveData;
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

    // Game state
    window_size: Vec2,
    paused: bool,
    current_save_name: Option<String>,
}

impl MultiplayerHost {
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

            window_size,
            paused: false,
            current_save_name: None,
        })
    }

    /// Initialize a new game with default starting conditions
    pub fn initialize_new_game(&mut self) {
        log::info!("Initializing new multiplayer host game");

        // Create main planet (Earth)
        self.world.spawn_planet_at_origin(
            GameConstants::MAIN_PLANET_MASS,
            GameConstants::MAIN_PLANET_RADIUS,
            BLUE,
        );

        // Create moon
        let moon_distance = GameConstants::MAIN_PLANET_RADIUS * 4.0;
        let moon_velocity = (GameConstants::G * GameConstants::MAIN_PLANET_MASS / moon_distance).sqrt();
        self.world.spawn_planet(
            Vec2::new(moon_distance, 0.0),
            Vec2::new(0.0, moon_velocity),
            GameConstants::MAIN_PLANET_MASS * 0.05,
            GameConstants::MAIN_PLANET_RADIUS * 0.5,
            GRAY,
        );

        // Spawn host's rocket (player 0)
        let spawn_distance = GameConstants::MAIN_PLANET_RADIUS + 200.0;
        let spawn_position = Vec2::new(spawn_distance, 0.0);
        let spawn_velocity = Vec2::new(0.0, 0.0);

        let rocket = Rocket::new(
            spawn_position,
            spawn_velocity,
            Color::from_rgba(255, 100, 100, 255), // Red for host
            50.0,
        );

        let rocket_id = self.world.spawn_rocket(rocket);
        self.active_rocket_id = Some(rocket_id);

        // Initialize camera
        self.camera.set_center(spawn_position);
        self.camera.set_target_zoom(1.0);

        log::info!("Multiplayer host game initialized - waiting for clients");
    }

    /// Load game from a save file
    pub fn load_from_save(&mut self, save_data: GameSaveData, save_name: String) {
        log::info!("Loading multiplayer host game from save: {}", save_name);

        // Clear existing world
        self.world = World::new();

        // Load planets
        for saved_planet in save_data.planets {
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
        for saved_rocket in save_data.rockets {
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

            // Set the active rocket for host (should be player_id 0)
            if saved_rocket.player_id == 0 {
                self.active_rocket_id = Some(rocket_id);
            }
        }

        // Load satellites
        for saved_satellite in save_data.satellites {
            self.world.spawn_satellite(
                saved_satellite.position,
                saved_satellite.velocity,
                saved_satellite.mass,
            );
        }

        // Restore camera state
        self.camera.set_center(save_data.camera_center);
        self.camera.set_target_zoom(save_data.camera_zoom);

        self.current_save_name = Some(save_name);
        log::info!("Multiplayer host save loaded successfully");
    }

    /// Handle input for the host player
    pub fn handle_input(&mut self) -> MultiplayerHostResult {
        // ESC - return to menu
        if is_key_pressed(KeyCode::Escape) {
            log::info!("ESC pressed - returning to menu");
            return MultiplayerHostResult::ReturnToMenu;
        }

        // Q - quit
        if is_key_pressed(KeyCode::Q) {
            return MultiplayerHostResult::Quit;
        }

        // P - pause/unpause
        if is_key_pressed(KeyCode::P) {
            self.paused = !self.paused;
            log::info!("Game {}", if self.paused { "paused" } else { "unpaused" });
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

                    // Spawn new rocket for host
                    let spawn_distance = GameConstants::MAIN_PLANET_RADIUS + 200.0;
                    let new_rocket = Rocket::new(
                        Vec2::new(spawn_distance, 0.0),
                        Vec2::new(0.0, 0.0),
                        Color::from_rgba(255, 100, 100, 255),
                        50.0,
                    );
                    self.active_rocket_id = Some(self.world.spawn_rocket(new_rocket));
                    log::info!("Host converted rocket to satellite and respawned");
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

        // Update UI
        if let Some(rocket_id) = self.active_rocket_id {
            if let Some(rocket) = self.world.get_rocket(rocket_id) {
                self.game_info.update_from_rocket(rocket, &self.world);
            }
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

                        // TODO: Spawn a rocket for this player
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

    /// Broadcast current game state snapshot to all connected clients
    fn broadcast_snapshot(&self) {
        // Create snapshot from current world state
        let snapshot = GameSaveData::from_world(&self.world, &self.camera, self.active_rocket_id);

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

        let save_data = GameSaveData::from_world(&self.world, &self.camera, self.active_rocket_id);

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

        // Render vehicle trajectories and info
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

        // Show host status
        let clients = self.clients.lock().unwrap();
        draw_text(
            &format!("HOST | Port: ? | Clients: {}", clients.len()),
            10.0,
            screen_height() - 20.0,
            20.0,
            GREEN,
        );

        if self.paused {
            draw_text(
                "PAUSED",
                screen_width() / 2.0 - 50.0,
                50.0,
                40.0,
                YELLOW,
            );
        }
    }

    /// Get connected client count
    pub fn client_count(&self) -> usize {
        self.clients.lock().unwrap().len()
    }
}
