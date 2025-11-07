// Single Player Game - Main single player game mode
// Integrates all systems for playable game

use macroquad::prelude::*;

use crate::entities::{GameObject, Planet, Rocket};
use crate::game_constants::GameConstants;
use crate::physics::{TrajectoryPoint, TrajectoryPredictor};
use crate::save_system::{GameSaveData, SavedCamera};
use crate::systems::World;
use crate::ui::{Camera, Hud};

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
    hud: Hud,
    trajectory_predictor: TrajectoryPredictor,
    game_time: f32,
    is_paused: bool,
    show_controls: bool,

    // Input state
    selected_thrust_level: f32, // 0.0 to 1.0 (0% to 100%)
    rotation_input: f32,
    idle_timer: f32, // Time since last input (for auto-expanding trajectory)

    // Trajectory caching (to prevent jitter in extended predictions)
    cached_trajectory_nodes: Vec<TrajectoryPoint>, // First 10 nodes locked after idle threshold
    cached_rocket_position: Vec2, // Rocket position when cache was created
    cached_rocket_velocity: Vec2, // Rocket velocity when cache was created
    cached_rocket_mass: f32, // Rocket mass when cache was created

    // Node-to-node travel tracking (for stable locked trajectories)
    trajectory_locked: bool, // True when trajectory is fully cached and stable
    current_node_index: usize, // Index of the next node the rocket is traveling toward
    consumed_trajectory_start: usize, // How many nodes have been consumed/passed
    fixed_marker_positions: Vec<Vec2>, // Fixed world positions of marker nodes when trajectory locks

    // Save/load
    current_save_name: Option<String>,
    last_auto_save: f32,
    auto_save_interval: f32,
}

impl SinglePlayerGame {
    pub fn new(window_size: Vec2) -> Self {
        SinglePlayerGame {
            world: World::new(),
            camera: Camera::new(window_size),
            hud: Hud::new(Vec2::new(10.0, 10.0)),
            trajectory_predictor: TrajectoryPredictor::new(),
            game_time: 0.0,
            is_paused: false,
            show_controls: false,
            selected_thrust_level: 0.0, // Start at 0% thrust
            rotation_input: 0.0,
            idle_timer: 0.0, // Start at 0
            cached_trajectory_nodes: Vec::new(),
            cached_rocket_position: Vec2::ZERO,
            cached_rocket_velocity: Vec2::ZERO,
            cached_rocket_mass: 0.0,
            trajectory_locked: false,
            current_node_index: 0,
            consumed_trajectory_start: 0,
            fixed_marker_positions: Vec::new(),
            current_save_name: None,
            last_auto_save: 0.0,
            auto_save_interval: 60.0, // Auto-save every 60 seconds
        }
    }

    /// Initialize a new game with default setup
    pub fn initialize_new_game(&mut self) {
        self.world.clear_all();
        self.game_time = 0.0;

        // Create main planet (like Earth)
        let main_planet = Planet::new(
            Vec2::new(GameConstants::MAIN_PLANET_X, GameConstants::MAIN_PLANET_Y),
            GameConstants::MAIN_PLANET_RADIUS,
            GameConstants::MAIN_PLANET_MASS,
            BLUE,
        );
        log::info!("Main planet: pos=({}, {}), radius={}, mass={}",
            GameConstants::MAIN_PLANET_X, GameConstants::MAIN_PLANET_Y,
            GameConstants::MAIN_PLANET_RADIUS, GameConstants::MAIN_PLANET_MASS);
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

        // Set orbital velocity for secondary planet
        secondary_planet.set_velocity(Vec2::new(
            0.0,
            -moon_velocity,
        ));

        self.world.add_planet(secondary_planet);

        // Create starting rocket near main planet
        let rocket_spawn_distance = GameConstants::MAIN_PLANET_RADIUS + 200.0;
        let rocket = Rocket::new(
            Vec2::new(
                GameConstants::MAIN_PLANET_X + rocket_spawn_distance,
                GameConstants::MAIN_PLANET_Y,
            ),
            Vec2::new(0.0, 0.0),
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

    /// Load game from save data
    pub fn load_from_save(&mut self, save_data: GameSaveData, save_name: String) {
        self.world.clear_all();
        self.game_time = save_data.game_time;

        // Load planets
        for saved_planet in save_data.planets {
            let (_id, planet) = saved_planet.to_planet();
            self.world.add_planet(planet);
        }

        // Load rockets
        for saved_rocket in save_data.rockets {
            let (_id, rocket) = saved_rocket.to_rocket();
            self.world.add_rocket(rocket);
        }

        // Load satellites
        for saved_satellite in save_data.satellites {
            let (_id, satellite) = saved_satellite.to_satellite();
            self.world.add_satellite(satellite);
        }

        // Set active rocket
        self.world.set_active_rocket(save_data.active_rocket_id);

        // Restore camera
        self.camera.set_center(save_data.camera.center.into());
        self.camera.set_target_zoom(save_data.camera.zoom);

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

    /// Create save data from current game state
    fn create_save_data(&self) -> GameSaveData {
        let mut save_data = GameSaveData::new();
        save_data.game_time = self.game_time;

        // Save planets (we need to iterate with IDs - simplified for now)
        // In a real implementation, World would provide an iterator with IDs

        // Save camera
        save_data.camera = SavedCamera {
            center: self.camera.camera().target.into(),
            zoom: self.camera.zoom_level(),
        };

        save_data.active_rocket_id = self.world.active_rocket_id();

        save_data
    }

    /// Handle input for game controls
    pub fn handle_input(&mut self) -> SinglePlayerResult {
        // Check for escape to return to menu or close controls popup
        if is_key_pressed(KeyCode::Escape) {
            if self.show_controls {
                self.show_controls = false;
                self.is_paused = false;
            } else {
                return SinglePlayerResult::ReturnToMenu;
            }
        }

        // Toggle controls menu with Enter key
        if is_key_pressed(KeyCode::Enter) {
            self.show_controls = !self.show_controls;
            self.is_paused = self.show_controls; // Pause when showing controls
        }

        // Handle mouse click for controls button and popup
        if is_mouse_button_pressed(MouseButton::Left) {
            let mouse_pos = mouse_position();
            let screen_w = screen_width();

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
                let popup_y = screen_height() / 2.0 - 250.0;
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
            }
        }

        // Toggle pause (only if controls not showing)
        if is_key_pressed(KeyCode::P) && !self.show_controls {
            self.is_paused = !self.is_paused;
        }

        // Quick save
        if is_key_pressed(KeyCode::F5) {
            if let Err(e) = self.save_game("quicksave") {
                log::error!("Failed to quick save: {}", e);
            }
        }

        // Mouse wheel zoom (reduced delta for finer control)
        let mouse_wheel = mouse_wheel().1;
        if mouse_wheel != 0.0 {
            self.camera.adjust_zoom(-mouse_wheel * 0.02);
            self.idle_timer = 0.0; // Reset idle timer on zoom
            self.reset_trajectory_cache(); // Clear cache on zoom
        }

        // Keyboard zoom controls (E = zoom out, Q = zoom in)
        // Note: zoom_scale = 1/zoom_level, so larger zoom_level = more zoomed out
        if is_key_down(KeyCode::Q) {
            self.camera.adjust_zoom(-0.02); // Gradual zoom in (decrease zoom_level)
            self.idle_timer = 0.0; // Reset idle timer on zoom
            self.reset_trajectory_cache(); // Clear cache on zoom
        }
        if is_key_down(KeyCode::E) {
            self.camera.adjust_zoom(0.02); // Gradual zoom out (increase zoom_level)
            self.idle_timer = 0.0; // Reset idle timer on zoom
            self.reset_trajectory_cache(); // Clear cache on zoom
        }

        SinglePlayerResult::Continue
    }

    /// Update game state
    pub fn update(&mut self, delta_time: f32) {
        if self.is_paused {
            return;
        }

        self.game_time += delta_time;

        // Increment idle timer
        self.idle_timer += delta_time;

        // Handle input for active rocket
        self.update_rocket_input();

        // Update world (physics, entities)
        self.world.update(delta_time);

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
        // Check if any input is detected and reset idle timer
        let has_input = is_key_pressed(KeyCode::Comma) || is_key_pressed(KeyCode::Period) ||
                        is_key_down(KeyCode::Space) ||
                        is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) ||
                        is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) ||
                        is_key_pressed(KeyCode::L) || is_key_pressed(KeyCode::T);

        if has_input {
            self.idle_timer = 0.0;
            self.reset_trajectory_cache(); // Clear cache on input
        }

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

        // Launch new rocket (L key)
        if is_key_pressed(KeyCode::L) {
            self.launch_new_rocket();
        }

        // Convert to satellite (T key)
        if is_key_pressed(KeyCode::T) {
            if let Some(rocket_id) = self.world.active_rocket_id() {
                if self.world.convert_rocket_to_satellite(rocket_id).is_some() {
                    log::info!("Rocket converted to satellite");
                }
            }
        }
    }

    /// Launch a new rocket from the active rocket's position
    fn launch_new_rocket(&mut self) {
        if let Some(current_rocket) = self.world.get_active_rocket() {
            let new_rocket = Rocket::new(
                current_rocket.position(),
                current_rocket.velocity(),
                Color::from_rgba(200, 200, 255, 255),
                GameConstants::ROCKET_BASE_MASS,
            );

            let new_id = self.world.add_rocket(new_rocket);
            self.world.set_active_rocket(Some(new_id));

            log::info!("New rocket launched");
        }
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

        // Draw moon trajectory (if moon exists)
        // Identify Earth (large planet) and Moon (small planet) by radius
        // HashMap iteration order is unpredictable, so we can't rely on index
        if all_planets.len() >= 2 {
            // Earth is the planet with the larger radius, Moon is the smaller one
            let (earth, moon) = if all_planets[0].radius() > all_planets[1].radius() {
                (all_planets[0], all_planets[1])
            } else {
                (all_planets[1], all_planets[0])
            };

            log::info!("Moon position: ({}, {}), velocity: ({}, {}), radius: {}",
                moon.position().x, moon.position().y,
                moon.velocity().x, moon.velocity().y, moon.radius());
            log::info!("Earth position: ({}, {}), radius: {}",
                earth.position().x, earth.position().y, earth.radius());

            let earth_only: Vec<&Planet> = vec![earth];

            let (moon_trajectory, moon_orbit_closes) = self.trajectory_predictor.predict_planet_trajectory(
                moon,
                &earth_only,
                0.5,
                840, // 420 seconds / 0.5s per step = 840 steps for full orbit
                true,
            );

            log::info!("Moon trajectory: {} points, orbit closes: {}, zoom_level: {}",
                moon_trajectory.len(), moon_orbit_closes, zoom_level);

            if !moon_trajectory.is_empty() {
                log::info!("First trajectory point: ({}, {})",
                    moon_trajectory[0].position.x, moon_trajectory[0].position.y);
                if let Some(last) = moon_trajectory.last() {
                    log::info!("Last trajectory point: ({}, {})",
                        last.position.x, last.position.y);
                }
                // Calculate what the line thickness will be
                let base_thickness = 8.0;
                let scaled_thickness = base_thickness * zoom_level.powf(0.8);
                log::info!("Moon trajectory line thickness: {} (base: {}, zoom: {})",
                    scaled_thickness, base_thickness, zoom_level);
            }

            // Draw moon trajectory in cyan/light blue
            let moon_color = if moon_orbit_closes {
                Color::new(0.0, 0.8, 1.0, 0.6) // Bright cyan if orbit closes
            } else {
                Color::new(0.5, 0.5, 1.0, 0.6) // Light purple if orbit doesn't close
            };

            self.trajectory_predictor.draw_trajectory(&moon_trajectory, moon_color, moon_orbit_closes, zoom_level, None);
            log::info!("Drew moon trajectory with {} points in color: {:?}", moon_trajectory.len(), moon_color);
        } else {
            log::warn!("Not enough planets for moon trajectory: {}", all_planets.len());
        }

        // Draw trajectory prediction for active rocket
        if let Some(rocket) = self.world.get_active_rocket() {
            // Calculate trajectory steps based on idle timer
            // If idle for more than threshold, extend to show full orbit
            let is_extended = self.idle_timer > GameConstants::TRAJECTORY_IDLE_EXPAND_SECONDS;
            let trajectory_steps = if is_extended {
                1000 // Extended steps for full orbit (~500 seconds)
            } else {
                200 // Normal steps (~100 seconds)
            };

            // Check if rocket has moved significantly (only check if trajectory is NOT locked)
            // Once trajectory is locked, we don't care about small movements - we follow the path node-to-node
            let rocket_moved = if self.trajectory_locked {
                false // Ignore movement when locked - we're following the predicted path
            } else {
                self.cached_trajectory_nodes.is_empty() ||
                (rocket.position() - self.cached_rocket_position).length() > 1.0 ||
                (rocket.velocity() - self.cached_rocket_velocity).length() > 0.1
            };

            let (trajectory_points, self_intersects) = if !is_extended || rocket_moved {
                // Not in extended mode OR rocket moved - clear cache and calculate full trajectory
                self.cached_trajectory_nodes.clear();
                self.trajectory_predictor.predict_trajectory(
                    rocket,
                    &all_planets,
                    0.5,
                    trajectory_steps,
                    true,
                )
            } else if self.cached_trajectory_nodes.is_empty() {
                // First time in extended mode - calculate full trajectory and cache first 10 nodes
                let (full_trajectory, intersects) = self.trajectory_predictor.predict_trajectory(
                    rocket,
                    &all_planets,
                    0.5,
                    trajectory_steps,
                    true,
                );

                // Cache first 10 nodes (5 seconds at 0.5s per step)
                if full_trajectory.len() >= 10 {
                    self.cached_trajectory_nodes = full_trajectory[..10].to_vec();
                    self.cached_rocket_position = rocket.position();
                    self.cached_rocket_velocity = rocket.velocity();
                    self.cached_rocket_mass = rocket.current_mass();
                    log::info!("Cached first 10 trajectory nodes for stability");
                }

                (full_trajectory, intersects)
            } else if self.cached_trajectory_nodes.len() >= trajectory_steps {
                // Fully cached! Lock trajectory and track node-to-node travel
                if !self.trajectory_locked {
                    self.trajectory_locked = true;

                    // Capture fixed marker positions (every 20th point)
                    self.fixed_marker_positions.clear();
                    for (i, point) in self.cached_trajectory_nodes.iter().enumerate() {
                        if i % 20 == 0 {
                            self.fixed_marker_positions.push(point.position);
                        }
                    }

                    log::info!("Trajectory LOCKED - rocket will follow exact predicted path node-to-node ({} fixed markers)",
                        self.fixed_marker_positions.len());
                }

                // Track which node the rocket is traveling toward
                // Nodes are marked every 20 points, so we track progress toward marker nodes
                let marker_interval = 20;

                // Find distance to current target node (every 20th point is a visible marker)
                if self.current_node_index < self.cached_trajectory_nodes.len() {
                    let target_node_idx = (self.current_node_index / marker_interval) * marker_interval;
                    if target_node_idx < self.cached_trajectory_nodes.len() {
                        let target_pos = self.cached_trajectory_nodes[target_node_idx].position;
                        let distance_to_node = (rocket.position() - target_pos).length();

                        // If close enough to node, advance to next marker node
                        if distance_to_node < 30.0 {
                            let next_marker = target_node_idx + marker_interval;
                            if next_marker < self.cached_trajectory_nodes.len() {
                                self.current_node_index = next_marker;
                                log::info!("Reached node {}, advancing to next marker at index {}",
                                    target_node_idx, next_marker);
                            }
                        }
                    }
                }

                // Consume trajectory points as rocket passes them
                // Remove points that are behind the rocket
                let mut points_to_consume = 0;
                for (idx, point) in self.cached_trajectory_nodes.iter().enumerate().skip(self.consumed_trajectory_start) {
                    let distance = (rocket.position() - point.position).length();
                    if distance < 10.0 && idx > self.consumed_trajectory_start {
                        points_to_consume = idx;
                    } else if idx > points_to_consume + 5 {
                        // Stop checking once we're well ahead
                        break;
                    }
                }

                if points_to_consume > self.consumed_trajectory_start {
                    self.consumed_trajectory_start = points_to_consume;
                    log::debug!("Consumed trajectory up to index {}", points_to_consume);
                }

                // Check if trajectory self-intersects by examining cached nodes
                let mut intersects = false;
                if self.cached_trajectory_nodes.len() > 20 {
                    for i in 20..self.cached_trajectory_nodes.len() {
                        let pos = self.cached_trajectory_nodes[i].position;
                        // Check against early trajectory points
                        for j in 0..(i.saturating_sub(20)) {
                            let distance = (self.cached_trajectory_nodes[j].position - pos).length();
                            if distance < 50.0 {
                                intersects = true;
                                break;
                            }
                        }
                        if intersects { break; }
                    }
                }

                (self.cached_trajectory_nodes.clone(), intersects)
            } else {
                // Incrementally cache one more node per frame until fully cached
                let cached_count = self.cached_trajectory_nodes.len();
                let last_cached = &self.cached_trajectory_nodes[cached_count - 1];

                // Calculate just ONE more node from the last cached position
                let (next_nodes, _) = self.trajectory_predictor.predict_trajectory_from_state(
                    last_cached.position,
                    last_cached.velocity,
                    self.cached_rocket_mass,
                    &all_planets,
                    0.5,
                    1, // Just ONE node
                    false, // Don't check intersection for single node
                );

                // Add the new node to cache (with adjusted time)
                if let Some(mut new_node) = next_nodes.into_iter().next() {
                    new_node.time += last_cached.time;
                    self.cached_trajectory_nodes.push(new_node);

                    let new_cached_count = self.cached_trajectory_nodes.len();
                    if new_cached_count % 50 == 0 || new_cached_count == trajectory_steps {
                        log::info!("Trajectory caching progress: {}/{} nodes ({:.1}%)",
                            new_cached_count, trajectory_steps,
                            (new_cached_count as f32 / trajectory_steps as f32) * 100.0);
                    }

                    // ORBIT REPETITION DETECTION
                    // Check if trajectory is repeating (completed one full orbit and starting to loop)
                    // This allows early locking when a stable orbit is detected
                    if new_cached_count > 100 && new_cached_count < trajectory_steps {
                        // Check if recent positions match starting positions (orbit looping)
                        let check_window = 20; // Check last 20 points against first 20 points
                        if new_cached_count >= check_window * 2 {
                            let mut is_repeating = true;
                            let tolerance = 15.0; // Position must be within 15 units

                            // Compare last N points with first N points
                            for i in 0..check_window {
                                let start_pos = self.cached_trajectory_nodes[i].position;
                                let recent_idx = new_cached_count - check_window + i;
                                let recent_pos = self.cached_trajectory_nodes[recent_idx].position;
                                let distance = (start_pos - recent_pos).length();

                                if distance > tolerance {
                                    is_repeating = false;
                                    break;
                                }
                            }

                            if is_repeating {
                                // Orbit is repeating! Lock it now, truncate to one complete orbit
                                log::info!("STABLE ORBIT DETECTED at {} nodes - locking trajectory immediately!", new_cached_count);

                                // Trim trajectory to just the completed orbit (exclude the repeating part)
                                self.cached_trajectory_nodes.truncate(new_cached_count - check_window);

                                // Force lock by setting count to trajectory_steps
                                // This will trigger the locking logic in the next frame
                                while self.cached_trajectory_nodes.len() < trajectory_steps {
                                    // Pad with last node to reach trajectory_steps
                                    let last = self.cached_trajectory_nodes.last().unwrap().clone();
                                    self.cached_trajectory_nodes.push(last);
                                }
                            }
                        }
                    }
                }

                // For display, calculate remaining trajectory from last cached node
                let last_cached_now = self.cached_trajectory_nodes.last().unwrap();
                let remaining_steps = trajectory_steps - self.cached_trajectory_nodes.len();

                if remaining_steps > 0 {
                    let (mut remaining_trajectory, intersects) = self.trajectory_predictor.predict_trajectory_from_state(
                        last_cached_now.position,
                        last_cached_now.velocity,
                        self.cached_rocket_mass,
                        &all_planets,
                        0.5,
                        remaining_steps,
                        true,
                    );

                    // Adjust time values in remaining trajectory
                    for point in &mut remaining_trajectory {
                        point.time += last_cached_now.time;
                    }

                    // Concatenate cached + remaining
                    let mut full_trajectory = self.cached_trajectory_nodes.clone();
                    full_trajectory.append(&mut remaining_trajectory);

                    (full_trajectory, intersects)
                } else {
                    // All cached, check for intersection
                    let intersects = false; // Will be checked in next frame when fully cached
                    (self.cached_trajectory_nodes.clone(), intersects)
                }
            };

            // Only draw trajectory from consumed_trajectory_start onward (skip already-traveled segments)
            let trajectory_to_draw = if self.trajectory_locked && self.consumed_trajectory_start > 0 {
                // Skip consumed trajectory points
                let start_idx = self.consumed_trajectory_start.min(trajectory_points.len());
                &trajectory_points[start_idx..]
            } else {
                // Draw full trajectory if not locked or nothing consumed yet
                &trajectory_points[..]
            };

            // Draw trajectory with color based on whether it self-intersects (completes orbit)
            let trajectory_color = if self_intersects {
                Color::new(0.0, 1.0, 0.0, 0.7) // Green if orbit closes
            } else {
                Color::new(1.0, 1.0, 0.0, 0.7) // Yellow if orbit is open
            };

            // Pass fixed markers if trajectory is locked, otherwise None
            let fixed_markers = if self.trajectory_locked && !self.fixed_marker_positions.is_empty() {
                Some(self.fixed_marker_positions.as_slice())
            } else {
                None
            };

            self.trajectory_predictor.draw_trajectory(trajectory_to_draw, trajectory_color, self_intersects, zoom_level, fixed_markers);
        }

        // Reset to default camera for HUD
        set_default_camera();

        // Render HUD
        if let Some(rocket) = self.world.get_active_rocket() {
            self.hud.draw_rocket_stats(rocket, self.selected_thrust_level);
        } else {
            self.hud.draw_message("No active rocket");
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
            let popup_w = 400.0;
            let popup_h = 500.0;
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

            // Controls list
            let controls = [
                ("COMMA", "Decrease thrust -5%"),
                ("PERIOD", "Increase thrust +5%"),
                ("SPACE", "Apply thrust"),
                ("A / LEFT", "Rotate right"),
                ("D / RIGHT", "Rotate left"),
                ("Q", "Zoom in"),
                ("E", "Zoom out"),
                ("MOUSE WHEEL", "Zoom"),
                ("L", "Convert to satellite"),
                ("T", "Launch new rocket"),
                ("P", "Pause/Unpause"),
                ("F5", "Quick save"),
                ("ENTER", "Toggle this menu"),
                ("ESC", "Return to menu / Close this"),
            ];

            let mut y = popup_y + 80.0;
            let font_size = 18.0;
            let line_height = 35.0;

            for (key, action) in &controls {
                // Draw key
                draw_text(
                    key,
                    popup_x + 30.0,
                    y,
                    font_size,
                    Color::new(0.8, 0.8, 1.0, 1.0), // Light blue
                );

                // Draw action
                draw_text(
                    action,
                    popup_x + 200.0,
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
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused
    }

    pub fn game_time(&self) -> f32 {
        self.game_time
    }

    /// Reset trajectory cache and locked state (called on user input or zoom)
    fn reset_trajectory_cache(&mut self) {
        self.cached_trajectory_nodes.clear();
        self.trajectory_locked = false;
        self.current_node_index = 0;
        self.consumed_trajectory_start = 0;
        self.fixed_marker_positions.clear();
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
