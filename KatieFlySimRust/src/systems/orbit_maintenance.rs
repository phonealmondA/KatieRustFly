// Orbit Maintenance System - Autonomous orbital station-keeping

use macroquad::prelude::*;
use crate::entities::{Satellite, Planet};
use crate::game_constants::GameConstants;
use crate::utils::vector_helper;
use std::f32::consts::PI;

/// Orbital drift analysis
#[derive(Debug, Clone)]
pub struct OrbitDriftAnalysis {
    pub radius_deviation: f32,      // Deviation from target orbital radius
    pub eccentricity_deviation: f32, // Deviation from target eccentricity
    pub period_deviation: f32,       // Deviation from target period
    pub drift_severity: DriftSeverity,
}

/// Drift severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriftSeverity {
    Nominal,   // Within tolerance
    Minor,     // Slight deviation
    Moderate,  // Noticeable deviation
    Severe,    // Requires immediate correction
    Critical,  // Emergency correction needed
}

/// Orbital maneuver type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManeuverType {
    Prograde,          // Speed up (raise apoapsis)
    Retrograde,        // Slow down (lower periapsis)
    Circularization,   // Reduce eccentricity
    InclinationCorrection, // Adjust orbital plane
}

/// Scheduled orbital maneuver
#[derive(Debug, Clone)]
pub struct ScheduledManeuver {
    pub maneuver_type: ManeuverType,
    pub delta_v: Vec2,
    pub fuel_cost: f32,
    pub priority: u32,
    pub execute_time: f32,
}

/// Orbit maintenance configuration
#[derive(Debug, Clone)]
pub struct MaintenanceConfig {
    pub target_orbital_radius: f32,
    pub target_eccentricity: f32,
    pub radius_tolerance: f32,
    pub eccentricity_tolerance: f32,
    pub check_interval: f32,
    pub prefer_small_frequent_burns: bool,
    pub emergency_decay_threshold: f32,
}

impl Default for MaintenanceConfig {
    fn default() -> Self {
        MaintenanceConfig {
            target_orbital_radius: 300.0,
            target_eccentricity: 0.0, // Circular orbit
            radius_tolerance: 20.0,
            eccentricity_tolerance: 0.1,
            check_interval: 5.0,
            prefer_small_frequent_burns: true,
            emergency_decay_threshold: 0.85, // 85% of target radius
        }
    }
}

/// Orbit Maintenance System
pub struct OrbitMaintenance {
    config: MaintenanceConfig,
    scheduled_maneuvers: Vec<ScheduledManeuver>,
    time_since_last_check: f32,
    total_delta_v_expended: f32,
    maintenance_count: u32,
    emergency_mode: bool,
}

impl OrbitMaintenance {
    pub fn new(config: MaintenanceConfig) -> Self {
        OrbitMaintenance {
            config,
            scheduled_maneuvers: Vec::new(),
            time_since_last_check: 0.0,
            total_delta_v_expended: 0.0,
            maintenance_count: 0,
            emergency_mode: false,
        }
    }

    pub fn with_target_radius(target_radius: f32) -> Self {
        let mut config = MaintenanceConfig::default();
        config.target_orbital_radius = target_radius;
        Self::new(config)
    }

    // === Configuration ===

    pub fn set_target_radius(&mut self, radius: f32) {
        self.config.target_orbital_radius = radius;
    }

    pub fn set_target_eccentricity(&mut self, eccentricity: f32) {
        self.config.target_eccentricity = eccentricity;
    }

    pub fn config(&self) -> &MaintenanceConfig {
        &self.config
    }

    // === Drift Analysis ===

    /// Perform maintenance check on satellite orbit
    pub fn perform_maintenance_check(
        &mut self,
        satellite: &Satellite,
        planet: &Planet,
    ) -> OrbitDriftAnalysis {
        let current_radius = vector_helper::distance(satellite.position(), planet.position());
        let current_velocity_magnitude = vector_helper::magnitude(satellite.velocity());

        // Calculate orbital parameters
        let orbital_velocity = self.calculate_orbital_velocity(
            planet.mass(),
            current_radius,
        );

        let escape_velocity = (2.0 * GameConstants::G * planet.mass() / current_radius).sqrt();

        // Analyze drift
        let radius_deviation = current_radius - self.config.target_orbital_radius;
        let velocity_deviation = current_velocity_magnitude - orbital_velocity;

        // Estimate eccentricity (simplified)
        let eccentricity = self.estimate_eccentricity(
            satellite.position(),
            satellite.velocity(),
            planet.position(),
            planet.mass(),
        );

        let eccentricity_deviation = eccentricity - self.config.target_eccentricity;

        // Calculate period
        let period = self.calculate_orbital_period(planet.mass(), current_radius);
        let target_period = self.calculate_orbital_period(
            planet.mass(),
            self.config.target_orbital_radius,
        );
        let period_deviation = period - target_period;

        // Determine severity
        let drift_severity = self.calculate_drift_severity(
            radius_deviation,
            eccentricity_deviation,
            current_radius,
        );

        OrbitDriftAnalysis {
            radius_deviation,
            eccentricity_deviation,
            period_deviation,
            drift_severity,
        }
    }

    /// Calculate drift severity
    fn calculate_drift_severity(
        &self,
        radius_deviation: f32,
        eccentricity_deviation: f32,
        current_radius: f32,
    ) -> DriftSeverity {
        // Check for critical decay
        if current_radius < self.config.target_orbital_radius * self.config.emergency_decay_threshold {
            return DriftSeverity::Critical;
        }

        let radius_deviation_abs = radius_deviation.abs();
        let eccentricity_deviation_abs = eccentricity_deviation.abs();

        if radius_deviation_abs > self.config.radius_tolerance * 3.0
            || eccentricity_deviation_abs > self.config.eccentricity_tolerance * 3.0
        {
            DriftSeverity::Severe
        } else if radius_deviation_abs > self.config.radius_tolerance * 2.0
            || eccentricity_deviation_abs > self.config.eccentricity_tolerance * 2.0
        {
            DriftSeverity::Moderate
        } else if radius_deviation_abs > self.config.radius_tolerance
            || eccentricity_deviation_abs > self.config.eccentricity_tolerance
        {
            DriftSeverity::Minor
        } else {
            DriftSeverity::Nominal
        }
    }

    // === Maneuver Planning ===

    /// Calculate required maneuvers to correct orbit
    pub fn calculate_required_maneuvers(
        &mut self,
        satellite: &Satellite,
        planet: &Planet,
        drift_analysis: &OrbitDriftAnalysis,
    ) -> Vec<ScheduledManeuver> {
        let mut maneuvers = Vec::new();

        match drift_analysis.drift_severity {
            DriftSeverity::Nominal => {
                // No correction needed
                return maneuvers;
            }
            DriftSeverity::Minor | DriftSeverity::Moderate => {
                // Gentle corrections
                maneuvers.extend(self.plan_gentle_correction(satellite, planet, drift_analysis));
            }
            DriftSeverity::Severe | DriftSeverity::Critical => {
                // Aggressive corrections
                self.emergency_mode = true;
                maneuvers.extend(self.plan_emergency_correction(satellite, planet, drift_analysis));
            }
        }

        maneuvers
    }

    /// Plan gentle orbital correction
    fn plan_gentle_correction(
        &self,
        satellite: &Satellite,
        planet: &Planet,
        drift_analysis: &OrbitDriftAnalysis,
    ) -> Vec<ScheduledManeuver> {
        let mut maneuvers = Vec::new();

        // Calculate correction delta-v
        let direction_to_planet = vector_helper::normalize(planet.position() - satellite.position());
        let velocity_direction = vector_helper::normalize(satellite.velocity());

        if drift_analysis.radius_deviation < -self.config.radius_tolerance {
            // Orbit too low - prograde burn to raise it
            let correction_magnitude = drift_analysis.radius_deviation.abs() * 0.1;
            let delta_v = velocity_direction * correction_magnitude;

            maneuvers.push(ScheduledManeuver {
                maneuver_type: ManeuverType::Prograde,
                delta_v,
                fuel_cost: correction_magnitude * 2.0,
                priority: 1,
                execute_time: 0.0,
            });
        } else if drift_analysis.radius_deviation > self.config.radius_tolerance {
            // Orbit too high - retrograde burn to lower it
            let correction_magnitude = drift_analysis.radius_deviation.abs() * 0.1;
            let delta_v = velocity_direction * -correction_magnitude;

            maneuvers.push(ScheduledManeuver {
                maneuver_type: ManeuverType::Retrograde,
                delta_v,
                fuel_cost: correction_magnitude * 2.0,
                priority: 1,
                execute_time: 0.0,
            });
        }

        // Circularization if eccentricity too high
        if drift_analysis.eccentricity_deviation.abs() > self.config.eccentricity_tolerance {
            let correction_magnitude = drift_analysis.eccentricity_deviation.abs() * 20.0;
            let perpendicular = Vec2::new(-velocity_direction.y, velocity_direction.x);
            let delta_v = perpendicular * correction_magnitude;

            maneuvers.push(ScheduledManeuver {
                maneuver_type: ManeuverType::Circularization,
                delta_v,
                fuel_cost: correction_magnitude * 2.0,
                priority: 2,
                execute_time: 0.5,
            });
        }

        maneuvers
    }

    /// Plan emergency orbital correction
    fn plan_emergency_correction(
        &self,
        satellite: &Satellite,
        planet: &Planet,
        drift_analysis: &OrbitDriftAnalysis,
    ) -> Vec<ScheduledManeuver> {
        let mut maneuvers = Vec::new();

        let velocity_direction = vector_helper::normalize(satellite.velocity());

        // Aggressive prograde burn to prevent decay
        let correction_magnitude = drift_analysis.radius_deviation.abs() * 0.5;
        let delta_v = velocity_direction * correction_magnitude;

        maneuvers.push(ScheduledManeuver {
            maneuver_type: ManeuverType::Prograde,
            delta_v,
            fuel_cost: correction_magnitude * 3.0,
            priority: 10,
            execute_time: 0.0,
        });

        maneuvers
    }

    /// Execute scheduled maneuvers
    pub fn execute_scheduled_maneuvers(
        &mut self,
        satellite: &mut Satellite,
        delta_time: f32,
    ) -> bool {
        let mut executed = false;

        // Update maneuver timing
        for maneuver in &mut self.scheduled_maneuvers {
            maneuver.execute_time -= delta_time;
        }

        // Execute ready maneuvers
        self.scheduled_maneuvers.retain(|maneuver| {
            if maneuver.execute_time <= 0.0 {
                // Check if satellite has enough fuel
                if satellite.current_fuel() >= maneuver.fuel_cost {
                    // Apply delta-v
                    let new_velocity = satellite.velocity() + maneuver.delta_v;
                    satellite.set_velocity(new_velocity);

                    // Consume fuel
                    satellite.consume_fuel(maneuver.fuel_cost);

                    self.total_delta_v_expended += vector_helper::magnitude(maneuver.delta_v);
                    self.maintenance_count += 1;
                    executed = true;

                    false // Remove from list
                } else {
                    true // Keep in list (fuel shortage)
                }
            } else {
                true // Keep in list (not ready yet)
            }
        });

        executed
    }

    // === Update ===

    pub fn update(
        &mut self,
        satellite: &mut Satellite,
        planet: &Planet,
        delta_time: f32,
    ) {
        self.time_since_last_check += delta_time;

        // Periodic maintenance check
        if self.time_since_last_check >= self.config.check_interval || self.emergency_mode {
            let drift_analysis = self.perform_maintenance_check(satellite, planet);

            // Calculate and schedule maneuvers
            let new_maneuvers = self.calculate_required_maneuvers(
                satellite,
                planet,
                &drift_analysis,
            );

            self.scheduled_maneuvers.extend(new_maneuvers);
            self.time_since_last_check = 0.0;

            // Clear emergency mode if drift is nominal
            if drift_analysis.drift_severity == DriftSeverity::Nominal {
                self.emergency_mode = false;
            }
        }

        // Execute maneuvers
        self.execute_scheduled_maneuvers(satellite, delta_time);
    }

    // === Orbital Mechanics Calculations ===

    fn calculate_orbital_velocity(&self, planet_mass: f32, orbital_radius: f32) -> f32 {
        (GameConstants::G * planet_mass / orbital_radius).sqrt()
    }

    fn calculate_orbital_period(&self, planet_mass: f32, orbital_radius: f32) -> f32 {
        2.0 * PI * (orbital_radius.powi(3) / (GameConstants::G * planet_mass)).sqrt()
    }

    fn estimate_eccentricity(
        &self,
        position: Vec2,
        velocity: Vec2,
        planet_pos: Vec2,
        planet_mass: f32,
    ) -> f32 {
        let r = position - planet_pos;
        let r_mag = vector_helper::magnitude(r);
        let v_mag = vector_helper::magnitude(velocity);

        let mu = GameConstants::G * planet_mass;

        // Specific orbital energy
        let energy = (v_mag * v_mag) / 2.0 - mu / r_mag;

        // Specific angular momentum (scalar)
        let h = r.x * velocity.y - r.y * velocity.x;

        // Eccentricity from vis-viva equation (simplified)
        let eccentricity_sq = 1.0 + (2.0 * energy * h * h) / (mu * mu);

        if eccentricity_sq > 0.0 {
            eccentricity_sq.sqrt()
        } else {
            0.0
        }
    }

    // === Statistics ===

    pub fn total_delta_v_expended(&self) -> f32 {
        self.total_delta_v_expended
    }

    pub fn maintenance_count(&self) -> u32 {
        self.maintenance_count
    }

    pub fn is_emergency_mode(&self) -> bool {
        self.emergency_mode
    }

    pub fn scheduled_maneuver_count(&self) -> usize {
        self.scheduled_maneuvers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orbit_maintenance_creation() {
        let config = MaintenanceConfig::default();
        let maintenance = OrbitMaintenance::new(config);

        assert_eq!(maintenance.maintenance_count(), 0);
        assert!(!maintenance.is_emergency_mode());
    }

    #[test]
    fn test_drift_severity_calculation() {
        let maintenance = OrbitMaintenance::with_target_radius(300.0);

        // Nominal
        let severity = maintenance.calculate_drift_severity(5.0, 0.01, 300.0);
        assert_eq!(severity, DriftSeverity::Nominal);

        // Severe
        let severity = maintenance.calculate_drift_severity(100.0, 0.5, 300.0);
        assert_eq!(severity, DriftSeverity::Severe);

        // Critical (decay)
        let severity = maintenance.calculate_drift_severity(-100.0, 0.1, 200.0);
        assert_eq!(severity, DriftSeverity::Critical);
    }

    #[test]
    fn test_orbital_velocity_calculation() {
        let maintenance = OrbitMaintenance::with_target_radius(300.0);
        let planet_mass = 10000.0;
        let orbital_radius = 300.0;

        let velocity = maintenance.calculate_orbital_velocity(planet_mass, orbital_radius);
        assert!(velocity > 0.0);
    }
}
