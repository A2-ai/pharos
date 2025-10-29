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
(0, 1.3683)     ; TVCL (L/hr)
(0, 39.2925)    ; TVV (L)
(0, 1.1639)     ; TVKA (1/hr)

$OMEGA
0.13474     ; ETA(CL)
0.12337     ; ETA(V)
0.124       ; ETA(KA)

$SIGMA
0.036734    ; Proportional error (variance, 20% CV)
0.0064      ; Additive error (variance, 0.01 mg/L SD)


$ESTIMATION METHOD=1 INTERACTION MAXEVAL=9999 PRINT=5 MSFO=run002a.msf
$COV PRINT=E MATRIX = R

$TABLE ID TIME DV PRED IPRED CWRES NPDE NOAPPEND NOPRINT ONEHEADER FILE=run002a.tab
$TABLE ID CL V KA ETAS(1:LAST) NOAPPEND NOPRINT ONEHEADER FIRSTONLY FILE=run002apar.tab
