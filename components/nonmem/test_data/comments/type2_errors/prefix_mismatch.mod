$PROB Prefix mismatch test

$INPUT ID TIME DV AMT

$DATA test.csv IGNORE=@

$THETA
 (0, 1.0)   ;THETA8 CL [L/h]
 (0, 2.0)   ;2 V [L]

$OMEGA BLOCK(2)
0.1          ;11 IIV CL ;exp
0.01 0.1     ;11 IIV V ;log

$SIGMA
1 FIX        ;SIGMA1 PropErr
