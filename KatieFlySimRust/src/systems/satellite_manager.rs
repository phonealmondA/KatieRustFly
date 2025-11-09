// Satellite Manager - Centralized satellite lifecycle and network management
// Coordinates satellite operations, fuel transfers, orbital maintenance, and visualization

use std::collections::HashMap;
use macroquad::prelude::*;

use crate::entities::{Satellite, Rocket, Planet, GameObject};
use crate::systems::{EntityId, FuelTransferNetwork, OrbitMaintenance};
use crate::systems::fuel_transfer_network::{TransferPriority, NetworkOptimizationMode};
use crate::physics::GravitySimulator;
use crate::game_constants::GameConstants;
use crate::utils::vector_helper;

/// Satellite operational status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SatelliteStatus {
    Active,          // Fully operational
    LowFuel,         // Fuel below 30%
    Critical,        // Fuel below 10%
    Depleted,        // Out of fuel
    Maintenance,     // Performing orbital corrections
    Transferring,    // Currently transferring fuel
}

/// Network-wide statistics
#[derive(Debug, Clone, Default)]
pub struct SatelliteNetworkStats {
    pub total_satellites: usize,
    pub operational_satellites: usize,
    pub total_network_fuel: f32,
    pub average_fuel_percentage: f32,
    pub average_orbital_accuracy: f32,
    pub active_fuel_transfers: usize,
    pub total_delta_v_expended: f32,
    pub satellites_by_status: HashMap<String, usize>,
}

/// Configuration for satellite system
#[derive(Debug, Clone, Copy)]
pub struct SatelliteManagerConfig {
    pub global_maintenance_interval: f32,
    pub global_orbit_tolerance: f32,
    pub enable_automatic_maintenance: bool,
    pub enable_automatic_collection: bool,
    pub collection_efficiency: f32,
    pub max_transfer_range: f32,
    pub emergency_fuel_threshold: f32,
    pub critical_fuel_threshold: f32,
}

impl Default for SatelliteManagerConfig {
    fn default() -> Self {
        SatelliteManagerConfig {
            global_maintenance_interval: 5.0,
            global_orbit_tolerance: 50.0,
            enable_automatic_maintenance: true,
            enable_automatic_collection: true,
            collection_efficiency: 1.0,
            max_transfer_range: 500.0,
            emergency_fuel_threshold: 0.30,
            critical_fuel_threshold: 0.10,
        }
    }
}

/// Satellite Manager - Centralized satellite management system
pub struct SatelliteManager {
    // Satellite storage
    satellites: HashMap<EntityId, Satellite>,
    next_satellite_id: EntityId,

    // Naming system
    satellite_name_counter: usize,

    // External references
    planets: Vec<EntityId>,
    nearby_rockets: Vec<EntityId>,

    // Systems
    fuel_transfer_network: FuelTransferNetwork,
    orbit_maintenance_systems: HashMap<EntityId, OrbitMaintenance>,

    // Configuration
    config: SatelliteManagerConfig,

    // Statistics
    network_stats: SatelliteNetworkStats,
    stats_update_interval: f32,
    time_since_stats_update: f32,

    // Visualization flags
    pub show_orbit_paths: bool,
    pub show_target_orbit_paths: bool,
    pub show_fuel_transfer_lines: bool,
    pub show_maintenance_burns: bool,
    pub show_status_indicators: bool,
}

impl SatelliteManager {
    pub fn new() -> Self {
        SatelliteManager {
            satellites: HashMap::new(),
            next_satellite_id: 1000, // Start satellite IDs at 1000
            satellite_name_counter: 1,
            planets: Vec::new(),
            nearby_rockets: Vec::new(),
            fuel_transfer_network: FuelTransferNetwork::new(),
            orbit_maintenance_systems: HashMap::new(),
            config: SatelliteManagerConfig::default(),
            network_stats: SatelliteNetworkStats::default(),
            stats_update_interval: 1.0,
            time_since_stats_update: 0.0,
            show_orbit_paths: true,
            show_target_orbit_paths: false,
            show_fuel_transfer_lines: true,
            show_maintenance_burns: true,
            show_status_indicators: true,
        }
    }

    pub fn with_config(config: SatelliteManagerConfig) -> Self {
        let mut manager = Self::new();
        manager.config = config;
        manager.fuel_transfer_network.set_max_transfer_range(config.max_transfer_range);
        manager
    }

    // === Satellite Lifecycle ===

    /// Create satellite from rocket conversion
    pub fn create_satellite_from_rocket(
        &mut self,
        rocket_position: Vec2,
        rocket_velocity: Vec2,
        rocket_fuel: f32,
        player_id: Option<usize>,
    ) -> EntityId {
        let satellite_id = self.next_satellite_id;
        self.next_satellite_id += 1;

        let _name = self.generate_satellite_name(player_id);
        let satellite = Satellite::from_rocket(rocket_position, rocket_velocity, rocket_fuel);

        self.satellites.insert(satellite_id, satellite);

        // Create orbit maintenance system for this satellite
        let orbit_maintenance = OrbitMaintenance::with_target_radius(
            vector_helper::magnitude(rocket_position)
        );
        self.orbit_maintenance_systems.insert(satellite_id, orbit_maintenance);

        println!("Created satellite '{}' (ID: {}) at position {:?}", _name, satellite_id, rocket_position);

        satellite_id
    }

    /// Create satellite with custom parameters
    pub fn create_satellite(
        &mut self,
        position: Vec2,
        velocity: Vec2,
        fuel: f32,
        target_orbit_radius: f32,
        player_id: Option<usize>,
    ) -> EntityId {
        let satellite_id = self.next_satellite_id;
        self.next_satellite_id += 1;

        let _name = self.generate_satellite_name(player_id);
        let mut satellite = Satellite::new(
            position,
            velocity,
            crate::game_constants::colors::SATELLITE_BODY_COLOR,
        );
        satellite.add_fuel(fuel);
        satellite.set_target_orbit_radius(target_orbit_radius);

        self.satellites.insert(satellite_id, satellite);

        // Create orbit maintenance system
        let orbit_maintenance = OrbitMaintenance::with_target_radius(target_orbit_radius);
        self.orbit_maintenance_systems.insert(satellite_id, orbit_maintenance);

        satellite_id
    }

    /// Generate standardized satellite name
    fn generate_satellite_name(&mut self, player_id: Option<usize>) -> String {
        let name = if let Some(pid) = player_id {
            format!("P{}-SAT-{}", pid, self.satellite_name_counter)
        } else {
            format!("SAT-{:03}", self.satellite_name_counter)
        };
        self.satellite_name_counter += 1;
        name
    }

    /// Remove satellite by ID
    pub fn remove_satellite(&mut self, satellite_id: EntityId) -> bool {
        if self.satellites.remove(&satellite_id).is_some() {
            self.orbit_maintenance_systems.remove(&satellite_id);
            println!("Removed satellite ID: {}", satellite_id);
            true
        } else {
            false
        }
    }

    /// Remove all satellites
    pub fn remove_all_satellites(&mut self) {
        let count = self.satellites.len();
        self.satellites.clear();
        self.orbit_maintenance_systems.clear();
        self.satellite_name_counter = 1;
        println!("Removed all {} satellites", count);
    }

    // === Queries ===

    /// Get satellite by ID
    pub fn get_satellite(&self, satellite_id: EntityId) -> Option<&Satellite> {
        self.satellites.get(&satellite_id)
    }

    /// Get mutable satellite by ID
    pub fn get_satellite_mut(&mut self, satellite_id: EntityId) -> Option<&mut Satellite> {
        self.satellites.get_mut(&satellite_id)
    }

    /// Get all satellites
    pub fn get_all_satellites(&self) -> Vec<(EntityId, &Satellite)> {
        self.satellites.iter().map(|(id, sat)| (*id, sat)).collect()
    }

    /// Get satellites in range of a position
    pub fn get_satellites_in_range(&self, position: Vec2, range: f32) -> Vec<(EntityId, &Satellite)> {
        self.satellites
            .iter()
            .filter(|(_, sat)| {
                vector_helper::distance(sat.position(), position) <= range
            })
            .map(|(id, sat)| (*id, sat))
            .collect()
    }

    /// Get operational satellites (have fuel and maintaining orbit)
    pub fn get_operational_satellites(&self) -> Vec<(EntityId, &Satellite)> {
        self.satellites
            .iter()
            .filter(|(_, sat)| {
                sat.fuel_percentage() > self.config.critical_fuel_threshold * 100.0
            })
            .map(|(id, sat)| (*id, sat))
            .collect()
    }

    /// Get satellite count
    pub fn satellite_count(&self) -> usize {
        self.satellites.len()
    }

    /// Get satellite status
    pub fn get_satellite_status(&self, satellite_id: EntityId) -> Option<SatelliteStatus> {
        self.satellites.get(&satellite_id).map(|sat| {
            let fuel_percent = sat.fuel_percentage() / 100.0;

            if fuel_percent == 0.0 {
                SatelliteStatus::Depleted
            } else if fuel_percent < self.config.critical_fuel_threshold {
                SatelliteStatus::Critical
            } else if fuel_percent < self.config.emergency_fuel_threshold {
                SatelliteStatus::LowFuel
            } else if sat.is_maintaining_orbit() {
                SatelliteStatus::Maintenance
            } else {
                SatelliteStatus::Active
            }
        })
    }

    // === Configuration ===

    pub fn set_maintenance_interval(&mut self, interval: f32) {
        self.config.global_maintenance_interval = interval;
    }

    pub fn set_orbit_tolerance(&mut self, tolerance: f32) {
        self.config.global_orbit_tolerance = tolerance;
    }

    pub fn set_automatic_maintenance(&mut self, enabled: bool) {
        self.config.enable_automatic_maintenance = enabled;
    }

    pub fn set_automatic_collection(&mut self, enabled: bool) {
        self.config.enable_automatic_collection = enabled;
    }

    pub fn set_network_optimization_mode(&mut self, mode: NetworkOptimizationMode) {
        self.fuel_transfer_network.set_optimization_mode(mode);
    }

    pub fn config(&self) -> &SatelliteManagerConfig {
        &self.config
    }

    // === Integration ===

    /// Integrate with gravity simulator
    pub fn integrate_with_gravity_simulator(&mut self, _gravity_simulator: &mut GravitySimulator) {
        // Gravity simulator will handle satellite physics in its update
        // This is called to ensure satellites are registered
    }

    /// Set nearby planets for fuel collection
    pub fn set_nearby_planets(&mut self, planet_ids: Vec<EntityId>) {
        self.planets = planet_ids;
    }

    /// Add nearby rocket for fuel transfer opportunities
    pub fn add_nearby_rocket(&mut self, rocket_id: EntityId) {
        if !self.nearby_rockets.contains(&rocket_id) {
            self.nearby_rockets.push(rocket_id);
        }
    }

    /// Update rocket proximity
    pub fn update_rocket_proximity(&mut self, rocket_positions: &HashMap<EntityId, Vec2>) {
        self.nearby_rockets.clear();

        for (rocket_id, rocket_pos) in rocket_positions {
            for (_sat_id, satellite) in &self.satellites {
                let distance = vector_helper::distance(satellite.position(), *rocket_pos);
                if distance <= self.config.max_transfer_range {
                    self.add_nearby_rocket(*rocket_id);
                    break;
                }
            }
        }
    }

    // === Fuel Operations ===

    /// Transfer fuel between satellites
    pub fn transfer_fuel_between_satellites(
        &mut self,
        source_id: EntityId,
        destination_id: EntityId,
        amount: f32,
        priority: TransferPriority,
    ) -> bool {
        // Request transfer through network
        let request_id = self.fuel_transfer_network.request_transfer(
            source_id,
            destination_id,
            amount,
            priority,
        );

        request_id != usize::MAX
    }

    /// Request fuel from network for a satellite
    pub fn request_fuel_from_network(&mut self, satellite_id: EntityId, amount: f32) {
        // Find satellite with most fuel
        let mut best_source: Option<EntityId> = None;
        let mut max_fuel = 0.0;

        for (id, sat) in &self.satellites {
            if *id != satellite_id && sat.current_fuel() > max_fuel {
                max_fuel = sat.current_fuel();
                best_source = Some(*id);
            }
        }

        if let Some(source_id) = best_source {
            let transfer_amount = amount.min(max_fuel * 0.5); // Don't drain source
            self.transfer_fuel_between_satellites(
                source_id,
                satellite_id,
                transfer_amount,
                TransferPriority::High,
            );
        }
    }

    /// Optimize network fuel distribution
    pub fn optimize_network_fuel_distribution(&mut self) {
        let fuel_levels: HashMap<EntityId, (f32, f32)> = self.satellites
            .iter()
            .map(|(id, sat)| (*id, (sat.current_fuel(), sat.max_fuel())))
            .collect();

        self.fuel_transfer_network.optimize_network(&fuel_levels);
    }

    /// Balance fuel across network
    pub fn balance_fuel_across_network(&mut self) {
        self.fuel_transfer_network.set_optimization_mode(NetworkOptimizationMode::Balanced);
        self.optimize_network_fuel_distribution();
    }

    /// Handle low fuel emergency for a satellite
    pub fn handle_low_fuel_emergency(&mut self, satellite_id: EntityId) {
        println!("Emergency: Satellite {} has critical fuel!", satellite_id);
        self.request_fuel_from_network(satellite_id, 100.0);
    }

    /// Shutdown non-essential satellites to conserve fuel
    pub fn shutdown_non_essential_satellites(&mut self, essential_satellite_ids: &[EntityId]) {
        // Collect transfer operations first to avoid borrow checker issues
        let mut transfers: Vec<(EntityId, EntityId, f32)> = Vec::new();

        // Find satellites to transfer from
        for (sat_id, satellite) in &self.satellites {
            if !essential_satellite_ids.contains(sat_id) && satellite.current_fuel() > 50.0 {
                // Find nearest essential satellite
                let mut nearest_essential: Option<EntityId> = None;
                let mut min_distance = f32::MAX;

                let sat_position = satellite.position();

                for essential_id in essential_satellite_ids {
                    if let Some(essential_sat) = self.satellites.get(essential_id) {
                        let distance = vector_helper::distance(sat_position, essential_sat.position());
                        if distance < min_distance {
                            min_distance = distance;
                            nearest_essential = Some(*essential_id);
                        }
                    }
                }

                if let Some(target_id) = nearest_essential {
                    let fuel_to_transfer = satellite.current_fuel() * 0.8;
                    transfers.push((*sat_id, target_id, fuel_to_transfer));
                }
            }
        }

        // Execute transfers
        for (source_id, target_id, amount) in transfers {
            self.transfer_fuel_between_satellites(
                source_id,
                target_id,
                amount,
                TransferPriority::Critical,
            );
        }
    }

    // === Conversion Validation ===

    /// Check if rocket can be converted to satellite
    pub fn can_convert_rocket_to_satellite(
        &self,
        rocket: &Rocket,
        min_altitude: f32,
        min_fuel: f32,
    ) -> bool {
        rocket.current_fuel() >= min_fuel
            && vector_helper::magnitude(rocket.position()) >= min_altitude
    }

    /// Get optimal conversion configuration
    pub fn get_optimal_conversion_config(
        &self,
        rocket_position: Vec2,
        rocket_velocity: Vec2,
    ) -> (f32, f32) {
        let orbital_radius = vector_helper::magnitude(rocket_position);
        let velocity_magnitude = vector_helper::magnitude(rocket_velocity);

        (orbital_radius, velocity_magnitude)
    }

    // === Update ===

    /// Update all satellites and systems
    pub fn update(
        &mut self,
        delta_time: f32,
        planets: &HashMap<EntityId, &Planet>,
    ) {
        // Update satellite positions from network
        let satellite_positions: HashMap<EntityId, Vec2> = self.satellites
            .iter()
            .map(|(id, sat)| (*id, sat.position()))
            .collect();

        self.fuel_transfer_network.update_connections(&satellite_positions);
        self.fuel_transfer_network.update(delta_time);

        // Update each satellite
        let mut satellites_to_update: Vec<EntityId> = self.satellites.keys().copied().collect();

        for satellite_id in satellites_to_update {
            // Perform orbital maintenance if enabled
            if self.config.enable_automatic_maintenance {
                if let (Some(satellite), Some(orbit_system)) = (
                    self.satellites.get_mut(&satellite_id),
                    self.orbit_maintenance_systems.get_mut(&satellite_id),
                ) {
                    // Find primary planet (closest massive body)
                    let mut primary_planet: Option<&Planet> = None;
                    let mut min_distance = f32::MAX;

                    for planet in planets.values() {
                        let distance = vector_helper::distance(satellite.position(), planet.position());
                        if distance < min_distance {
                            min_distance = distance;
                            primary_planet = Some(planet);
                        }
                    }

                    if let Some(planet) = primary_planet {
                        orbit_system.update(satellite, planet, delta_time);
                    }
                }
            }

            // Check for low fuel emergencies
            if let Some(satellite) = self.satellites.get(&satellite_id) {
                let fuel_percent = satellite.fuel_percentage() / 100.0;

                if fuel_percent < self.config.critical_fuel_threshold && fuel_percent > 0.0 {
                    self.handle_low_fuel_emergency(satellite_id);
                }
            }

            // Update satellite itself
            if let Some(satellite) = self.satellites.get_mut(&satellite_id) {
                satellite.update(delta_time);
            }
        }

        // Update network optimization
        if self.config.enable_automatic_collection {
            self.optimize_network_fuel_distribution();
        }

        // Update statistics
        self.time_since_stats_update += delta_time;
        if self.time_since_stats_update >= self.stats_update_interval {
            self.update_statistics();
            self.time_since_stats_update = 0.0;
        }
    }

    /// Update network statistics
    fn update_statistics(&mut self) {
        self.network_stats.total_satellites = self.satellites.len();
        self.network_stats.operational_satellites = self.get_operational_satellites().len();

        let mut total_fuel = 0.0;
        let mut total_max_fuel = 0.0;
        let mut total_delta_v = 0.0;
        let mut status_counts: HashMap<String, usize> = HashMap::new();

        for (sat_id, satellite) in &self.satellites {
            total_fuel += satellite.current_fuel();
            total_max_fuel += satellite.max_fuel();

            if let Some(orbit_system) = self.orbit_maintenance_systems.get(sat_id) {
                total_delta_v += orbit_system.total_delta_v_expended();
            }

            if let Some(status) = self.get_satellite_status(*sat_id) {
                let status_str = format!("{:?}", status);
                *status_counts.entry(status_str).or_insert(0) += 1;
            }
        }

        self.network_stats.total_network_fuel = total_fuel;
        self.network_stats.average_fuel_percentage = if total_max_fuel > 0.0 {
            (total_fuel / total_max_fuel) * 100.0
        } else {
            0.0
        };
        self.network_stats.total_delta_v_expended = total_delta_v;
        self.network_stats.active_fuel_transfers = self.fuel_transfer_network.active_transfer_count();
        self.network_stats.satellites_by_status = status_counts;
    }

    // === Visualization ===

    /// Draw all satellites with visualization options
    pub fn draw(&self) {
        // Draw fuel transfer network connections
        if self.show_fuel_transfer_lines {
            let satellite_positions: HashMap<EntityId, Vec2> = self.satellites
                .iter()
                .map(|(id, sat)| (*id, sat.position()))
                .collect();
            self.fuel_transfer_network.draw_network(&satellite_positions);
        }

        // Draw each satellite
        for (sat_id, satellite) in &self.satellites {
            satellite.draw();

            // Draw status indicator
            if self.show_status_indicators {
                if let Some(status) = self.get_satellite_status(*sat_id) {
                    self.draw_status_indicator(satellite.position(), status);
                }
            }
        }
    }

    /// Draw status indicator for satellite
    fn draw_status_indicator(&self, position: Vec2, status: SatelliteStatus) {
        let color = match status {
            SatelliteStatus::Active => GREEN,
            SatelliteStatus::LowFuel => YELLOW,
            SatelliteStatus::Critical => ORANGE,
            SatelliteStatus::Depleted => RED,
            SatelliteStatus::Maintenance => BLUE,
            SatelliteStatus::Transferring => PURPLE,
        };

        let offset = Vec2::new(0.0, -15.0);
        let indicator_pos = position + offset;
        draw_circle(indicator_pos.x, indicator_pos.y, 3.0, color);
    }

    // === Reporting ===

    /// Print network status to console
    pub fn print_network_status(&self) {
        println!("=== Satellite Network Status ===");
        println!("Total Satellites: {}", self.network_stats.total_satellites);
        println!("Operational: {}", self.network_stats.operational_satellites);
        println!("Total Network Fuel: {:.1}", self.network_stats.total_network_fuel);
        println!("Average Fuel: {:.1}%", self.network_stats.average_fuel_percentage);
        println!("Active Transfers: {}", self.network_stats.active_fuel_transfers);
        println!("Total ΔV Expended: {:.1}", self.network_stats.total_delta_v_expended);
        println!("Status Breakdown:");
        for (status, count) in &self.network_stats.satellites_by_status {
            println!("  {}: {}", status, count);
        }
        println!("================================");
    }

    /// Get network status report (for UI)
    pub fn get_network_status_report(&self) -> String {
        format!(
            "Satellites: {}/{}\nFuel: {:.0}/{:.0} ({:.1}%)\nTransfers: {}\nΔV: {:.1}",
            self.network_stats.operational_satellites,
            self.network_stats.total_satellites,
            self.network_stats.total_network_fuel,
            self.satellites.iter().map(|(_, s)| s.max_fuel()).sum::<f32>(),
            self.network_stats.average_fuel_percentage,
            self.network_stats.active_fuel_transfers,
            self.network_stats.total_delta_v_expended,
        )
    }

    /// Get network statistics
    pub fn network_stats(&self) -> &SatelliteNetworkStats {
        &self.network_stats
    }
}

impl Default for SatelliteManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_satellite_manager_creation() {
        let manager = SatelliteManager::new();
        assert_eq!(manager.satellite_count(), 0);
    }

    #[test]
    fn test_create_satellite_from_rocket() {
        let mut manager = SatelliteManager::new();
        let id = manager.create_satellite_from_rocket(
            Vec2::new(300.0, 0.0),
            Vec2::new(0.0, 50.0),
            100.0,
            Some(1),
        );

        assert_eq!(manager.satellite_count(), 1);
        assert!(manager.get_satellite(id).is_some());
    }

    #[test]
    fn test_satellite_name_generation() {
        let mut manager = SatelliteManager::new();

        manager.create_satellite_from_rocket(Vec2::ZERO, Vec2::ZERO, 50.0, None);
        manager.create_satellite_from_rocket(Vec2::ZERO, Vec2::ZERO, 50.0, Some(1));
        manager.create_satellite_from_rocket(Vec2::ZERO, Vec2::ZERO, 50.0, Some(2));

        assert_eq!(manager.satellite_count(), 3);
    }

    #[test]
    fn test_remove_satellite() {
        let mut manager = SatelliteManager::new();
        let id = manager.create_satellite_from_rocket(Vec2::ZERO, Vec2::ZERO, 50.0, None);

        assert!(manager.remove_satellite(id));
        assert_eq!(manager.satellite_count(), 0);
        assert!(!manager.remove_satellite(id)); // Already removed
    }

    #[test]
    fn test_get_satellites_in_range() {
        let mut manager = SatelliteManager::new();

        manager.create_satellite_from_rocket(Vec2::new(100.0, 0.0), Vec2::ZERO, 50.0, None);
        manager.create_satellite_from_rocket(Vec2::new(1000.0, 0.0), Vec2::ZERO, 50.0, None);

        let nearby = manager.get_satellites_in_range(Vec2::ZERO, 200.0);
        assert_eq!(nearby.len(), 1);

        let all_in_range = manager.get_satellites_in_range(Vec2::ZERO, 2000.0);
        assert_eq!(all_in_range.len(), 2);
    }

    #[test]
    fn test_network_statistics() {
        let mut manager = SatelliteManager::new();

        manager.create_satellite_from_rocket(Vec2::ZERO, Vec2::ZERO, 100.0, None);
        manager.create_satellite_from_rocket(Vec2::ZERO, Vec2::ZERO, 50.0, None);

        manager.update_statistics();

        assert_eq!(manager.network_stats.total_satellites, 2);
        assert!(manager.network_stats.total_network_fuel > 0.0);
    }
}
