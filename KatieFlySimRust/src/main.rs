// KatieFlySimRust - Main Entry Point
// Rust port of FlySimNewA space flight simulator
// Now using macroquad for pure Rust graphics (no external dependencies!)

use macroquad::prelude::*;

use katie_fly_sim_rust::game_constants::GameConstants;
use katie_fly_sim_rust::game_modes::{
    SinglePlayerGame, SinglePlayerResult,
    SplitScreenGame, SplitScreenResult,
    MultiplayerHost, MultiplayerHostResult,
    MultiplayerClient, MultiplayerClientResult,
};
use katie_fly_sim_rust::game_state::{GameMode, GameState};
use katie_fly_sim_rust::map_config::MapConfiguration;
use katie_fly_sim_rust::menus::{
    MainMenu, SavesMenu, SavesMenuResult,
    MapSelectionMenu, MapSelectionResult,
    MultiplayerMenu, MultiplayerMenuResult,
    OnlineMultiplayerMenu, OnlineMultiplayerMenuResult,
    OnlineHostMenu, OnlineHostMenuResult,
    MultiplayerSavesMenu, MultiplayerSavesMenuResult,
    OnlineJoinMenu, OnlineJoinMenuResult,
};
use katie_fly_sim_rust::save_system::GameSaveData;

// Window configuration
fn window_conf() -> Conf {
    Conf {
        window_title: "KatieFlySimRust - Space Flight Simulator".to_owned(),
        window_width: 1920,
        window_height: 1080,
        window_resizable: false,
        fullscreen: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // Initialize logger
    env_logger::init();
    log::info!("Starting KatieFlySimRust v0.1.0");
    log::info!("Rust port of FlySimNewA - Space Flight Simulator");
    log::info!("Using macroquad for pure Rust graphics (no SFML dependency!)");

    let window_width = screen_width();
    let window_height = screen_height();

    log::info!("Window created: {}x{}", window_width, window_height);
    log::info!("Gravitational constant G = {}", GameConstants::G);

    let window_size = Vec2::new(window_width, window_height);

    // Game state
    let mut game_state = GameState::MainMenu;
    let mut main_menu = MainMenu::new(window_size);
    let mut saves_menu = SavesMenu::new(window_size);
    let mut map_selection_menu = MapSelectionMenu::new(window_size);
    let mut multiplayer_menu = MultiplayerMenu::new(window_size);
    let mut online_multiplayer_menu = OnlineMultiplayerMenu::new(window_size);
    let mut online_host_menu = OnlineHostMenu::new(window_size);
    let mut multiplayer_saves_menu = MultiplayerSavesMenu::new(window_size);
    let mut online_join_menu = OnlineJoinMenu::new(window_size);
    let mut single_player_game: Option<SinglePlayerGame> = None;
    let mut split_screen_game: Option<SplitScreenGame> = None;
    let mut multiplayer_host: Option<MultiplayerHost> = None;
    let mut multiplayer_client: Option<MultiplayerClient> = None;

    // Store player name and port from menus
    let mut host_player_name: Option<String> = None;
    let mut host_port: Option<u16> = None;

    // Frame tracking
    let mut frame_count = 0u64;
    let mut fps_timer = 0.0f32;

    // Fixed timestep for smooth physics
    const PHYSICS_TIMESTEP: f32 = 1.0 / 120.0; // 120 Hz physics for ultra-smooth movement
    let mut physics_accumulator = 0.0f32;

    log::info!("Entering main game loop with fixed timestep physics (120 Hz)");

    // Main game loop
    loop {
        let delta_time = get_frame_time().min(0.1); // Cap max frame time to prevent spiral of death
        frame_count += 1;
        fps_timer += delta_time;

        // Accumulate frame time for fixed timestep physics
        physics_accumulator += delta_time;

        // Handle input based on game state
        match game_state {
            GameState::MainMenu => {
                let selected = main_menu.update();
                match selected {
                    GameMode::SinglePlayer => {
                        log::info!("Single Player mode selected");
                        game_state = GameState::SavesMenu;
                        saves_menu.refresh_saves();
                    }
                    GameMode::Multiplayer => {
                        log::info!("Multiplayer mode selected");
                        game_state = GameState::MultiplayerMenu;
                    }
                    GameMode::Quit => {
                        log::info!("Quit selected");
                        break;
                    }
                    GameMode::None => {}
                }
            }

            GameState::SavesMenu => {
                let result = saves_menu.update();
                match result {
                    SavesMenuResult::NewGame => {
                        log::info!("New game selected, showing map selection");
                        game_state = GameState::MapSelection;
                    }
                    SavesMenuResult::LoadGame(save_name) => {
                        log::info!("Loading game: {}", save_name);
                        match GameSaveData::load_from_file(&save_name) {
                            Ok(save_data) => {
                                let mut loaded_game = SinglePlayerGame::new(window_size);
                                loaded_game.load_from_save(save_data, save_name);
                                single_player_game = Some(loaded_game);
                                game_state = GameState::Playing;
                            }
                            Err(e) => {
                                log::error!("Failed to load save: {}", e);
                            }
                        }
                    }
                    SavesMenuResult::Back => {
                        log::info!("Returning to main menu from saves");
                        game_state = GameState::MainMenu;
                    }
                    SavesMenuResult::None => {}
                }
            }

            GameState::MapSelection => {
                let result = map_selection_menu.update();
                match result {
                    MapSelectionResult::MapSelected(map_name) => {
                        log::info!("Map selected: {}", map_name);
                        // Find the map configuration by name from all available maps
                        let all_maps = MapConfiguration::all_maps();
                        let selected_map = all_maps
                            .into_iter()
                            .find(|m| m.name == map_name)
                            .unwrap_or_else(|| {
                                log::warn!("Map '{}' not found, using default", map_name);
                                MapConfiguration::earth_moon()
                            });

                        let mut new_game = SinglePlayerGame::new_with_map(window_size, selected_map);
                        new_game.initialize_new_game();
                        single_player_game = Some(new_game);
                        game_state = GameState::Playing;
                    }
                    MapSelectionResult::Back => {
                        log::info!("Returning to saves menu from map selection");
                        game_state = GameState::SavesMenu;
                    }
                    MapSelectionResult::None => {}
                }
            }

            GameState::Playing => {
                if let Some(ref mut game) = single_player_game {
                    // Handle input
                    match game.handle_input() {
                        SinglePlayerResult::ReturnToMenu => {
                            log::info!("Returning to main menu");
                            game_state = GameState::MainMenu;
                            main_menu.reset();
                        }
                        SinglePlayerResult::Quit => {
                            log::info!("Quit requested from game");
                            break;
                        }
                        _ => {}
                    }

                    // Fixed timestep physics update
                    while physics_accumulator >= PHYSICS_TIMESTEP {
                        game.update(PHYSICS_TIMESTEP);
                        physics_accumulator -= PHYSICS_TIMESTEP;
                    }
                }
            }

            GameState::Paused => {
                // Paused state - don't update game, but still render
            }

            GameState::Quit => {
                break;
            }

            GameState::MultiplayerMenu => {
                let result = multiplayer_menu.update();
                match result {
                    MultiplayerMenuResult::SplitScreen => {
                        log::info!("Split-Screen selected");
                        let mut new_game = SplitScreenGame::new(window_size);
                        new_game.initialize_new_game();
                        split_screen_game = Some(new_game);
                        game_state = GameState::SplitScreen;
                    }
                    MultiplayerMenuResult::OnlineMultiplayer => {
                        log::info!("Online Multiplayer selected");
                        game_state = GameState::OnlineMultiplayerMenu;
                    }
                    MultiplayerMenuResult::Back => {
                        log::info!("Returning to main menu from multiplayer");
                        game_state = GameState::MainMenu;
                    }
                    MultiplayerMenuResult::None => {}
                }
            }

            GameState::SplitScreen => {
                if let Some(ref mut game) = split_screen_game {
                    // Handle input
                    match game.handle_input() {
                        SplitScreenResult::ReturnToMenu => {
                            log::info!("Returning to multiplayer menu from split-screen");
                            game_state = GameState::MultiplayerMenu;
                        }
                        SplitScreenResult::Quit => {
                            log::info!("Quit requested from split-screen");
                            break;
                        }
                        _ => {}
                    }

                    // Fixed timestep physics update
                    while physics_accumulator >= PHYSICS_TIMESTEP {
                        game.update(PHYSICS_TIMESTEP);
                        physics_accumulator -= PHYSICS_TIMESTEP;
                    }
                }
            }

            GameState::OnlineMultiplayerMenu => {
                let result = online_multiplayer_menu.update();
                match result {
                    OnlineMultiplayerMenuResult::Host => {
                        log::info!("Host selected - showing host configuration menu");
                        game_state = GameState::OnlineHostMenu;
                    }
                    OnlineMultiplayerMenuResult::Join => {
                        log::info!("Join selected - showing join menu");
                        game_state = GameState::OnlineJoinMenu;
                    }
                    OnlineMultiplayerMenuResult::Back => {
                        log::info!("Returning to multiplayer menu from online menu");
                        game_state = GameState::MultiplayerMenu;
                    }
                    OnlineMultiplayerMenuResult::None => {}
                }
            }

            GameState::OnlineHostMenu => {
                let result = online_host_menu.update();
                match result {
                    OnlineHostMenuResult::StartHost(player_name, port) => {
                        log::info!("Host '{}' proceeding to multiplayer saves menu on port {}", player_name, port);
                        host_player_name = Some(player_name);
                        host_port = Some(port);
                        game_state = GameState::MultiplayerSavesMenu;
                        multiplayer_saves_menu.refresh_saves();
                    }
                    OnlineHostMenuResult::Back => {
                        log::info!("Returning to online multiplayer menu from host menu");
                        game_state = GameState::OnlineMultiplayerMenu;
                    }
                    OnlineHostMenuResult::None => {}
                }
            }

            GameState::MultiplayerSavesMenu => {
                let result = multiplayer_saves_menu.update();
                match result {
                    MultiplayerSavesMenuResult::NewGame(port) => {
                        let player_name = host_player_name.as_deref().unwrap_or("Host");
                        log::info!("Starting new multiplayer game '{}' on port {}", player_name, port);
                        match MultiplayerHost::new(window_size, player_name.to_string(), port) {
                            Ok(mut host) => {
                                host.initialize_new_game();
                                multiplayer_host = Some(host);
                                game_state = GameState::MultiplayerHost;
                            }
                            Err(e) => {
                                log::error!("Failed to start host: {}", e);
                                // Stay in menu
                            }
                        }
                    }
                    MultiplayerSavesMenuResult::LoadGame(save_name, port) => {
                        let player_name = host_player_name.as_deref().unwrap_or("Host");
                        log::info!("Loading multiplayer game '{}': {} on port {}", player_name, save_name, port);
                        match GameSaveData::load_from_multi_file(&save_name) {
                            Ok(save_data) => {
                                match MultiplayerHost::new(window_size, player_name.to_string(), port) {
                                    Ok(mut host) => {
                                        host.load_from_save(save_data, save_name);
                                        multiplayer_host = Some(host);
                                        game_state = GameState::MultiplayerHost;
                                    }
                                    Err(e) => {
                                        log::error!("Failed to start host: {}", e);
                                        // Stay in menu
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to load multiplayer save: {}", e);
                                // Stay in menu
                            }
                        }
                    }
                    MultiplayerSavesMenuResult::Back => {
                        log::info!("Returning to online multiplayer menu from saves");
                        game_state = GameState::OnlineMultiplayerMenu;
                    }
                    MultiplayerSavesMenuResult::None => {}
                }
            }

            GameState::OnlineJoinMenu => {
                let result = online_join_menu.update();
                match result {
                    OnlineJoinMenuResult::Connect(player_name, ip, port) => {
                        log::info!("'{}' connecting to {}:{}", player_name, ip, port);
                        match MultiplayerClient::new(window_size, player_name, &ip, port) {
                            Ok(client) => {
                                multiplayer_client = Some(client);
                                game_state = GameState::MultiplayerClient;
                            }
                            Err(e) => {
                                log::error!("Failed to connect: {}", e);
                                // Stay in menu
                            }
                        }
                    }
                    OnlineJoinMenuResult::Back => {
                        log::info!("Returning to online multiplayer menu from join menu");
                        game_state = GameState::OnlineMultiplayerMenu;
                    }
                    OnlineJoinMenuResult::None => {}
                }
            }

            GameState::MultiplayerHost => {
                let mut should_drop_host = false;
                if let Some(ref mut host) = multiplayer_host {
                    match host.handle_input() {
                        MultiplayerHostResult::ReturnToMenu => {
                            log::info!("Returning to multiplayer menu from host");
                            should_drop_host = true;
                            game_state = GameState::MultiplayerMenu;
                        }
                        MultiplayerHostResult::Quit => {
                            log::info!("Quit requested from host");
                            break;
                        }
                        _ => {}
                    }

                    if !should_drop_host {
                        // Fixed timestep physics update
                        while physics_accumulator >= PHYSICS_TIMESTEP {
                            host.update(PHYSICS_TIMESTEP);
                            physics_accumulator -= PHYSICS_TIMESTEP;
                        }
                    }
                }
                // Drop the host to close the UDP socket and free the port
                if should_drop_host {
                    multiplayer_host = None;
                }
            }

            GameState::MultiplayerClient => {
                let mut should_drop_client = false;
                if let Some(ref mut client) = multiplayer_client {
                    match client.handle_input() {
                        MultiplayerClientResult::ReturnToMenu => {
                            log::info!("Returning to multiplayer menu from client");
                            should_drop_client = true;
                            game_state = GameState::MultiplayerMenu;
                        }
                        MultiplayerClientResult::Quit => {
                            log::info!("Quit requested from client");
                            break;
                        }
                        MultiplayerClientResult::ConnectionLost => {
                            log::warn!("Connection to host lost");
                            should_drop_client = true;
                            game_state = GameState::MultiplayerMenu;
                        }
                        _ => {}
                    }

                    if !should_drop_client {
                        // Fixed timestep physics update
                        while physics_accumulator >= PHYSICS_TIMESTEP {
                            client.update(PHYSICS_TIMESTEP);
                            physics_accumulator -= PHYSICS_TIMESTEP;
                        }
                    }
                }
                // Drop the client to close the UDP socket
                if should_drop_client {
                    multiplayer_client = None;
                }
            }
        }

        // Render based on game state
        clear_background(BLACK);

        match game_state {
            GameState::MainMenu => {
                main_menu.draw();
            }

            GameState::SavesMenu => {
                saves_menu.draw();
            }

            GameState::MapSelection => {
                map_selection_menu.render();
            }

            GameState::MultiplayerMenu => {
                multiplayer_menu.draw();
            }

            GameState::Playing | GameState::Paused => {
                if let Some(ref mut game) = single_player_game {
                    game.render();
                }
            }

            GameState::SplitScreen => {
                if let Some(ref mut game) = split_screen_game {
                    game.render();
                }
            }

            GameState::Quit => {}

            GameState::OnlineMultiplayerMenu => {
                online_multiplayer_menu.draw();
            }

            GameState::OnlineHostMenu => {
                online_host_menu.draw();
            }

            GameState::MultiplayerSavesMenu => {
                multiplayer_saves_menu.draw();
            }

            GameState::OnlineJoinMenu => {
                online_join_menu.draw();
            }

            GameState::MultiplayerHost => {
                if let Some(ref mut host) = multiplayer_host {
                    host.render();
                }
            }

            GameState::MultiplayerClient => {
                if let Some(ref mut client) = multiplayer_client {
                    client.render();
                }
            }
        }

        // Log FPS every second
        if fps_timer >= 1.0 {
            let fps = get_fps();
            log::debug!("FPS: {} | State: {:?}", fps, game_state);
            fps_timer = 0.0;
        }

        // Wait for next frame
        next_frame().await;
    }

    log::info!("Game exited cleanly");
    log::info!("Total frames: {}", frame_count);
    log::info!("Total playtime: {:.1} seconds", frame_count as f32 / 60.0);
}
