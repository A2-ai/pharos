# hyperion.nonmem-summary print works

    Code
      mod_sum
    Message
      
      -- Model Summary: run002 -------------------------------------------------------
      Problem: Base one-compartment oral absorption model
      Records: 240 | Observations: 210 | Subjects: 30
      Final OFV: -103.5
      
      -- Estimation Methods --
      
      * First Order Conditional Estimation with Interaction
        Condition Number: 29.63
      
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
      TVCL       1.247     0.1288  10.33    No   
      TVV        40.85     3.027   7.411    No   
      TVKA       1.244     0.1134  9.117    No   
       
    Message
      -- Omega Parameters --
      
    Output
       
      Parameter   Random Effect  Estimate  SE       RSE (%)  Shrinkage (%)  Fixed
      ──────────  ─────────────  ────────  ───────  ───────  ─────────────  ─────
      OM1 (TVCL)  ETA1           0.1304    0.06019  46.15    18.06          No   
      OM2 (TVV)   ETA2           0.1363    0.03971  29.13    4.986          No   
      OM3 (TVKA)  ETA3           0.1144    0.06144  53.71    27.19          No   
       
    Message
      -- Sigma Parameters --
      
    Output
       
      Parameter   Random Effect  Estimate  SE       RSE (%)  Shrinkage (%)  Fixed
      ──────────  ─────────────  ────────  ───────  ───────  ─────────────  ─────
      SIGMA(1,1)  EPS1           0.03723   0.0116   31.16    15.44          No   
      SIGMA(2,2)  EPS2           0.006607  0.02792  422.6    15.44          No   

