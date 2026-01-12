# hyperion.nonmem-summary print works

    Code
      mod_sum
    Message
      
      
      -- Model Summary: run003 -------------------------------------------------------
      Problem: Base one-compartment oral absorption model
      Records: 240 | Observations: 210 | Subjects: 30
      Final OFV: -109.6
      
      -- Estimation Methods --
      
      * First Order Conditional Estimation with Interaction
        Condition Number: 15.29
      
      -- Heuristic Checks --
      
      [OK] Minimization Successful
      [OK] Covariance Step Successful
      [OK] No Eigenvalue Issues
      [OK] No Parameters Near Boundary
      [OK] No Hessian Resets
    Output
       
    Message
      
      -- Theta Parameters --
      
    Output
       
      Parameter  Estimate  SE      RSE (%)  Fixed
      ─────────  ────────  ──────  ───────  ─────
      TVCL       1.32      0.1389  10.52    No   
      TVV        40.18     3.075   7.653    No   
      TVKA       1.211     0.1381  11.4     No   
       
    Message
      -- Omega Parameters --
      
    Output
       
      Parameter   Random Effect  Estimate  SE       RSE (%)  Shrinkage (%)  Fixed
      ──────────  ─────────────  ────────  ───────  ───────  ─────────────  ─────
      OM1 (TVCL)  ETA1           0.1189    0.05908  49.67    14.03          No   
      OMEGA(2,1)  ETA1:ETA2      0.07457   0.04026  53.99    NA             No   
      OM2 (TVV)   ETA2           0.1251    0.04464  35.68    4.583          No   
      OM3 (TVKA)  ETA3           0.1236    0.07067  57.17    24.71          No   
       
    Message
      -- Sigma Parameters --
      
    Output
       
      Parameter   Random Effect  Estimate  SE        RSE (%)  Shrinkage (%)  Fixed
      ──────────  ─────────────  ────────  ────────  ───────  ─────────────  ─────
      SIGMA(1,1)  EPS1           0.0359    0.008686  24.2     14.53          No   
      SIGMA(2,2)  EPS2           0.01      NA        NA       NA             Yes  

