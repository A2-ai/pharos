$PROBLEM Base one-compartment oral absorption model created from pharos see run004_metadata.json for details.

$INPUT ID TIME EVID AMT CMT DV MDV WT SEX

$DATA ../../data/derived/onecmpt-oral-30ind.csv IGNORE=@

$SUBROUTINES ADVAN2 TRANS2

$PK
; Typical values
TVCL = THETA1
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
(0, 1.17)    ;TVCL (L/hr)
(0, 41.98)   ;TVV (L)
(0, 1.24)   ;TVKA (1/hr)

$OMEGA
0.126   ;OM1  TVCL
0.133    ;OM2 TVV
0.1 FIX   ;OM3 TVKA

$SIGMA
0.0364    ; 1. Proportional error (variance, 20% CV)
0.01 FIX ; 2. Additive error (variance, 0.01 mg/L SD)

$ESTIMATION METHOD=1 INTERACTION MAXEVAL=9999 PRINT=5
$TABLE ID TIME DV PRED IPRED CWRES NPDE NOAPPEND NOPRINT ONEHEADER FILE=run004.tab
$TABLE ID CL V KA ETAS(1:LAST) NOAPPEND NOPRINT ONEHEADER FIRSTONLY FILE=run004par.tab
