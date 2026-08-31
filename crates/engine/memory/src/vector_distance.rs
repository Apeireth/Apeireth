//! Vector distance utilities (salvage of canonical `apeireth-vector::distance`).
//!
//! Canonical [`crate::canonical::vector::cosine_similarity`] already owns
//! cosine for the in-memory index. This module recovers the extra metrics
//! (L2, L2², dot, Manhattan) and the canonical distance→score maps used by the
//! persistent KNN fallback. Length mismatch or non-finite input returns
//! `None` instead of panicking (the canonical used `assert_eq!`).

use crate::canonical::vector::cosine_similarity;

/// Distance / similarity metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DistanceMetric {
    Euclidean,
    EuclideanSquared,
    Cosine,
    DotProduct,
    Manhattan,
}

impl DistanceMetric {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Euclidean => "euclidean",
            Self::EuclideanSquared => "euclidean_squared",
            Self::Cosine => "cosine",
            Self::DotProduct => "dot_product",
            Self::Manhattan => "manhattan",
        }
    }
}

fn same_finite_len(a: &[f32], b: &[f32]) -> bool {
    a.len() == b.len() && a.iter().all(|x| x.is_finite()) && b.iter().all(|x| x.is_finite())
}

/// L2 distance. `None` on length mismatch or non-finite values.
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> Option<f32> {
    euclidean_distance_sq(a, b).map(f32::sqrt)
}

/// Squared L2 distance (avoids `sqrt` when only ordering is needed).
pub fn euclidean_distance_sq(a: &[f32], b: &[f32]) -> Option<f32> {
    if !same_finite_len(a, b) {
        return None;
    }
    Some(
        a.iter()
            .zip(b)
            .map(|(x, y)| {
                let d = x - y;
                d * d
            })
            .sum(),
    )
}

/// Cosine similarity via the canonical implementation. `None` on mismatch.
pub fn cosine(a: &[f32], b: &[f32]) -> Option<f32> {
    if !same_finite_len(a, b) {
        return None;
    }
    Some(cosine_similarity(a, b))
}

/// Cosine distance = `1 - cosine_similarity`.
pub fn cosine_distance(a: &[f32], b: &[f32]) -> Option<f32> {
    cosine(a, b).map(|s| 1.0 - s)
}

/// Dot product (unnormalized).
pub fn dot_product(a: &[f32], b: &[f32]) -> Option<f32> {
    if !same_finite_len(a, b) {
        return None;
    }
    Some(a.iter().zip(b).map(|(x, y)| x * y).sum())
}

/// Manhattan (L1) distance.
pub fn manhattan_distance(a: &[f32], b: &[f32]) -> Option<f32> {
    if !same_finite_len(a, b) {
        return None;
    }
    Some(a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum())
}

/// L2 norm. `None` when any component is non-finite.
pub fn l2_norm(v: &[f32]) -> Option<f32> {
    if v.iter().any(|x| !x.is_finite()) {
        return None;
    }
    Some(v.iter().map(|x| x * x).sum::<f32>().sqrt())
}

/// Unit vector, or a copy of the input when the norm is zero.
pub fn normalize(v: &[f32]) -> Option<Vec<f32>> {
    let n = l2_norm(v)?;
    if n == 0.0 {
        Some(v.to_vec())
    } else {
        Some(v.iter().map(|x| x / n).collect())
    }
}

/// Distance according to `metric`. Dot product is returned negated so that
/// smaller values remain "closer" (canonical convention).
pub fn distance(a: &[f32], b: &[f32], metric: DistanceMetric) -> Option<f32> {
    match metric {
        DistanceMetric::Euclidean => euclidean_distance(a, b),
        DistanceMetric::EuclideanSquared => euclidean_distance_sq(a, b),
        DistanceMetric::Cosine => cosine_distance(a, b),
        DistanceMetric::DotProduct => dot_product(a, b).map(|d| -d),
        DistanceMetric::Manhattan => manhattan_distance(a, b),
    }
}

/// Engine sqlite-vec score maps, kept as ranking heuristics without the C
/// extension:
///
/// - cosine distance `d ∈ [0, 2]` → `1 - d/2` (similarity in `[-1, 1]`,
///   identical vectors score `1`)
/// - L2 distance `d ≥ 0` → `1 / (1 + d)` (monotonic, `(0, 1]`)
pub fn cosine_distance_to_score(distance: f32) -> f32 {
    if !distance.is_finite() {
        return 0.0;
    }
    1.0 - distance * 0.5
}

pub fn l2_distance_to_score(distance: f32) -> f32 {
    if !distance.is_finite() || distance < 0.0 {
        return 0.0;
    }
    1.0 / (1.0 + distance)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn euclidean_and_manhattan_known_values() {
        let a = [1.0, 2.0, 3.0];
        let b = [4.0, 6.0, 8.0];
        assert!(approx(euclidean_distance(&a, &b).unwrap(), 7.071));
        assert!(approx(euclidean_distance_sq(&a, &b).unwrap(), 50.0));
        assert_eq!(manhattan_distance(&a, &b).unwrap(), 12.0);
        assert_eq!(euclidean_distance(&a, &a).unwrap(), 0.0);
    }

    #[test]
    fn cosine_matches_canonical_and_orthogonal() {
        assert!(approx(
            cosine(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0]).unwrap(),
            1.0
        ));
        assert!(approx(cosine(&[1.0, 0.0], &[0.0, 1.0]).unwrap(), 0.0));
        assert!(approx(
            cosine_distance(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0]).unwrap(),
            0.0
        ));
        assert_eq!(
            dot_product(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]).unwrap(),
            32.0
        );
    }

    #[test]
    fn normalize_and_zero() {
        let n = normalize(&[3.0, 4.0]).unwrap();
        assert!(approx(n[0], 0.6));
        assert!(approx(n[1], 0.8));
        assert_eq!(normalize(&[0.0, 0.0]).unwrap(), vec![0.0, 0.0]);
        assert_eq!(l2_norm(&[3.0, 4.0]).unwrap(), 5.0);
    }

    #[test]
    fn mismatch_and_nonfinite_are_none() {
        assert!(euclidean_distance(&[1.0], &[1.0, 2.0]).is_none());
        assert!(cosine(&[1.0, f32::NAN], &[1.0, 0.0]).is_none());
        assert!(l2_norm(&[f32::INFINITY]).is_none());
    }

    #[test]
    fn score_maps() {
        assert!((cosine_distance_to_score(0.0) - 1.0).abs() < 1e-6);
        assert!((l2_distance_to_score(0.0) - 1.0).abs() < 1e-6);
        assert!(l2_distance_to_score(1.0) < 1.0);
        assert_eq!(cosine_distance_to_score(f32::NAN), 0.0);
        assert_eq!(DistanceMetric::Cosine.as_str(), "cosine");
        assert_eq!(
            distance(&[1.0], &[1.0], DistanceMetric::Euclidean).unwrap(),
            0.0
        );
    }
}
