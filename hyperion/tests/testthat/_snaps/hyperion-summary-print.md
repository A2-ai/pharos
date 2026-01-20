# hyperion.nonmem-summary print works

    Code
      print(mod_sum)
    Message
      
      
      -- Model Summary: run001 -------------------------------------------------------
      Problem: Base one-compartment oral absorption model
      Records: 240 | Observations: 210 | Subjects: 30
      Final OFV: -103.3
      
      -- Estimation Methods --
      
      * First Order Conditional Estimation with Interaction
        Condition Number: 1.98
      
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
      THETA1     1.241     0.1129  9.096    No   
      THETA2     40.86     3       7.343    No   
      THETA3     1.241     0.108   8.697    No   
       
    Message
      -- Omega Parameters --
      
    Output
       
      Parameter   Random Effect  Estimate  SE       RSE (%)  Shrinkage (%)  Fixed
      ──────────  ─────────────  ────────  ───────  ───────  ─────────────  ─────
      OMEGA(1,1)  ETA1           0.1309    0.05481  41.86    18.98          No   
      OMEGA(2,2)  ETA2           0.1357    0.03891  28.68    4.909          No   
      OMEGA(3,3)  ETA3           0.1       NA       NA       NA             Yes  
       
    Message
      -- Sigma Parameters --
      
    Output
       
      Parameter   Random Effect  Estimate  SE        RSE (%)  Shrinkage (%)  Fixed
      ──────────  ─────────────  ────────  ────────  ───────  ─────────────  ─────
      SIGMA(1,1)  EPS1           0.03635   0.005009  13.78    15.28          No   
      SIGMA(2,2)  EPS2           0.01      NA        NA       NA             Yes  

---

    Code
      print(mod_sum)
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

---

    Code
      print(mod_sum)
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

---

    Code
      print(mod_sum)
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

