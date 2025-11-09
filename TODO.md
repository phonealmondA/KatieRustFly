# KatieRustFly - TODO List

**Last Updated**: 2025-11-09

This document contains all actionable tasks needed to complete the integration of existing framework code into the playable game.

---

## 🔴 HIGH PRIORITY - Integration Tasks

These tasks will make existing framework code actually playable.

### 1. Integrate Multiplayer into main.rs

**Files to modify**:
- `src/main.rs` (lines 142-170)
- `src/game_modes/multiplayer_host.rs` (create new file)
- `src/game_modes/multiplayer_client.rs` (create new file)

**Current status**:
- `src/networking/multiplayer_host.rs` ✅ Complete (~457 lines)
- `src/networking/multiplayer_client.rs` ✅ Complete (~475 lines)
- `src/networking/network_manager.rs` ✅ Complete (~343 lines)

**Tasks**:
- [ ] Remove TODO at `src/main.rs:144` - Implement multiplayer menu handling
- [ ] Remove TODO at `src/main.rs:150` - Implement online multiplayer menu handling
- [ ] Remove TODO at `src/main.rs:156` - Implement multiplayer host handling
- [ ] Remove TODO at `src/main.rs:162` - Implement multiplayer client handling
- [ ] Create `MultiplayerHostGame` wrapper to integrate host into game loop
- [ ] Create `MultiplayerClientGame` wrapper to integrate client into game loop
- [ ] Add player state synchronization to `World` system
- [ ] Hook up multiplayer menus to actually launch host/client

**Estimated effort**: 4-6 hours

---

### 2. Integrate Split-Screen Mode into main.rs

**Files to modify**:
- `src/main.rs` (line 168)
- `src/game_modes/split_screen.rs` (needs integration work)

**Current status**:
- `src/game_modes/split_screen.rs` ✅ Complete (~552 lines)
- Player input mappings ✅ Defined
- Viewport system ✅ Implemented

**Tasks**:
- [ ] Remove TODO at `src/main.rs:168` - Implement split screen handling
- [ ] Create `SplitScreenGame` instance in main.rs game state
- [ ] Add split-screen initialization for 2-4 players
- [ ] Hook up player input routing
- [ ] Implement viewport rendering in main loop
- [ ] Remove TODO at `src/game_modes/split_screen.rs:278` - Implement stage separation

**Estimated effort**: 3-4 hours

---

### 3. Integrate GameInfoDisplay into SinglePlayerGame

**Files to modify**:
- `src/game_modes/single_player.rs`
- `src/ui/hud.rs` (may deprecate or extend)

**Current status**:
- `src/ui/game_info_display.rs` ✅ Complete (~350 lines)
- 5 panels implemented: Rocket, Planet, Orbit, Controls, Network
- Currently using basic `Hud` instead

**Tasks**:
- [ ] Replace `hud: Hud` with `info_display: GameInfoDisplay` in `SinglePlayerGame`
- [ ] Add update calls for all 5 panels
- [ ] Integrate rocket info panel (replace basic HUD)
- [ ] Integrate planet info panel (nearest planet detection)
- [ ] Integrate orbit info panel (apoapsis/periapsis calculations)
- [ ] Integrate controls panel (keyboard shortcuts display)
- [ ] Add keyboard toggles for panel visibility (1-5 keys?)
- [ ] Update render method to draw all panels

**Estimated effort**: 2-3 hours

---

### 4. Integrate SatelliteManager into World System

**Files to modify**:
- `src/systems/world.rs`
- `src/game_modes/single_player.rs`

**Current status**:
- `src/systems/satellite_manager.rs` ✅ Complete (~900 lines)
- `src/systems/fuel_transfer_network.rs` ✅ Complete (~550 lines)
- `src/systems/orbit_maintenance.rs` ✅ Complete (~450 lines)
- None are instantiated or used

**Tasks**:
- [ ] Add `satellite_manager: SatelliteManager` to `World` struct
- [ ] Initialize satellite manager in `World::new()`
- [ ] Hook satellite creation to go through manager
- [ ] Integrate fuel transfer network into satellite updates
- [ ] Integrate orbit maintenance into satellite updates
- [ ] Add satellite network statistics to HUD/info display
- [ ] Add keyboard shortcuts for satellite commands
- [ ] Wire up autonomous fuel collection
- [ ] Wire up station-keeping system

**Estimated effort**: 4-5 hours

---

### 5. Integrate VehicleManager

**Files to modify**:
- `src/game_modes/single_player.rs`

**Current status**:
- `src/systems/vehicle_manager.rs` ✅ Complete (~230 lines)
- Visualization controls implemented
- Not used by SinglePlayerGame

**Tasks**:
- [ ] Add `vehicle_manager: VehicleManager` to `SinglePlayerGame`
- [ ] Move rocket management from World to VehicleManager
- [ ] Integrate trajectory visualization toggles (T key)
- [ ] Integrate gravity force visualization toggles (G key)
- [ ] Add visualization HUD overlay

**Estimated effort**: 2-3 hours

---

## 🟡 MEDIUM PRIORITY - Missing Features

### 6. Implement UIManager System

**Files to modify**:
- Create `src/ui/ui_manager_wrapper.rs` (integration layer)
- `src/game_modes/single_player.rs`

**Current status**:
- `src/ui/ui_manager.rs` ✅ Exists (~400 lines)
- Not integrated

**Tasks**:
- [ ] Create font management system
- [ ] Integrate fuel collection line visualization
- [ ] Integrate satellite network line visualization
- [ ] Add transfer animation system
- [ ] Wire up to rendering pipeline

**Estimated effort**: 3-4 hours

---

### 7. Complete Physics System

**File**: `src/systems/world.rs`

**Tasks**:
- [ ] Remove TODO at `src/systems/world.rs:313` - Apply rocket-to-rocket gravity
- [ ] Implement N-body simulation for rockets
- [ ] Add performance optimization for many rockets (spatial partitioning?)
- [ ] Add configuration toggle for rocket-to-rocket gravity

**Estimated effort**: 2-3 hours

---

### 8. Configuration System

**Files to create**:
- `src/config.rs`
- `config.toml` (root directory)

**Tasks**:
- [ ] Create configuration structure
- [ ] Add TOML file loading with `toml` crate
- [ ] Remove hardcoded player names
  - Fix TODO at `src/networking/multiplayer_client.rs:300`
- [ ] Remove hardcoded network settings
  - Fix TODO at `src/networking/multiplayer_host.rs:171`
- [ ] Add graphics options
- [ ] Add control bindings customization
- [ ] Add configuration menu (optional)

**Estimated effort**: 3-4 hours

---

## 🟢 LOW PRIORITY - Polish & Enhancements

### 9. Visual Enhancements

**Tasks**:
- [ ] Add rocket parts rendering (engines, fuel tanks)
  - Currently just triangles
- [ ] Add thrust visual effects
- [ ] Add particle systems for exhaust
- [ ] Improve satellite rendering
- [ ] Add orbital path rendering for satellites
- [ ] Add fuel transfer line animations

**Estimated effort**: 4-6 hours

---

### 10. Sound System

**Tasks**:
- [ ] Add `quad-snd` or similar audio crate
- [ ] Add thrust sound effects
- [ ] Add UI click sounds
- [ ] Add collision sounds
- [ ] Add ambient space music
- [ ] Add volume controls

**Estimated effort**: 4-5 hours

---

### 11. Additional Content

**Tasks**:
- [ ] Add more planet types (gas giants, asteroids)
- [ ] Add more challenging scenarios
- [ ] Add tutorial system for new players
- [ ] Add achievements system
- [ ] Add mission objectives
- [ ] Add different rocket types

**Estimated effort**: Variable (8+ hours)

---

### 12. Testing & Quality

**Tasks**:
- [ ] Fix 5 failing unit tests (macroquad context issues)
  - `src/ui/game_info_display.rs` tests
- [ ] Add integration tests for multiplayer
- [ ] Add integration tests for split-screen
- [ ] Add performance benchmarks
- [ ] Add memory leak detection
- [ ] Run `cargo clippy` and fix all warnings
- [ ] Add CI/CD pipeline

**Estimated effort**: 4-6 hours

---

### 13. Documentation

**Tasks**:
- [ ] Add inline documentation for all public APIs
- [ ] Generate rustdoc with `cargo doc`
- [ ] Create architecture diagrams
- [ ] Create gameplay video/GIFs for README
- [ ] Add contributing guide (CONTRIBUTING.md)
- [ ] Add code of conduct

**Estimated effort**: 3-4 hours

---

## 📋 Summary by Priority

| Priority | Tasks | Estimated Hours | Impact |
|----------|-------|-----------------|--------|
| 🔴 High | 5 | 17-24 hours | Makes framework code playable |
| 🟡 Medium | 3 | 8-11 hours | Completes missing features |
| 🟢 Low | 5 | 23-35+ hours | Polish and content |
| **Total** | **13** | **48-70 hours** | Full feature completion |

---

## 🎯 Recommended Order

1. **GameInfoDisplay integration** (2-3h) - Quick win, better UX
2. **SatelliteManager integration** (4-5h) - Unlocks advanced features
3. **VehicleManager integration** (2-3h) - Better visualization
4. **Split-screen integration** (3-4h) - Local multiplayer working
5. **Multiplayer integration** (4-6h) - Online multiplayer working
6. **Rocket-to-rocket gravity** (2-3h) - Physics completion
7. **Configuration system** (3-4h) - Better customization
8. **UIManager** (3-4h) - Visual polish
9. **Testing fixes** (4-6h) - Quality assurance
10. Everything else as desired

**Estimated to "Feature Complete"**: ~25-35 hours (items 1-6)

---

## 📂 Code Locations Reference

### Working Code (Integrated)
- ✅ `src/entities/` - All entity types
- ✅ `src/physics/gravity_simulator.rs` - Gravity physics
- ✅ `src/physics/trajectory.rs` - Trajectory prediction
- ✅ `src/systems/world.rs` - Entity management (partial)
- ✅ `src/ui/camera.rs` - Camera system
- ✅ `src/ui/hud.rs` - Basic HUD
- ✅ `src/menus/` - Menu system
- ✅ `src/save_system/` - Save/load
- ✅ `src/game_modes/single_player.rs` - Single player

### Framework Code (Not Integrated)
- ⚠️ `src/networking/` - Multiplayer framework
- ⚠️ `src/game_modes/split_screen.rs` - Split-screen framework
- ⚠️ `src/ui/game_info_display.rs` - Advanced UI
- ⚠️ `src/ui/ui_manager.rs` - UI coordination
- ⚠️ `src/systems/satellite_manager.rs` - Satellite management
- ⚠️ `src/systems/fuel_transfer_network.rs` - Fuel routing
- ⚠️ `src/systems/orbit_maintenance.rs` - Station-keeping
- ⚠️ `src/systems/vehicle_manager.rs` - Vehicle visualization

---

## 🚀 Quick Start for Contributors

Want to help? Pick a task from the High Priority section!

1. Read the existing code in the "Framework Code" location
2. Check the "Files to modify" section
3. Look at the existing integration in SinglePlayerGame as a reference
4. Follow the task checklist
5. Run `cargo test` to ensure nothing breaks
6. Submit a PR!

---

**Status**: This TODO represents ~48-70 hours of work to reach 100% feature parity with the original C++ version.

**Current Progress**: Single player complete (~70% of original features working)
