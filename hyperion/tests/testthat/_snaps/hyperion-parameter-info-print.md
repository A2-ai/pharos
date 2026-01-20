# hyperion_nonmem_parameter_info print works

    Code
      print(info)
    Message
      
      
      -- Model Parameter Info --------------------------------------------------------
    Output
       
    Message
      
      -- Theta Parameters --
      
    Output
       
      parameter  name  display  description  unit  parameterization
      ─────────  ────  ───────  ───────────  ────  ────────────────
      THETA1     TVCL  NA       NA           NA    NA              
      THETA2     TVV   NA       NA           NA    NA              
      THETA3     TVKA  NA       NA           NA    NA              
       
    Message
      -- Omega Parameters --
      
    Output
       
      parameter   name  display  description  parameterization  associated_theta
      ──────────  ────  ───────  ───────────  ────────────────  ────────────────
      OMEGA(1,1)  OM1   NA       NA           LogNormal         TVCL            
      OMEGA(2,2)  OM2   NA       NA           LogNormal         TVV             
      OMEGA(3,3)  OM3   NA       NA           LogNormal         TVKA            
       
    Message
      -- Sigma Parameters --
      
    Output
       
      parameter   name  display  description  parameterization
      ──────────  ────  ───────  ───────────  ────────────────
      SIGMA(1,1)  NA    NA       NA           NA              
      SIGMA(2,2)  NA    NA       NA           NA              

---

    Code
      print(info)
    Message
      
      
      -- Model Parameter Info --------------------------------------------------------
    Output
       
    Message
      
      -- Theta Parameters --
      
    Output
       
      parameter  name  display  description  unit  parameterization
      ─────────  ────  ───────  ───────────  ────  ────────────────
      THETA1     TVCL  NA       NA           L/hr  NA              
      THETA2     TVV   NA       NA           L     NA              
      THETA3     TVKA  NA       NA           1/hr  NA              
       
    Message
      -- Omega Parameters --
      
    Output
       
      parameter   name  display  description  parameterization  associated_theta
      ──────────  ────  ───────  ───────────  ────────────────  ────────────────
      OMEGA(1,1)  OM1   NA       NA           LogNormal         TVCL            
      OMEGA(2,2)  OM2   NA       NA           LogNormal         TVV             
      OMEGA(3,3)  OM3   NA       NA           LogNormal         TVKA            
       
    Message
      -- Sigma Parameters --
      
    Output
       
      parameter   name  display  description  parameterization
      ──────────  ────  ───────  ───────────  ────────────────
      SIGMA(1,1)  SIG1  NA       NA           NA              
      SIGMA(2,2)  SIG2  NA       NA           NA              

---

    Code
      print(info)
    Message
      
      
      -- Model Parameter Info --------------------------------------------------------
    Output
       
    Message
      
      -- Theta Parameters --
      
    Output
       
      parameter  name  display  description  unit  parameterization
      ─────────  ────  ───────  ───────────  ────  ────────────────
      THETA1     TVCL  NA       NA           L/hr  NA              
      THETA2     TVV   NA       NA           L     NA              
      THETA3     TVKA  NA       NA           1/hr  NA              
       
    Message
      -- Omega Parameters --
      
    Output
       
      parameter   name   display  description  parameterization  associated_theta
      ──────────  ─────  ───────  ───────────  ────────────────  ────────────────
      OMEGA(1,1)  OM1    NA       NA           LogNormal         TVCL            
      OMEGA(2,1)  OM1,2  NA       NA           LogNormal         TVCL, TVV       
      OMEGA(2,2)  OM2    NA       NA           LogNormal         TVV             
      OMEGA(3,3)  OM3    NA       NA           LogNormal         TVKA            
       
    Message
      -- Sigma Parameters --
      
    Output
       
      parameter   name  display  description  parameterization
      ──────────  ────  ───────  ───────────  ────────────────
      SIGMA(1,1)  SIG1  NA       NA           NA              
      SIGMA(2,2)  SIG2  NA       NA           NA              

---

    Code
      print(info)
    Message
      
      
      -- Model Parameter Info --------------------------------------------------------
    Output
       
    Message
      
      -- Theta Parameters --
      
    Output
       
      parameter  name      display  description  unit  parameterization
      ─────────  ────────  ───────  ───────────  ────  ────────────────
      THETA1     TVCL      NA       NA           L/hr  NA              
      THETA2     WT-on-CL  NA       NA           NA    NA              
      THETA3     TVV       NA       NA           L     NA              
      THETA4     TVKA      NA       NA           1/hr  NA              
       
    Message
      -- Omega Parameters --
      
    Output
       
      parameter   name   display  description  parameterization  associated_theta
      ──────────  ─────  ───────  ───────────  ────────────────  ────────────────
      OMEGA(1,1)  OM1    NA       NA           LogNormal         TVCL            
      OMEGA(2,1)  OM1,2  NA       NA           LogNormal         TVCL, TVV       
      OMEGA(2,2)  OM2    NA       NA           LogNormal         TVV             
      OMEGA(3,3)  OM3    NA       NA           LogNormal         TVKA            
       
    Message
      -- Sigma Parameters --
      
    Output
       
      parameter   name  display  description  parameterization
      ──────────  ────  ───────  ───────────  ────────────────
      SIGMA(1,1)  SIG1  NA       NA           NA              
      SIGMA(2,2)  SIG2  NA       NA           NA              

