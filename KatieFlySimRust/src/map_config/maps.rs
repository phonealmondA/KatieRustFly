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

    /// Solar System: Realistic scale with Sun + 9 planets + Moon
    /// Using 1 AU = 400,000 pixels, Earth radius = 10,000 as reference
    pub fn solar_1() -> Self {
        // Earth mass and radius as reference (unchanged from original game)
        let earth_mass = GameConstants::MAIN_PLANET_MASS; // 198,910,000
        let earth_radius = GameConstants::MAIN_PLANET_RADIUS; // 10,000
        let au = 400_000.0; // 1 Astronomical Unit in pixels

        MapConfiguration {
            name: "solar 1".to_string(),
            description: "Realistic solar system with proper scales and distances".to_string(),
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
                    orbital_distance: Some(au * 0.39), // 156,000
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
                    orbital_distance: Some(au * 0.72), // 288,000
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
                    orbital_distance: Some(au), // 400,000
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
                    orbital_distance: Some(au * 1.52), // 608,000
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
                    orbital_distance: Some(au * 5.2), // 2,080,000
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
                    orbital_distance: Some(au * 9.54), // 3,816,000
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
                    orbital_distance: Some(au * 19.19), // 7,676,000
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
                    orbital_distance: Some(au * 30.07), // 12,028,000
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
                    orbital_distance: Some(au * 39.48), // 15,792,000 (VERY FAR!)
                    orbital_period: None,
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
