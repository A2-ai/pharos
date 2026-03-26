$PROB RUN# Type2 Comment Test

$INPUT C SET ID JID TIME DV=CONC AMT=DOSE RATE EVID MDV CMT

$DATA example1.csv IGNORE=C

$THETA
 (0, 18.62)        ;1  CL/F [L/h]
 (0, 232.5)        ;2  VC/F [L]
 (0, 2.82)         ;3  KA (1/h) ;exp
 1 FIX             ;4  F1
 (0, 25)           ;THETA5 Q [L/h] :LOG
 (0, 25)           ;6  VP/F [L]
 (0, 0.23)         ;RUV :add

$OMEGA BLOCK(3)
0.1                ;11 IIV CL/F ;exp
0.01 0.1           ;22 IIV VC/F ;log
0.01               ;OMEGA(3,1) Corr CL/F,KA
0.01 0.1           ;33 IIV KA :LOG

$OMEGA
0.1                ;IIV F1 ;exp
0.1                ;IIV Q ;log

$SIGMA
 0.1               ;11 PropErr ;Proportional
 2                  ;22 AddErr [ng/mL] ;AddErr
