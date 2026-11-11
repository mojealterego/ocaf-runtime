//! TRS experimental discrete mathematics: 3-SAT energy and graph topology.
//! These are exact finite computations, NOT proofs of P != NP, RH, Hodge or BSD.
//! Formula clauses are signed 1-based literals; e.g. [1,2,-3].
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrsMathError {
    InvalidVariable,
    TooManyVariables,
    InvalidClause,
    InvalidEdge,
    NonFinite,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clause3(pub [i32; 3]);

impl Clause3 {
    pub fn validate(&self, variables: usize) -> Result<(), TrsMathError> {
        if variables == 0 || variables > 24 {
            return Err(TrsMathError::TooManyVariables);
        }
        for &literal in &self.0 {
            if literal == 0 || literal.unsigned_abs() as usize > variables {
                return Err(TrsMathError::InvalidVariable);
            }
        }
        Ok(())
    }

    /// Unsatisfied-clause indicator in {0,1}. For spin sigma=1-2*x:
    /// (x1 OR x2 OR NOT x3) => (1+s1)(1+s2)(1-s3)/8.
    pub fn penalty(&self, assignment: u64) -> u8 {
        let unsatisfied = self.0.iter().all(|&lit| {
            let x = (assignment >> (lit.unsigned_abs() - 1)) & 1 != 0;
            if lit > 0 { !x } else { x }
        });
        u8::from(unsatisfied)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SatResult {
    pub minimum_energy: u32,
    pub minimizers: Vec<u64>,
    pub satisfying_count: u64,
}

/// Exhaustive exact finite baseline. Limited to 24 variables to avoid
/// accidentally exposing an exponential algorithm as a production SAT solver.
pub fn enumerate_3sat(
    variables: usize,
    clauses: &[Clause3],
) -> Result<SatResult, TrsMathError> {
    if variables == 0 || variables > 24 {
        return Err(TrsMathError::TooManyVariables);
    }
    for clause in clauses {
        clause.validate(variables)?;
    }
    let mut minimum_energy = u32::MAX;
    let mut minimizers = Vec::new();
    let mut satisfying_count = 0;
    for assignment in 0..(1u64 << variables) {
        let energy: u32 = clauses.iter().map(|c| u32::from(c.penalty(assignment))).sum();
        if energy == 0 {
            satisfying_count += 1;
        }
        if energy < minimum_energy {
            minimum_energy = energy;
            minimizers.clear();
            minimizers.push(assignment);
        } else if energy == minimum_energy {
            minimizers.push(assignment);
        }
    }
    Ok(SatResult { minimum_energy, minimizers, satisfying_count })
}

/// First Betti number of the 1-dimensional graph as a cell complex:
/// beta_1 = edges - vertices + connected_components.
/// It is NOT the Betti number of a SAT solution manifold.
pub fn graph_betti_one(vertices: usize, edges: &[(usize, usize)]) -> Result<usize, TrsMathError> {
    let mut parent: Vec<usize> = (0..vertices).collect();
    fn root(parent: &mut [usize], x: usize) -> usize {
        if parent[x] != x {
            let p = parent[x];
            parent[x] = root(parent, p);
        }
        parent[x]
    }
    let mut unique = HashSet::new();
    for &(u, v) in edges {
        if u >= vertices || v >= vertices || u == v {
            return Err(TrsMathError::InvalidEdge);
        }
        let edge = (u.min(v), u.max(v));
        if !unique.insert(edge) {
            return Err(TrsMathError::InvalidEdge);
        }
        let a = root(&mut parent, u);
        let b = root(&mut parent, v);
        if a != b { parent[a] = b; }
    }
    let components = (0..vertices).filter(|&v| root(&mut parent, v) == v).count();
    Ok(unique.len() + components - vertices)
}

/// One explicit Euler step of combinatorial heat flow u' = -L u.
/// Requires 0 <= dt <= 1/max_degree to preserve positivity and mass.
pub fn graph_heat_step(
    values: &[f64],
    edges: &[(usize, usize)],
    dt: f64,
) -> Result<Vec<f64>, TrsMathError> {
    if !dt.is_finite() || dt < 0.0 || values.iter().any(|v| !v.is_finite()) {
        return Err(TrsMathError::NonFinite);
    }
    let mut degrees = vec![0usize; values.len()];
    let mut unique = HashSet::new();
    for &(u, v) in edges {
        if u >= values.len() || v >= values.len() || u == v {
            return Err(TrsMathError::InvalidEdge);
        }
        if !unique.insert((u.min(v), u.max(v))) {
            return Err(TrsMathError::InvalidEdge);
        }
        degrees[u] += 1;
        degrees[v] += 1;
    }
    let max_degree = degrees.iter().copied().max().unwrap_or(0);
    if max_degree > 0 && dt > 1.0 / max_degree as f64 {
        return Err(TrsMathError::NonFinite);
    }
    let mut next = values.to_vec();
    for &(u, v) in edges {
        let flux = dt * (values[v] - values[u]);
        next[u] += flux;
        next[v] -= flux;
    }
    if next.iter().any(|v| !v.is_finite()) {
        return Err(TrsMathError::NonFinite);
    }
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clauses_match_exact_spin_penalties() {
        let a = Clause3([1, 2, -3]);
        let b = Clause3([-1, 3, 4]);
        assert_eq!(a.penalty(0b100), 1); // x1=0,x2=0,x3=1
        assert_eq!(a.penalty(0b000), 0);
        assert_eq!(b.penalty(0b001), 1); // x1=1,x3=0,x4=0
        assert_eq!(b.penalty(0b101), 0);
    }

    #[test]
    fn reported_two_clause_instance_is_satisfiable() {
        let result = enumerate_3sat(4, &[Clause3([1, 2, -3]), Clause3([-1, 3, 4])]).unwrap();
        assert_eq!(result.minimum_energy, 0);
        assert!(result.satisfying_count > 0);
    }

    #[test]
    fn contradictory_clauses_have_positive_energy() {
        let clauses = [Clause3([1, 1, 1]), Clause3([-1, -1, -1])];
        let result = enumerate_3sat(1, &clauses).unwrap();
        assert_eq!(result.minimum_energy, 1);
        assert_eq!(result.satisfying_count, 0);
    }

    #[test]
    fn graph_cycle_has_one_independent_loop() {
        assert_eq!(graph_betti_one(3, &[(0, 1), (1, 2), (2, 0)]), Ok(1));
        assert_eq!(graph_betti_one(3, &[(0, 1), (1, 2)]), Ok(0));
        assert_eq!(graph_betti_one(4, &[(0, 1), (2, 3)]), Ok(0));
    }

    #[test]
    fn heat_step_conserves_mass_and_smooths() {
        let before = [1.0, 0.0];
        let after = graph_heat_step(&before, &[(0, 1)], 0.25).unwrap();
        assert_eq!(after, vec![0.75, 0.25]);
        assert_eq!(after.iter().sum::<f64>(), before.iter().sum::<f64>());
        assert_eq!(graph_heat_step(&before, &[(0, 1)], 1.1), Err(TrsMathError::NonFinite));
    }
}
