$PROB RUN# Example 1 (from samp5l)
$INPUT C SET ID JID TIME  DV=CONC AMT=DOSE RATE EVID MDV CMT CLX
V1X QX V2X SDIX SDSX
$DATA example1.csv IGNORE=C

$THETA
(0, 27.5)                  ;TVCL (L/h)
(0, 1.365)                 ;TVV (L)
(0, 1.1)                   ;TVKA (1/h)
(0, 0.254)                 ;TLAG (h)
(0, 0.23)                  ;RES ERR :stdev
(0, 0.006, 0.02941)        ;CRCL cov
(0, 19)                    ;CL/F (L/h)

$OMEGA
0.135                      ;OM1 TVCL :OMIT_TBL
0.1                        ;OM2 TVV :EXP
0.734                      ;OM3 TVKA :OMIT_TBL

$SIGMA
1 FIX                      ;SIG1 :OMIT_TBL