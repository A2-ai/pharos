# hyperion.nonmem-summary print works

    Code
      mod_sum
    Message
      
      
      -- Model Summary: run003b1 -----------------------------------------------------
      Problem: Base one-compartment oral absorption model created from pharos see
      run003b1_metadata.json for details.
      Records: 240 | Observations: 210 | Subjects: 30
      Final OFV: -108.9
      
      -- Estimation Methods --
      
      * First Order Conditional Estimation with Interaction
      
      -- Heuristic Checks --
      
      [OK] Minimization Successful
      [OK] Covariance Step Successful
      [OK] No Eigenvalue Issues
      [x] Parameters Near Boundary
      [OK] No Hessian Resets
    Output
       
    Message
      
      -- Theta Parameters --
      
    Output
       
      Parameter  Estimate  Fixed
      ─────────  ────────  ─────
      TVCL       1.25      No   
      THETA2     0.545     No   
      TVV        40.28     No   
      TVKA       1.218     No   
       
    Message
      -- Omega Parameters --
      
    Output
       
      Parameter   Random Effect  Estimate  Shrinkage (%)  Fixed
      ──────────  ─────────────  ────────  ─────────────  ─────
      OM1 (TVCL)  ETA1           0.1233    13.66          No   
      OMEGA(2,1)  ETA1:ETA2      0.07218   NA             No   
      OM2 (TVV)   ETA2           0.1246    4.625          No   
      OM3 (TVKA)  ETA3           0.1239    24.36          No   
       
    Message
      -- Sigma Parameters --
      
    Output
       
      Parameter   Random Effect  Estimate  Shrinkage (%)  Fixed
      ──────────  ─────────────  ────────  ─────────────  ─────
      SIGMA(1,1)  EPS1           0.03735   14.51          No   
      SIGMA(2,2)  EPS2           0.005894  14.51          No   

