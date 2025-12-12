# hyperion.nonmem-model print works

    Code
      mod
    Message
      
      -- NONMEM Model: everything ----------------------------------------------------
      Problem: Some header #2
      Dataset: ..\data.csv
      Ignore: #, DVID.EQ.3, ID.EQ.3.14
      Records: 200
      Dropped Columns: DATE
      Aliased Columns: AMT→DOSE
    Output
       
    Message
      
      -- Theta Parameters --
      
    Output
       
      Parameter  Initial  Lower  Upper  Fixed  Comment              
      ─────────  ───────  ─────  ─────  ─────  ─────────────────────
      THETA1     1.5      NA     NA     No     THETA(1) and THETA(2)
      THETA2     0.5      0      2      No     THETA(1) and THETA(2)
      THETA3     2.3      NA     NA     Yes    THETA(3)             
      THETA4     0.8      NA     NA     No     THETA(4) and THETA(5)
      THETA5     0.25     NA     NA     No     THETA(4) and THETA(5)
      THETA6     2.3      1      NA     Yes    THETA(6)             
      THETA7     0.75     NA     NA     Yes    THETA(7)             
       
    Message
      -- Omega Parameters --
      
    Output
       
      Parameter   Initial  Lower  Upper  Fixed  Parametrization  Comment                                    
      ──────────  ───────  ─────  ─────  ─────  ───────────────  ───────────────────────────────────────────
      OMEGA(1,1)  0.04     NA     NA     No                      ETA(1) - CL (diagonal)                     
      OMEGA(2,2)  0.17     NA     NA     No                                                                 
      OMEGA(3,3)  0.2      NA     NA     No     Correlation      ETA(2) - V (SD)                            
      OMEGA(4,3)  0.3      NA     NA     No     Correlation      ETA(2)-ETA(3) correlation, ETA(3) - KA (SD)
      OMEGA(4,4)  0.15     NA     NA     No     Correlation      ETA(2)-ETA(3) correlation, ETA(3) - KA (SD)
      OMEGA(5,5)  0.2      NA     NA     No     Correlation      ETA(2) - V (SD)                            
      OMEGA(6,5)  0.3      NA     NA     No     Correlation      ETA(2)-ETA(3) correlation, ETA(3) - KA (SD)
      OMEGA(6,6)  0.15     NA     NA     No     Correlation      ETA(2)-ETA(3) correlation, ETA(3) - KA (SD)
      OMEGA(7,7)  0.1      0      1      Yes                     ETA(6) - fixed diagonal                    
       
    Message
      -- Sigma Parameters --
      
    Output
       
      Parameter   Initial  Fixed  Comment                                     
      ──────────  ───────  ─────  ────────────────────────────────────────────
      SIGMA(1,1)  0.01     No     Proportional error variance                 
      SIGMA(2,1)  0.002    No     Prop-Add covariance, Additive error variance
      SIGMA(2,2)  0.25     No     Prop-Add covariance, Additive error variance

