# Katie Map Maker

A simple GUI tool for creating custom maps for KatieFlySimRust.

## Quick Start

### Running the Map Maker

**Windows:**
```cmd
run-map-maker.cmd
```

**Linux/Mac:**
```bash
cargo run --release
```

### Building the Executable

```bash
cargo build --release
```

The executable will be in `target/release/katie_map_maker.exe` (Windows) or `target/release/katie_map_maker` (Linux/Mac).

## Deployment

**For Release/Distribution:**
1. Place `katie_map_maker.exe` in the **same folder** as `katie_fly_sim_rust.exe`
2. Both executables will share the same `maps/` folder
3. Maps created in the Map Maker will automatically appear in the game!

**Folder Structure:**
```
YourGameFolder/
├── katie_fly_sim_rust.exe
├── katie_map_maker.exe
└── maps/
    ├── example_binary_system.ron
    ├── your_custom_map.ron
    └── ...
```

## How to Use

1. **Launch the Map Maker** - Run the executable
2. **Click "New Map"** to create a new map
3. **Add Celestial Bodies:**
   - Click "Add Celestial Body" to add planets, moons, stars, etc.
   - Currently creates default bodies - future versions will allow full editing
4. **Save Your Map:**
   - Click "Save Map" to save to `maps/` folder (same directory as the executable)
   - Maps are saved as `.ron` files (Rust Object Notation)
5. **Use in Game:**
   - Launch KatieFlySimRust
   - Go to Map Selection menu
   - Your custom map will appear in the list!

## Map File Format

Maps are stored in RON (Rust Object Notation) format. Example:

```ron
(
    name: "My Custom Map",
    description: "A cool custom solar system",
    celestial_bodies: [
        (
            name: "Sun",
            mass: 1000000000.0,
            radius: 20000.0,
            color: (r: 1.0, g: 0.9, b: 0.0, a: 1.0),
            orbital_parent_index: None,
            orbital_distance: None,
            orbital_period: None,
            initial_angle: 0.0,
            is_pinned: true,
        ),
        (
            name: "Planet",
            mass: 50000000.0,
            radius: 8000.0,
            color: (r: 0.3, g: 0.6, b: 1.0, a: 1.0),
            orbital_parent_index: Some(0),
            orbital_distance: Some(100000.0),
            orbital_period: Some(600.0),
            initial_angle: 0.0,
            is_pinned: false,
        ),
    ],
    player_spawn_body_index: 1,
    central_body_index: Some(0),
)
```

## Map Properties

### MapConfiguration
- `name` - Map identifier (used for loading/saving)
- `description` - Description shown in menu
- `celestial_bodies` - List of planets/moons/stars
- `player_spawn_body_index` - Which body the player spawns on (0-based)
- `central_body_index` - Which body is the center of the system (optional)

### CelestialBodyConfig
- `name` - Display name
- `mass` - Mass (affects gravity). Typical values: 10,000,000 to 1,000,000,000
- `radius` - Visual radius in pixels. Typical values: 2,000 to 20,000
- `color` - RGBA color (r, g, b, a all 0.0 to 1.0)
- `orbital_parent_index` - Which body this orbits (None = stationary)
- `orbital_distance` - Distance from parent (pixels)
- `orbital_period` - Time to complete orbit (seconds)
- `initial_angle` - Starting angle in radians (0 = right, π/2 = up)
- `is_pinned` - If true, body doesn't move

## Tips for Map Making

1. **Start Simple:** Begin with 2-3 bodies to test
2. **Mass Matters:** Higher mass = stronger gravity
3. **Orbital Distance:** Should be at least 3x the radius of the parent
4. **Pinned Bodies:** Use for suns/central stars that shouldn't move
5. **Spawn Location:** Choose a body with stable orbit for player spawn
6. **Test Often:** Save and test in the game frequently

## Future Features

The current version is basic. Planned features:
- Full UI for editing body properties
- Color picker for celestial bodies
- Visual preview of orbits
- Load existing maps for editing
- Validation warnings
- Copy/paste bodies
- Templates for common systems

## Troubleshooting

**Map doesn't appear in game:**
- Check that the file is in `KatieFlySimRust/maps/`
- Verify the file has a `.ron` extension
- Check the console for loading errors

**Game crashes when loading map:**
- Verify all required fields are present
- Check that `player_spawn_body_index` is valid (less than number of bodies)
- Ensure masses and radii are positive numbers

**Map loads but looks wrong:**
- Check orbital_distance values
- Verify orbital_period is reasonable (60-1000 seconds)
- Make sure is_pinned is true for at least one body (the center)
