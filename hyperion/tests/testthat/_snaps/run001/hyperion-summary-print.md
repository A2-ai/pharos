# hyperion.nonmem-summary print works

    Code
      mod_sum
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
    Output
       
    Message
      
      -- Theta Parameters --
      
    Output
       
      Parameter  Estimate  Fixed
      ─────────  ────────  ─────
      THETA1     1.241     No   
      THETA2     40.86     No   
      THETA3     1.241     No   
       
    Message
      -- Omega Parameters --
      
    Output
       
      Parameter   Random Effect  Estimate  Shrinkage (%)  Fixed
      ──────────  ─────────────  ────────  ─────────────  ─────
      OMEGA(1,1)  ETA1           0.1309    18.98          No   
      OMEGA(2,2)  ETA2           0.1357    4.909          No   
      OMEGA(3,3)  ETA3           0.1       NA             Yes  
       
    Message
      -- Sigma Parameters --
      
    Output
       
      Parameter   Random Effect  Estimate  Shrinkage (%)  Fixed
      ──────────  ─────────────  ────────  ─────────────  ─────
      SIGMA(1,1)  EPS1           0.03635   15.28          No   
      SIGMA(2,2)  EPS2           0.01      NA             Yes  

