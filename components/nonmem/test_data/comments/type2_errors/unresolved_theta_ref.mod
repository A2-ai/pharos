$PROB Unresolved theta ref test

$INPUT ID TIME DV AMT

$DATA test.csv IGNORE=@

$THETA
 (0, 1.0)   ;CL [L/h]
 (0, 2.0)   ;V [L]

$OMEGA
0.1          ;IIV NONEXISTENT ;exp

$SIGMA
1 FIX        ;PropErr ;prop
