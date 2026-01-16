# hyperion.nonmem-summary print works

    Code
      mod_sum
    Message
      
      
      -- Model Summary: run003 -------------------------------------------------------
      Problem: Base one-compartment oral absorption model created from pharos see
      run003_metadata.json for details.
      Records: 240 | Observations: 210 | Subjects: 30
      Final OFV: -109.8
      
      -- Estimation Methods --
      
      * First Order Conditional Estimation with Interaction
        Condition Number: 6.172
      
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
      TVCL       1.325     0.1115  8.411    No   
      TVV        40.16     2.839   7.069    No   
      TVKA       1.212     0.1097  9.057    No   
       
    Message
      -- Omega Parameters --
      
    Output
       
      Parameter   Random Effect  Estimate  SE       RSE (%)  Shrinkage (%)  Fixed
      ──────────  ─────────────  ────────  ───────  ───────  ─────────────  ─────
      OM1 (TVCL)  ETA1           0.1223    0.05036  41.16    13.14          No   
      OMEGA(2,1)  ETA1:ETA2      0.07454   0.03134  42.04    NA             No   
      OM2 (TVV)   ETA2           0.1239    0.03675  29.66    4.631          No   
      OM3 (TVKA)  ETA3           0.1224    0.05628  45.97    24.34          No   
       
    Message
      -- Sigma Parameters --
      
    Output
       
      Parameter   Random Effect  Estimate  SE        RSE (%)  Shrinkage (%)  Fixed
      ──────────  ─────────────  ────────  ────────  ───────  ─────────────  ─────
      SIGMA(1,1)  EPS1           0.03754   0.006035  16.08    14.42          No   
      SIGMA(2,2)  EPS2           0.005272  0.009211  174.7    14.42          No   

