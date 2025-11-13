# Multi-Map System Implementation Plan
## Solar System Configuration with 9 Planets + Sun

---

## 🎯 OVERVIEW

Transform the current 2-body Earth-Moon system into a flexible multi-map architecture supporting different planetary configurations. The first new map will be a realistic solar system with:
- **10 celestial bodies:** Sun + 9 planets (Mercury through Pluto)
- **Sun as central body:** All planets orbit the Sun
- **Moon orbits Earth:** Earth orbits Sun, Moon orbits Earth (hierarchical orbits)
- **Player spawns on Earth:** Rocket spawns on orbiting Earth
- **Current masses preserved:** Earth and Moon keep their existing masses

---

## 📋 IMPLEMENTATION PHASES

### **PHASE 1: Create Map Configuration System**
Extract hardcoded planet data into flexible configuration structure.

#### 1.1 Create Map Configuration Module
**File:** `src/map_config/mod.rs` (new)

```rust
use macroquad::prelude::*;

#[derive(Clone, Debug)]
pub struct MapConfiguration {
    pub name: String,
    pub description: String,
    pub celestial_bodies: Vec<CelestialBodyConfig>,
    pub player_spawn_body_index: usize, // Which body to spawn on
    pub central_body_index: Option<usize>, // Which body is the center (if any)
}

#[derive(Clone, Debug)]
pub struct CelestialBodyConfig {
    pub name: String,
    pub mass: f32,
    pub radius: f32,
    pub color: Color,
    pub orbital_parent_index: Option<usize>, // None = stationary, Some(i) = orbits body i
    pub orbital_distance: Option<f32>, // Distance from parent
    pub orbital_period: Option<f32>, // Seconds to complete orbit
    pub initial_angle: f32, // Starting angle in radians (0 = right, π/2 = up)
    pub is_pinned: bool, // If true, doesn't move (for backward compat)
}

impl MapConfiguration {
    pub fn earth_moon_classic() -> Self { /* ... */ }
    pub fn solar_system() -> Self { /* ... */ }
}
```

**Key Design Decisions:**
- `orbital_parent_index`: Creates hierarchical orbits (Moon → Earth → Sun)
- `initial_angle`: Distributes planets around their orbits for visual variety
- `is_pinned`: Maintains backward compatibility with current Earth behavior
- `player_spawn_body_index`: Flexible spawning on any planet

---

#### 1.2 Calculate Initial Positions and Velocities
**File:** `src/map_config/orbit_calculator.rs` (new)

```rust
pub struct InitialState {
    pub position: Vec2,
    pub velocity: Vec2,
}

/// Calculate initial position/velocity for all bodies in a map
pub fn calculate_initial_states(
    map: &MapConfiguration,
    gravity_constant: f32,
) -> Vec<InitialState> {
    // 1. Position stationary/pinned bodies at origin or fixed positions
    // 2. For orbiting bodies, calculate position relative to parent
    // 3. Calculate orbital velocity using v = sqrt(G * M / r)
    // 4. Handle hierarchical orbits (Moon velocity = Earth velocity + Moon-around-Earth velocity)
}

/// Convert polar coordinates (distance, angle) to Cartesian (x, y)
fn polar_to_cartesian(parent_pos: Vec2, distance: f32, angle: f32) -> Vec2 {
    Vec2::new(
        parent_pos.x + distance * angle.cos(),
        parent_pos.y + distance * angle.sin(),
    )
}

/// Calculate orbital velocity perpendicular to radius vector
fn calculate_orbital_velocity(
    parent_pos: Vec2,
    body_pos: Vec2,
    parent_mass: f32,
    gravity_constant: f32,
) -> Vec2 {
    // v = sqrt(G * M / r), perpendicular to radius
}
```

**Hierarchical Orbit Example:**
```
Moon velocity = Earth_velocity + Moon_orbital_velocity_around_Earth
              = (Earth orbiting Sun) + (Moon orbiting Earth)
```

---

#### 1.3 Define Solar System Map
**File:** `src/map_config/maps.rs` (new)

```rust
use super::*;

impl MapConfiguration {
    /// Original Earth-Moon system (backward compatibility)
    pub fn earth_moon_classic() -> Self {
        MapConfiguration {
            name: "Earth-Moon Classic".to_string(),
            description: "The classic two-body problem".to_string(),
            celestial_bodies: vec![
                CelestialBodyConfig {
                    name: "Earth".to_string(),
                    mass: 198_910_000.0, // Current value
                    radius: 10_000.0,
                    color: BLUE,
                    orbital_parent_index: None,
                    orbital_distance: None,
                    orbital_period: None,
                    initial_angle: 0.0,
                    is_pinned: true, // Stays at (400, 300)
                },
                CelestialBodyConfig {
                    name: "Moon".to_string(),
                    mass: 11_934_600.0, // 0.06 * Earth mass
                    radius: 2_500.0,
                    color: Color::from_rgba(150, 150, 150, 255),
                    orbital_parent_index: Some(0), // Orbits Earth
                    orbital_distance: Some(*PLANET_ORBIT_DISTANCE), // ~50k pixels
                    orbital_period: Some(420.0),
                    initial_angle: 0.0, // Starts to the right of Earth
                    is_pinned: false,
                },
            ],
            player_spawn_body_index: 0, // Spawn on Earth
            central_body_index: Some(0), // Earth is center
        }
    }

    /// Solar System: Sun + 9 planets (Mercury to Pluto) + Moon
    pub fn solar_system() -> Self {
        MapConfiguration {
            name: "Solar System".to_string(),
            description: "All nine planets orbiting the Sun".to_string(),
            celestial_bodies: vec![
                // --- SUN (index 0) ---
                CelestialBodyConfig {
                    name: "Sun".to_string(),
                    mass: 2_000_000_000.0, // 10x Earth mass for gameplay
                    radius: 20_000.0, // 2x Earth radius
                    color: YELLOW,
                    orbital_parent_index: None,
                    orbital_distance: None,
                    orbital_period: None,
                    initial_angle: 0.0,
                    is_pinned: true, // Center of solar system
                },

                // --- MERCURY (index 1) ---
                CelestialBodyConfig {
                    name: "Mercury".to_string(),
                    mass: 66_000_000.0, // ~0.33x Earth
                    radius: 4_800.0,
                    color: Color::from_rgba(169, 169, 169, 255), // Dark gray
                    orbital_parent_index: Some(0), // Orbits Sun
                    orbital_distance: Some(40_000.0),
                    orbital_period: Some(200.0), // Fast orbit
                    initial_angle: 0.0,
                    is_pinned: false,
                },

                // --- VENUS (index 2) ---
                CelestialBodyConfig {
                    name: "Venus".to_string(),
                    mass: 163_000_000.0, // ~0.82x Earth
                    radius: 9_500.0,
                    color: Color::from_rgba(255, 198, 73, 255), // Orange-yellow
                    orbital_parent_index: Some(0),
                    orbital_distance: Some(70_000.0),
                    orbital_period: Some(350.0),
                    initial_angle: std::f32::consts::PI / 4.0, // 45 degrees
                    is_pinned: false,
                },

                // --- EARTH (index 3) ---
                CelestialBodyConfig {
                    name: "Earth".to_string(),
                    mass: 198_910_000.0, // KEEP CURRENT MASS
                    radius: 10_000.0,
                    color: BLUE,
                    orbital_parent_index: Some(0),
                    orbital_distance: Some(100_000.0),
                    orbital_period: Some(500.0),
                    initial_angle: std::f32::consts::PI / 2.0, // 90 degrees
                    is_pinned: false, // NOW ORBITS!
                },

                // --- MOON (index 4) ---
                CelestialBodyConfig {
                    name: "Moon".to_string(),
                    mass: 11_934_600.0, // KEEP CURRENT MASS
                    radius: 2_500.0,
                    color: Color::from_rgba(150, 150, 150, 255),
                    orbital_parent_index: Some(3), // Orbits Earth
                    orbital_distance: Some(15_000.0), // Closer for gameplay
                    orbital_period: Some(50.0), // Relative to Earth
                    initial_angle: 0.0,
                    is_pinned: false,
                },

                // --- MARS (index 5) ---
                CelestialBodyConfig {
                    name: "Mars".to_string(),
                    mass: 135_000_000.0, // ~0.68x Earth
                    radius: 5_300.0,
                    color: Color::from_rgba(193, 68, 14, 255), // Red
                    orbital_parent_index: Some(0),
                    orbital_distance: Some(140_000.0),
                    orbital_period: Some(800.0),
                    initial_angle: std::f32::consts::PI,
                    is_pinned: false,
                },

                // --- JUPITER (index 6) ---
                CelestialBodyConfig {
                    name: "Jupiter".to_string(),
                    mass: 400_000_000.0, // ~2x Earth (scaled down for gameplay)
                    radius: 25_000.0, // Largest planet
                    color: Color::from_rgba(201, 176, 55, 255), // Tan
                    orbital_parent_index: Some(0),
                    orbital_distance: Some(200_000.0),
                    orbital_period: Some(1200.0),
                    initial_angle: 3.0 * std::f32::consts::PI / 2.0,
                    is_pinned: false,
                },

                // --- SATURN (index 7) ---
                CelestialBodyConfig {
                    name: "Saturn".to_string(),
                    mass: 350_000_000.0, // ~1.76x Earth
                    radius: 23_000.0,
                    color: Color::from_rgba(250, 227, 133, 255), // Pale yellow
                    orbital_parent_index: Some(0),
                    orbital_distance: Some(260_000.0),
                    orbital_period: Some(1600.0),
                    initial_angle: 2.0 * std::f32::consts::PI / 3.0,
                    is_pinned: false,
                },

                // --- URANUS (index 8) ---
                CelestialBodyConfig {
                    name: "Uranus".to_string(),
                    mass: 180_000_000.0, // ~0.9x Earth
                    radius: 12_000.0,
                    color: Color::from_rgba(79, 208, 231, 255), // Cyan
                    orbital_parent_index: Some(0),
                    orbital_distance: Some(320_000.0),
                    orbital_period: Some(2000.0),
                    initial_angle: std::f32::consts::PI / 6.0,
                    is_pinned: false,
                },

                // --- NEPTUNE (index 9) ---
                CelestialBodyConfig {
                    name: "Neptune".to_string(),
                    mass: 190_000_000.0, // ~0.95x Earth
                    radius: 12_500.0,
                    color: Color::from_rgba(62, 84, 232, 255), // Deep blue
                    orbital_parent_index: Some(0),
                    orbital_distance: Some(380_000.0),
                    orbital_period: Some(2400.0),
                    initial_angle: 5.0 * std::f32::consts::PI / 6.0,
                    is_pinned: false,
                },

                // --- PLUTO (index 10) ---
                CelestialBodyConfig {
                    name: "Pluto".to_string(),
                    mass: 30_000_000.0, // ~0.15x Earth
                    radius: 3_000.0,
                    color: Color::from_rgba(165, 149, 140, 255), // Brown-gray
                    orbital_parent_index: Some(0),
                    orbital_distance: Some(450_000.0),
                    orbital_period: Some(3000.0),
                    initial_angle: 4.0 * std::f32::consts::PI / 3.0,
                    is_pinned: false,
                },
            ],
            player_spawn_body_index: 3, // Spawn on Earth (index 3)
            central_body_index: Some(0), // Sun is center
        }
    }
}
```

**Planet Scale Decisions:**
- **Masses:** Scaled for gameplay balance (real ratios would make Jupiter/Sun too dominant)
- **Distances:** Logarithmic spacing for visual clarity (not to scale)
- **Periods:** Faster than reality for gameplay (real year = 31M seconds)
- **Initial angles:** Distributed around orbits for visual appeal

---

### **PHASE 2: Refactor Game Initialization**

#### 2.1 Update SinglePlayerGame Structure
**File:** `src/game_modes/single_player.rs`

**Add Fields:**
```rust
pub struct SinglePlayerGame {
    // ... existing fields ...
    pub current_map: MapConfiguration,
    pub spawn_planet_index: usize, // Which planet to spawn on
}
```

**Modify Constructor:**
```rust
impl SinglePlayerGame {
    pub fn new() -> Self {
        Self {
            // ... existing initialization ...
            current_map: MapConfiguration::earth_moon_classic(), // Default map
            spawn_planet_index: 0,
        }
    }

    pub fn new_with_map(map: MapConfiguration) -> Self {
        let spawn_planet_index = map.player_spawn_body_index;
        Self {
            // ... existing initialization ...
            current_map: map,
            spawn_planet_index,
        }
    }
}
```

---

#### 2.2 Refactor initialize_new_game()
**File:** `src/game_modes/single_player.rs` (lines 79-144)

**Replace hardcoded planet creation with map-based initialization:**

```rust
pub fn initialize_new_game(&mut self) {
    self.world.clear_all();

    // Calculate initial states for all bodies
    let initial_states = orbit_calculator::calculate_initial_states(
        &self.current_map,
        GameConstants::G,
    );

    // Create all celestial bodies
    for (i, body_config) in self.current_map.celestial_bodies.iter().enumerate() {
        let state = &initial_states[i];

        let mut planet = Planet::new(
            state.position,
            body_config.radius,
            body_config.mass,
            body_config.color,
        );

        planet.set_velocity(state.velocity);
        planet.set_name(body_config.name.clone()); // NEW: Planet names
        planet.set_pinned(body_config.is_pinned); // NEW: Pinned flag

        let planet_id = self.world.add_planet(planet);

        // Track spawn planet
        if i == self.current_map.player_spawn_body_index {
            self.spawn_planet_id = Some(planet_id);
        }
    }

    // Spawn player rocket on designated planet
    self.spawn_rocket();
}
```

**New spawn_rocket() helper:**
```rust
fn spawn_rocket(&mut self) {
    if let Some(spawn_planet_id) = self.spawn_planet_id {
        if let Some(spawn_planet) = self.world.get_planet(spawn_planet_id) {
            // Calculate spawn position 200 pixels above planet surface
            let spawn_distance = spawn_planet.radius + 200.0;
            let spawn_position = spawn_planet.position + Vec2::new(spawn_distance, 0.0);

            // Rocket inherits planet's velocity for stable orbit
            let spawn_velocity = spawn_planet.velocity;

            let rocket = Rocket::new(
                spawn_position,
                spawn_velocity,
                WHITE,
                GameConstants::ROCKET_BASE_MASS,
            );
            rocket.set_fuel(GameConstants::ROCKET_STARTING_FUEL);

            self.world.add_rocket(rocket);
        }
    }
}
```

**CRITICAL: Rocket must inherit Earth's velocity in solar system map!**

---

### **PHASE 3: Update Entity System**

#### 3.1 Add Planet Properties
**File:** `src/entities/planet.rs`

```rust
pub struct Planet {
    // ... existing fields ...
    pub name: Option<String>, // NEW: Planet name for UI
    pub is_pinned: bool, // NEW: If true, ignore gravity forces
}

impl Planet {
    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    pub fn set_pinned(&mut self, pinned: bool) {
        self.is_pinned = pinned;
    }
}
```

---

#### 3.2 Update World System
**File:** `src/systems/world.rs`

**Add getter for planets:**
```rust
impl World {
    pub fn get_planet(&self, id: EntityId) -> Option<&Planet> {
        self.planets.get(&id)
    }

    pub fn get_planet_mut(&mut self, id: EntityId) -> Option<&mut Planet> {
        self.planets.get_mut(&id)
    }
}
```

---

### **PHASE 4: Update Physics System**

#### 4.1 Modify Planet-to-Planet Gravity
**File:** `src/systems/world.rs` (lines 595-630)

**Current implementation only applies gravity from planet 0 to planet 1.**

**New implementation: Apply gravity between all planet pairs, respecting pinned flag:**

```rust
pub fn apply_planet_to_planet_gravity(&mut self, dt: f32) {
    let planet_ids: Vec<EntityId> = self.planets.keys().cloned().collect();

    for i in 0..planet_ids.len() {
        for j in (i + 1)..planet_ids.len() {
            let id_a = planet_ids[i];
            let id_b = planet_ids[j];

            // Get positions and masses
            let (pos_a, mass_a, is_pinned_a) = {
                let p = self.planets.get(&id_a).unwrap();
                (p.position, p.mass, p.is_pinned)
            };

            let (pos_b, mass_b, is_pinned_b) = {
                let p = self.planets.get(&id_b).unwrap();
                (p.position, p.mass, p.is_pinned)
            };

            // Calculate gravitational force
            let force = self.gravity_simulator.calculate_gravitational_force(
                pos_a, mass_a, pos_b, mass_b,
            );

            // Apply force to both planets (unless pinned)
            if !is_pinned_a {
                let accel_a = force / mass_a;
                self.planets.get_mut(&id_a).unwrap().velocity += accel_a * dt;
            }

            if !is_pinned_b {
                let accel_b = -force / mass_b; // Opposite direction
                self.planets.get_mut(&id_b).unwrap().velocity += accel_b * dt;
            }
        }
    }
}
```

**Why this matters:**
- In Earth-Moon map: Earth is pinned, Moon orbits Earth (current behavior preserved)
- In Solar System map: Sun is pinned, all planets orbit Sun, Moon orbits Earth
- All planets exert gravity on each other (N-body simulation)

---

#### 4.2 Verify Gravity Application Order
**File:** `src/systems/world.rs` (update() method)

**Ensure planet-to-planet gravity is applied AFTER all rocket/satellite updates:**
```rust
pub fn update(&mut self, dt: f32) -> UpdateResult {
    self.update_planets(dt);
    self.apply_gravity_to_rockets(dt);
    self.apply_rocket_to_rocket_gravity(dt);
    self.update_rockets(dt);
    self.apply_gravity_to_satellites(dt);
    self.update_satellites(dt);
    self.handle_fuel_collection(dt);
    self.handle_fuel_transfers();
    self.check_landing_collisions();
    self.apply_gravity_to_bullets(dt);
    self.update_bullets(dt);
    self.check_bullet_collisions();
    self.apply_planet_to_planet_gravity(dt); // ← UPDATED METHOD
    self.remove_destroyed_entities();

    // ... rest of update ...
}
```

---

### **PHASE 5: Update Camera System**

#### 5.1 Handle Large Map Scales
**File:** `src/rendering/camera.rs`

**Current camera centers on active rocket. For solar system map, need to:**
- Zoom out more for overview
- Allow switching camera focus (rocket vs planet vs system overview)

**Add Camera Modes:**
```rust
pub enum CameraMode {
    FollowRocket,        // Current behavior
    FollowPlanet(EntityId), // Focus on specific planet
    SystemOverview,      // Show entire solar system
}

pub struct Camera {
    // ... existing fields ...
    pub mode: CameraMode,
    pub zoom_levels: Vec<f32>, // [0.1, 0.2, 0.5, 1.0, 2.0, 5.0]
    pub current_zoom_index: usize,
}
```

**Update camera target calculation:**
```rust
pub fn update(&mut self, world: &World, active_rocket_id: Option<EntityId>) {
    match self.mode {
        CameraMode::FollowRocket => {
            if let Some(id) = active_rocket_id {
                if let Some(rocket) = world.get_rocket(id) {
                    self.target = rocket.position;
                }
            }
        }
        CameraMode::FollowPlanet(planet_id) => {
            if let Some(planet) = world.get_planet(planet_id) {
                self.target = planet.position;
            }
        }
        CameraMode::SystemOverview => {
            // Calculate center of mass of all planets
            self.target = self.calculate_system_center(world);
        }
    }
}
```

---

#### 5.2 Add Zoom Controls
**File:** `src/game_modes/single_player.rs` (input handling)

**Add keybindings for camera zoom:**
```rust
// In handle_input() method
if is_key_pressed(KeyCode::Equal) || is_key_pressed(KeyCode::KpAdd) {
    self.camera.zoom_in();
}
if is_key_pressed(KeyCode::Minus) || is_key_pressed(KeyCode::KpSubtract) {
    self.camera.zoom_out();
}
if is_key_pressed(KeyCode::Key0) {
    self.camera.reset_zoom();
}
```

---

### **PHASE 6: Update Save/Load System**

#### 6.1 Save Map Configuration
**File:** `src/save_system/game_save_data.rs`

**Add map identifier to save data:**
```rust
#[derive(Serialize, Deserialize)]
pub struct GameSaveData {
    pub game_time: f32,
    pub planets: Vec<SavedPlanet>,
    pub rockets: Vec<SavedRocket>,
    pub satellites: Vec<SavedSatellite>,
    pub active_rocket_id: Option<EntityId>,
    pub camera: SavedCamera,
    pub map_name: String, // NEW: Which map is being played
}
```

**Update save logic:**
```rust
impl SinglePlayerGame {
    pub fn create_save_data(&self) -> GameSaveData {
        GameSaveData {
            // ... existing fields ...
            map_name: self.current_map.name.clone(),
        }
    }
}
```

---

#### 6.2 Load Map on Restore
**File:** `src/game_modes/single_player.rs`

**When loading a save, restore the correct map:**
```rust
pub fn load_from_save(&mut self, save_data: GameSaveData) {
    // Determine which map to load based on save_data.map_name
    self.current_map = match save_data.map_name.as_str() {
        "Earth-Moon Classic" => MapConfiguration::earth_moon_classic(),
        "Solar System" => MapConfiguration::solar_system(),
        _ => MapConfiguration::earth_moon_classic(), // Fallback
    };

    // ... rest of load logic ...
}
```

---

### **PHASE 7: Create Map Selection UI**

#### 7.1 Add Map Selection Menu
**File:** `src/ui/saves_menu.rs` or new `src/ui/map_selection_menu.rs`

**Create new menu screen before "New Game":**
```rust
pub struct MapSelectionMenu {
    pub available_maps: Vec<MapConfiguration>,
    pub selected_index: usize,
}

impl MapSelectionMenu {
    pub fn new() -> Self {
        Self {
            available_maps: vec![
                MapConfiguration::earth_moon_classic(),
                MapConfiguration::solar_system(),
                // Future maps here...
            ],
            selected_index: 0,
        }
    }

    pub fn render(&self) {
        // Display list of maps with descriptions
        for (i, map) in self.available_maps.iter().enumerate() {
            let color = if i == self.selected_index { GREEN } else { WHITE };
            draw_text(&map.name, x, y, font_size, color);
            draw_text(&map.description, x, y + 20, small_font, GRAY);
        }
    }

    pub fn handle_input(&mut self) -> Option<MapConfiguration> {
        if is_key_pressed(KeyCode::Up) {
            self.selected_index = self.selected_index.saturating_sub(1);
        }
        if is_key_pressed(KeyCode::Down) {
            self.selected_index = (self.selected_index + 1).min(self.available_maps.len() - 1);
        }
        if is_key_pressed(KeyCode::Enter) {
            return Some(self.available_maps[self.selected_index].clone());
        }
        None
    }
}
```

---

#### 7.2 Update Main Menu Flow
**File:** `src/main.rs`

**Add MapSelectionMenu to game state:**
```rust
enum GameState {
    MainMenu,
    MapSelection, // NEW
    SinglePlayer(SinglePlayerGame),
    Multiplayer(MultiplayerGame),
    // ...
}
```

**Update menu navigation:**
```rust
// In main loop
match game_state {
    GameState::MainMenu => {
        // User selects "New Game"
        if new_game_selected {
            game_state = GameState::MapSelection;
        }
    }
    GameState::MapSelection => {
        if let Some(selected_map) = map_selection_menu.handle_input() {
            let mut game = SinglePlayerGame::new_with_map(selected_map);
            game.initialize_new_game();
            game_state = GameState::SinglePlayer(game);
        }
    }
    // ...
}
```

---

### **PHASE 8: Testing & Balancing**

#### 8.1 Test Cases
1. **Earth-Moon Classic Map:**
   - Verify Moon still orbits Earth correctly
   - Verify Earth remains stationary
   - Verify rocket spawns on Earth surface
   - Verify save/load works

2. **Solar System Map:**
   - Verify all 9 planets orbit Sun
   - Verify Moon orbits Earth (hierarchical orbit)
   - Verify rocket spawns on Earth with correct velocity
   - Verify planets don't collide or escape
   - Verify camera can zoom out to view entire system

3. **Multiplayer Compatibility:**
   - Ensure multiplayer can use custom maps
   - Test rocket-to-rocket gravity with multiple planets

---

#### 8.2 Balance Adjustments
After initial testing, adjust:
- **Planet masses:** Ensure Jupiter/Saturn don't dominate too much
- **Orbital distances:** Ensure planets are visually distinct
- **Orbital periods:** Fast enough for gameplay, slow enough to be stable
- **Camera zoom levels:** Ensure both close-up and overview are usable
- **Rocket spawn velocity:** Earth's orbital velocity must be inherited

---

### **PHASE 9: Documentation & Future Maps**

#### 9.1 Create Map Creation Guide
**File:** `docs/CREATING_MAPS.md`

Document how to:
- Define new `MapConfiguration` structs
- Balance planet masses and orbital periods
- Test for orbital stability
- Add custom colors and visual themes

---

#### 9.2 Future Map Ideas
- **Binary Star System:** Two suns orbiting each other
- **Three-Body Problem:** Chaotic orbits
- **Rogue Planet:** Planet with no star, drifting through space
- **Asteroid Field:** Many small bodies
- **Black Hole System:** Extreme gravity near center

---

## 📊 SUMMARY OF CHANGES

### Files to Create (10 new files)
| File | Purpose |
|------|---------|
| `src/map_config/mod.rs` | Map configuration types |
| `src/map_config/maps.rs` | Predefined map definitions |
| `src/map_config/orbit_calculator.rs` | Calculate initial states |
| `src/ui/map_selection_menu.rs` | Map selection UI |
| `docs/CREATING_MAPS.md` | Map creation guide |

### Files to Modify (8 existing files)
| File | Changes |
|------|---------|
| `src/game_modes/single_player.rs` | Add map field, refactor initialization |
| `src/entities/planet.rs` | Add name and is_pinned fields |
| `src/systems/world.rs` | Update planet-to-planet gravity (N-body) |
| `src/rendering/camera.rs` | Add zoom and camera modes |
| `src/save_system/game_save_data.rs` | Save map name |
| `src/main.rs` | Add map selection state |
| `src/game_constants.rs` | Make constants map-agnostic |
| `Cargo.toml` | No changes needed (uses existing deps) |

### Key Features
✅ Backward compatible (Earth-Moon Classic map preserves current behavior)
✅ Hierarchical orbits (Moon → Earth → Sun)
✅ Flexible map system (easy to add new maps)
✅ N-body gravity simulation (all planets affect each other)
✅ Scalable camera system (zoom for large maps)
✅ Save/load with map identification
✅ Visual variety (distributed initial angles)

---

## 🎮 GAMEPLAY IMPACT

### Earth-Moon Classic Map (Current)
- **No changes** to existing gameplay
- Earth stays at (400, 300)
- Moon orbits Earth
- Same fuel collection mechanics

### Solar System Map (New)
- **10 celestial bodies** visible at once
- **Earth orbits Sun** - rocket must inherit Earth's velocity
- **Challenging navigation** - multiple gravity wells
- **Resource distribution** - fuel on different planets
- **Long-distance travel** - requires orbital mechanics knowledge
- **Visual spectacle** - see entire solar system in motion

---

## ⚠️ CRITICAL IMPLEMENTATION NOTES

### 1. Rocket Spawning on Orbiting Earth
When rocket spawns on Earth in solar system map:
```rust
spawn_velocity = earth.velocity; // MUST inherit this!
```
Otherwise, rocket will drift away from Earth immediately.

### 2. Hierarchical Orbit Calculation
Moon's total velocity = Earth's velocity + Moon's orbital velocity around Earth:
```rust
moon_velocity = earth_velocity + orbital_velocity_around_earth;
```

### 3. Pinned Planets
Use `is_pinned` flag to prevent planets from moving:
- Earth-Moon Classic: Earth pinned
- Solar System: Sun pinned
- This prevents numerical instability in large systems

### 4. N-Body Stability
With 10+ bodies, numerical integration errors accumulate:
- Use smaller time steps if orbits decay
- Consider symplectic integrator (Verlet) instead of Euler
- Monitor total energy of system during testing

### 5. Camera Zoom
Solar system map spans 900,000 pixels (Sun to Pluto):
- Need zoom levels from 0.01x to 5x
- Default to system overview when spawning
- Allow player to toggle FollowRocket vs SystemOverview

---

## 🚀 ESTIMATED EFFORT

| Phase | Effort | Priority |
|-------|--------|----------|
| Phase 1: Map Config System | 4 hours | HIGH |
| Phase 2: Game Init Refactor | 3 hours | HIGH |
| Phase 3: Entity Updates | 1 hour | MEDIUM |
| Phase 4: Physics Updates | 2 hours | HIGH |
| Phase 5: Camera System | 3 hours | MEDIUM |
| Phase 6: Save/Load | 1 hour | LOW |
| Phase 7: Map Selection UI | 2 hours | MEDIUM |
| Phase 8: Testing & Balance | 4 hours | HIGH |
| Phase 9: Documentation | 1 hour | LOW |
| **TOTAL** | **21 hours** | |

---

## 📝 NEXT STEPS

1. **Review this plan** and confirm approach
2. **Start with Phase 1** - create map configuration system
3. **Test incrementally** - verify each phase before moving to next
4. **Iterate on balance** - planet masses/distances will need tuning

This plan provides a complete roadmap for transforming the game from a 2-body system to a flexible multi-map architecture with a full solar system implementation.
