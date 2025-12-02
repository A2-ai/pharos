$PROBLEM Test extended theta syntax perturbation
$INPUT ID TIME DV
$DATA ../data.csv
$THETA (0, 0.5)x3           ; Three identical thetas
$THETA CL=(0, 1.5, 10)      ; Named theta
$THETA NAMES(KA, V) (0, 0.3) (0, 50)  ; NAMES syntax
