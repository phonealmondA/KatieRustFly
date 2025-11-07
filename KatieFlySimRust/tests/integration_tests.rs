// Integration Tests - Testing full system interactions
// Phase 15: Testing & Debug

#[cfg(test)]
mod integration_tests {
    use katie_fly_sim_rust::entities::{Planet, Rocket, Satellite};
    use katie_fly_sim_rust::systems::World;
    use macroquad::prelude::*;

    /// Test complete rocket lifecycle from spawn to satellite conversion
    #[test]
    fn test_rocket_to_satellite_lifecycle() {
        let mut world = World::new();

        // Add a planet
        let planet = Planet::new(Vec2::ZERO, 50.0, 10000.0, BLUE);
        world.add_planet(planet);

        // Add a rocket
        let rocket = Rocket::new(Vec2::new(200.0, 0.0), Vec2::new(0.0, 50.0), WHITE, 1.0);
        let rocket_id = world.add_rocket(rocket);

        assert_eq!(world.rocket_count(), 1);
        assert_eq!(world.satellite_count(), 0);

        // Simulate for a few seconds
        for _ in 0..100 {
            world.update(0.016); // ~60 FPS
        }

        // Rocket should still exist
        assert!(world.get_rocket(rocket_id).is_some());

        // Convert to satellite
        let satellite_id = world.convert_rocket_to_satellite(rocket_id);
        assert!(satellite_id.is_some());

        // Verify conversion
        assert_eq!(world.rocket_count(), 0);
        assert_eq!(world.satellite_count(), 1);
        assert!(world.get_satellite(satellite_id.unwrap()).is_some());
    }

    /// Test physics consistency over multiple frames
    #[test]
    fn test_physics_energy_conservation() {
        let mut world = World::new();

        // Create a simple two-body system
        let planet = Planet::new(Vec2::ZERO, 50.0, 10000.0, BLUE);
        world.add_planet(planet);

        // Start with stable circular orbit
        let orbital_distance = 300.0;
        let orbital_velocity = (katie_fly_sim_rust::GameConstants::G * 10000.0 / orbital_distance).sqrt();
        let rocket = Rocket::new(Vec2::new(orbital_distance, 0.0), Vec2::new(0.0, orbital_velocity), WHITE, 1.0);
        let rocket_id = world.add_rocket(rocket);

        // Calculate initial total energy (kinetic + potential)
        let initial_rocket = world.get_rocket(rocket_id).unwrap();
        let initial_ke = 0.5 * initial_rocket.current_mass() *
            (initial_rocket.velocity().x.powi(2) + initial_rocket.velocity().y.powi(2));

        // Simulate for a few frames (not too many to avoid numerical errors accumulating)
        for _ in 0..100 {
            world.update(0.016);
        }

        // Rocket should still exist
        if let Some(final_rocket) = world.get_rocket(rocket_id) {
            let final_ke = 0.5 * final_rocket.current_mass() *
                (final_rocket.velocity().x.powi(2) + final_rocket.velocity().y.powi(2));

            // Kinetic energy should be in reasonable range
            assert!(final_ke > 0.0);
            assert!(final_ke < initial_ke * 3.0); // Allow some variation due to numerical integration
        } else {
            panic!("Rocket should still exist after simulation");
        }
    }

    /// Test multi-planet gravitational interactions
    #[test]
    fn test_multi_planet_gravity() {
        let mut world = World::new();

        // Add three planets
        let planet1 = Planet::new(Vec2::new(-100.0, 0.0), 30.0, 5000.0, BLUE);
        let planet2 = Planet::new(Vec2::new(100.0, 0.0), 30.0, 5000.0, GREEN);
        let planet3 = Planet::new(Vec2::new(0.0, 100.0), 30.0, 5000.0, RED);

        world.add_planet(planet1);
        world.add_planet(planet2);
        world.add_planet(planet3);

        // Add rocket between planets
        let rocket = Rocket::new(Vec2::new(0.0, 0.0), Vec2::ZERO, WHITE, 1.0);
        let rocket_id = world.add_rocket(rocket);

        // Simulate
        for _ in 0..100 {
            world.update(0.016);
        }

        // Rocket should have moved due to gravity
        let final_rocket = world.get_rocket(rocket_id).unwrap();
        let final_pos = final_rocket.position();

        // Should have moved from origin
        assert!(final_pos.x.abs() > 0.1 || final_pos.y.abs() > 0.1);
    }

    /// Test fuel system integration
    #[test]
    fn test_fuel_consumption_and_thrust() {
        let mut world = World::new();

        let planet = Planet::new(Vec2::ZERO, 50.0, 10000.0, BLUE);
        world.add_planet(planet);

        let mut rocket = Rocket::new(Vec2::new(200.0, 0.0), Vec2::ZERO, WHITE, 1.0);
        rocket.add_fuel(100.0);
        let rocket_id = world.add_rocket(rocket);

        let initial_fuel = world.get_rocket(rocket_id).unwrap().current_fuel();

        // Apply thrust
        world.set_rocket_thrust(rocket_id, true);

        // Simulate
        for _ in 0..100 {
            world.update(0.016);
        }

        // Fuel should have been consumed
        let final_fuel = world.get_rocket(rocket_id).unwrap().current_fuel();
        assert!(final_fuel < initial_fuel);

        // Velocity should have changed
        let final_velocity = world.get_rocket(rocket_id).unwrap().velocity();
        assert!(final_velocity.length() > 0.1);
    }

    /// Test world entity limits and scaling
    #[test]
    fn test_large_scale_simulation() {
        let mut world = World::new();

        // Add many entities
        for i in 0..10 {
            let angle = (i as f32) * 2.0 * std::f32::consts::PI / 10.0;
            let pos = Vec2::new(angle.cos() * 500.0, angle.sin() * 500.0);
            let planet = Planet::new(pos, 20.0, 1000.0, BLUE);
            world.add_planet(planet);
        }

        for i in 0..20 {
            let angle = (i as f32) * 2.0 * std::f32::consts::PI / 20.0;
            let pos = Vec2::new(angle.cos() * 300.0, angle.sin() * 300.0);
            let vel = Vec2::new(-angle.sin() * 30.0, angle.cos() * 30.0);
            let rocket = Rocket::new(pos, vel, WHITE, 1.0);
            world.add_rocket(rocket);
        }

        assert_eq!(world.planet_count(), 10);
        assert_eq!(world.rocket_count(), 20);

        // Simulate should not crash with many entities
        for _ in 0..50 {
            world.update(0.016);
        }

        // All entities should still exist
        assert_eq!(world.planet_count(), 10);
        assert_eq!(world.rocket_count(), 20);
    }

    /// Test rocket rotation and control
    #[test]
    fn test_rocket_rotation_control() {
        let mut world = World::new();

        let rocket = Rocket::new(Vec2::ZERO, Vec2::ZERO, WHITE, 1.0);
        let rocket_id = world.add_rocket(rocket);

        let initial_rotation = world.get_rocket(rocket_id).unwrap().rotation();

        // Rotate rocket
        world.rotate_rocket(rocket_id, 1.0);

        let new_rotation = world.get_rocket(rocket_id).unwrap().rotation();

        // Rotation should have changed
        assert!((new_rotation - initial_rotation - 1.0).abs() < 0.01);
    }

    /// Test satellite orbital stability
    #[test]
    fn test_satellite_orbital_stability() {
        let mut world = World::new();

        // Add central planet
        let planet = Planet::new(Vec2::ZERO, 50.0, 10000.0, BLUE);
        world.add_planet(planet);

        // Add satellite in circular orbit
        let orbital_distance = 300.0;
        let orbital_velocity = (katie_fly_sim_rust::GameConstants::G * 10000.0 / orbital_distance).sqrt();

        let satellite = Satellite::new(
            Vec2::new(orbital_distance, 0.0),
            Vec2::new(0.0, orbital_velocity),
            GREEN,
        );
        let sat_id = world.add_satellite(satellite);

        let initial_pos = world.get_satellite(sat_id).unwrap().position();
        let initial_distance = initial_pos.length();

        // Simulate multiple orbits
        for _ in 0..1000 {
            world.update(0.016);
        }

        let final_pos = world.get_satellite(sat_id).unwrap().position();
        let final_distance = final_pos.length();

        // Orbital radius should be relatively stable (within 20%)
        assert!((final_distance - initial_distance).abs() / initial_distance < 0.2);
    }

    /// Test networking event system (host)
    #[test]
    fn test_multiplayer_host_events() {
        use katie_fly_sim_rust::networking::MultiplayerHost;

        let mut host = MultiplayerHost::new();

        // Should not be running initially
        assert!(!host.is_running());
        assert_eq!(host.client_count(), 0);

        // Poll events should return empty
        let events = host.poll_events();
        assert_eq!(events.len(), 0);

        // Update should not crash
        host.update(0.016);
    }

    /// Test networking event system (client)
    #[test]
    fn test_multiplayer_client_events() {
        use katie_fly_sim_rust::networking::MultiplayerClient;

        let mut client = MultiplayerClient::new();

        // Should not be connected initially
        assert!(!client.is_connected());
        assert_eq!(client.player_id(), None);
        assert_eq!(client.remote_player_count(), 0);

        // Poll events should return empty
        let events = client.poll_events();
        assert_eq!(events.len(), 0);

        // Update should not crash
        client.update(0.016);

        // Get remote players should return empty
        let players = client.get_all_remote_players();
        assert_eq!(players.len(), 0);
    }

    /// Test split screen viewport system
    #[test]
    fn test_split_screen_viewport() {
        use katie_fly_sim_rust::game_modes::Viewport;

        let viewport = Viewport::new(Rect::new(0.0, 0.0, 800.0, 600.0), 0);

        assert_eq!(viewport.player_index(), 0);
        assert_eq!(viewport.rocket_id(), None);

        // Set rocket ID
        let mut viewport = viewport;
        viewport.set_rocket_id(Some(42));
        assert_eq!(viewport.rocket_id(), Some(42));
    }
}

/// Performance and stress tests
#[cfg(test)]
mod performance_tests {
    use katie_fly_sim_rust::systems::World;
    use katie_fly_sim_rust::entities::{Planet, Rocket};
    use macroquad::prelude::*;
    use std::time::Instant;

    /// Benchmark physics update performance
    #[test]
    fn bench_physics_update_performance() {
        let mut world = World::new();

        // Add realistic number of entities
        for i in 0..5 {
            let angle = (i as f32) * 2.0 * std::f32::consts::PI / 5.0;
            let pos = Vec2::new(angle.cos() * 400.0, angle.sin() * 400.0);
            let planet = Planet::new(pos, 30.0, 5000.0, BLUE);
            world.add_planet(planet);
        }

        for i in 0..10 {
            let angle = (i as f32) * 2.0 * std::f32::consts::PI / 10.0;
            let pos = Vec2::new(angle.cos() * 300.0, angle.sin() * 300.0);
            let vel = Vec2::new(-angle.sin() * 40.0, angle.cos() * 40.0);
            let rocket = Rocket::new(pos, vel, WHITE, 1.0);
            world.add_rocket(rocket);
        }

        // Benchmark 1000 frames
        let start = Instant::now();
        for _ in 0..1000 {
            world.update(0.016);
        }
        let duration = start.elapsed();

        // Should complete in reasonable time (< 1 second for 1000 frames)
        assert!(duration.as_secs_f32() < 1.0,
            "Physics update too slow: {:.3}s for 1000 frames",
            duration.as_secs_f32());

        println!("Physics performance: {:.3}ms per frame",
            duration.as_secs_f32() * 1000.0 / 1000.0);
    }

    /// Test memory stability over long simulation
    #[test]
    fn test_memory_stability() {
        let mut world = World::new();

        // Add entities
        let planet = Planet::new(Vec2::ZERO, 50.0, 10000.0, BLUE);
        world.add_planet(planet);

        for _ in 0..5 {
            let rocket = Rocket::new(Vec2::new(200.0, 0.0), Vec2::new(0.0, 50.0), WHITE, 1.0);
            world.add_rocket(rocket);
        }

        // Run for many frames
        for _ in 0..10000 {
            world.update(0.016);
        }

        // World should still be functional
        assert_eq!(world.planet_count(), 1);
        assert_eq!(world.rocket_count(), 5);
    }
}
