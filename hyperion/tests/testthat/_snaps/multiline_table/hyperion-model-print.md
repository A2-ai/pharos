# hyperion.nonmem-model print works

    Code
      mod
    Message
      
      -- NONMEM Model: multiline_table -----------------------------------------------
      Problem: Some header #2
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

