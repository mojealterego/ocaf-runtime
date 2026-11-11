//! Finite information geometry for TRS research.
//! This module implements classical Fisher-Rao geometry on the probability
//! simplex and a graph Hodge 0-Laplacian. It does not prove Hodge conjecture
//! or identify the proposed infinite-dimensional Kahler manifold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryError {
    Empty, DimensionMismatch, NonFinite, InvalidProbability,
    NotNormalized, InvalidEdge, InvalidStep,
}

const TOL: f64 = 1e-10;

fn validate_probability(p: &[f64]) -> Result<(), GeometryError> {
    if p.is_empty() { return Err(GeometryError::Empty); }
    if p.iter().any(|x| !x.is_finite()) { return Err(GeometryError::NonFinite); }
    if p.iter().any(|x| *x <= 0.0) { return Err(GeometryError::InvalidProbability); }
    let sum: f64 = p.iter().sum();
    if !sum.is_finite() { return Err(GeometryError::NonFinite); }
    if (sum - 1.0).abs() > TOL { return Err(GeometryError::NotNormalized); }
    Ok(())
}

/// Fisher information metric for categorical probabilities in simplex
/// tangent coordinates: g_p(u,v) = sum_i u_i*v_i/p_i.
pub fn fisher_metric(p: &[f64], u: &[f64], v: &[f64]) -> Result<f64, GeometryError> {
    validate_probability(p)?;
    if p.len() != u.len() || p.len() != v.len() {
        return Err(GeometryError::DimensionMismatch);
    }
    if u.iter().chain(v.iter()).any(|x| !x.is_finite()) {
        return Err(GeometryError::NonFinite);
    }
    if u.iter().sum::<f64>().abs() > TOL || v.iter().sum::<f64>().abs() > TOL {
        return Err(GeometryError::InvalidStep);
    }
    let result: f64 = p.iter().zip(u).zip(v).map(|((pi, ui), vi)| ui * vi / pi).sum();
    if !result.is_finite() { return Err(GeometryError::NonFinite); }
    Ok(result)
}

/// Geodesic distance under the classical Fisher-Rao metric:
/// 2 * acos(sum sqrt(p_i*q_i)), with p_i,q_i > 0.
pub fn fisher_rao_distance(p: &[f64], q: &[f64]) -> Result<f64, GeometryError> {
    validate_probability(p)?;
    validate_probability(q)?;
    if p.len() != q.len() { return Err(GeometryError::DimensionMismatch); }
    let affinity: f64 = p.iter().zip(q).map(|(a, b)| (a * b).sqrt()).sum();
    if !affinity.is_finite() { return Err(GeometryError::NonFinite); }
    Ok(2.0 * affinity.clamp(0.0, 1.0).acos())
}

/// Hodge 0-Laplacian L0 = B1 B1^T for a finite oriented graph.
/// Orientation cancels in L0; this returns L0*x without forming a dense matrix.
pub fn hodge_zero_laplacian(
    values: &[f64],
    edges: &[(usize, usize)],
) -> Result<Vec<f64>, GeometryError> {
    if values.is_empty() { return Err(GeometryError::Empty); }
    if values.iter().any(|v| !v.is_finite()) { return Err(GeometryError::NonFinite); }
    let mut out = vec![0.0; values.len()];
    let mut seen = std::collections::HashSet::new();
    for &(a, b) in edges {
        if a >= values.len() || b >= values.len() || a == b ||
            !seen.insert((a.min(b), a.max(b))) {
            return Err(GeometryError::InvalidEdge);
        }
        let delta = values[a] - values[b];
        out[a] += delta;
        out[b] -= delta;
    }
    if out.iter().any(|v| !v.is_finite()) { return Err(GeometryError::NonFinite); }
    Ok(out)
}

/// Discrete Dirichlet energy <x,Lx> = sum_edges (x_u-x_v)^2.
/// Nonnegative for all finite inputs (subject to floating point rounding).
pub fn dirichlet_energy(values: &[f64], edges: &[(usize, usize)]) -> Result<f64, GeometryError> {
    let lap = hodge_zero_laplacian(values, edges)?;
    let result: f64 = values.iter().zip(lap.iter()).map(|(x, lx)| x * lx).sum();
    if !result.is_finite() { return Err(GeometryError::NonFinite); }
    Ok(result.max(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fisher_distance_is_symmetric_and_zero_on_diagonal() {
        let p = [0.5, 0.5];
        let q = [0.25, 0.75];
        assert!(fisher_rao_distance(&p, &p).unwrap() < 1e-7);
        let d1 = fisher_rao_distance(&p, &q).unwrap();
        let d2 = fisher_rao_distance(&q, &p).unwrap();
        assert!((d1 - d2).abs() < 1e-12);
        assert!(d1 > 0.0);
    }
    #[test]
    fn fisher_metric_is_positive_on_nonzero_tangent() {
        let p = [0.25, 0.75];
        let tangent = [0.1, -0.1];
        let g = fisher_metric(&p, &tangent, &tangent).unwrap();
        assert!((g - (0.01 / 0.25 + 0.01 / 0.75)).abs() < 1e-12);
    }
    #[test]
    fn fisher_rejects_boundary_and_invalid_tangent() {
        assert_eq!(fisher_rao_distance(&[0.0, 1.0], &[0.5, 0.5]), Err(GeometryError::InvalidProbability));
        assert_eq!(fisher_metric(&[0.5, 0.5], &[0.1, 0.0], &[0.0, 0.0]), Err(GeometryError::InvalidStep));
    }
    #[test]
    fn hodge_laplacian_annihilates_constant_functions() {
        assert_eq!(hodge_zero_laplacian(&[3.0, 3.0, 3.0], &[(0, 1), (1, 2)]).unwrap(), vec![0.0; 3]);
    }
    #[test]
    fn hodge_energy_matches_edge_differences() {
        let x = [1.0, 2.0, 4.0];
        let edges = [(0, 1), (1, 2)];
        assert!((dirichlet_energy(&x, &edges).unwrap() - 5.0).abs() < 1e-12);
        assert_eq!(hodge_zero_laplacian(&x, &[(0, 1), (1, 0)]), Err(GeometryError::InvalidEdge));
    }
}
