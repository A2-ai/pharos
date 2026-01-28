$PROB Duplicate omega names with space-separated associated thetas

$INPUT ID TIME DV
$DATA dummy.csv IGNORE=@

$SUBROUTINES ADVAN1 TRANS2

$PK
CL = THETA(1) * EXP(ETA(1))
V  = THETA(2) * EXP(ETA(2))
KA = THETA(3) * EXP(ETA(3))

$ERROR
Y = F + EPS(1)

$THETA
(0, 1) ; CL
(0, 1) ; V
(0, 1) ; KA

$OMEGA
0.1 ; IIV CL :EXP
0.2 ; IIV V :EXP
0.3 ; IIV KA :EXP

$SIGMA
0.1 ; PROP
