$PROBLEM Base one-compartment oral absorption model created from pharos see run003b1_metadata.json for details.

$INPUT ID TIME EVID AMT CMT DV MDV WT SEX

$DATA /data/user-homes/matthews/Packages/hyperion/vignettes/test_data/data/derived/onecmpt-oral-30ind-cov.csv IGNORE=@

$SUBROUTINES ADVAN2 TRANS2

$PK
; Typical values
TVCL = THETA(1)
WTonCL = THETA(2)
TVV  = THETA(3)
TVKA = THETA(4)

; Individual parameters
CL = TVCL * EXP(ETA(1)) * (WT / 70) ** WTonCL
V  = TVV  * EXP(ETA(2))
KA = TVKA * EXP(ETA(3))

; NONMEM scaling
S2 = V

$ERROR
; Proportional + additive error model (matches mrgsolve)
IPRED = F
Y = IPRED * (1 + EPS(1)) + EPS(2)

$THETA
(0, 1.768615)     ;TVCL (L/hr)
(0.5, 5)           ;WT-on-CL ()
(0, 34.80303)    ;TVV (L)
(0, 0.939213)     ;TVKA (1/hr)

$OMEGA BLOCK(2)
0.103       ;OM1 TVCL :EXP
0.00009    ;OM1,2 TVCL:TVV :EXP
0.109       ;OM2 TVV :EXP
$OMEGA
0.099       ;OM3 TVKA :EXP

$SIGMA
0.03464956    ;SIG1 Proportional error (variance, 20% CV)
0.00602      ;SIG2 Additive error (variance, 0.01 mg/L SD)


$ESTIMATION METHOD=1 INTERACTION MAXEVAL=9999 PRINT=5 MSFO=run003b1.msf
$COV PRINT=E MATRIX = R

$TABLE ID TIME DV PRED IPRED CWRES NPDE NOAPPEND NOPRINT ONEHEADER FILE=run003b1.tab
$TABLE ID CL V KA ETAS(1:LAST) NOAPPEND NOPRINT ONEHEADER FIRSTONLY FILE=run003b1par.tab
