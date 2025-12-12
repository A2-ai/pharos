# hyperion.nonmem-model print works

    Code
      mod
    Message
      
      -- NONMEM Model: example1 ------------------------------------------------------
      Problem: RUN# Example 1 (from samp5l)
      Dataset: example1.csv
      Ignore: C
      Aliased Columns: CONC→DV, DOSE→AMT
    Output
       
    Message
      
      -- Theta Parameters --
      
    Output
       
      Parameter  Initial  Lower  Fixed  Comment 
      ─────────  ───────  ─────  ─────  ────────
      THETA1     2        0.001  No     [LN(CL)]
      THETA2     2        0.001  No     [LN(V1)]
      THETA3     2        0.001  No     [LN(Q)] 
      THETA4     2        0.001  No     [LN(V2)]
       
    Message
      -- Omega Parameters --
      
    Output
       
      Parameter   Initial  Fixed  Comment
      ──────────  ───────  ─────  ───────
      OMEGA(1,1)  0.15     No     [P]    
      OMEGA(2,1)  0.01     No     [F]    
      OMEGA(2,2)  0.15     No     [P]    
      OMEGA(3,1)  0.01     No     [F]    
      OMEGA(3,2)  0.01     No     [F]    
      OMEGA(3,3)  0.15     No     [P]    
      OMEGA(4,1)  0.01     No     [F]    
      OMEGA(4,2)  0.01     No     [F]    
      OMEGA(4,3)  0.01     No     [F]    
      OMEGA(4,4)  0.15     No     [P]    
       
    Message
      -- Sigma Parameters --
      
    Output
       
      Parameter   Initial  Fixed  Comment
      ──────────  ───────  ─────  ───────
      SIGMA(1,1)  0.6      No     [P]    

