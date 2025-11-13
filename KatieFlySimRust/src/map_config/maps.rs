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

    /// Solar System: Sun + 9 planets (Mercury to Pluto) + Moon
    pub fn solar_1() -> Self {
        MapConfiguration {
            name: "solar 1".to_string(),
            description: "Full solar system with all nine planets".to_string(),
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
                    orbital_period: Some(200.0),
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
                    mass: GameConstants::MAIN_PLANET_MASS, // KEEP CURRENT MASS
                    radius: GameConstants::MAIN_PLANET_RADIUS,
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
                    mass: GameConstants::SECONDARY_PLANET_MASS, // KEEP CURRENT MASS
                    radius: GameConstants::SECONDARY_PLANET_RADIUS,
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
                    initial_angle: std::f32::consts::PI, // 180 degrees
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
                    initial_angle: 3.0 * std::f32::consts::PI / 2.0, // 270 degrees
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
                    initial_angle: 2.0 * std::f32::consts::PI / 3.0, // 120 degrees
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
                    initial_angle: std::f32::consts::PI / 6.0, // 30 degrees
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
                    initial_angle: 5.0 * std::f32::consts::PI / 6.0, // 150 degrees
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
                    initial_angle: 4.0 * std::f32::consts::PI / 3.0, // 240 degrees
                    is_pinned: false,
                },
            ],
            player_spawn_body_index: 3, // Spawn on Earth (index 3)
            central_body_index: Some(0), // Sun is center
        }
    }

    /// Get all available maps
    pub fn all_maps() -> Vec<MapConfiguration> {
        vec![
            MapConfiguration::earth_moon(),
            MapConfiguration::solar_1(),
        ]
    }
}
