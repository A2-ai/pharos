use anyhow::{Result as AnyhowResult, bail};
use nalgebra::{DMatrix, linalg};

pub(crate) const EPS_PD: f64 = 1e-8;
pub(crate) const TOL: f64 = 1e-8;
pub(crate) const MAX_ITERS: usize = 200;

/// Project a symmetric matrix to the nearest positive definite matrix
/// via spectral decomposition with fixed epsilon clamping.
pub(crate) fn nearest_pd(mat: &DMatrix<f64>) -> DMatrix<f64> {
    project_pd(mat, EPS_PD)
}

fn project_pd(m: &DMatrix<f64>, eps_pd: f64) -> DMatrix<f64> {
    let sym = (m + m.transpose()) / 2.0;
    let eigen = linalg::SymmetricEigen::new(sym);
    let clamped = DMatrix::from_diagonal(&eigen.eigenvalues.map(|e| e.max(eps_pd)));
    let result = &eigen.eigenvectors * clamped * eigen.eigenvectors.transpose();
    (&result + result.transpose()) / 2.0
}

fn project_fixed(
    m: &DMatrix<f64>,
    fixed_mask: &DMatrix<bool>,
    fixed_values: &DMatrix<f64>,
) -> DMatrix<f64> {
    let mut out = m.clone();
    let n = out.nrows();
    for i in 0..n {
        for j in 0..=i {
            if fixed_mask[(i, j)] {
                let v = fixed_values[(i, j)];
                out[(i, j)] = v;
                out[(j, i)] = v;
            }
        }
    }
    out
}

fn min_eigenvalue_sym(m: &DMatrix<f64>) -> f64 {
    let sym = (m + m.transpose()) / 2.0;
    let eigen = linalg::SymmetricEigen::new(sym);
    eigen
        .eigenvalues
        .iter()
        .fold(f64::INFINITY, |acc, &v| acc.min(v))
}

/// Project a symmetric matrix with fixed elements to the
/// nearest positive definite matrix maintaing fixed elements
pub(crate) fn constrained_nearest_pd(
    mat: &DMatrix<f64>,
    fixed_mask: &DMatrix<bool>,
    eps_pd: f64,
    max_iters: usize,
    tol: f64,
) -> AnyhowResult<DMatrix<f64>> {
    let n = mat.nrows();
    if n != mat.ncols() {
        bail!("constrained_nearest_pd: mat must be square");
    }
    if fixed_mask.nrows() != n || fixed_mask.ncols() != n {
        bail!("constrained_nearest_pd: fixed_mask shape must match mat");
    }

    for i in 0..n {
        for j in 0..n {
            if fixed_mask[(i, j)] != fixed_mask[(j, i)] {
                bail!("constrained_nearest_pd: fixed_mask must be symmetric");
            }
        }
    }

    let mut fixed_values = DMatrix::<f64>::zeros(n, n);
    for i in 0..n {
        for j in 0..n {
            if fixed_mask[(i, j)] {
                fixed_values[(i, j)] = mat[(i, j)];
            }
        }
    }

    for i in 0..n {
        if fixed_mask[(i, i)] && fixed_values[(i, i)] <= eps_pd {
            bail!(
                "constrained_nearest_pd infeasible: fixed diagonal ({},{})={} <= eps_pd={}",
                i + 1,
                i + 1,
                fixed_values[(i, i)],
                eps_pd
            );
        }
    }

    let all_fixed = fixed_mask.iter().all(|&b| b);
    if all_fixed {
        let x = (mat + mat.transpose()) / 2.0;
        if min_eigenvalue_sym(&x) >= eps_pd - tol {
            return Ok(x);
        }
        bail!("constrained_nearest_pd infeasible: all entries fixed and matrix is not PD");
    }

    let mut x = (mat + mat.transpose()) / 2.0;
    let mut p = DMatrix::<f64>::zeros(n, n);
    let mut q = DMatrix::<f64>::zeros(n, n);

    for _ in 0..max_iters {
        let y_in = &x + &p;
        let y = project_pd(&y_in, eps_pd);
        p = y_in - &y;

        let x_in = &y + &q;
        let x_new = project_fixed(&x_in, fixed_mask, &fixed_values);
        q = x_in - &x_new;

        let rel = (&x_new - &x).norm() / x.norm().max(1.0);
        x = x_new;

        if rel < tol {
            if min_eigenvalue_sym(&x) < eps_pd - tol {
                bail!("constrained_nearest_pd converged but PD constraint not met");
            }
            return Ok((&x + x.transpose()) / 2.0);
        }
    }

    bail!(
        "constrained_nearest_pd failed to converge within {} iterations",
        max_iters
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_pd_projects_to_positive_definite() {
        let mat = DMatrix::<f64>::from_row_slice(2, 2, &[1.0, 2.0, 2.0, 1.0]);
        let repaired = nearest_pd(&mat);
        assert!(min_eigenvalue_sym(&repaired) >= EPS_PD - TOL);
    }

    #[test]
    fn constrained_pd_preserves_fixed_entries_and_repairs() {
        let mat = DMatrix::<f64>::from_row_slice(2, 2, &[1.0, 1.01, 1.01, 1.0]);
        let mut fixed_mask = DMatrix::<bool>::from_element(2, 2, false);
        fixed_mask[(0, 0)] = true;

        let repaired = constrained_nearest_pd(&mat, &fixed_mask, EPS_PD, MAX_ITERS, TOL).unwrap();

        assert!((repaired[(0, 0)] - 1.0).abs() <= TOL);
        assert!(min_eigenvalue_sym(&repaired) >= EPS_PD - TOL);
    }

    #[test]
    fn constrained_pd_rejects_nonpositive_fixed_diagonal() {
        let mat = DMatrix::<f64>::from_row_slice(2, 2, &[0.0, 0.0, 0.0, 1.0]);
        let mut fixed_mask = DMatrix::<bool>::from_element(2, 2, false);
        fixed_mask[(0, 0)] = true;

        let err = constrained_nearest_pd(&mat, &fixed_mask, EPS_PD, MAX_ITERS, TOL)
            .unwrap_err()
            .to_string();
        assert!(err.contains("fixed diagonal"), "unexpected error: {err}");
    }

    #[test]
    fn constrained_pd_rejects_all_fixed_non_pd_matrix() {
        let mat = DMatrix::<f64>::from_row_slice(2, 2, &[1.0, 2.0, 2.0, 1.0]);
        let fixed_mask = DMatrix::<bool>::from_element(2, 2, true);

        let err = constrained_nearest_pd(&mat, &fixed_mask, EPS_PD, MAX_ITERS, TOL)
            .unwrap_err()
            .to_string();
        assert!(err.contains("all entries fixed"), "unexpected error: {err}");
    }

    #[test]
    fn constrained_pd_repairs_5x5_with_fixed_zero_covariances() {
        // Symmetric, intentionally non-PD matrix with several fixed zero covariances.
        let mat = DMatrix::<f64>::from_row_slice(
            5,
            5,
            &[
                0.5, 0.0, 0.8, 0.0, 0.0, //
                0.0, 0.5, 0.0, 0.7, 0.0, //
                0.8, 0.0, 0.5, 0.0, 0.6, //
                0.0, 0.7, 0.0, 0.5, 0.0, //
                0.0, 0.0, 0.6, 0.0, 0.5, //
            ],
        );
        assert!(min_eigenvalue_sym(&mat) < 0.0);

        let mut fixed_mask = DMatrix::<bool>::from_element(5, 5, false);
        let fixed_pairs = [(0, 1), (0, 3), (0, 4), (1, 2), (1, 4), (2, 3), (3, 4)];
        for (i, j) in fixed_pairs {
            fixed_mask[(i, j)] = true;
            fixed_mask[(j, i)] = true;
        }

        let repaired = constrained_nearest_pd(&mat, &fixed_mask, EPS_PD, MAX_ITERS, TOL).unwrap();

        // PD check
        assert!(min_eigenvalue_sym(&repaired) >= EPS_PD - TOL);

        // Fixed zeros must stay fixed.
        for (i, j) in fixed_pairs {
            assert!(repaired[(i, j)].abs() <= TOL, "({i},{j}) drifted");
            assert!(repaired[(j, i)].abs() <= TOL, "({j},{i}) drifted");
        }
    }

    #[test]
    fn constrained_pd_5x5_with_no_constraints_matches_unconstrained_behavior() {
        let mat = DMatrix::<f64>::from_row_slice(
            5,
            5,
            &[
                0.5, 0.0, 0.8, 0.0, 0.0, //
                0.0, 0.5, 0.0, 0.7, 0.0, //
                0.8, 0.0, 0.5, 0.0, 0.6, //
                0.0, 0.7, 0.0, 0.5, 0.0, //
                0.0, 0.0, 0.6, 0.0, 0.5, //
            ],
        );
        assert!(min_eigenvalue_sym(&mat) < 0.0);

        let fixed_mask = DMatrix::<bool>::from_element(5, 5, false);
        let repaired_constrained =
            constrained_nearest_pd(&mat, &fixed_mask, EPS_PD, MAX_ITERS, TOL).unwrap();
        let repaired_unconstrained = nearest_pd(&mat);

        assert!(min_eigenvalue_sym(&repaired_constrained) >= EPS_PD - TOL);
        assert!(min_eigenvalue_sym(&repaired_unconstrained) >= EPS_PD - TOL);
    }
}
