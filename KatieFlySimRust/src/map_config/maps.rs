use super::*;
use crate::game_constants::*;

impl MapConfiguration {
    /// Original Earth-Moon system (backward compatibility)
    pub fn earth_moon() -> Self {
        MapConfiguration {
            name: "earth moon".to_string(),
            description: "Classic two-body system: Earth and Moon".to_string(),
            celestial_bodies: vec![
                // --- EARTH (index 0) ---
                CelestialBodyConfig {
                    name: "Earth".to_string(),
                    mass: GameConstants::MAIN_PLANET_MASS,
                    radius: GameConstants::MAIN_PLANET_RADIUS,
                    color: BLUE,
                    orbital_parent_index: None,
                    orbital_distance: None,
                    orbital_period: None,
                    initial_angle: 0.0,
                    is_pinned: true, // Stays at origin
                },
                // --- MOON (index 1) ---
                CelestialBodyConfig {
                    name: "Moon".to_string(),
                    mass: GameConstants::SECONDARY_PLANET_MASS,
                    radius: GameConstants::SECONDARY_PLANET_RADIUS,
                    color: Color::from_rgba(150, 150, 150, 255),
                    orbital_parent_index: Some(0), // Orbits Earth
                    orbital_distance: Some(*PLANET_ORBIT_DISTANCE),
                    orbital_period: Some(GameConstants::ORBIT_PERIOD),
                    initial_angle: 0.0, // Starts to the right of Earth
                    is_pinned: false,
                },
            ],
            player_spawn_body_index: 0, // Spawn on Earth
            central_body_index: Some(0), // Earth is center
        }
    }

    /// Solar System: Gameplay-optimized scale with Sun + 9 planets + Moon
    /// Scaled for slower orbital periods: Earth takes 365.25 minutes to orbit Sun (1 real minute = 1 game day)
    /// Using 1 AU = 42,000,000 pixels (14x larger than realistic for slower orbits)
    pub fn solar_1() -> Self {
        // Earth mass and radius as reference (unchanged from original game)
        let earth_mass = GameConstants::MAIN_PLANET_MASS; // 198,910,000
        let earth_radius = GameConstants::MAIN_PLANET_RADIUS; // 10,000
        let au = 42_000_000.0; // 1 Astronomical Unit in pixels (14x larger for 52x slower orbits)

        MapConfiguration {
            name: "solar 1".to_string(),
            description: "Solar system scaled for gameplay: Earth orbits in 365.25 minutes (1 min = 1 day)".to_string(),
            celestial_bodies: vec![
                // --- SUN (index 0) ---
                // Mass: 333,000 Earth masses, Radius: 109 Earth radii
                CelestialBodyConfig {
                    name: "Sun".to_string(),
                    mass: earth_mass * 333_000.0, // 66,236,130,000,000
                    radius: earth_radius * 109.0, // 1,090,000 pixels (MASSIVE!)
                    color: YELLOW,
                    orbital_parent_index: None,
                    orbital_distance: None,
                    orbital_period: None,
                    initial_angle: 0.0,
                    is_pinned: true, // Center of solar system
                },

                // --- MERCURY (index 1) ---
                // Orbital distance: 0.39 AU, Mass: 0.055 Earth, Radius: 0.38 Earth
                CelestialBodyConfig {
                    name: "Mercury".to_string(),
                    mass: earth_mass * 0.055, // 10,940,050
                    radius: earth_radius * 0.38, // 3,800
                    color: Color::from_rgba(169, 169, 169, 255), // Dark gray
                    orbital_parent_index: Some(0), // Orbits Sun
                    orbital_distance: Some(au * 0.39), // 1,170,000 (safely outside Sun!)
                    orbital_period: None, // Calculated from physics
                    initial_angle: 0.0,
                    is_pinned: false,
                },

                // --- VENUS (index 2) ---
                // Orbital distance: 0.72 AU, Mass: 0.815 Earth, Radius: 0.95 Earth
                CelestialBodyConfig {
                    name: "Venus".to_string(),
                    mass: earth_mass * 0.815, // 162,111,650
                    radius: earth_radius * 0.95, // 9,500
                    color: Color::from_rgba(255, 198, 73, 255), // Orange-yellow
                    orbital_parent_index: Some(0),
                    orbital_distance: Some(au * 0.72), // 2,160,000
                    orbital_period: None,
                    initial_angle: std::f32::consts::PI / 4.0, // 45 degrees
                    is_pinned: false,
                },

                // --- EARTH (index 3) ---
                // Orbital distance: 1.0 AU, Mass: 1.0 Earth, Radius: 1.0 Earth
                CelestialBodyConfig {
                    name: "Earth".to_string(),
                    mass: earth_mass, // 198,910,000 (UNCHANGED)
                    radius: earth_radius, // 10,000 (UNCHANGED)
                    color: BLUE,
                    orbital_parent_index: Some(0),
                    orbital_distance: Some(au), // 3,000,000
                    orbital_period: None, // Physics-calculated
                    initial_angle: std::f32::consts::PI / 2.0, // 90 degrees
                    is_pinned: false, // Orbits Sun
                },

                // --- MOON (index 4) ---
                // Orbital distance: 0.00257 AU from Earth, Mass: 0.0123 Earth, Radius: 0.27 Earth
                CelestialBodyConfig {
                    name: "Moon".to_string(),
                    mass: earth_mass * 0.0123, // 2,446,593 (realistic mass)
                    radius: earth_radius * 0.27, // 2,700 (realistic radius)
                    color: Color::from_rgba(150, 150, 150, 255),
                    orbital_parent_index: Some(3), // Orbits Earth
                    orbital_distance: Some(30_000.0), // ~0.075 AU from Earth (scaled for gameplay)
                    orbital_period: None,
                    initial_angle: 0.0,
                    is_pinned: false,
                },

                // --- MARS (index 5) ---
                // Orbital distance: 1.52 AU, Mass: 0.107 Earth, Radius: 0.53 Earth
                CelestialBodyConfig {
                    name: "Mars".to_string(),
                    mass: earth_mass * 0.107, // 21,283,370
                    radius: earth_radius * 0.53, // 5,300
                    color: Color::from_rgba(193, 68, 14, 255), // Red
                    orbital_parent_index: Some(0),
                    orbital_distance: Some(au * 1.52), // 4,560,000
                    orbital_period: None,
                    initial_angle: std::f32::consts::PI, // 180 degrees
                    is_pinned: false,
                },

                // --- JUPITER (index 6) ---
                // Orbital distance: 5.2 AU, Mass: 317.8 Earth, Radius: 11.2 Earth
                CelestialBodyConfig {
                    name: "Jupiter".to_string(),
                    mass: earth_mass * 317.8, // 63,197,998,000 (REALISTIC MASS!)
                    radius: earth_radius * 11.2, // 112,000 (HUGE!)
                    color: Color::from_rgba(201, 176, 55, 255), // Tan
                    orbital_parent_index: Some(0),
                    orbital_distance: Some(au * 5.2), // 15,600,000
                    orbital_period: None,
                    initial_angle: 3.0 * std::f32::consts::PI / 2.0, // 270 degrees
                    is_pinned: false,
                },

                // --- SATURN (index 7) ---
                // Orbital distance: 9.54 AU, Mass: 95.2 Earth, Radius: 9.45 Earth
                CelestialBodyConfig {
                    name: "Saturn".to_string(),
                    mass: earth_mass * 95.2, // 18,934,232,000
                    radius: earth_radius * 9.45, // 94,500
                    color: Color::from_rgba(250, 227, 133, 255), // Pale yellow
                    orbital_parent_index: Some(0),
                    orbital_distance: Some(au * 9.54), // 28,620,000
                    orbital_period: None,
                    initial_angle: 2.0 * std::f32::consts::PI / 3.0, // 120 degrees
                    is_pinned: false,
                },

                // --- URANUS (index 8) ---
                // Orbital distance: 19.19 AU, Mass: 14.5 Earth, Radius: 4.0 Earth
                CelestialBodyConfig {
                    name: "Uranus".to_string(),
                    mass: earth_mass * 14.5, // 2,884,195,000
                    radius: earth_radius * 4.0, // 40,000
                    color: Color::from_rgba(79, 208, 231, 255), // Cyan
                    orbital_parent_index: Some(0),
                    orbital_distance: Some(au * 19.19), // 57,570,000
                    orbital_period: None,
                    initial_angle: std::f32::consts::PI / 6.0, // 30 degrees
                    is_pinned: false,
                },

                // --- NEPTUNE (index 9) ---
                // Orbital distance: 30.07 AU, Mass: 17.1 Earth, Radius: 3.88 Earth
                CelestialBodyConfig {
                    name: "Neptune".to_string(),
                    mass: earth_mass * 17.1, // 3,401,361,000
                    radius: earth_radius * 3.88, // 38,800
                    color: Color::from_rgba(62, 84, 232, 255), // Deep blue
                    orbital_parent_index: Some(0),
                    orbital_distance: Some(au * 30.07), // 90,210,000
                    orbital_period: None,
                    initial_angle: 5.0 * std::f32::consts::PI / 6.0, // 150 degrees
                    is_pinned: false,
                },

                // --- PLUTO (index 10) ---
                // Orbital distance: 39.48 AU, Mass: 0.0022 Earth, Radius: 0.18 Earth
                CelestialBodyConfig {
                    name: "Pluto".to_string(),
                    mass: earth_mass * 0.0022, // 437,602
                    radius: earth_radius * 0.18, // 1,800
                    color: Color::from_rgba(165, 149, 140, 255), // Brown-gray
                    orbital_parent_index: Some(0),
                    orbital_distance: Some(au * 39.48), // 118,440,000 (VERY FAR!)
                    orbital_period: None,
                    initial_angle: 4.0 * std::f32::consts::PI / 3.0, // 240 degrees
                    is_pinned: false,
                },
            ],
            player_spawn_body_index: 3, // Spawn on Earth (index 3)
            central_body_index: Some(0), // Sun is center
        }
    }

    pub fn katie_1() -> Self {
        MapConfiguration {
            name: "Katie_1".to_string(),
            description: "Katie system: Earth, Moon, and Katie".to_string(),
            celestial_bodies: vec![
                // --- EARTH (index 0) - Central body ---
                CelestialBodyConfig {
                    name: "Earth".to_string(),
                    mass: 198_910_000.0,        // Earth's mass
                    radius: 10_000.0,           // Earth's radius in pixels
                    color: BLUE,
                    orbital_parent_index: None,
                    orbital_distance: None,
                    orbital_period: None,
                    initial_angle: 0.0,
                    is_pinned: true,            // Stays at origin (center of map)
                },

                // --- MOON (index 1) - Orbits Earth ---
                CelestialBodyConfig {
                    name: "Moon".to_string(),
                    mass: 11_934_600.0,         // Moon's mass (6% of Earth)
                    radius: 600.0,              // Moon's radius in pixels (6% of Earth)
                    color: Color::from_rgba(150, 150, 150, 255), // Gray
                    orbital_parent_index: Some(0), // Orbits Earth
                    orbital_distance: Some(30_000.0), // Distance from Earth in pixels
                    orbital_period: Some(120.0), // Time for one orbit in seconds
                    initial_angle: 0.0,         // Starting position (0 = right, PI/2 = top, PI = left)
                    is_pinned: false,
                },

                // --- KATIE (index 2) - Orbits Moon ---
                CelestialBodyConfig {
                    name: "Katie".to_string(),
                    mass: 3_978_200.0,        // Katie's mass
                    radius: 200.0,            // Katie's radius in pixels (20% of Earth size)
                    color: Color::from_rgba(255, 105, 180, 255), // HOT PINK for easy visibility!
                    orbital_parent_index: Some(1), // Orbits Moon
                    orbital_distance: Some(10_000.0), // Distance from Moon
                    orbital_period: Some(60.0), // Time for one orbit in seconds (1/2x Moon's period)
                    initial_angle: 0.0, // Starting position
                    is_pinned: false,
                },
            ],
            player_spawn_body_index: 0, // Spawn on Earth (index 0)
            central_body_index: Some(0), // Earth is center of view
        }
    }

    /// Load custom maps from the maps/ folder
    pub fn load_custom_maps() -> Vec<MapConfiguration> {
        let mut custom_maps = Vec::new();
        let maps_folder = "maps";

        // Create maps folder if it doesn't exist
        let _ = std::fs::create_dir_all(maps_folder);

        // Try to read the maps directory
        if let Ok(entries) = std::fs::read_dir(maps_folder) {
            for entry in entries.flatten() {
                if let Ok(path) = entry.path().canonicalize() {
                    if path.extension().and_then(|s| s.to_str()) == Some("ron") {
                        match MapConfiguration::load_from_file(path.to_str().unwrap()) {
                            Ok(map) => {
                                println!("Loaded custom map: {}", map.name);
                                custom_maps.push(map);
                            }
                            Err(e) => {
                                println!("Failed to load map from {:?}: {}", path, e);
                            }
                        }
                    }
                }
            }
        }

        custom_maps
    }

    /// Get all available maps (built-in + custom)
    pub fn all_maps() -> Vec<MapConfiguration> {
        let mut maps = vec![
            MapConfiguration::earth_moon(),
            MapConfiguration::solar_1(),
            MapConfiguration::katie_1(),
        ];

        // Add custom maps
        maps.extend(MapConfiguration::load_custom_maps());

        maps
    }
}
