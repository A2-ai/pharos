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
      [!] Covariance Step Not Run
      [!] Eigenvalue Check Not Available
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

---

    Code
      print(mod_sum)
    Message
      
      
      -- Running Model: run004 -------------------------------------------------------
      i Model is currently running
      
      -- Recent Iterations --
      
    Output
       
       
      iteration  method  THETA1  THETA2  THETA3  SIGMA.1.1.  SIGMA.2.1.  SIGMA.2.2.  OMEGA.1.1.  OMEGA.2.1.  OMEGA.2.2.  OMEGA.3.1.  OMEGA.3.2.  OMEGA.3.3.
      ─────────  ──────  ──────  ──────  ──────  ──────────  ──────────  ──────────  ──────────  ──────────  ──────────  ──────────  ──────────  ──────────
      0          FOCE    1.612   36.18   1.004   0.03574     0           0.006       0.1         0.0001      0.1         0           0           0.1       
      5          FOCE    1.251   40.66   1.246   0.03667     0           0.006033    0.1262      0.000117    0.1325      0           0           0.1067    
      10         FOCE    1.247   40.73   1.24    0.03753     0           0.006265    0.1314      0.0004518   0.1371      0           0           0.114     
      15         FOCE    1.326   40.06   1.212   0.03914     0           0.000918    0.1261      0.07338     0.1228      0           0           0.1207    
      20         FOCE    1.325   40.09   1.21    0.03744     0           0.005545    0.1221      0.07462     0.1239      0           0           0.1223    
      25         FOCE    1.325   40.13   1.211   0.03754     0           0.005267    0.1223      0.07459     0.1239      0           0           0.1224    
      30         FOCE    1.325   40.16   1.212   0.03754     0           0.005272    0.1223      0.07454     0.1239      0           0           0.1224    
    Message
      -- Recent Gradients --
      
    Output
       
       
      iteration  method  GRD.TVCL.  GRD.TVV.  GRD.TVKA.   GRD.ETA1.   GRD.ETA2.    GRD.EPS1.   GRD.7.      GRD.8.     GRD.9.    
      ─────────  ──────  ─────────  ────────  ──────────  ──────────  ───────────  ──────────  ──────────  ─────────  ──────────
      0          FOCE    73.89      -28.82    -39.24      -19.1       -0.2895      -15.42      -3.366      -25.08     -1.02     
      5          FOCE    0.726      -0.4298   1.646       -1.356      -0.1662      -1.269      -1.461      -5.953     -0.471    
      10         FOCE    -0.292     -0.1409   -0.6106     0.221       -0.1557      0.5171      0.04447     1.326      0.00219   
      15         FOCE    -0.476     0.05529   0.08351     0.05463     -0.002578    0.3325      -0.03349    -0.8686    -0.1977   
      20         FOCE    0.07577    0.1036    0.01967     -0.01282    0.0001055    -0.02382    -0.03252    0.08144    0.05834   
      25         FOCE    -0.09361   -0.2471   -0.06087    -0.002033   0.0001151    -0.007342   -0.005697   -0.006275  -0.002337 
      30         FOCE    -0.002697  0.00215   -0.0007652  -0.0003618  0.000001674  -0.0002712  0.00001258  -0.00347   -0.0002455

# hyperion.nonmem-summary print works for run005 (not_run)

    Code
      print(mod_sum)
    Message
      
      -- Model: run005 ---------------------------------------------------------------
      Run Status: Not Run
      Problem: Base one-compartment oral absorption model created from pharos see
      run004_metadata.json for details.
      Dataset: ../../data/derived/onecmpt-oral-30ind.csv
      
      i This model has not been executed. To run it, use one of:
      * submit_model_to_slurm()
      * submit_model_to_sge()

# hyperion.nonmem-summary print works for run004 (running)

    Code
      print(mod_sum)
    Message
      
      
      -- Running Model: run004 -------------------------------------------------------
      i Model is currently running
      
      -- Recent Iterations --
      
    Output
       
       
      iteration  method  THETA1  THETA2  THETA3  SIGMA.1.1.  SIGMA.2.1.  SIGMA.2.2.  OMEGA.1.1.  OMEGA.2.1.  OMEGA.2.2.  OMEGA.3.1.  OMEGA.3.2.  OMEGA.3.3.
      ─────────  ──────  ──────  ──────  ──────  ──────────  ──────────  ──────────  ──────────  ──────────  ──────────  ──────────  ──────────  ──────────
      0          FOCE    1.612   36.18   1.004   0.03574     0           0.006       0.1         0.0001      0.1         0           0           0.1       
      5          FOCE    1.251   40.66   1.246   0.03667     0           0.006033    0.1262      0.000117    0.1325      0           0           0.1067    
      10         FOCE    1.247   40.73   1.24    0.03753     0           0.006265    0.1314      0.0004518   0.1371      0           0           0.114     
      15         FOCE    1.326   40.06   1.212   0.03914     0           0.000918    0.1261      0.07338     0.1228      0           0           0.1207    
      20         FOCE    1.325   40.09   1.21    0.03744     0           0.005545    0.1221      0.07462     0.1239      0           0           0.1223    
      25         FOCE    1.325   40.13   1.211   0.03754     0           0.005267    0.1223      0.07459     0.1239      0           0           0.1224    
      30         FOCE    1.325   40.16   1.212   0.03754     0           0.005272    0.1223      0.07454     0.1239      0           0           0.1224    
    Message
      -- Recent Gradients --
      
    Output
       
       
      iteration  method  GRD.TVCL.  GRD.TVV.  GRD.TVKA.   GRD.ETA1.   GRD.ETA2.    GRD.EPS1.   GRD.7.      GRD.8.     GRD.9.    
      ─────────  ──────  ─────────  ────────  ──────────  ──────────  ───────────  ──────────  ──────────  ─────────  ──────────
      0          FOCE    73.89      -28.82    -39.24      -19.1       -0.2895      -15.42      -3.366      -25.08     -1.02     
      5          FOCE    0.726      -0.4298   1.646       -1.356      -0.1662      -1.269      -1.461      -5.953     -0.471    
      10         FOCE    -0.292     -0.1409   -0.6106     0.221       -0.1557      0.5171      0.04447     1.326      0.00219   
      15         FOCE    -0.476     0.05529   0.08351     0.05463     -0.002578    0.3325      -0.03349    -0.8686    -0.1977   
      20         FOCE    0.07577    0.1036    0.01967     -0.01282    0.0001055    -0.02382    -0.03252    0.08144    0.05834   
      25         FOCE    -0.09361   -0.2471   -0.06087    -0.002033   0.0001151    -0.007342   -0.005697   -0.006275  -0.002337 
      30         FOCE    -0.002697  0.00215   -0.0007652  -0.0003618  0.000001674  -0.0002712  0.00001258  -0.00347   -0.0002455

# hyperion.nonmem-summary print fails gracefully for ill-formatted comments

    Code
      print(mod_sum)
    Message
      
      
      -- Model Summary: run-err ------------------------------------------------------
      Problem: Base one-compartment oral absorption model created from pharos see
      run004_metadata.json for details.
      Records: 240 | Observations: 210 | Subjects: 30
      Final OFV: -103.3
      
      -- Estimation Methods --
      
      * First Order Conditional Estimation with Interaction
      
      -- Heuristic Checks --
      
      [OK] Minimization Successful
      [!] Covariance Step Not Run
      [!] Eigenvalue Check Not Available
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

