# print and knit_print snapshots cover core classes

    Code
      cat(capture.output(print(x)))
    Message
      -- Model Summary: run001 -------------------------------------------------------
      Problem: Base one-compartment oral absorption model
      Records: 240 | Observations: 210 | Subjects: 30
      Final OFV: -103.3
      -- Estimation Methods --
      * First Order Conditional Estimation with Interaction
      -- Heuristic Checks --
      [OK] Minimization Successful
      [OK] Covariance Step Successful
      [OK] No Eigenvalue Issues
      [OK] No Parameters Near Boundary
      [OK] No Hessian Resets
      -- Theta Parameters --
      -- Omega Parameters --
      -- Sigma Parameters --
    Output
          Parameter  Estimate  Fixed ─────────  ────────  ───── THETA1     1.241     No    THETA2     40.86     No    THETA3     1.241     No        Parameter   Random Effect  Estimate  Shrinkage (%)  Fixed ──────────  ─────────────  ────────  ─────────────  ───── OMEGA(1,1)  ETA1           0.1309    18.98          No    OMEGA(2,2)  ETA2           0.1357    4.909          No    OMEGA(3,3)  ETA3           0.1       NA             Yes       Parameter   Random Effect  Estimate  Shrinkage (%)  Fixed ──────────  ─────────────  ────────  ─────────────  ───── SIGMA(1,1)  EPS1           0.03635   15.28          No    SIGMA(2,2)  EPS2           0.01      NA             Yes

