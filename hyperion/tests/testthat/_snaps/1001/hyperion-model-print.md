# hyperion.nonmem-model print works

    Code
      mod
    Message
      
      -- NONMEM Model: 1001 ----------------------------------------------------------
      Problem: PK Structural Model
      Dataset: ../../../../data/derived/PK_Oral_Ex1.csv
      Ignore: @
    Output
       
    Message
      
      -- Theta Parameters --
      
    Output
       
      Parameter  Initial  Lower  Fixed  Comment      
      ─────────  ───────  ─────  ─────  ─────────────
      THETA1     19       0      No     CL/F (L/h)   
      THETA2     304      0      No     VC/F (L)     
      THETA3     2        0      No     KA (1/hr)    
      THETA4     1        NA     Yes    F1 (fraction)
       
    Message
      -- Omega Parameters --
      
    Output
       
      Parameter   Initial  Fixed  Comment    
      ──────────  ───────  ─────  ───────────
      OMEGA(1,1)  0.1      No     OM1 CL :EXP
      OMEGA(2,2)  0.1      No     OM2 VC :EXP
      OMEGA(3,3)  0.1      No     OM3 KA :EXP
       
    Message
      -- Sigma Parameters --
      
    Output
       
      Parameter   Initial  Fixed  Comment
      ──────────  ───────  ─────  ───────
      SIGMA(1,1)  0.1      No     SIG1   
      SIGMA(2,2)  2        No     SIG2   

