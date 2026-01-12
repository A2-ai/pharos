# hyperion.nonmem-model print works

    Code
      mod
    Message
      
      
      -- NONMEM Model: iov -----------------------------------------------------------
      Problem: created from pharos see iov_metadata.json for details.
      Dataset: test.csv
      Ignore: @
    Output
       
    Message
      
      -- Theta Parameters --
      
    Output
       
      Parameter  Initial  Fixed  Comment       
      ─────────  ───────  ─────  ──────────────
      THETA1     1.75     No     KA (1/hr) :LOG
      THETA2     7.3      No     CL (L/hr) :LOG
      THETA3     4        No     V2 (L) :LOG   
      THETA4     12       No     Q (L/hr) :LOG 
      THETA5     12.2     No     V3 (L) :LOG   
      THETA6     0        Yes    F1 ([]) :LOG  
      THETA7     0.75     Yes    WT_on_CL ([]) 
       
    Message
      -- Omega Parameters --
      
    Output
       
      Parameter     Initial  Fixed  Comment    
      ────────────  ───────  ─────  ───────────
      OMEGA(1,1)    0.35     No     OM1 KA :LOG
      OMEGA(2,2)    0.15     No     OM2 CL :LOG
      OMEGA(3,3)    0.12     No     OM3 V2 :LOG
      OMEGA(4,4)    0        Yes    OM4 Q  :LOG
      OMEGA(5,5)    0.07     No     OM5 V3 :LOG
      OMEGA(6,6)    0        Yes    OM6 F1 :LOG
      OMEGA(7,7)    0.06     No     IOV :LOG   
      OMEGA(8,8)    0.06     No     IOV :LOG   
      OMEGA(9,9)    0.06     No     IOV :LOG   
      OMEGA(10,10)  0.06     No     IOV :LOG   
      OMEGA(11,11)  0.06     No     IOV :LOG   
       
    Message
      -- Sigma Parameters --
      
    Output
       
      Parameter   Initial  Fixed  Comment   
      ──────────  ───────  ─────  ──────────
      SIGMA(1,1)  0.14     No     SIG1 :PROP
      SIGMA(2,2)  0.05     No     SIG2 :ADD 

