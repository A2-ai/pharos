$PROBLEM Some header #2
$INPUT ID TIME DV DOSE=AMT DV WT AGE SEX CREA DATE=DROP
$DATA ..\data.csv IGNORE=#
    IGNORE=(DVID.EQ.3)
    IGNORE=(ID.EQ.3.14)
    ACCEPT=(AGE.GT.3,SEX.EQ.1)
    RECORDS=200
    LAST20=00
$SUBROUTINES ADVAN4 TRANS4 OTHER=fa.90
$PK
    TVCL = THETA(1)*(WT/70)**THETA(6)
    CL   = TVCL * EXP(ETA(1))
$THETA 1.5 (0,0.5,2)    ; THETA(1) and THETA(2)
$THETA 2.3 FIX          ; THETA(3)
$THETA 0.8 0.25         ; THETA(4) and THETA(5)
$THETA
        (1,2.3 FIX)         ; THETA(6)
        (0.75 FIX)      ; THETA(7)

$OMEGA
0.04            ; ETA(1) - CL (diagonal)
$OMEGA .17
$OMEGA BLOCK(2) CORR
0.2             ; ETA(2) - V (SD)
0.3 0.15        ; ETA(2)-ETA(3) correlation, ETA(3) - KA (SD)

$OMEGA BLOCK(2) SAME    ; ETA(4), ETA(5) - same structure as above

$OMEGA
(0,0.1,1 FIX)   ; ETA(6) - fixed diagonal

$SIGMA BLOCK(2)
0.01            ; Proportional error variance
0.002 0.25      ; Prop-Add covariance, Additive error variance

$EST METHOD=0 SLOW
$EST MAXEVAL=9999 METHOD=1 INTER PRINT=5 MSFO=../2.MSF
$EST MAXEVAL=9999 METHOD=1 INTER PRINT=5 FILE=run001.est
$ESTIMATION MAXEVAL=9999 METHOD=IMP INTER FILE=est
$TABLE ID TIME AMT EVID IPRED AGE WT MDV ONEHEADER NOPRINT FILE=../2.TAB
$TABLE ID FILE=001.tab
$TABLE ID TIME AMT EVID AGE WT MDV  KA CL V2 V3 Q BETA HLBE
ONEHEADER NOPRINT FILE=../2par.TAB
$SIM ONLYSIM



Parameter     Initial   Lower   Upper  Fixed   Parametrization   Comment
-----------  --------  ------  ------  ------  ----------------  --------------------------------------------
OMEGA(1,1)       0.04      NA      NA  no                        ETA(1) - CL (diagonal)
OMEGA(2,2)       0.17      NA      NA  no
OMEGA(3,3)       0.20      NA      NA  no      Correlation       ETA(2) - V (SD)
OMEGA(4,3)       0.30      NA      NA  no      Correlation       ETA(2)-ETA(3) correlation, ETA(3) - KA (SD)
OMEGA(4,4)       0.15      NA      NA  no      Correlation       ETA(2)-ETA(3) correlation, ETA(3) - KA (SD)
OMEGA(5,5)       0.20      NA      NA  no      Correlation       ETA(4), ETA(5) - same structure as above
OMEGA(6,5)       0.30      NA      NA  no      Correlation       ETA(4), ETA(5) - same structure as above
OMEGA(6,6)       0.15      NA      NA  no      Correlation       ETA(4), ETA(5) - same structure as above
OMEGA(7,7)       0.10       0       1  yes                       ETA(6) - fixed diagonal
