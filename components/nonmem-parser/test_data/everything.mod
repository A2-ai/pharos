$PROBLEM Some header #2
$INPUT ID TIME DV DOSE=AMT DV WT AGE SEX CREA DATE=DROP
$DATA "..\path with spaces\data.csv" IGNORE=#
    IGNORE=(DVID.EQ.3)
    IGN(ID.EQ.3.14)
    IGNORE=(DVID==3)
    IGNORE=(AGE>=18)
    IGNORE=(AGE>3,AGE<100)
    IGNORE=(AGE<=65)
    IGNORE=(TYPE/=0)
    IGNORE=(TYPE=1)
    IGNORE=(TYPE.EQN.1)
    IGNORE=(TYPE.NEN.2)
    IGNORE=(TYPE 1)
    RECORDS=200
    NULL=.
    LAST20=00
    TRANSLATE=(1,DV)
$SUBROUTINES ADVAN4 TOL=9 TRANS4 OTHER=fa.90
$ABBR REPLACE ETA(1)=ETA(3)
$ABBR REPLACE THETA(1)=THETA(5)
$ABBREVIATED COMRES=5 DERIV2=NO
$PK
    TVCL = THETA(1)*(WT/70)**THETA(6)
    CL   = TVCL * EXP(ETA(1))
    IF (CMT.EQ.4.AND.EVID.EQ.0) CL = CL * 2
    IF (AGE.GT.18) IF (SEX.EQ.1) CL = CL * 1.1   ; inline nested IF nests in the body, not ELSEIF
    IF (WT.GT.70) THEN
        V = 2
        IF (AGE.GT.65) THEN                      ; nested block IF stays in the body
            V = 3
        ENDIF
    ELSEIF (WT.LT.50) THEN                       ; real ELSEIF must survive
        V = 1
    ELSE
        IF (SEX.EQ.0) THEN                       ; ELSE body may itself contain an IF
            V = 4
        ENDIF
    ENDIF
    I = 0
    DOWHILE (I.LT.3)                             ; single-token DO WHILE spelling
        I = I + 1
    ENDDO
    J = 0
    DO WHILE (J.LT.2)                            ; split DO WHILE spelling
        J = J + 1
    ENDDO
    CALL                                         ; bare CALL lowers to Unknown, must not panic
$THETA 1.5 (0,0.5,2)    ; THETA(1) and THETA(2)
$THETA (-INF, 0.5, 10)  ; THETA with -INF lower bound
$THETA (0, 5, INF)      ; THETA with INF upper bound
$THETA (0, 0.1)x3       ; Three identical THETAs
$THETA CL=(0, 1.5, 10)  ; Named THETA
$THETA NAMES(KA, V2, Q) (0, 0.5) (0, 10) (0, 2)  ; NAMES syntax
$THETA NAMES(A, B, C)  (1, 1.1)x3       ; Three identical THETAs with NAMES
$THETA 2.3 FIX          ; THETA(3)
$THETA 0.8 0.25         ; THETA(4) and THETA(5)
$THETA
        (1,2.3 FIX)         ; THETA(6)
        (0.75 FIX)      ; THETA(7)
$THETA (-.1);
$OMEGA
0.04            ; ETA(1) - CL (diagonal)
$OMEGA .17
$OMEGA BLOCK(2) CORR
0.2             ; ETA(2) - V (SD)
0.3 0.15        ; ETA(2)-ETA(3) correlation, ETA(3) - KA (SD)

$OMEGA BLOCK(2) SAME    ; ETA(4), ETA(5) - same structure as above
$OMEGA BLOCK SAME    ; ETA(4), ETA(5) - same structure as above, no number for blocks means it's taking the one from before

$OMEGA BLOCK(2) FIX    ; ETA(7), ETA(8) - same structure as above
0.011207
0 0.338724

$OMEGA BLOCK(4)
0.1
0.01 0.1
(0.01)x2 0.1
(0.01)x3 0.1

$OMEGA STANDARD CORRELATION BLOCK(2)   ; flags before BLOCK
0.2
0.3 0.15

$SIGMA BLOCK(2)
0.01            ; Proportional error variance
0.002 0.25      ; Prop-Add covariance, Additive error variance

$SIGMA
1 FIXED
0.0360

$OMEGA ECL=.4               ; Label=Value syntax for diagonal
$OMEGA BLOCK(2)
EV1= 0.3
EQ=  0.01 0.35              ; Label=Value syntax in block

$OMEGA BLOCK(4) NAMES(ECL2,EV2,EQ2,EV3) VALUES(0.03,0.01)  ; NAMES with VALUES

$OMEGA BLOCK(3) CORR                   ; flag before values
0.2
0.3 0.15
0.1 0.05 0.3

$OMEGA BLOCK(3)                        ; flag after values
0.2
0.3 0.15
0.1 0.05 0.3 CORR

$OMEGA BLOCK(3)                        ; FIX interleaved among values
6.
.005 FIX .3
.001 .002 .1

$SIGMA PROP=0.04            ; Label=Value syntax for SIGMA
$SIGMA 0.01 0.02                    ; diagonal SIGMA

$EST METHOD=0 SLOW
$EST MAXEVAL=9999 METHOD=1 INTER PRINT=5 MSFO=../2.MSF
$EST MAXEVAL=9999 METHOD=1 INTER PRINT=5 FILE=run001.est
$ESTIMATION MAXEVAL=9999 METHOD=IMP INTER FILE=est
$TABLE ID TIME AMT EVID IPRED AGE WT MDV ETAS(1:LAST) ONEHEADER NOPRINT FILE=../2.TAB
$TABLE ID FILE=001.tab FORMAT=,1PE15.9
$TABLE ID TIME AMT EVID AGE WT MDV  KA,CL V2 V3 Q BETA HLBE
ONEHEADER NOPRINT FILE=../2par.TAB
$MSFI msfb.msf
$SIM (1) (2 NONPARAMETRIC) NSUB=1