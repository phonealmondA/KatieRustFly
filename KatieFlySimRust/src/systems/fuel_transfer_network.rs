// Fuel Transfer Network - Advanced fuel distribution system with routing optimization

use std::collections::{HashMap, BinaryHeap, VecDeque};
use std::cmp::Ordering;
use crate::systems::EntityId;
use crate::utils::vector_helper;
use macroquad::prelude::*;

/// Fuel transfer request priority
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Emergency = 3,
    Critical = 4,
}

/// Transfer request status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// Fuel transfer request
#[derive(Debug, Clone)]
pub struct FuelTransferRequest {
    pub id: usize,
    pub source_id: EntityId,
    pub destination_id: EntityId,
    pub amount: f32,
    pub priority: TransferPriority,
    pub status: TransferStatus,
    pub created_time: f32,
}

/// Connection between two satellites
#[derive(Debug, Clone)]
pub struct SatelliteConnection {
    pub satellite1_id: EntityId,
    pub satellite2_id: EntityId,
    pub distance: f32,
    pub transfer_efficiency: f32, // 0.0 to 1.0
}

/// Network flow statistics
#[derive(Debug, Clone, Default)]
pub struct NetworkFlowStats {
    pub total_fuel_transferred: f32,
    pub active_transfers: usize,
    pub completed_transfers: usize,
    pub failed_transfers: usize,
    pub average_efficiency: f32,
}

/// Network optimization mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkOptimizationMode {
    Balanced,        // Balance between all satellites
    PriorityInner,   // Prioritize inner satellites
    PriorityOuter,   // Prioritize outer satellites
    EmergencyOnly,   // Only handle emergency transfers
    MaintenanceFirst, // Prioritize orbital maintenance needs
}

/// Fuel Transfer Network manager
pub struct FuelTransferNetwork {
    // Transfer requests
    requests: VecDeque<FuelTransferRequest>,
    active_transfers: HashMap<EntityId, FuelTransferRequest>,
    next_request_id: usize,

    // Network topology
    connections: Vec<SatelliteConnection>,
    max_transfer_range: f32,
    max_simultaneous_transfers: usize,

    // Configuration
    optimization_mode: NetworkOptimizationMode,
    emergency_fuel_threshold: f32,  // 10%
    critical_fuel_threshold: f32,   // 5%

    // Statistics
    stats: NetworkFlowStats,
    game_time: f32,
}

impl FuelTransferNetwork {
    pub fn new() -> Self {
        FuelTransferNetwork {
            requests: VecDeque::new(),
            active_transfers: HashMap::new(),
            next_request_id: 0,
            connections: Vec::new(),
            max_transfer_range: 500.0,
            max_simultaneous_transfers: 5,
            optimization_mode: NetworkOptimizationMode::Balanced,
            emergency_fuel_threshold: 0.10,
            critical_fuel_threshold: 0.05,
            stats: NetworkFlowStats::default(),
            game_time: 0.0,
        }
    }

    // === Configuration ===

    pub fn set_optimization_mode(&mut self, mode: NetworkOptimizationMode) {
        self.optimization_mode = mode;
    }

    pub fn set_max_transfer_range(&mut self, range: f32) {
        self.max_transfer_range = range;
    }

    pub fn set_max_simultaneous_transfers(&mut self, max: usize) {
        self.max_simultaneous_transfers = max;
    }

    // === Request Management ===

    /// Request a fuel transfer
    pub fn request_transfer(
        &mut self,
        source_id: EntityId,
        destination_id: EntityId,
        amount: f32,
        priority: TransferPriority,
    ) -> usize {
        let request = FuelTransferRequest {
            id: self.next_request_id,
            source_id,
            destination_id,
            amount,
            priority,
            status: TransferStatus::Pending,
            created_time: self.game_time,
        };

        let request_id = self.next_request_id;
        self.next_request_id += 1;

        // Insert based on priority (emergency requests go first)
        if priority as u32 >= TransferPriority::Emergency as u32 {
            self.requests.push_front(request);
        } else {
            self.requests.push_back(request);
        }

        request_id
    }

    /// Update network connections based on satellite positions
    pub fn update_connections(&mut self, satellite_positions: &HashMap<EntityId, Vec2>) {
        self.connections.clear();

        let satellites: Vec<(&EntityId, &Vec2)> = satellite_positions.iter().collect();

        for i in 0..satellites.len() {
            for j in (i + 1)..satellites.len() {
                let (id1, pos1) = satellites[i];
                let (id2, pos2) = satellites[j];

                let distance = vector_helper::distance(*pos1, *pos2);

                if distance <= self.max_transfer_range {
                    let efficiency = self.calculate_transfer_efficiency(distance);
                    self.connections.push(SatelliteConnection {
                        satellite1_id: *id1,
                        satellite2_id: *id2,
                        distance,
                        transfer_efficiency: efficiency,
                    });
                }
            }
        }
    }

    /// Calculate transfer efficiency based on distance
    fn calculate_transfer_efficiency(&self, distance: f32) -> f32 {
        // Efficiency decreases with distance
        let ratio = distance / self.max_transfer_range;
        (1.0 - ratio * 0.5).max(0.5).min(1.0)
    }

    /// Check if two satellites are connected
    pub fn are_satellites_connected(&self, sat1_id: EntityId, sat2_id: EntityId) -> bool {
        self.connections.iter().any(|conn| {
            (conn.satellite1_id == sat1_id && conn.satellite2_id == sat2_id)
                || (conn.satellite1_id == sat2_id && conn.satellite2_id == sat1_id)
        })
    }

    /// Get all satellites connected to a given satellite
    pub fn get_connected_satellites(&self, sat_id: EntityId) -> Vec<EntityId> {
        let mut connected = Vec::new();

        for conn in &self.connections {
            if conn.satellite1_id == sat_id {
                connected.push(conn.satellite2_id);
            } else if conn.satellite2_id == sat_id {
                connected.push(conn.satellite1_id);
            }
        }

        connected
    }

    // === Dijkstra's Algorithm for Optimal Routing ===

    /// Find optimal fuel transfer path using Dijkstra's algorithm
    pub fn dijkstra_fuel_path(
        &self,
        start_id: EntityId,
        goal_id: EntityId,
    ) -> Option<(Vec<EntityId>, f32)> {
        #[derive(Clone, PartialEq)]
        struct Node {
            id: EntityId,
            cost: f32,
            path: Vec<EntityId>,
        }

        impl Eq for Node {}

        impl Ord for Node {
            fn cmp(&self, other: &Self) -> Ordering {
                other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
            }
        }

        impl PartialOrd for Node {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        let mut heap = BinaryHeap::new();
        let mut visited = HashMap::new();

        heap.push(Node {
            id: start_id,
            cost: 0.0,
            path: vec![start_id],
        });

        while let Some(Node { id, cost, path }) = heap.pop() {
            if id == goal_id {
                return Some((path, cost));
            }

            if visited.contains_key(&id) {
                continue;
            }

            visited.insert(id, cost);

            // Explore neighbors
            for conn in &self.connections {
                let (neighbor_id, edge_cost) = if conn.satellite1_id == id {
                    (conn.satellite2_id, conn.distance * (2.0 - conn.transfer_efficiency))
                } else if conn.satellite2_id == id {
                    (conn.satellite1_id, conn.distance * (2.0 - conn.transfer_efficiency))
                } else {
                    continue;
                };

                if !visited.contains_key(&neighbor_id) {
                    let mut new_path = path.clone();
                    new_path.push(neighbor_id);

                    heap.push(Node {
                        id: neighbor_id,
                        cost: cost + edge_cost,
                        path: new_path,
                    });
                }
            }
        }

        None
    }

    // === Optimization Strategies ===

    /// Optimize network based on current mode
    pub fn optimize_network(&mut self, satellite_fuel_levels: &HashMap<EntityId, (f32, f32)>) {
        match self.optimization_mode {
            NetworkOptimizationMode::Balanced => {
                self.balance_fuel_distribution(satellite_fuel_levels);
            }
            NetworkOptimizationMode::EmergencyOnly => {
                self.handle_emergency_transfers_only(satellite_fuel_levels);
            }
            _ => {
                // Other modes can be implemented as needed
            }
        }
    }

    /// Balance fuel distribution across all satellites
    fn balance_fuel_distribution(&mut self, satellite_fuel_levels: &HashMap<EntityId, (f32, f32)>) {
        // Calculate average fuel level
        let mut total_fuel = 0.0;
        let mut count = 0.0;

        for (fuel, _max_fuel) in satellite_fuel_levels.values() {
            total_fuel += fuel;
            count += 1.0;
        }

        if count == 0.0 {
            return;
        }

        let average_fuel = total_fuel / count;

        // Create transfer requests to balance
        for (sat_id, (fuel, max_fuel)) in satellite_fuel_levels {
            let fuel_ratio = fuel / max_fuel;

            // If satellite has much more than average, donate
            if *fuel > average_fuel * 1.5 {
                // Find a satellite with low fuel
                for (target_id, (target_fuel, target_max)) in satellite_fuel_levels {
                    if target_id != sat_id && *target_fuel < average_fuel * 0.5 {
                        let amount = (fuel - average_fuel).min(target_max - target_fuel).min(50.0);
                        if amount > 0.0 {
                            self.request_transfer(*sat_id, *target_id, amount, TransferPriority::Normal);
                        }
                    }
                }
            }
        }
    }

    /// Handle only emergency transfers
    fn handle_emergency_transfers_only(&mut self, satellite_fuel_levels: &HashMap<EntityId, (f32, f32)>) {
        for (sat_id, (fuel, max_fuel)) in satellite_fuel_levels {
            let fuel_ratio = fuel / max_fuel;

            if fuel_ratio < self.critical_fuel_threshold {
                // Find nearest satellite with fuel to donate
                for (source_id, (source_fuel, _source_max)) in satellite_fuel_levels {
                    if source_id != sat_id && *source_fuel > 100.0 {
                        let amount = (100.0 - fuel).min(50.0);
                        self.request_transfer(*source_id, *sat_id, amount, TransferPriority::Critical);
                        break;
                    }
                }
            } else if fuel_ratio < self.emergency_fuel_threshold {
                // Find nearest satellite with fuel to donate
                for (source_id, (source_fuel, _source_max)) in satellite_fuel_levels {
                    if source_id != sat_id && *source_fuel > 100.0 {
                        let amount = (200.0 - fuel).min(50.0);
                        self.request_transfer(*source_id, *sat_id, amount, TransferPriority::Emergency);
                        break;
                    }
                }
            }
        }
    }

    // === Update ===

    pub fn update(&mut self, delta_time: f32) {
        self.game_time += delta_time;

        // Process pending requests
        while self.active_transfers.len() < self.max_simultaneous_transfers {
            if let Some(mut request) = self.requests.pop_front() {
                request.status = TransferStatus::InProgress;
                self.active_transfers.insert(request.destination_id, request);
                self.stats.active_transfers += 1;
            } else {
                break;
            }
        }
    }

    /// Complete a transfer
    pub fn complete_transfer(&mut self, destination_id: EntityId, success: bool) {
        if let Some(mut request) = self.active_transfers.remove(&destination_id) {
            request.status = if success {
                TransferStatus::Completed
            } else {
                TransferStatus::Failed
            };

            if success {
                self.stats.completed_transfers += 1;
                self.stats.total_fuel_transferred += request.amount;
            } else {
                self.stats.failed_transfers += 1;
            }

            self.stats.active_transfers = self.stats.active_transfers.saturating_sub(1);
        }
    }

    // === Getters ===

    pub fn stats(&self) -> &NetworkFlowStats {
        &self.stats
    }

    pub fn active_transfer_count(&self) -> usize {
        self.active_transfers.len()
    }

    pub fn pending_request_count(&self) -> usize {
        self.requests.len()
    }

    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    // === Visualization ===

    /// Draw network connections
    pub fn draw_network(&self, satellite_positions: &HashMap<EntityId, Vec2>) {
        for conn in &self.connections {
            if let (Some(pos1), Some(pos2)) = (
                satellite_positions.get(&conn.satellite1_id),
                satellite_positions.get(&conn.satellite2_id),
            ) {
                let color = if conn.transfer_efficiency > 0.8 {
                    Color::new(0.0, 1.0, 0.0, 0.3)
                } else if conn.transfer_efficiency > 0.6 {
                    Color::new(1.0, 1.0, 0.0, 0.3)
                } else {
                    Color::new(1.0, 0.5, 0.0, 0.3)
                };

                draw_line(pos1.x, pos1.y, pos2.x, pos2.y, 2.0, color);
            }
        }
    }
}

impl Default for FuelTransferNetwork {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuel_transfer_request() {
        let mut network = FuelTransferNetwork::new();

        let request_id = network.request_transfer(
            1,
            2,
            100.0,
            TransferPriority::Normal,
        );

        assert_eq!(request_id, 0);
        assert_eq!(network.pending_request_count(), 1);
    }

    #[test]
    fn test_emergency_priority() {
        let mut network = FuelTransferNetwork::new();

        network.request_transfer(1, 2, 50.0, TransferPriority::Normal);
        network.request_transfer(3, 4, 100.0, TransferPriority::Emergency);

        // Both requests should be processed (max_simultaneous_transfers is 5)
        network.update(0.0);
        assert_eq!(network.active_transfer_count(), 2);
    }

    #[test]
    fn test_dijkstra_routing() {
        let mut network = FuelTransferNetwork::new();

        let mut positions = HashMap::new();
        positions.insert(0, Vec2::new(0.0, 0.0));
        positions.insert(1, Vec2::new(100.0, 0.0));
        positions.insert(2, Vec2::new(200.0, 0.0));

        network.update_connections(&positions);

        let path = network.dijkstra_fuel_path(0, 2);
        assert!(path.is_some());

        if let Some((route, _cost)) = path {
            assert!(route.len() >= 2);
            assert_eq!(route[0], 0);
            assert_eq!(*route.last().unwrap(), 2);
        }
    }
}
