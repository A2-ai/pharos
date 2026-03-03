use anyhow::Result as AnyhowResult;
use nalgebra::{DMatrix, linalg};

pub const EPS_PD: f64 = 1e-8;
pub const TOL: f64 = 1e-10;
pub const MAX_ITERS: usize = 200;

/// Project a symmetric matrix to the nearest positive definite matrix
/// via spectral decomposition with fixed epsilon clamping.
pub(crate) fn nearest_pd(mat: &DMatrix<f64>) -> DMatrix<f64> {
    let sym = (mat + mat.transpose()) / 2.0;
    let eigen = linalg::SymmetricEigen::new(sym);
    let clamped = DMatrix::from_diagonal(&eigen.eigenvalues.map(|e| e.max(EPS_PD)));
    let result = &eigen.eigenvectors * clamped * eigen.eigenvectors.transpose();
    (&result + result.transpose()) / 2.0
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
    // grab fixed values first
    //let fixed_values = mat[fixed_mask]
    todo!()
}
