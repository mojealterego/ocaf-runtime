//! Advisory Ricci-inspired routing step from OCAF reference architecture, p. 21.
//! This is a traffic/latency proxy, NOT a Wasserstein-1 Ollivier-Ricci solver.
//! Invalid inputs fail closed without partially mutating the caller's graph.

#[derive(Debug, Clone, PartialEq)]
pub struct RoutingGraphEdge {
    pub source: usize,
    pub target: usize,
    pub latency_cost: f64,
    pub traffic_load: f64,
    pub current_weight: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RicciError {
    InvalidConfiguration,
    InvalidEdge,
    NumericalInstability,
    ResourceExhausted,
}

#[derive(Debug, Clone)]
pub struct RicciFlowOptimizer {
    learning_rate: f64,
    min_weight: f64,
    max_weight: f64,
}

impl RicciFlowOptimizer {
    pub fn new(learning_rate: f64) -> Result<Self, RicciError> {
        Self::with_bounds(learning_rate, 0.001, 1000.0)
    }

    pub fn with_bounds(
        learning_rate: f64,
        min_weight: f64,
        max_weight: f64,
    ) -> Result<Self, RicciError> {
        if !learning_rate.is_finite()
            || learning_rate <= 0.0
            || learning_rate > 1.0
            || !min_weight.is_finite()
            || min_weight <= 0.0
            || !max_weight.is_finite()
            || max_weight < min_weight
        {
            return Err(RicciError::InvalidConfiguration);
        }
        Ok(Self {
            learning_rate,
            min_weight,
            max_weight,
        })
    }

    /// Computes a whole-graph update. A failed update leaves all weights unchanged.
    /// Callers should route with validated original weights (e.g. Dijkstra) on error.
    pub fn compute_ricci_step(&self, edges: &mut [RoutingGraphEdge]) -> Result<(), RicciError> {
        // Reject pathological batches before allocating candidate storage.
        // No graph weights are mutated on any error path.
        const MAX_BATCH_EDGES: usize = 1_000_000;
        if edges.len() > MAX_BATCH_EDGES {
            return Err(RicciError::ResourceExhausted);
        }
        let mut candidates = Vec::new();
        candidates
            .try_reserve_exact(edges.len())
            .map_err(|_| RicciError::ResourceExhausted)?;
        for edge in edges.iter() {
            if !edge.latency_cost.is_finite()
                || edge.latency_cost <= 0.0
                || !edge.traffic_load.is_finite()
                || edge.traffic_load < 0.0
                || !edge.current_weight.is_finite()
                || edge.current_weight <= 0.0
            {
                return Err(RicciError::InvalidEdge);
            }
            let metric_distance = edge.latency_cost.max(0.0001);
            let distribution_cost = edge.traffic_load * 0.5 + edge.latency_cost * 0.5;
            let curvature = 1.0 - distribution_cost / metric_distance;
            let adjustment = self.learning_rate * curvature * edge.current_weight;
            let candidate = edge.current_weight - adjustment;
            if !distribution_cost.is_finite()
                || !curvature.is_finite()
                || !adjustment.is_finite()
                || !candidate.is_finite()
            {
                return Err(RicciError::NumericalInstability);
            }
            candidates.push(candidate.clamp(self.min_weight, self.max_weight));
        }
        for (edge, weight) in edges.iter_mut().zip(candidates) {
            edge.current_weight = weight;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge() -> RoutingGraphEdge {
        RoutingGraphEdge {
            source: 0, target: 1, latency_cost: 10.0,
            traffic_load: 2.0, current_weight: 4.0,
        }
    }

    #[test]
    fn finite_step_is_deterministic() {
        let optimizer = RicciFlowOptimizer::new(0.5).unwrap();
        let mut a = vec![edge()];
        let mut b = a.clone();
        optimizer.compute_ricci_step(&mut a).unwrap();
        optimizer.compute_ricci_step(&mut b).unwrap();
        assert_eq!(a, b);
        assert!(a[0].current_weight.is_finite());
        assert!(a[0].current_weight > 0.0);
    }

    #[test]
    fn rejects_invalid_configuration() {
        for rate in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -1.0, 1.01] {
            assert_eq!(RicciFlowOptimizer::new(rate).unwrap_err(), RicciError::InvalidConfiguration);
        }
    }

    #[test]
    fn invalid_edges_do_not_partially_mutate_graph() {
        let optimizer = RicciFlowOptimizer::new(0.5).unwrap();
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -1.0] {
            let mut edges = vec![edge(), RoutingGraphEdge { latency_cost: invalid, ..edge() }];
            let before = edges[0].current_weight;
            assert!(optimizer.compute_ricci_step(&mut edges).is_err());
            assert_eq!(edges[0].current_weight, before);
        }
    }

    #[test]
    fn intermediate_overflow_is_rejected_without_mutation() {
        let optimizer = RicciFlowOptimizer::new(1.0).unwrap();
        let mut edges = vec![edge(), RoutingGraphEdge {
            traffic_load: f64::MAX, latency_cost: f64::MIN_POSITIVE, ..edge()
        }];
        let before = edges[0].current_weight;
        assert_eq!(optimizer.compute_ricci_step(&mut edges), Err(RicciError::NumericalInstability));
        assert_eq!(edges[0].current_weight, before);
    }

    #[test]
    fn invalid_weight_and_load_leave_graph_unchanged() {
        let optimizer = RicciFlowOptimizer::new(0.5).unwrap();
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0] {
            let mut edges = vec![edge(), RoutingGraphEdge { traffic_load: invalid, ..edge() }];
            let snapshot = edges.clone();
            assert_eq!(optimizer.compute_ricci_step(&mut edges), Err(RicciError::InvalidEdge));
            assert_eq!(edges, snapshot);
            let mut edges = vec![edge(), RoutingGraphEdge { current_weight: invalid, ..edge() }];
            let snapshot = edges.clone();
            assert_eq!(optimizer.compute_ricci_step(&mut edges), Err(RicciError::InvalidEdge));
            assert_eq!(edges, snapshot);
        }
    }

    #[test]
    fn zero_edges_are_valid() {
        let optimizer = RicciFlowOptimizer::new(0.5).unwrap();
        let mut edges = Vec::new();
        assert_eq!(optimizer.compute_ricci_step(&mut edges), Ok(()));
    }

    #[test]
    fn bounded_output() {
        let optimizer = RicciFlowOptimizer::with_bounds(1.0, 0.1, 5.0).unwrap();
        let mut edges = vec![edge()];
        optimizer.compute_ricci_step(&mut edges).unwrap();
        assert!((0.1..=5.0).contains(&edges[0].current_weight));
    }
}
