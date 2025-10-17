$PROBLEM Base one-compartment oral absorption model

$INPUT ID TIME EVID AMT CMT DV MDV WT SEX

$DATA /data/user-homes/matthews/Projects/nonmem-reference-scenarios/data/derived/onecmpt-oral-30ind.csv IGNORE=@

$SUBROUTINES ADVAN2 TRANS2

$PK
; Typical values
TVCL = THETA(1)
TVV  = THETA(2)
TVKA = THETA(3)

; Individual parameters
CL = TVCL * EXP(ETA(1))
V  = TVV  * EXP(ETA(2))
KA = TVKA * EXP(ETA(3))

; NONMEM scaling
S2 = V

$ERROR
; Proportional + additive error model (matches mrgsolve)
IPRED = F
Y = IPRED * (1 + EPS(1)) + EPS(2)

$THETA
(0, 1.24)     ; 1. TVCL (L/hr)
(0, 40.86)    ; 2. TVV (L)
(0, 1.24)     ; 3. TVKA (1/hr)

$OMEGA
0.131     ; 1. ETA(CL)
0.136     ; 2. ETA(V)
0.1       ; 3. ETA(KA)

$SIGMA
0.0364    ; 1. Proportional error (variance, 20% CV)
0.01      ; 2. Additive error (variance, 0.01 mg/L SD)


$ESTIMATION METHOD=1 INTERACTION MAXEVAL=9999 PRINT=5
$COV PRINT=E MATRIX = R

$TABLE ID TIME DV PRED IPRED CWRES NPDE NOAPPEND NOPRINT ONEHEADER FILE=run002.tab
$TABLE ID CL V KA ETAS(1:LAST) NOAPPEND NOPRINT ONEHEADER FIRSTONLY FILE=run002par.tab
