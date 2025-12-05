$PROBLEM Test extended theta syntax perturbation
$INPUT ID TIME DV
$DATA ../data.csv

$THETA (0,1)x2 ; Test 1: Basic x2
$THETA CL=(0,1)x2 ; Test 2: Named x2
$THETA NAMES(A,B,C) (0,1) (0,2) (0,3) ; Test 3: NAMES only
$THETA NAMES(A,B,C) (0,1)x3 ; Test 4: NAMES+x3
$THETA NAMES(A,B,C) CL=(0,1) (0,2) (0,3) ; Test 5: NAMES+named
$THETA NAMES(A,B,C) CL=(0,1)x2 (0,3) ; Test 6: NAMES+named x2
$THETA CL=(0,1) V=(0,2) (0,3) ; Test 7: Multi named
$THETA CL=(0,1)x2 (0,3) ; Test 8: Named x2+reg
