$PROBLEM Base one-compartment oral absorption model created from pharos see run002a_metadata.json for details.

$INPUT ID TIME EVID AMT CMT DV MDV WT SEX

$DATA ../../data/derived/onecmpt-oral-30ind.csv IGNORE=@

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
(0, 1.3174)     ;TVCL (L/hr)
(0, 37.0699)    ;TVV (L)
(0, 1.1869)     ;TVKA (1/hr)

$OMEGA
0.13906     ;OM1 TVCL :EXP
0.14167     ;OM2 TVV :EXP
0.112       ;OM3 TVKA :EXP

$SIGMA
0.039616    ;SIG1 Proportional error (variance, 20% CV)
0.0068      ;SIG2 Additive error (variance, 0.01 mg/L SD)


$ESTIMATION METHOD=1 INTERACTION MAXEVAL=9999 PRINT=5 MSFO=run002a.msf
$COV PRINT=E MATRIX = R

$TABLE ID TIME DV PRED IPRED CWRES NPDE NOAPPEND NOPRINT ONEHEADER FILE=run002a.tab
$TABLE ID CL V KA ETAS(1:LAST) NOAPPEND NOPRINT ONEHEADER FIRSTONLY FILE=run002apar.tab
