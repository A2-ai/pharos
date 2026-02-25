# hyperion.nonmem-model print works

    Code
      print(mod)
    Message
      
      
      -- NONMEM Model: 1001 ----------------------------------------------------------
      Problem: PK Structural Model
      Run Status: Not Run
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

---

    Code
      print(mod)
    Message
      
      
      -- NONMEM Model: everything ----------------------------------------------------
      Problem: Some header #2
      Run Status: Not Run
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

---

    Code
      print(mod)
    Message
      
      
      -- NONMEM Model: example1 ------------------------------------------------------
      Problem: RUN# Example 1 (from samp5l)
      Run Status: Not Run
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

---

    Code
      print(mod)
    Message
      
      
      -- NONMEM Model: iiv-cov -------------------------------------------------------
      Problem: PK Structural Model created from pharos see 1002_metadata.json for
      details.
      Run Status: Not Run
      Dataset: ../../data/derived/PK_Oral_Ex1.csv
      Ignore: @
    Output
       
    Message
      
      -- Theta Parameters --
      
    Output
       
      Parameter  Initial  Lower  Fixed  Comment         
      ─────────  ───────  ─────  ─────  ────────────────
      THETA1     19.65    0      No     1  CL/F [L/h]   
      THETA2     211      0      No     2  VC/F [L]     
      THETA3     2.18     0      No     3  KA [1/hr]    
      THETA4     1        NA     Yes    4  F1 [fraction]
      THETA5     2.5      0      No     5  Q/F [L/h]    
      THETA6     22       0      No     6  V2/F [L]     
       
    Message
      -- Omega Parameters --
      
    Output
       
      Parameter   Initial  Fixed  Comment                       
      ──────────  ───────  ─────  ──────────────────────────────
      OMEGA(1,1)  0.8      No     IIV CL/F :lognormal           
      OMEGA(2,1)  0.7      No     OMEGA(2,1) Cov CL/F:V2/F ;corr
      OMEGA(2,2)  0.9      No     IIV V2/F :lognormal           
      OMEGA(3,3)  0.6      No     IIV KA :lognormal             
      OMEGA(4,4)  0        Yes    IIV Q/F :lognormal            
       
    Message
      -- Sigma Parameters --
      
    Output
       
      Parameter   Initial  Fixed  Comment                        
      ──────────  ───────  ─────  ───────────────────────────────
      SIGMA(1,1)  0.068    No     11 PropErr ;Proportional [prop]
      SIGMA(2,2)  0        Yes    22 AddErr ;AddErr [ng/mL]      

---

    Code
      print(mod)
    Message
      
      
      -- NONMEM Model: iov -----------------------------------------------------------
      Problem: created from pharos see iov_metadata.json for details.
      Run Status: Not Run
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

---

    Code
      print(mod)
    Message
      
      
      -- NONMEM Model: multiline_table -----------------------------------------------
      Problem: Some header #2
      Run Status: Not Run
      Dataset: ..\data.csv
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

---

    Code
      print(mod)
    Message
      
      
      -- NONMEM Model: nmexample -----------------------------------------------------
      Problem: RUN# Example 1 (from samp5l)
      Run Status: Not Run
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

