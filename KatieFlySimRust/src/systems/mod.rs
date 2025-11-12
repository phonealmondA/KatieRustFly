// Game systems module

pub mod world;
pub mod fuel_transfer_network;
pub mod orbit_maintenance;
pub mod vehicle_manager;
pub mod satellite_manager;
pub mod player_input;

pub use world::{World, EntityId, DestroyedRocketInfo};
pub use fuel_transfer_network::{
    FuelTransferNetwork, FuelTransferRequest, TransferPriority,
    TransferStatus, NetworkOptimizationMode, NetworkFlowStats,
};
pub use orbit_maintenance::{
    OrbitMaintenance, OrbitDriftAnalysis, DriftSeverity,
    ManeuverType, MaintenanceConfig,
};
pub use vehicle_manager::{VehicleManager, VisualizationOptions};
pub use satellite_manager::{
    SatelliteManager, SatelliteStatus, SatelliteNetworkStats,
    SatelliteManagerConfig,
};
pub use player_input::{PlayerInput, PlayerInputState};
