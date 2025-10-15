Tue 10/22/2024 
05:47 PM
;Model Desc: Interoccasion Variability
;Project Name: nm7examples
;Project ID: NO PROJECT DESCRIPTION

$PROB run# example7 (from ad1tr2_occ)
$INPUT C SET ID  TIME  AMT RATE EVID MDV CMT DV
$DATA example7.csv IGNORE=C

$SUBROUTINES ADVAN1 TRANS2

$PRIOR NWPRI NTHETA=2, NETA=5, NTHP=0, NETP=5, NPEXP=1

$PK
MU_1=THETA(1)
MU_2=THETA(2)
V=DEXP(MU_1+ETA(1))
CLB=DEXP(MU_2+ETA(2))
DCL1=DEXP(ETA(3))
DCL2=DEXP(ETA(4))
DCL3=DEXP(ETA(5))
S1=V
DCL=DCL1
IF(TIME.GE.5.0) DCL=DCL2
IF(TIME.GE.10.0) DCL=DCL3
CL=CLB*DCL
VC=V

$ERROR
IPRED=F
Y = F+F*EPS(1)

;Initial Thetas
$THETA
 2.0  ;[MU_1]
 2.0  ;[MU_2]

;Initial omegas
$OMEGA BLOCK(2)
 .3 ;[p]
 -.01  ;[f]
 .3 ;[p]
$OMEGA BLOCK(1)
 .1  ;[p]
$OMEGA BLOCK(1) SAME
$OMEGA  BLOCK(1) SAME

$SIGMA
 0.1 ;[p]

; Degrees of freedom for Prior Omega blocks
$THETA (2.0 FIXED) (1.0 FIXED)
; Prior Omegas
$OMEGA BLOCK(2)
 .14 FIX
 0.0 .125
$OMEGA BLOCK(1) .0164 FIX
$OMEGA BLOCK(1) SAME
$OMEGA  BLOCK(1) SAME

$EST METHOD=ITS INTERACTION FILE=example7.ext   NITER=10000 PRINT=5 NOABORT SIGL=8 CTYPE=3 CITER=10
 NOPRIOR=1 CALPHA=0.05 NSIG=2
$EST METHOD=SAEM INTERACTION NBURN=30000 NITER=500 SIGL=8 ISAMPLE=2 PRINT=10 SEED=1556678 CTYPE=3
 CITER=10 CALPHA=0.05 NOPRIOR=1
$EST METHOD=IMP  INTERACTION EONLY=1 NITER=4 ISAMPLE=3000 PRINT=1 SIGL=10 NOPRIOR=1 MAPITER=0 
$EST METHOD=BAYES INTERACTION FILE=example7.txt NBURN=10000 NITER=10000 PRINT=100 CTYPE=3 CITER=10
CALPHA=0.05 NOPRIOR=0
$EST METHOD=COND INTERACTION MAXEVAL=9999 NSIG=3 SIGL=10 PRINT=5 NOABORT NOPRIOR=1
     FILE=example7.ext
$COV MATRIX=R PRINT=E UNCONDITIONAL
  
NM-TRAN MESSAGES 
  
 WARNINGS AND ERRORS (IF ANY) FOR PROBLEM    1
             
 (WARNING  2) NM-TRAN INFERS THAT THE DATA ARE POPULATION.
  
Note: Analytical 2nd Derivatives are constructed in FSUBS but are never used.
      You may insert $ABBR DERIV2=NO after the first $PROB to save FSUBS construction and compilation time
  
  
License Registered to: NONMEM license (with RADAR5NM) for ICON Pharmacometrics Team
Expiration Date:    31 DEC 2030
Current Date:       22 OCT 2024
Days until program expires :2259
1NONLINEAR MIXED EFFECTS MODEL PROGRAM (NONMEM) VERSION 7.6.0 beta 4 (nm76b4)
 ORIGINALLY DEVELOPED BY STUART BEAL, LEWIS SHEINER, AND ALISON BOECKMANN
 CURRENT DEVELOPERS ARE ROBERT BAUER, ICON DEVELOPMENT SOLUTIONS,
 AND ALISON BOECKMANN. IMPLEMENTATION, EFFICIENCY, AND STANDARDIZATION
 PERFORMED BY NOUS INFOSYSTEMS.

 PROBLEM NO.:         1
 run# example7 (from ad1tr2_occ)
0DATA CHECKOUT RUN:              NO
 DATA SET LOCATED ON UNIT NO.:    2
 THIS UNIT TO BE REWOUND:        NO
 CREATE/ADD TO FDATA.csv:        YES
 NO. OF DATA RECS IN DATA SET:     4500
 NO. OF DATA ITEMS IN DATA SET:  10
 ID DATA ITEM IS DATA ITEM NO.:   3
 DEP VARIABLE IS DATA ITEM NO.:  10
 MDV DATA ITEM IS DATA ITEM NO.:  8
0INDICES PASSED TO SUBROUTINE PRED:
   7   4   5   6   0   0   9   0   0   0   0
0LABELS FOR DATA ITEMS:
 C SET ID TIME AMT RATE EVID MDV CMT DV
0FORMAT FOR DATA:
 (E2.0,E3.0,E4.0,E5.0,E4.0,4E2.0,E11.0)

 TOT. NO. OF OBS RECS:     3750
 TOT. NO. OF INDIVIDUALS:      250
0LENGTH OF THETA:   4
0DEFAULT THETA BOUNDARY TEST OMITTED:    NO
0OMEGA HAS BLOCK FORM:
  1
  1  1
  0  0  2
  0  0  0  2
  0  0  0  0  2
  0  0  0  0  0  3
  0  0  0  0  0  3  3
  0  0  0  0  0  0  0  4
  0  0  0  0  0  0  0  0  4
  0  0  0  0  0  0  0  0  0  4
0DEFAULT OMEGA BOUNDARY TEST OMITTED:    NO
0SIGMA HAS SIMPLE DIAGONAL FORM WITH DIMENSION:   1
0DEFAULT SIGMA BOUNDARY TEST OMITTED:    NO
0INITIAL ESTIMATE OF THETA:
 LOWER BOUND    INITIAL EST    UPPER BOUND
 -0.1000E+07     0.2000E+01     0.1000E+07
 -0.1000E+07     0.2000E+01     0.1000E+07
  0.2000E+01     0.2000E+01     0.2000E+01
  0.1000E+01     0.1000E+01     0.1000E+01
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.3000E+00
                 -0.1000E-01   0.3000E+00
        2                                                                                   NO
                  0.1000E+00
        3                                                                                  YES
                  0.1400E+00
                  0.0000E+00   0.1250E+00
        4                                                                                  YES
                  0.1640E-01
0INITIAL ESTIMATE OF SIGMA:
 0.1000E+00
0COVARIANCE STEP OMITTED:        NO
 R MATRIX SUBSTITUTED:          YES
 S MATRIX SUBSTITUTED:           NO
 EIGENVLS. PRINTED:             YES
 COMPRESSED FORMAT:              NO
 GRADIENT METHOD USED:     NOSLOW
 SIGDIGITS ETAHAT (SIGLO):                  -1
 SIGDIGITS GRADIENTS (SIGL):                -1
 EXCLUDE COV FOR FOCE (NOFCOV):              NO
 Cholesky Transposition of R Matrix (CHOLROFF):0
 KNUTHSUMOFF:                                -1
 RESUME COV ANALYSIS (RESUME):               NO
 SIR SAMPLE SIZE (SIRSAMPLE):
 NON-LINEARLY TRANSFORM THETAS DURING COV (THBND): 1
 PRECONDTIONING CYCLES (PRECOND):        0
 PRECONDTIONING TYPES (PRECONDS):        TOS
 FORCED PRECONDTIONING CYCLES (PFCOND):0
 PRECONDTIONING TYPE (PRETYPE):        0
 FORCED POS. DEFINITE SETTING DURING PRECONDITIONING: (FPOSDEF):0
 SIMPLE POS. DEFINITE SETTING: (POSDEF):-1
0
 PRIOR SUBROUTINE USER-SUPPLIED
1DOUBLE PRECISION PREDPP VERSION 7.6.0 beta 4 (nm76b4)

 ONE COMPARTMENT MODEL (ADVAN1)
0MAXIMUM NO. OF BASIC PK PARAMETERS:   2
0BASIC PK PARAMETERS (AFTER TRANSLATION):
   ELIMINATION RATE (K) IS BASIC PK PARAMETER NO.:  1

 TRANSLATOR WILL CONVERT PARAMETERS
 CLEARANCE (CL) AND VOLUME (V) TO K (TRANS2)
0COMPARTMENT ATTRIBUTES
 COMPT. NO.   FUNCTION   INITIAL    ON/OFF      DOSE      DEFAULT    DEFAULT
                         STATUS     ALLOWED    ALLOWED    FOR DOSE   FOR OBS.
    1         CENTRAL      ON         NO         YES        YES        YES
    2         OUTPUT       OFF        YES        NO         NO         NO
1
 ADDITIONAL PK PARAMETERS - ASSIGNMENT OF ROWS IN GG
 COMPT. NO.                             INDICES
              SCALE      BIOAVAIL.   ZERO-ORDER  ZERO-ORDER  ABSORB
                         FRACTION    RATE        DURATION    LAG
    1            3           *           *           *           *
    2            *           -           -           -           -
             - PARAMETER IS NOT ALLOWED FOR THIS MODEL
             * PARAMETER IS NOT SUPPLIED BY PK SUBROUTINE;
               WILL DEFAULT TO ONE IF APPLICABLE
0DATA ITEM INDICES USED BY PRED ARE:
   EVENT ID DATA ITEM IS DATA ITEM NO.:      7
   TIME DATA ITEM IS DATA ITEM NO.:          4
   DOSE AMOUNT DATA ITEM IS DATA ITEM NO.:   5
   DOSE RATE DATA ITEM IS DATA ITEM NO.:     6
   COMPT. NO. DATA ITEM IS DATA ITEM NO.:    9

0PK SUBROUTINE CALLED WITH EVERY EVENT RECORD.
 PK SUBROUTINE NOT CALLED AT NONEVENT (ADDITIONAL OR LAGGED) DOSE TIMES.
0ERROR SUBROUTINE CALLED WITH EVERY EVENT RECORD.
1
 
 
 #TBLN:      1
 #METH: Iterative Two Stage (No Prior)
 
 ESTIMATION STEP OMITTED:                 NO
 SHRINK INFO WITH EVALUATION (EVALSHRINK) NO
 ANALYSIS TYPE:                           POPULATION
 NUMBER OF SADDLE POINT RESET ITERATIONS:      0
 GRADIENT METHOD USED:               NOSLOW
 CONDITIONAL ESTIMATES USED:              YES
 CENTERED ETA:                            NO
 EPS-ETA INTERACTION:                     YES
 LAPLACIAN OBJ. FUNC.:                    NO
 NO. OF FUNCT. EVALS. ALLOWED:            1224
 NO. OF SIG. FIGURES REQUIRED:            2
 INTERMEDIATE PRINTOUT:                   YES
 ESTIMATE OUTPUT TO MSF:                  NO
 ABORT WITH PRED EXIT CODE 1:             NO
 IND. OBJ. FUNC. VALUES SORTED:           NO
 NUMERICAL DERIVATIVE
       FILE REQUEST (NUMDER):               NONE
 MAP (ETAHAT) ESTIMATION METHOD (OPTMAP):   0
 ETA HESSIAN EVALUATION METHOD (ETADER):    0
 INITIAL ETA FOR MAP ESTIMATION (MCETA):    0
 SIGDIGITS FOR MAP ESTIMATION (SIGLO):      8
 GRADIENT SIGDIGITS OF
       FIXED EFFECTS PARAMETERS (SIGL):     8
 NOPRIOR SETTING (NOPRIOR):                 1
 NOCOV SETTING (NOCOV):                     OFF
 DERCONT SETTING (DERCONT):                 OFF
 FINAL ETA RE-EVALUATION (FNLETA):          1
 EXCLUDE NON-INFLUENTIAL (NON-INFL.) ETAS
       IN SHRINKAGE (ETASTYPE):             NO
 NON-INFL. ETA CORRECTION (NONINFETA):      0
 RAW OUTPUT FILE (FILE): example7.ext
 EXCLUDE TITLE (NOTITLE):                   NO
 EXCLUDE COLUMN LABELS (NOLABEL):           NO
 FORMAT FOR ADDITIONAL FILES (FORMAT):      S1PE12.5
 PARAMETER ORDER FOR OUTPUTS (ORDER):       TSOL
 KNUTHSUMOFF:                               0
 INCLUDE LNTWOPI:                           NO
 INCLUDE CONSTANT TERM TO PRIOR (PRIORC):   NO
 INCLUDE CONSTANT TERM TO OMEGA (ETA) (OLNTWOPI):NO
 EM OR BAYESIAN METHOD USED:                ITERATIVE TWO STAGE (ITS)
 MU MODELING PATTERN (MUM):
 GRADIENT/GIBBS PATTERN (GRD):
 AUTOMATIC SETTING FEATURE (AUTO):          0
 CONVERGENCE TYPE (CTYPE):                  3
 CONVERGENCE INTERVAL (CINTERVAL):          5
 CONVERGENCE ITERATIONS (CITER):            10
 CONVERGENCE ALPHA ERROR (CALPHA):          5.000000000000000E-02
 ITERATIONS (NITER):                        10000
 ANNEAL SETTING (CONSTRAIN):                 1

 
 THE FOLLOWING LABELS ARE EQUIVALENT
 PRED=PREDI
 RES=RESI
 WRES=WRESI
 IWRS=IWRESI
 IPRD=IPREDI
 IRS=IRESI
 
 EM/BAYES SETUP:
 THETAS THAT ARE MU MODELED:
   1   2
 THETAS THAT ARE SIGMA-LIKE:
 
 
 MONITORING OF SEARCH:

 iteration            0  OBJ=  -5991.99597489474
 iteration            5  OBJ=  -14742.9108443554
 iteration           10  OBJ=  -17336.6193368356
 iteration           15  OBJ=  -19539.0015933220
 iteration           20  OBJ=  -19599.6329180867
 iteration           25  OBJ=  -19599.6329879729
 iteration           30  OBJ=  -19599.6329883940
 iteration           35  OBJ=  -19599.6329884612
 iteration           40  OBJ=  -19599.6329884588
 iteration           45  OBJ=  -19599.6329883658
 iteration           50  OBJ=  -19599.6329883763
 iteration           55  OBJ=  -19599.6329883718
 Convergence achieved
 
 #TERM:
 OPTIMIZATION WAS COMPLETED


 ETABAR IS THE ARITHMETIC MEAN OF THE ETA-ESTIMATES,
 AND THE P-VALUE IS GIVEN FOR THE NULL HYPOTHESIS THAT THE TRUE MEAN IS 0.
 
 ETABAR:         6.2675E-11  1.0171E-10 -6.6930E-03  2.6067E-03  4.0863E-03
 SE:             2.3551E-02  2.1839E-02  6.6354E-03  6.6172E-03  6.8549E-03
 N:                     250         250         250         250         250
 
 P VAL.:         1.0000E+00  1.0000E+00  3.1313E-01  6.9364E-01  5.5110E-01
 
 ETASHRINKSD(%)  1.3901E-01  2.1759E+00  1.8631E+01  1.8994E+01  1.6050E+01
 ETASHRINKVR(%)  2.7783E-01  4.3045E+00  3.3791E+01  3.4380E+01  2.9524E+01
 EBVSHRINKSD(%)  1.3901E-01  2.1759E+00  1.7842E+01  1.7940E+01  1.7861E+01
 EBVSHRINKVR(%)  2.7782E-01  4.3045E+00  3.2501E+01  3.2661E+01  3.2532E+01
 RELATIVEINF(%)  9.9432E+01  1.0000E-10  1.0000E-10  1.0000E-10  1.0000E-10
 EPSSHRINKSD(%)  1.4062E+01
 EPSSHRINKVR(%)  2.6146E+01
 
  
 TOTAL DATA POINTS NORMALLY DISTRIBUTED (N):         3750
 N*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    6892.03899903504     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -19599.6329883718     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -12707.5939893367     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 TOTAL EFFECTIVE ETAS (NIND*NETA):                          1250
  
 #TERE:
 Elapsed estimation  time in seconds:    29.73
 Elapsed covariance  time in seconds:     0.05
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 #OBJT:**************                        FINAL VALUE OF OBJECTIVE FUNCTION                       ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 





 #OBJV:********************************************        -19599.633       *********************************************
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 ********************                             FINAL PARAMETER ESTIMATE                           ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2     
 
         3.89E+00  3.68E+00
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4      ETA5     
 
 ETA1
+        1.39E-01
 
 ETA2
+       -8.04E-02  1.25E-01
 
 ETA3
+        0.00E+00  0.00E+00  1.67E-02
 
 ETA4
+        0.00E+00  0.00E+00  0.00E+00  1.67E-02
 
 ETA5
+        0.00E+00  0.00E+00  0.00E+00  0.00E+00  1.67E-02
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        2.50E-03
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4      ETA5     
 
 ETA1
+        3.73E-01
 
 ETA2
+       -6.11E-01  3.53E-01
 
 ETA3
+        0.00E+00  0.00E+00  1.29E-01
 
 ETA4
+        0.00E+00  0.00E+00  0.00E+00  1.29E-01
 
 ETA5
+        0.00E+00  0.00E+00  0.00E+00  0.00E+00  1.29E-01
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        5.00E-02
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 ********************                          STANDARD ERROR OF ESTIMATE (S)                        ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2     
 
         2.39E-02  2.32E-02
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4      ETA5     
 
 ETA1
+        1.26E-02
 
 ETA2
+        1.11E-02  1.31E-02
 
 ETA3
+        0.00E+00  0.00E+00  1.14E-03
 
 ETA4
+        0.00E+00  0.00E+00  0.00E+00  1.14E-03
 
 ETA5
+        0.00E+00  0.00E+00  0.00E+00  0.00E+00  1.14E-03
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        6.46E-05
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4      ETA5     
 
 ETA1
+        1.69E-02
 
 ETA2
+        4.73E-02  1.85E-02
 
 ETA3
+       ......... .........  4.41E-03
 
 ETA4
+       ......... ......... ......... .........
 
 ETA5
+       ......... ......... ......... ......... .........
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        6.46E-04
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 ********************                        COVARIANCE MATRIX OF ESTIMATE (S)                       ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      OM11      OM12      OM13      OM14      OM15      OM22      OM23      OM24      OM25      OM33  
             OM34      OM35      OM44      OM45      OM55      SG11  
 
 TH 1
+        5.70E-04
 
 TH 2
+       -3.33E-04  5.38E-04
 
 OM11
+        2.39E-05 -6.13E-06  1.58E-04
 
 OM12
+       -9.83E-07 -4.21E-06 -1.02E-04  1.22E-04
 
 OM13
+       ......... ......... ......... ......... .........
 
 OM14
+       ......... ......... ......... ......... ......... .........
 
 OM15
+       ......... ......... ......... ......... ......... ......... .........
 
 OM22
+       -8.01E-07 -7.99E-06  7.40E-05 -1.07E-04  0.00E+00  0.00E+00  0.00E+00  1.71E-04
 
 OM23
+       ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM24
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM25
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM33
+        2.85E-06 -4.31E-06  1.40E-06 -6.86E-07  0.00E+00  0.00E+00  0.00E+00  1.23E-06  0.00E+00  0.00E+00  0.00E+00  1.30E-06
 
 OM34
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         .........
 
 OM35
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... .........
 
 OM44
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... .........
 
 OM45
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... .........
 
 OM55
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... .........
 
 SG11
+       -1.59E-08  2.56E-08  1.89E-08 -1.92E-08  0.00E+00  0.00E+00  0.00E+00 -2.55E-08  0.00E+00  0.00E+00  0.00E+00 -3.78E-09
          0.00E+00  0.00E+00  0.00E+00  0.00E+00  0.00E+00  4.17E-09
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 ********************                        CORRELATION MATRIX OF ESTIMATE (S)                      ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      OM11      OM12      OM13      OM14      OM15      OM22      OM23      OM24      OM25      OM33  
             OM34      OM35      OM44      OM45      OM55      SG11  
 
 TH 1
+        2.39E-02
 
 TH 2
+       -6.00E-01  2.32E-02
 
 OM11
+        7.96E-02 -2.10E-02  1.26E-02
 
 OM12
+       -3.72E-03 -1.64E-02 -7.37E-01  1.11E-02
 
 OM13
+       ......... ......... ......... ......... .........
 
 OM14
+       ......... ......... ......... ......... ......... .........
 
 OM15
+       ......... ......... ......... ......... ......... ......... .........
 
 OM22
+       -2.56E-03 -2.63E-02  4.50E-01 -7.39E-01  0.00E+00  0.00E+00  0.00E+00  1.31E-02
 
 OM23
+       ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM24
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM25
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM33
+        1.05E-01 -1.63E-01  9.75E-02 -5.44E-02  0.00E+00  0.00E+00  0.00E+00  8.23E-02  0.00E+00  0.00E+00  0.00E+00  1.14E-03
 
 OM34
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         .........
 
 OM35
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... .........
 
 OM44
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... .........
 
 OM45
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... .........
 
 OM55
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... .........
 
 SG11
+       -1.03E-02  1.70E-02  2.33E-02 -2.69E-02  0.00E+00  0.00E+00  0.00E+00 -3.02E-02  0.00E+00  0.00E+00  0.00E+00 -5.14E-02
          0.00E+00  0.00E+00  0.00E+00  0.00E+00  0.00E+00  6.46E-05
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 ********************                    INVERSE COVARIANCE MATRIX OF ESTIMATE (S)                   ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      OM11      OM12      OM13      OM14      OM15      OM22      OM23      OM24      OM25      OM33  
             OM34      OM35      OM44      OM45      OM55      SG11  
 
 TH 1
+        2.78E+03
 
 TH 2
+        1.71E+03  2.97E+03
 
 OM11
+       -6.17E+02 -1.29E+02  1.48E+04
 
 OM12
+       -2.66E+02  3.98E+02  1.50E+04  3.34E+04
 
 OM13
+       ......... ......... ......... ......... .........
 
 OM14
+       ......... ......... ......... ......... ......... .........
 
 OM15
+       ......... ......... ......... ......... ......... ......... .........
 
 OM22
+        1.95E+02  4.08E+02  3.04E+03  1.45E+04  0.00E+00  0.00E+00  0.00E+00  1.37E+04
 
 OM23
+       ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM24
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM25
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM33
+       -2.31E+01  2.02E+03 -3.31E+03 -3.28E+03  0.00E+00  0.00E+00  0.00E+00 -2.41E+03  0.00E+00  0.00E+00  0.00E+00  8.93E+04
 
 OM34
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         .........
 
 OM35
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... .........
 
 OM44
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... .........
 
 OM45
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... .........
 
 OM55
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... .........
 
 SG11
+        2.81E+03 -1.26E+03  1.02E+04  1.63E+05  0.00E+00  0.00E+00  0.00E+00  1.29E+05  0.00E+00  0.00E+00  0.00E+00  2.15E+05
          0.00E+00  0.00E+00  0.00E+00  0.00E+00  0.00E+00  2.42E+08
 
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 ********************                    EIGENVALUES OF COR MATRIX OF ESTIMATE (S)                   ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

             1         2         3         4         5         6         7
 
         1.54E-01  3.84E-01  5.57E-01  9.15E-01  1.03E+00  1.65E+00  2.31E+00
 
1
 
 
 #TBLN:      2
 #METH: Stochastic Approximation Expectation-Maximization (No Prior)
 
 ESTIMATION STEP OMITTED:                 NO
 SHRINK INFO WITH EVALUATION (EVALSHRINK) NO
 ANALYSIS TYPE:                           POPULATION
 NUMBER OF SADDLE POINT RESET ITERATIONS:      0
 GRADIENT METHOD USED:               NOSLOW
 CONDITIONAL ESTIMATES USED:              YES
 CENTERED ETA:                            NO
 EPS-ETA INTERACTION:                     YES
 LAPLACIAN OBJ. FUNC.:                    NO
 NO. OF FUNCT. EVALS. ALLOWED:            1224
 NO. OF SIG. FIGURES REQUIRED:            2
 INTERMEDIATE PRINTOUT:                   YES
 ESTIMATE OUTPUT TO MSF:                  NO
 ABORT WITH PRED EXIT CODE 1:             NO
 IND. OBJ. FUNC. VALUES SORTED:           NO
 NUMERICAL DERIVATIVE
       FILE REQUEST (NUMDER):               NONE
 MAP (ETAHAT) ESTIMATION METHOD (OPTMAP):   0
 ETA HESSIAN EVALUATION METHOD (ETADER):    0
 INITIAL ETA FOR MAP ESTIMATION (MCETA):    0
 SIGDIGITS FOR MAP ESTIMATION (SIGLO):      8
 GRADIENT SIGDIGITS OF
       FIXED EFFECTS PARAMETERS (SIGL):     8
 NOPRIOR SETTING (NOPRIOR):                 1
 NOCOV SETTING (NOCOV):                     OFF
 DERCONT SETTING (DERCONT):                 OFF
 FINAL ETA RE-EVALUATION (FNLETA):          1
 EXCLUDE NON-INFLUENTIAL (NON-INFL.) ETAS
       IN SHRINKAGE (ETASTYPE):             NO
 NON-INFL. ETA CORRECTION (NONINFETA):      0
 RAW OUTPUT FILE (FILE): example7.ext
 EXCLUDE TITLE (NOTITLE):                   NO
 EXCLUDE COLUMN LABELS (NOLABEL):           NO
 FORMAT FOR ADDITIONAL FILES (FORMAT):      S1PE12.5
 PARAMETER ORDER FOR OUTPUTS (ORDER):       TSOL
 KNUTHSUMOFF:                               0
 INCLUDE LNTWOPI:                           NO
 INCLUDE CONSTANT TERM TO PRIOR (PRIORC):   NO
 INCLUDE CONSTANT TERM TO OMEGA (ETA) (OLNTWOPI):NO
 EM OR BAYESIAN METHOD USED:                STOCHASTIC APPROXIMATION EXPECTATION MAXIMIZATION (SAEM)
 MU MODELING PATTERN (MUM):
 GRADIENT/GIBBS PATTERN (GRD):
 AUTOMATIC SETTING FEATURE (AUTO):          0
 CONVERGENCE TYPE (CTYPE):                  3
 CONVERGENCE INTERVAL (CINTERVAL):          10
 CONVERGENCE ITERATIONS (CITER):            10
 CONVERGENCE ALPHA ERROR (CALPHA):          5.000000000000000E-02
 BURN-IN ITERATIONS (NBURN):                30000
 FIRST ITERATION FOR MAP (MAPITERS):          NO
 ITERATIONS (NITER):                        500
 ANNEAL SETTING (CONSTRAIN):                 1
 STARTING SEED FOR MC METHODS (SEED):       1556678
 MC SAMPLES PER SUBJECT (ISAMPLE):          2
 RANDOM SAMPLING METHOD (RANMETHOD):        3U
 EXPECTATION ONLY (EONLY):                  0
 PROPOSAL DENSITY SCALING RANGE
              (ISCALE_MIN, ISCALE_MAX):     1.000000000000000E-06   ,1000000.00000000
 SAMPLE ACCEPTANCE RATE (IACCEPT):          0.400000000000000
 METROPOLIS HASTINGS SAMPLING FOR INDIVIDUAL ETAS:
 SAMPLES FOR GLOBAL SEARCH KERNEL (ISAMPLE_M1):          2
 SAMPLES FOR NEIGHBOR SEARCH KERNEL (ISAMPLE_M1A):       0
 SAMPLES FOR MASS/IMP/POST. MATRIX SEARCH (ISAMPLE_M1B): 2
 SAMPLES FOR LOCAL SEARCH KERNEL (ISAMPLE_M2):           2
 SAMPLES FOR LOCAL UNIVARIATE KERNEL (ISAMPLE_M3):       2
 PWR. WT. MASS/IMP/POST MATRIX ACCUM. FOR ETAS (IKAPPA): 1.00000000000000
 MASS/IMP./POST. MATRIX REFRESH SETTING (MASSRESET):      -1

 
 THE FOLLOWING LABELS ARE EQUIVALENT
 PRED=PREDI
 RES=RESI
 WRES=WRESI
 IWRS=IWRESI
 IPRD=IPREDI
 IRS=IRESI
 
 EM/BAYES SETUP:
 THETAS THAT ARE MU MODELED:
   1   2
 THETAS THAT ARE SIGMA-LIKE:
 
 
 MONITORING OF SEARCH:

 Stochastic/Burn-in Mode
 iteration       -30000  SAEMOBJ=  -29257.3503218071
 iteration       -29990  SAEMOBJ=  -28820.7271590841
 iteration       -29980  SAEMOBJ=  -28790.2474671614
 iteration       -29970  SAEMOBJ=  -28717.9621001830
 iteration       -29960  SAEMOBJ=  -28734.1984459759
 iteration       -29950  SAEMOBJ=  -28686.2659660952
 iteration       -29940  SAEMOBJ=  -28689.4113814718
 iteration       -29930  SAEMOBJ=  -28671.4735405527
 iteration       -29920  SAEMOBJ=  -28655.6305239407
 iteration       -29910  SAEMOBJ=  -28638.7266673990
 iteration       -29900  SAEMOBJ=  -28625.4208175661
 iteration       -29890  SAEMOBJ=  -28626.9258198136
 iteration       -29880  SAEMOBJ=  -28528.5356505392
 iteration       -29870  SAEMOBJ=  -28602.0303259821
 iteration       -29860  SAEMOBJ=  -28601.3831274099
 iteration       -29850  SAEMOBJ=  -28571.4601962839
 iteration       -29840  SAEMOBJ=  -28553.8274358444
 iteration       -29830  SAEMOBJ=  -28610.6381894939
 iteration       -29820  SAEMOBJ=  -28603.5778369283
 iteration       -29810  SAEMOBJ=  -28572.6953398688
 iteration       -29800  SAEMOBJ=  -28565.7675928455
 iteration       -29790  SAEMOBJ=  -28606.0661831647
 iteration       -29780  SAEMOBJ=  -28562.4098815859
 iteration       -29770  SAEMOBJ=  -28541.8224010176
 iteration       -29760  SAEMOBJ=  -28585.4300501168
 iteration       -29750  SAEMOBJ=  -28618.1088871606
 iteration       -29740  SAEMOBJ=  -28541.1638099480
 iteration       -29730  SAEMOBJ=  -28514.1152407582
 iteration       -29720  SAEMOBJ=  -28538.4012923115
 iteration       -29710  SAEMOBJ=  -28519.7236197264
 Convergence achieved
 Elapsed burn-in time in seconds:    98.59
 Reduced Stochastic/Accumulation Mode
 iteration            0  SAEMOBJ=  -28452.8348872169
 iteration           10  SAEMOBJ=  -28659.7200851884
 iteration           20  SAEMOBJ=  -28673.8274465024
 iteration           30  SAEMOBJ=  -28682.6071207593
 iteration           40  SAEMOBJ=  -28682.9368131398
 iteration           50  SAEMOBJ=  -28683.4435909721
 iteration           60  SAEMOBJ=  -28687.6409840843
 iteration           70  SAEMOBJ=  -28688.3617864928
 iteration           80  SAEMOBJ=  -28688.1818933511
 iteration           90  SAEMOBJ=  -28687.0633604275
 iteration          100  SAEMOBJ=  -28687.7906338111
 iteration          110  SAEMOBJ=  -28688.0547672040
 iteration          120  SAEMOBJ=  -28686.8281487729
 iteration          130  SAEMOBJ=  -28686.5263091109
 iteration          140  SAEMOBJ=  -28685.1003868878
 iteration          150  SAEMOBJ=  -28684.1780437626
 iteration          160  SAEMOBJ=  -28683.3428840385
 iteration          170  SAEMOBJ=  -28682.7083146531
 iteration          180  SAEMOBJ=  -28682.1657409091
 iteration          190  SAEMOBJ=  -28681.9959187941
 iteration          200  SAEMOBJ=  -28682.3715672073
 iteration          210  SAEMOBJ=  -28681.2680743164
 iteration          220  SAEMOBJ=  -28681.1693337337
 iteration          230  SAEMOBJ=  -28681.3718223699
 iteration          240  SAEMOBJ=  -28680.9956922327
 iteration          250  SAEMOBJ=  -28681.0843147951
 iteration          260  SAEMOBJ=  -28681.2433921005
 iteration          270  SAEMOBJ=  -28681.1795353523
 iteration          280  SAEMOBJ=  -28680.6897463224
 iteration          290  SAEMOBJ=  -28680.3608424853
 iteration          300  SAEMOBJ=  -28680.1650977011
 iteration          310  SAEMOBJ=  -28680.4085137937
 iteration          320  SAEMOBJ=  -28680.1512324392
 iteration          330  SAEMOBJ=  -28679.9543576546
 iteration          340  SAEMOBJ=  -28679.8109737215
 iteration          350  SAEMOBJ=  -28679.4206341973
 iteration          360  SAEMOBJ=  -28678.7197901018
 iteration          370  SAEMOBJ=  -28678.5216393042
 iteration          380  SAEMOBJ=  -28678.5132284084
 iteration          390  SAEMOBJ=  -28678.1380914489
 iteration          400  SAEMOBJ=  -28677.8061391313
 iteration          410  SAEMOBJ=  -28677.5563445922
 iteration          420  SAEMOBJ=  -28677.5618596105
 iteration          430  SAEMOBJ=  -28677.0850378833
 iteration          440  SAEMOBJ=  -28676.6422848997
 iteration          450  SAEMOBJ=  -28676.1515265912
 iteration          460  SAEMOBJ=  -28675.8337597299
 iteration          470  SAEMOBJ=  -28675.8136534764
 iteration          480  SAEMOBJ=  -28675.7371099150
 iteration          490  SAEMOBJ=  -28675.2143545386
 iteration          500  SAEMOBJ=  -28675.0546339801
 
 #TERM:
 STOCHASTIC PORTION WAS COMPLETED
 REDUCED STOCHASTIC PORTION WAS COMPLETED

 ETABAR IS THE ARITHMETIC MEAN OF THE ETA-ESTIMATES,
 AND THE P-VALUE IS GIVEN FOR THE NULL HYPOTHESIS THAT THE TRUE MEAN IS 0.
 
 ETABAR:         2.1235E-06  1.0435E-05 -7.2368E-03  2.4283E-03  3.8949E-03
 SE:             2.3563E-02  2.2005E-02  6.6552E-03  6.5911E-03  6.9264E-03
 N:                     250         250         250         250         250
 
 P VAL.:         9.9993E-01  9.9962E-01  2.7687E-01  7.1256E-01  5.7389E-01
 
 ETASHRINKSD(%)  1.3730E-01  1.8541E+00  1.6717E+01  1.7692E+01  1.3473E+01
 ETASHRINKVR(%)  2.7442E-01  3.6738E+00  3.0640E+01  3.2254E+01  2.5131E+01
 EBVSHRINKSD(%)  1.3717E-01  1.8549E+00  1.5891E+01  1.6030E+01  1.5914E+01
 EBVSHRINKVR(%)  2.7414E-01  3.6753E+00  2.9256E+01  2.9490E+01  2.9295E+01
 RELATIVEINF(%)  9.9517E+01  6.5336E+01  2.7807E+01  2.7749E+01  2.7813E+01
 EPSSHRINKSD(%)  1.4008E+01
 EPSSHRINKVR(%)  2.6053E+01
 
  
 TOTAL DATA POINTS NORMALLY DISTRIBUTED (N):         3750
 N*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    6892.03899903504     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -28675.0546339801     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -21783.0156349451     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 TOTAL EFFECTIVE ETAS (NIND*NETA):                          1250
 NIND*NETA*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    2297.34633301168     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -28675.0546339801     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -26377.7083009685     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 #TERE:
 Elapsed estimation  time in seconds:   265.97
 Elapsed covariance  time in seconds:     0.04
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************           STOCHASTIC APPROXIMATION EXPECTATION-MAXIMIZATION (NO PRIOR)         ********************
 #OBJT:**************                        FINAL VALUE OF LIKELIHOOD FUNCTION                      ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 





 #OBJV:********************************************        -28675.055       *********************************************
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************           STOCHASTIC APPROXIMATION EXPECTATION-MAXIMIZATION (NO PRIOR)         ********************
 ********************                             FINAL PARAMETER ESTIMATE                           ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2     
 
         3.89E+00  3.68E+00
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4      ETA5     
 
 ETA1
+        1.39E-01
 
 ETA2
+       -8.13E-02  1.26E-01
 
 ETA3
+        0.00E+00  0.00E+00  1.60E-02
 
 ETA4
+        0.00E+00  0.00E+00  0.00E+00  1.60E-02
 
 ETA5
+        0.00E+00  0.00E+00  0.00E+00  0.00E+00  1.60E-02
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        2.50E-03
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4      ETA5     
 
 ETA1
+        3.73E-01
 
 ETA2
+       -6.15E-01  3.55E-01
 
 ETA3
+        0.00E+00  0.00E+00  1.27E-01
 
 ETA4
+        0.00E+00  0.00E+00  0.00E+00  1.27E-01
 
 ETA5
+        0.00E+00  0.00E+00  0.00E+00  0.00E+00  1.27E-01
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        5.00E-02
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************           STOCHASTIC APPROXIMATION EXPECTATION-MAXIMIZATION (NO PRIOR)         ********************
 ********************                          STANDARD ERROR OF ESTIMATE (S)                        ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2     
 
         2.38E-02  2.31E-02
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4      ETA5     
 
 ETA1
+        1.26E-02
 
 ETA2
+        1.11E-02  1.30E-02
 
 ETA3
+        0.00E+00  0.00E+00  1.05E-03
 
 ETA4
+        0.00E+00  0.00E+00  0.00E+00  1.05E-03
 
 ETA5
+        0.00E+00  0.00E+00  0.00E+00  0.00E+00  1.05E-03
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        6.39E-05
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4      ETA5     
 
 ETA1
+        1.69E-02
 
 ETA2
+        4.68E-02  1.84E-02
 
 ETA3
+       ......... .........  4.15E-03
 
 ETA4
+       ......... ......... ......... .........
 
 ETA5
+       ......... ......... ......... ......... .........
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        6.39E-04
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************           STOCHASTIC APPROXIMATION EXPECTATION-MAXIMIZATION (NO PRIOR)         ********************
 ********************                        COVARIANCE MATRIX OF ESTIMATE (S)                       ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      OM11      OM12      OM13      OM14      OM15      OM22      OM23      OM24      OM25      OM33  
             OM34      OM35      OM44      OM45      OM55      SG11  
 
 TH 1
+        5.67E-04
 
 TH 2
+       -3.32E-04  5.34E-04
 
 OM11
+        2.29E-05 -7.18E-06  1.59E-04
 
 OM12
+       -2.19E-06 -1.45E-06 -1.04E-04  1.23E-04
 
 OM13
+       ......... ......... ......... ......... .........
 
 OM14
+       ......... ......... ......... ......... ......... .........
 
 OM15
+       ......... ......... ......... ......... ......... ......... .........
 
 OM22
+        2.03E-07 -1.05E-05  7.57E-05 -1.07E-04  0.00E+00  0.00E+00  0.00E+00  1.70E-04
 
 OM23
+       ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM24
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM25
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM33
+        1.77E-06 -3.11E-06  1.68E-06 -1.10E-06  0.00E+00  0.00E+00  0.00E+00  1.63E-06  0.00E+00  0.00E+00  0.00E+00  1.10E-06
 
 OM34
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         .........
 
 OM35
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... .........
 
 OM44
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... .........
 
 OM45
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... .........
 
 OM55
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... .........
 
 SG11
+       -1.39E-08  1.97E-08  1.95E-08 -1.95E-08  0.00E+00  0.00E+00  0.00E+00 -2.64E-08  0.00E+00  0.00E+00  0.00E+00 -2.80E-09
          0.00E+00  0.00E+00  0.00E+00  0.00E+00  0.00E+00  4.09E-09
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************           STOCHASTIC APPROXIMATION EXPECTATION-MAXIMIZATION (NO PRIOR)         ********************
 ********************                        CORRELATION MATRIX OF ESTIMATE (S)                      ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      OM11      OM12      OM13      OM14      OM15      OM22      OM23      OM24      OM25      OM33  
             OM34      OM35      OM44      OM45      OM55      SG11  
 
 TH 1
+        2.38E-02
 
 TH 2
+       -6.03E-01  2.31E-02
 
 OM11
+        7.61E-02 -2.46E-02  1.26E-02
 
 OM12
+       -8.30E-03 -5.65E-03 -7.44E-01  1.11E-02
 
 OM13
+       ......... ......... ......... ......... .........
 
 OM14
+       ......... ......... ......... ......... ......... .........
 
 OM15
+       ......... ......... ......... ......... ......... ......... .........
 
 OM22
+        6.54E-04 -3.49E-02  4.61E-01 -7.44E-01  0.00E+00  0.00E+00  0.00E+00  1.30E-02
 
 OM23
+       ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM24
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM25
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM33
+        7.06E-02 -1.28E-01  1.27E-01 -9.42E-02  0.00E+00  0.00E+00  0.00E+00  1.19E-01  0.00E+00  0.00E+00  0.00E+00  1.05E-03
 
 OM34
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         .........
 
 OM35
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... .........
 
 OM44
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... .........
 
 OM45
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... .........
 
 OM55
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... .........
 
 SG11
+       -9.13E-03  1.33E-02  2.42E-02 -2.75E-02  0.00E+00  0.00E+00  0.00E+00 -3.17E-02  0.00E+00  0.00E+00  0.00E+00 -4.17E-02
          0.00E+00  0.00E+00  0.00E+00  0.00E+00  0.00E+00  6.39E-05
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************           STOCHASTIC APPROXIMATION EXPECTATION-MAXIMIZATION (NO PRIOR)         ********************
 ********************                    INVERSE COVARIANCE MATRIX OF ESTIMATE (S)                   ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      OM11      OM12      OM13      OM14      OM15      OM22      OM23      OM24      OM25      OM33  
             OM34      OM35      OM44      OM45      OM55      SG11  
 
 TH 1
+        2.80E+03
 
 TH 2
+        1.74E+03  3.00E+03
 
 OM11
+       -5.85E+02 -1.27E+02  1.50E+04
 
 OM12
+       -2.32E+02  3.84E+02  1.53E+04  3.40E+04
 
 OM13
+       ......... ......... ......... ......... .........
 
 OM14
+       ......... ......... ......... ......... ......... .........
 
 OM15
+       ......... ......... ......... ......... ......... ......... .........
 
 OM22
+        2.12E+02  4.30E+02  3.09E+03  1.49E+04  0.00E+00  0.00E+00  0.00E+00  1.41E+04
 
 OM23
+       ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM24
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM25
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM33
+        2.59E+02  1.86E+03 -3.85E+03 -3.16E+03  0.00E+00  0.00E+00  0.00E+00 -3.17E+03  0.00E+00  0.00E+00  0.00E+00  1.05E+05
 
 OM34
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         .........
 
 OM35
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... .........
 
 OM44
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... .........
 
 OM45
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... .........
 
 OM55
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... .........
 
 SG11
+        4.74E+03  5.47E+02  1.18E+04  1.76E+05  0.00E+00  0.00E+00  0.00E+00  1.39E+05  0.00E+00  0.00E+00  0.00E+00  1.90E+05
          0.00E+00  0.00E+00  0.00E+00  0.00E+00  0.00E+00  2.47E+08
 
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************           STOCHASTIC APPROXIMATION EXPECTATION-MAXIMIZATION (NO PRIOR)         ********************
 ********************                    EIGENVALUES OF COR MATRIX OF ESTIMATE (S)                   ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

             1         2         3         4         5         6         7
 
         1.50E-01  3.82E-01  5.47E-01  9.28E-01  1.02E+00  1.63E+00  2.34E+00
 
1
 
 
 #TBLN:      3
 #METH: Objective Function Evaluation by Importance Sampling (No Prior)
 
 ESTIMATION STEP OMITTED:                 NO
 SHRINK INFO WITH EVALUATION (EVALSHRINK) NO
 ANALYSIS TYPE:                           POPULATION
 NUMBER OF SADDLE POINT RESET ITERATIONS:      0
 GRADIENT METHOD USED:               NOSLOW
 CONDITIONAL ESTIMATES USED:              YES
 CENTERED ETA:                            NO
 EPS-ETA INTERACTION:                     YES
 LAPLACIAN OBJ. FUNC.:                    NO
 NO. OF FUNCT. EVALS. ALLOWED:            1224
 NO. OF SIG. FIGURES REQUIRED:            2
 INTERMEDIATE PRINTOUT:                   YES
 ESTIMATE OUTPUT TO MSF:                  NO
 ABORT WITH PRED EXIT CODE 1:             NO
 IND. OBJ. FUNC. VALUES SORTED:           NO
 NUMERICAL DERIVATIVE
       FILE REQUEST (NUMDER):               NONE
 MAP (ETAHAT) ESTIMATION METHOD (OPTMAP):   0
 ETA HESSIAN EVALUATION METHOD (ETADER):    0
 INITIAL ETA FOR MAP ESTIMATION (MCETA):    0
 SIGDIGITS FOR MAP ESTIMATION (SIGLO):      10
 GRADIENT SIGDIGITS OF
       FIXED EFFECTS PARAMETERS (SIGL):     10
 NOPRIOR SETTING (NOPRIOR):                 1
 NOCOV SETTING (NOCOV):                     OFF
 DERCONT SETTING (DERCONT):                 OFF
 FINAL ETA RE-EVALUATION (FNLETA):          1
 EXCLUDE NON-INFLUENTIAL (NON-INFL.) ETAS
       IN SHRINKAGE (ETASTYPE):             NO
 NON-INFL. ETA CORRECTION (NONINFETA):      0
 RAW OUTPUT FILE (FILE): example7.ext
 EXCLUDE TITLE (NOTITLE):                   NO
 EXCLUDE COLUMN LABELS (NOLABEL):           NO
 FORMAT FOR ADDITIONAL FILES (FORMAT):      S1PE12.5
 PARAMETER ORDER FOR OUTPUTS (ORDER):       TSOL
 KNUTHSUMOFF:                               0
 INCLUDE LNTWOPI:                           NO
 INCLUDE CONSTANT TERM TO PRIOR (PRIORC):   NO
 INCLUDE CONSTANT TERM TO OMEGA (ETA) (OLNTWOPI):NO
 EM OR BAYESIAN METHOD USED:                IMPORTANCE SAMPLING (IMP)
 MU MODELING PATTERN (MUM):
 GRADIENT/GIBBS PATTERN (GRD):
 AUTOMATIC SETTING FEATURE (AUTO):          0
 CONVERGENCE TYPE (CTYPE):                  3
 CONVERGENCE INTERVAL (CINTERVAL):          1
 CONVERGENCE ITERATIONS (CITER):            10
 CONVERGENCE ALPHA ERROR (CALPHA):          5.000000000000000E-02
 ITERATIONS (NITER):                        4
 ANNEAL SETTING (CONSTRAIN):                 1
 STARTING SEED FOR MC METHODS (SEED):       1556678
 MC SAMPLES PER SUBJECT (ISAMPLE):          3000
 RANDOM SAMPLING METHOD (RANMETHOD):        3U
 EXPECTATION ONLY (EONLY):                  1
 PROPOSAL DENSITY SCALING RANGE
              (ISCALE_MIN, ISCALE_MAX):     0.100000000000000       ,10.0000000000000
 SAMPLE ACCEPTANCE RATE (IACCEPT):          0.400000000000000
 LONG TAIL SAMPLE ACCEPT. RATE (IACCEPTL):   0.00000000000000
 T-DIST. PROPOSAL DENSITY (DF):             0
 NO. ITERATIONS FOR MAP (MAPITER):          0
 INTERVAL ITER. FOR MAP (MAPINTER):         0
 MAP COVARIANCE/MODE SETTING (MAPCOV):      1
 Gradient Quick Value (GRDQ):               0.00000000000000

 
 THE FOLLOWING LABELS ARE EQUIVALENT
 PRED=PREDI
 RES=RESI
 WRES=WRESI
 IWRS=IWRESI
 IPRD=IPREDI
 IRS=IRESI
 
 EM/BAYES SETUP:
 THETAS THAT ARE MU MODELED:
   1   2
 THETAS THAT ARE SIGMA-LIKE:
 
 
 MONITORING OF SEARCH:

 iteration            0  OBJ=  -19601.1941168593 eff.=    3435. Smpl.=    3000. Fit.= 0.97740
 iteration            1  OBJ=  -19600.6170408478 eff.=    1109. Smpl.=    3000. Fit.= 0.92531
 iteration            2  OBJ=  -19600.6824278290 eff.=    1179. Smpl.=    3000. Fit.= 0.92925
 iteration            3  OBJ=  -19600.7211524355 eff.=    1203. Smpl.=    3000. Fit.= 0.93054
 iteration            4  OBJ=  -19601.2998715247 eff.=    1202. Smpl.=    3000. Fit.= 0.93044
 
 #TERM:
 EXPECTATION ONLY PROCESS WAS NOT COMPLETED


 ETABAR IS THE ARITHMETIC MEAN OF THE ETA-ESTIMATES,
 AND THE P-VALUE IS GIVEN FOR THE NULL HYPOTHESIS THAT THE TRUE MEAN IS 0.
 
 ETABAR:        -1.3216E-04 -3.7096E-04 -6.9619E-03  2.7080E-03  4.1422E-03
 SE:             2.3578E-02  2.1927E-02  6.6341E-03  6.6188E-03  6.8489E-03
 N:                     250         250         250         250         250
 
 P VAL.:         9.9553E-01  9.8650E-01  2.9399E-01  6.8244E-01  5.4531E-01
 
 ETASHRINKSD(%)  7.6289E-02  2.2034E+00  1.6995E+01  1.7341E+01  1.4433E+01
 ETASHRINKVR(%)  1.5252E-01  4.3582E+00  3.1101E+01  3.1674E+01  2.6783E+01
 EBVSHRINKSD(%)  1.3900E-01  2.0824E+00  1.7931E+01  1.8036E+01  1.7978E+01
 EBVSHRINKVR(%)  2.7781E-01  4.1215E+00  3.2646E+01  3.2819E+01  3.2723E+01
 RELATIVEINF(%)  9.8889E+01  1.0000E-10  1.0000E-10  1.0000E-10  1.0000E-10
 EPSSHRINKSD(%)  1.4054E+01
 EPSSHRINKVR(%)  2.6133E+01
 
  
 TOTAL DATA POINTS NORMALLY DISTRIBUTED (N):         3750
 N*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    6892.03899903504     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -19601.2998715247     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -12709.2608724897     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 TOTAL EFFECTIVE ETAS (NIND*NETA):                          1250
  
 #TERE:
 Elapsed estimation  time in seconds:   105.30
 Elapsed covariance  time in seconds:    29.31
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************         OBJECTIVE FUNCTION EVALUATION BY IMPORTANCE SAMPLING (NO PRIOR)        ********************
 #OBJT:**************                        FINAL VALUE OF OBJECTIVE FUNCTION                       ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 





 #OBJV:********************************************        -19601.300       *********************************************
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************         OBJECTIVE FUNCTION EVALUATION BY IMPORTANCE SAMPLING (NO PRIOR)        ********************
 ********************                             FINAL PARAMETER ESTIMATE                           ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2     
 
         3.89E+00  3.68E+00
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4      ETA5     
 
 ETA1
+        1.39E-01
 
 ETA2
+       -8.13E-02  1.26E-01
 
 ETA3
+        0.00E+00  0.00E+00  1.60E-02
 
 ETA4
+        0.00E+00  0.00E+00  0.00E+00  1.60E-02
 
 ETA5
+        0.00E+00  0.00E+00  0.00E+00  0.00E+00  1.60E-02
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        2.50E-03
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4      ETA5     
 
 ETA1
+        3.73E-01
 
 ETA2
+       -6.15E-01  3.55E-01
 
 ETA3
+        0.00E+00  0.00E+00  1.27E-01
 
 ETA4
+        0.00E+00  0.00E+00  0.00E+00  1.27E-01
 
 ETA5
+        0.00E+00  0.00E+00  0.00E+00  0.00E+00  1.27E-01
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        5.00E-02
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************         OBJECTIVE FUNCTION EVALUATION BY IMPORTANCE SAMPLING (NO PRIOR)        ********************
 ********************                          STANDARD ERROR OF ESTIMATE (R)                        ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2     
 
         2.36E-02  2.29E-02
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4      ETA5     
 
 ETA1
+        1.25E-02
 
 ETA2
+        1.00E-02  1.18E-02
 
 ETA3
+        0.00E+00  0.00E+00  9.92E-04
 
 ETA4
+        0.00E+00  0.00E+00  0.00E+00  9.92E-04
 
 ETA5
+        0.00E+00  0.00E+00  0.00E+00  0.00E+00  9.92E-04
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        6.73E-05
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4      ETA5     
 
 ETA1
+        1.67E-02
 
 ETA2
+        4.09E-02  1.66E-02
 
 ETA3
+       ......... .........  3.92E-03
 
 ETA4
+       ......... ......... ......... .........
 
 ETA5
+       ......... ......... ......... ......... .........
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        6.73E-04
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************         OBJECTIVE FUNCTION EVALUATION BY IMPORTANCE SAMPLING (NO PRIOR)        ********************
 ********************                        COVARIANCE MATRIX OF ESTIMATE (R)                       ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      OM11      OM12      OM13      OM14      OM15      OM22      OM23      OM24      OM25      OM33  
             OM34      OM35      OM44      OM45      OM55      SG11  
 
 TH 1
+        5.58E-04
 
 TH 2
+       -3.24E-04  5.25E-04
 
 OM11
+        3.14E-07 -3.35E-07  1.55E-04
 
 OM12
+       -2.56E-08  6.45E-08 -9.08E-05  9.99E-05
 
 OM13
+       ......... ......... ......... ......... .........
 
 OM14
+       ......... ......... ......... ......... ......... .........
 
 OM15
+       ......... ......... ......... ......... ......... ......... .........
 
 OM22
+       -2.79E-08  4.30E-07  5.32E-05 -8.56E-05  0.00E+00  0.00E+00  0.00E+00  1.38E-04
 
 OM23
+       ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM24
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM25
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM33
+        2.92E-08 -1.06E-08  7.66E-08 -4.80E-08  0.00E+00  0.00E+00  0.00E+00 -2.34E-07  0.00E+00  0.00E+00  0.00E+00  9.84E-07
 
 OM34
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         .........
 
 OM35
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... .........
 
 OM44
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... .........
 
 OM45
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... .........
 
 OM55
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... .........
 
 SG11
+        4.64E-09  3.37E-09  1.79E-09 -2.15E-09  0.00E+00  0.00E+00  0.00E+00  2.51E-09  0.00E+00  0.00E+00  0.00E+00  3.95E-11
          0.00E+00  0.00E+00  0.00E+00  0.00E+00  0.00E+00  4.54E-09
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************         OBJECTIVE FUNCTION EVALUATION BY IMPORTANCE SAMPLING (NO PRIOR)        ********************
 ********************                        CORRELATION MATRIX OF ESTIMATE (R)                      ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      OM11      OM12      OM13      OM14      OM15      OM22      OM23      OM24      OM25      OM33  
             OM34      OM35      OM44      OM45      OM55      SG11  
 
 TH 1
+        2.36E-02
 
 TH 2
+       -5.99E-01  2.29E-02
 
 OM11
+        1.07E-03 -1.17E-03  1.25E-02
 
 OM12
+       -1.08E-04  2.82E-04 -7.29E-01  1.00E-02
 
 OM13
+       ......... ......... ......... ......... .........
 
 OM14
+       ......... ......... ......... ......... ......... .........
 
 OM15
+       ......... ......... ......... ......... ......... ......... .........
 
 OM22
+       -1.00E-04  1.60E-03  3.63E-01 -7.29E-01  0.00E+00  0.00E+00  0.00E+00  1.18E-02
 
 OM23
+       ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM24
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM25
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM33
+        1.24E-03 -4.65E-04  6.20E-03 -4.84E-03  0.00E+00  0.00E+00  0.00E+00 -2.00E-02  0.00E+00  0.00E+00  0.00E+00  9.92E-04
 
 OM34
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         .........
 
 OM35
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... .........
 
 OM44
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... .........
 
 OM45
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... .........
 
 OM55
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... .........
 
 SG11
+        2.92E-03  2.18E-03  2.14E-03 -3.19E-03  0.00E+00  0.00E+00  0.00E+00  3.17E-03  0.00E+00  0.00E+00  0.00E+00  5.91E-04
          0.00E+00  0.00E+00  0.00E+00  0.00E+00  0.00E+00  6.73E-05
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************         OBJECTIVE FUNCTION EVALUATION BY IMPORTANCE SAMPLING (NO PRIOR)        ********************
 ********************                    INVERSE COVARIANCE MATRIX OF ESTIMATE (R)                   ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      OM11      OM12      OM13      OM14      OM15      OM22      OM23      OM24      OM25      OM33  
             OM34      OM35      OM44      OM45      OM55      SG11  
 
 TH 1
+        2.79E+03
 
 TH 2
+        1.73E+03  2.97E+03
 
 OM11
+       -9.56E+00 -3.13E+00  1.58E+04
 
 OM12
+       -2.18E+01 -2.34E+01  1.95E+04  4.55E+04
 
 OM13
+       ......... ......... ......... ......... .........
 
 OM14
+       ......... ......... ......... ......... ......... .........
 
 OM15
+       ......... ......... ......... ......... ......... ......... .........
 
 OM22
+       -1.47E+01 -2.22E+01  6.02E+03  2.07E+04  0.00E+00  0.00E+00  0.00E+00  1.77E+04
 
 OM23
+       ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM24
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM25
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM33
+       -2.26E+01 -8.41E+00  3.84E+02  1.87E+03  0.00E+00  0.00E+00  0.00E+00  1.58E+03  0.00E+00  0.00E+00  0.00E+00  1.13E+05
 
 OM34
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         .........
 
 OM35
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... .........
 
 OM44
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... .........
 
 OM45
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... .........
 
 OM55
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... .........
 
 SG11
+       -4.14E+03 -3.97E+03 -3.40E+02  2.36E+03  0.00E+00  0.00E+00  0.00E+00 -2.41E+03  0.00E+00  0.00E+00  0.00E+00 -3.07E+03
          0.00E+00  0.00E+00  0.00E+00  0.00E+00  0.00E+00  2.21E+08
 
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************         OBJECTIVE FUNCTION EVALUATION BY IMPORTANCE SAMPLING (NO PRIOR)        ********************
 ********************                    EIGENVALUES OF COR MATRIX OF ESTIMATE (R)                   ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

             1         2         3         4         5         6         7
 
         1.35E-01  4.01E-01  6.36E-01  1.00E+00  1.00E+00  1.60E+00  2.23E+00
 
1
 
 
 #TBLN:      4
 #METH: MCMC Bayesian Analysis
 
 ESTIMATION STEP OMITTED:                 NO
 SHRINK INFO WITH EVALUATION (EVALSHRINK) NO
 ANALYSIS TYPE:                           POPULATION
 NUMBER OF SADDLE POINT RESET ITERATIONS:      0
 GRADIENT METHOD USED:               NOSLOW
 CONDITIONAL ESTIMATES USED:              YES
 CENTERED ETA:                            NO
 EPS-ETA INTERACTION:                     YES
 LAPLACIAN OBJ. FUNC.:                    NO
 NO. OF FUNCT. EVALS. ALLOWED:            1224
 NO. OF SIG. FIGURES REQUIRED:            2
 INTERMEDIATE PRINTOUT:                   YES
 ESTIMATE OUTPUT TO MSF:                  NO
 ABORT WITH PRED EXIT CODE 1:             NO
 IND. OBJ. FUNC. VALUES SORTED:           NO
 NUMERICAL DERIVATIVE
       FILE REQUEST (NUMDER):               NONE
 MAP (ETAHAT) ESTIMATION METHOD (OPTMAP):   0
 ETA HESSIAN EVALUATION METHOD (ETADER):    0
 INITIAL ETA FOR MAP ESTIMATION (MCETA):    0
 SIGDIGITS FOR MAP ESTIMATION (SIGLO):      10
 GRADIENT SIGDIGITS OF
       FIXED EFFECTS PARAMETERS (SIGL):     10
 NOPRIOR SETTING (NOPRIOR):                 0
 NOCOV SETTING (NOCOV):                     OFF
 DERCONT SETTING (DERCONT):                 OFF
 FINAL ETA RE-EVALUATION (FNLETA):          1
 EXCLUDE NON-INFLUENTIAL (NON-INFL.) ETAS
       IN SHRINKAGE (ETASTYPE):             NO
 NON-INFL. ETA CORRECTION (NONINFETA):      0
 RAW OUTPUT FILE (FILE): example7.txt
 EXCLUDE TITLE (NOTITLE):                   NO
 EXCLUDE COLUMN LABELS (NOLABEL):           NO
 FORMAT FOR ADDITIONAL FILES (FORMAT):      S1PE12.5
 PARAMETER ORDER FOR OUTPUTS (ORDER):       TSOL
 KNUTHSUMOFF:                               0
 INCLUDE LNTWOPI:                           NO
 INCLUDE CONSTANT TERM TO PRIOR (PRIORC):   NO
 INCLUDE CONSTANT TERM TO OMEGA (ETA) (OLNTWOPI):NO
 EM OR BAYESIAN METHOD USED:                MCMC BAYESIAN (BAYES)
 BAYES INDIVIDUAL PARAMETERS ONLY: NO
 MU MODELING PATTERN (MUM):
 GRADIENT/GIBBS PATTERN (GRD):
 AUTOMATIC SETTING FEATURE (AUTO):          0
 CONVERGENCE TYPE (CTYPE):                  3
 KEEP ITERATIONS (THIN):            1
 CONVERGENCE INTERVAL (CINTERVAL):          100
 CONVERGENCE ITERATIONS (CITER):            10
 CONVERGENCE ALPHA ERROR (CALPHA):          5.000000000000000E-02
 BURN-IN ITERATIONS (NBURN):                10000
 FIRST ITERATION FOR MAP (MAPITERS):          NO
 ITERATIONS (NITER):                        10000
 ANNEAL SETTING (CONSTRAIN):                 1
 STARTING SEED FOR MC METHODS (SEED):       1556678
 MC SAMPLES PER SUBJECT (ISAMPLE):          1
 RANDOM SAMPLING METHOD (RANMETHOD):        3U
 PROPOSAL DENSITY SCALING RANGE
              (ISCALE_MIN, ISCALE_MAX):     1.000000000000000E-06   ,1000000.00000000
 SAMPLE ACCEPTANCE RATE (IACCEPT):          0.400000000000000
 METROPOLIS HASTINGS SAMPLING FOR INDIVIDUAL ETAS:
 SAMPLES FOR GLOBAL SEARCH KERNEL (ISAMPLE_M1):          2
 SAMPLES FOR NEIGHBOR SEARCH KERNEL (ISAMPLE_M1A):       0
 SAMPLES FOR MASS/IMP/POST. MATRIX SEARCH (ISAMPLE_M1B): 2
 SAMPLES FOR LOCAL SEARCH KERNEL (ISAMPLE_M2):           2
 SAMPLES FOR LOCAL UNIVARIATE KERNEL (ISAMPLE_M3):       2
 PWR. WT. MASS/IMP/POST MATRIX ACCUM. FOR ETAS (IKAPPA): 1.00000000000000
 MASS/IMP./POST. MATRIX REFRESH SETTING (MASSRESET):      -1
 METROPOLIS HASTINGS POPULATION SAMPLING FOR NON-GIBBS
 SAMPLED THETAS AND SIGMAS:
 PROPOSAL DENSITY SCALING RANGE
              (PSCALE_MIN, PSCALE_MAX):   1.000000000000000E-02   ,1000.00000000000
 SAMPLE ACCEPTANCE RATE (PACCEPT):                       0.500000000000000
 SAMPLES FOR GLOBAL SEARCH KERNEL (PSAMPLE_M1):          1
 SAMPLES FOR LOCAL SEARCH KERNEL (PSAMPLE_M2):           -1
 SAMPLES FOR LOCAL UNIVARIATE KERNEL (PSAMPLE_M3):       1
 METROPOLIS HASTINGS POPULATION SAMPLING FOR NON-GIBBS
 SAMPLED OMEGAS:
 SAMPLE ACCEPTANCE RATE (OACCEPT):                       0.500000000000000
 SAMPLES FOR GLOBAL SEARCH KERNEL (OSAMPLE_M1):          -1
 SAMPLES FOR LOCAL SEARCH KERNEL (OSAMPLE_M2):           6
 SAMPLES FOR LOCAL UNIVARIATE SEARCH KERNEL (OSAMPLE_M3):6
 USER DEFINED PRIOR SETTING FOR THETAS: (TPU):        0.00000000000000
 WEIGHT FACTOR FOR STD PRIOR FOR SIGMAS (SVARF): -1.000000000000000+300

 
 THE FOLLOWING LABELS ARE EQUIVALENT
 PRED=PREDI
 RES=RESI
 WRES=WRESI
 IWRS=IWRESI
 IPRD=IPREDI
 IRS=IRESI
 
 EM/BAYES SETUP:
 THETAS THAT ARE MU MODELED:
   1   2
 THETAS THAT ARE GIBBS SAMPLED:
   1   2
 THETAS THAT ARE METROPOLIS-HASTINGS SAMPLED:
 
 SIGMAS THAT ARE GIBBS SAMPLED:
   1
 SIGMAS THAT ARE METROPOLIS-HASTINGS SAMPLED:
 
 OMEGAS ARE GIBBS SAMPLED
 
 MONITORING OF SEARCH:

 Burn-in Mode
 iteration       -10000 MCMCOBJ=   -28485.1166519870     
 iteration        -9900 MCMCOBJ=   -28212.5728496782     
 iteration        -9800 MCMCOBJ=   -28177.0565409857     
 iteration        -9700 MCMCOBJ=   -28287.8833707517     
 iteration        -9600 MCMCOBJ=   -28174.3219089297     
 iteration        -9500 MCMCOBJ=   -28183.8532283693     
 iteration        -9400 MCMCOBJ=   -28239.4740910823     
 iteration        -9300 MCMCOBJ=   -28259.8876869219     
 iteration        -9200 MCMCOBJ=   -28236.6229053111     
 iteration        -9100 MCMCOBJ=   -28153.5705477477     
 iteration        -9000 MCMCOBJ=   -28248.1700023273     
 Convergence achieved
 Elapsed burn-in time in seconds:   163.69
 Sampling Mode
 iteration            0 MCMCOBJ=   -28265.0477211797     
 iteration          100 MCMCOBJ=   -28198.5540749308     
 iteration          200 MCMCOBJ=   -28296.4883414999     
 iteration          300 MCMCOBJ=   -28166.2866249907     
 iteration          400 MCMCOBJ=   -28266.4973676573     
 iteration          500 MCMCOBJ=   -28197.5092516575     
 iteration          600 MCMCOBJ=   -28175.4560361835     
 iteration          700 MCMCOBJ=   -28242.3838421273     
 iteration          800 MCMCOBJ=   -28236.7927954159     
 iteration          900 MCMCOBJ=   -28155.3227609195     
 iteration         1000 MCMCOBJ=   -28303.9075164551     
 iteration         1100 MCMCOBJ=   -28160.4814524835     
 iteration         1200 MCMCOBJ=   -28272.2853242506     
 iteration         1300 MCMCOBJ=   -28194.6165596832     
 iteration         1400 MCMCOBJ=   -28233.4876448454     
 iteration         1500 MCMCOBJ=   -28230.8221055512     
 iteration         1600 MCMCOBJ=   -28248.9320407698     
 iteration         1700 MCMCOBJ=   -28233.1199036875     
 iteration         1800 MCMCOBJ=   -28337.1091960795     
 iteration         1900 MCMCOBJ=   -28205.0471705756     
 iteration         2000 MCMCOBJ=   -28197.9615237529     
 iteration         2100 MCMCOBJ=   -28198.5762731998     
 iteration         2200 MCMCOBJ=   -28191.7483958028     
 iteration         2300 MCMCOBJ=   -28197.5410520167     
 iteration         2400 MCMCOBJ=   -28140.5451211623     
 iteration         2500 MCMCOBJ=   -28339.8754873273     
 iteration         2600 MCMCOBJ=   -28212.0831361524     
 iteration         2700 MCMCOBJ=   -28241.2307784118     
 iteration         2800 MCMCOBJ=   -28228.7377616209     
 iteration         2900 MCMCOBJ=   -28172.9331092016     
 iteration         3000 MCMCOBJ=   -28269.6461709901     
 iteration         3100 MCMCOBJ=   -28262.5643057218     
 iteration         3200 MCMCOBJ=   -28245.9952294940     
 iteration         3300 MCMCOBJ=   -28224.8413156976     
 iteration         3400 MCMCOBJ=   -28258.2992517975     
 iteration         3500 MCMCOBJ=   -28232.5829008657     
 iteration         3600 MCMCOBJ=   -28137.5426093412     
 iteration         3700 MCMCOBJ=   -28241.2469291935     
 iteration         3800 MCMCOBJ=   -28193.9359244184     
 iteration         3900 MCMCOBJ=   -28231.8607093463     
 iteration         4000 MCMCOBJ=   -28153.4355737737     
 iteration         4100 MCMCOBJ=   -28157.4605690505     
 iteration         4200 MCMCOBJ=   -28265.1769209981     
 iteration         4300 MCMCOBJ=   -28297.0690729615     
 iteration         4400 MCMCOBJ=   -28249.8125990387     
 iteration         4500 MCMCOBJ=   -28263.7613515801     
 iteration         4600 MCMCOBJ=   -28178.4864088899     
 iteration         4700 MCMCOBJ=   -28267.4922421513     
 iteration         4800 MCMCOBJ=   -28164.8184716289     
 iteration         4900 MCMCOBJ=   -28201.1871444647     
 iteration         5000 MCMCOBJ=   -28161.8530821069     
 iteration         5100 MCMCOBJ=   -28172.1231904877     
 iteration         5200 MCMCOBJ=   -28200.7750039148     
 iteration         5300 MCMCOBJ=   -28281.6677965770     
 iteration         5400 MCMCOBJ=   -28288.9355509978     
 iteration         5500 MCMCOBJ=   -28339.5511112475     
 iteration         5600 MCMCOBJ=   -28163.4069645634     
 iteration         5700 MCMCOBJ=   -28151.1192818359     
 iteration         5800 MCMCOBJ=   -28194.1896619565     
 iteration         5900 MCMCOBJ=   -28251.3299011299     
 iteration         6000 MCMCOBJ=   -28254.5590964591     
 iteration         6100 MCMCOBJ=   -28223.6325862203     
 iteration         6200 MCMCOBJ=   -28231.8477037964     
 iteration         6300 MCMCOBJ=   -28253.6663998016     
 iteration         6400 MCMCOBJ=   -28250.0077519547     
 iteration         6500 MCMCOBJ=   -28151.7578631688     
 iteration         6600 MCMCOBJ=   -28292.7129958968     
 iteration         6700 MCMCOBJ=   -28136.6655703165     
 iteration         6800 MCMCOBJ=   -28164.3456763980     
 iteration         6900 MCMCOBJ=   -28335.4164948967     
 iteration         7000 MCMCOBJ=   -28366.4472978273     
 iteration         7100 MCMCOBJ=   -28242.0912609204     
 iteration         7200 MCMCOBJ=   -28188.6401754219     
 iteration         7300 MCMCOBJ=   -28216.3780582713     
 iteration         7400 MCMCOBJ=   -28140.7552460523     
 iteration         7500 MCMCOBJ=   -28269.2643459962     
 iteration         7600 MCMCOBJ=   -28184.4885566383     
 iteration         7700 MCMCOBJ=   -28262.1048242445     
 iteration         7800 MCMCOBJ=   -28097.7645228373     
 iteration         7900 MCMCOBJ=   -28257.4321107476     
 iteration         8000 MCMCOBJ=   -28240.1001259412     
 iteration         8100 MCMCOBJ=   -28284.5796231327     
 iteration         8200 MCMCOBJ=   -28165.1015115357     
 iteration         8300 MCMCOBJ=   -28269.8365465336     
 iteration         8400 MCMCOBJ=   -28223.0325809213     
 iteration         8500 MCMCOBJ=   -28220.4944981376     
 iteration         8600 MCMCOBJ=   -28319.8025579055     
 iteration         8700 MCMCOBJ=   -28170.3688304672     
 iteration         8800 MCMCOBJ=   -28186.0441052107     
 iteration         8900 MCMCOBJ=   -28220.2944583039     
 iteration         9000 MCMCOBJ=   -28212.2086918464     
 iteration         9100 MCMCOBJ=   -28173.1583325509     
 iteration         9200 MCMCOBJ=   -28164.2108913724     
 iteration         9300 MCMCOBJ=   -28274.7632251295     
 iteration         9400 MCMCOBJ=   -28181.7485405495     
 iteration         9500 MCMCOBJ=   -28190.3748293791     
 iteration         9600 MCMCOBJ=   -28174.8433075472     
 iteration         9700 MCMCOBJ=   -28268.6332258734     
 iteration         9800 MCMCOBJ=   -28183.1291866153     
 iteration         9900 MCMCOBJ=   -28261.5955403518     
 iteration        10000 MCMCOBJ=   -28207.4495609957     
 
 #TERM:
 BURN-IN WAS COMPLETED
 STATISTICAL PORTION WAS COMPLETED

 ETABAR IS THE ARITHMETIC MEAN OF THE ETA-ESTIMATES,
 AND THE P-VALUE IS GIVEN FOR THE NULL HYPOTHESIS THAT THE TRUE MEAN IS 0.
 
 ETABAR:         3.6021E-04 -6.7085E-05 -5.9889E-03  3.7958E-03  5.2379E-03
 SE:             2.3586E-02  2.1774E-02  6.6482E-03  6.6423E-03  6.9206E-03
 N:                     250         250         250         250         250
 
 P VAL.:         9.8781E-01  9.9754E-01  3.6768E-01  5.6769E-01  4.4913E-01
 
 ETASHRINKSD(%)  7.7694E-01  2.9047E+00  1.9680E+01  1.9829E+01  1.6429E+01
 ETASHRINKVR(%)  1.5478E+00  5.7250E+00  3.5487E+01  3.5725E+01  3.0160E+01
 EBVSHRINKSD(%)  1.3721E-01  2.2484E+00  1.8086E+01  1.8183E+01  1.8112E+01
 EBVSHRINKVR(%)  2.7424E-01  4.4462E+00  3.2901E+01  3.3060E+01  3.2944E+01
 RELATIVEINF(%)  9.9451E+01  1.0000E-10  1.0000E-10  1.0000E-10  1.0000E-10
 EPSSHRINKSD(%)  1.4163E+01
 EPSSHRINKVR(%)  2.6321E+01
 
  
 TOTAL DATA POINTS NORMALLY DISTRIBUTED (N):         3750
 N*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    6892.03899903504     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -28217.3547380622     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -21325.3157390272     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 TOTAL EFFECTIVE ETAS (NIND*NETA):                          1250
 NIND*NETA*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    2297.34633301168     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -28217.3547380622     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -25920.0084050505     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 PRIOR CONSTANT TO OBJECTIVE FUNCTION:    16.3289195783656     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -28217.3547380622     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -28201.0258184838     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 #TERE:
 Elapsed estimation  time in seconds:  1845.34
 Elapsed covariance  time in seconds:     0.00
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 #OBJT:**************                       AVERAGE VALUE OF LIKELIHOOD FUNCTION                     ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 





 #OBJV:********************************************        -28217.355       *********************************************
 #OBJS:********************************************            59.971 (STD) *********************************************
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 ********************                             FINAL PARAMETER ESTIMATE                           ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2     
 
         3.89E+00  3.68E+00
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4      ETA5     
 
 ETA1
+        1.41E-01
 
 ETA2
+       -8.07E-02  1.26E-01
 
 ETA3
+        0.00E+00  0.00E+00  1.72E-02
 
 ETA4
+        0.00E+00  0.00E+00  0.00E+00  1.72E-02
 
 ETA5
+        0.00E+00  0.00E+00  0.00E+00  0.00E+00  1.72E-02
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        2.50E-03
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4      ETA5     
 
 ETA1
+        3.75E-01
 
 ETA2
+       -6.04E-01  3.54E-01
 
 ETA3
+        0.00E+00  0.00E+00  1.31E-01
 
 ETA4
+        0.00E+00  0.00E+00  0.00E+00  1.31E-01
 
 ETA5
+        0.00E+00  0.00E+00  0.00E+00  0.00E+00  1.31E-01
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        5.00E-02
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 ********************                STANDARD ERROR OF ESTIMATE (From Sample Variance)               ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2     
 
         2.35E-02  2.26E-02
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4      ETA5     
 
 ETA1
+        1.28E-02
 
 ETA2
+        1.01E-02  1.19E-02
 
 ETA3
+        0.00E+00  0.00E+00  1.13E-03
 
 ETA4
+        0.00E+00  0.00E+00  0.00E+00  1.13E-03
 
 ETA5
+        0.00E+00  0.00E+00  0.00E+00  0.00E+00  1.13E-03
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        6.79E-05
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4      ETA5     
 
 ETA1
+        1.70E-02
 
 ETA2
+        4.17E-02  1.67E-02
 
 ETA3
+        0.00E+00  0.00E+00  4.29E-03
 
 ETA4
+        0.00E+00  0.00E+00  0.00E+00  4.29E-03
 
 ETA5
+        0.00E+00  0.00E+00  0.00E+00  0.00E+00  4.29E-03
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        6.79E-04
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 ********************               COVARIANCE MATRIX OF ESTIMATE (From Sample Variance)             ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      OM11      OM12      OM13      OM14      OM15      OM22      OM23      OM24      OM25      OM33  
             OM34      OM35      OM44      OM45      OM55      SG11  
 
 TH 1
+        5.52E-04
 
 TH 2
+       -3.04E-04  5.12E-04
 
 OM11
+       -2.72E-06  4.10E-06  1.65E-04
 
 OM12
+        7.71E-07 -2.96E-06 -9.47E-05  1.03E-04
 
 OM13
+       ......... ......... ......... ......... .........
 
 OM14
+       ......... ......... ......... ......... ......... .........
 
 OM15
+       ......... ......... ......... ......... ......... ......... .........
 
 OM22
+       -2.64E-07  3.25E-06  5.45E-05 -8.70E-05  0.00E+00  0.00E+00  0.00E+00  1.41E-04
 
 OM23
+       ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM24
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM25
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM33
+        2.45E-07 -3.77E-08  1.77E-07 -5.62E-08  0.00E+00  0.00E+00  0.00E+00 -5.33E-07  0.00E+00  0.00E+00  0.00E+00  1.27E-06
 
 OM34
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         .........
 
 OM35
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... .........
 
 OM44
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... .........
 
 OM45
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... .........
 
 OM55
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... .........
 
 SG11
+       -1.81E-08  1.41E-08  3.18E-09  1.11E-10  0.00E+00  0.00E+00  0.00E+00 -7.62E-09  0.00E+00  0.00E+00  0.00E+00 -4.90E-10
          0.00E+00  0.00E+00  0.00E+00  0.00E+00  0.00E+00  4.62E-09
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 ********************              CORRELATION MATRIX OF ESTIMATE (From Sample Variance)             ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      OM11      OM12      OM13      OM14      OM15      OM22      OM23      OM24      OM25      OM33  
             OM34      OM35      OM44      OM45      OM55      SG11  
 
 TH 1
+        2.35E-02
 
 TH 2
+       -5.73E-01  2.26E-02
 
 OM11
+       -9.02E-03  1.41E-02  1.28E-02
 
 OM12
+        3.23E-03 -1.29E-02 -7.26E-01  1.01E-02
 
 OM13
+       ......... ......... ......... ......... .........
 
 OM14
+       ......... ......... ......... ......... ......... .........
 
 OM15
+       ......... ......... ......... ......... ......... ......... .........
 
 OM22
+       -9.46E-04  1.21E-02  3.57E-01 -7.21E-01  0.00E+00  0.00E+00  0.00E+00  1.19E-02
 
 OM23
+       ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM24
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM25
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM33
+        9.25E-03 -1.48E-03  1.22E-02 -4.91E-03  0.00E+00  0.00E+00  0.00E+00 -3.97E-02  0.00E+00  0.00E+00  0.00E+00  1.13E-03
 
 OM34
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         .........
 
 OM35
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... .........
 
 OM44
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... .........
 
 OM45
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... .........
 
 OM55
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... .........
 
 SG11
+       -1.13E-02  9.20E-03  3.64E-03  1.61E-04  0.00E+00  0.00E+00  0.00E+00 -9.44E-03  0.00E+00  0.00E+00  0.00E+00 -6.39E-03
          0.00E+00  0.00E+00  0.00E+00  0.00E+00  0.00E+00  6.79E-05
 
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 ********************           EIGENVALUES OF COR MATRIX OF ESTIMATE (From Sample Variance)         ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

             1         2         3         4         5         6         7
 
         1.39E-01  4.27E-01  6.39E-01  9.96E-01  1.01E+00  1.57E+00  2.22E+00
 
1
 
 
 #TBLN:      5
 #METH: First Order Conditional Estimation with Interaction (No Prior)
 
 ESTIMATION STEP OMITTED:                 NO
 SHRINK INFO WITH EVALUATION (EVALSHRINK) NO
 ANALYSIS TYPE:                           POPULATION
 NUMBER OF SADDLE POINT RESET ITERATIONS:      0
 GRADIENT METHOD USED:               NOSLOW
 CONDITIONAL ESTIMATES USED:              YES
 CENTERED ETA:                            NO
 EPS-ETA INTERACTION:                     YES
 LAPLACIAN OBJ. FUNC.:                    NO
 NO. OF FUNCT. EVALS. ALLOWED:            9999
 NO. OF SIG. FIGURES REQUIRED:            3
 INTERMEDIATE PRINTOUT:                   YES
 ESTIMATE OUTPUT TO MSF:                  NO
 ABORT WITH PRED EXIT CODE 1:             NO
 IND. OBJ. FUNC. VALUES SORTED:           NO
 NUMERICAL DERIVATIVE
       FILE REQUEST (NUMDER):               NONE
 MAP (ETAHAT) ESTIMATION METHOD (OPTMAP):   0
 ETA HESSIAN EVALUATION METHOD (ETADER):    0
 INITIAL ETA FOR MAP ESTIMATION (MCETA):    0
 SIGDIGITS FOR MAP ESTIMATION (SIGLO):      10
 GRADIENT SIGDIGITS OF
       FIXED EFFECTS PARAMETERS (SIGL):     10
 NOPRIOR SETTING (NOPRIOR):                 1
 NOCOV SETTING (NOCOV):                     OFF
 DERCONT SETTING (DERCONT):                 OFF
 FINAL ETA RE-EVALUATION (FNLETA):          1
 EXCLUDE NON-INFLUENTIAL (NON-INFL.) ETAS
       IN SHRINKAGE (ETASTYPE):             NO
 NON-INFL. ETA CORRECTION (NONINFETA):      0
 RAW OUTPUT FILE (FILE): example7.ext
 EXCLUDE TITLE (NOTITLE):                   NO
 EXCLUDE COLUMN LABELS (NOLABEL):           NO
 FORMAT FOR ADDITIONAL FILES (FORMAT):      S1PE12.5
 PARAMETER ORDER FOR OUTPUTS (ORDER):       TSOL
 KNUTHSUMOFF:                               0
 INCLUDE LNTWOPI:                           NO
 INCLUDE CONSTANT TERM TO PRIOR (PRIORC):   NO
 INCLUDE CONSTANT TERM TO OMEGA (ETA) (OLNTWOPI):NO
 ADDITIONAL CONVERGENCE TEST (CTYPE=4)?:    NO
 EM OR BAYESIAN METHOD USED:                 NONE

 
 THE FOLLOWING LABELS ARE EQUIVALENT
 PRED=PREDI
 RES=RESI
 WRES=WRESI
 IWRS=IWRESI
 IPRD=IPREDI
 IRS=IRESI
 
 MONITORING OF SEARCH:

 
0ITERATION NO.:    0    OBJECTIVE VALUE:  -19599.3404430573        NO. OF FUNC. EVALS.:   5
 CUMULATIVE NO. OF FUNC. EVALS.:        5
 NPARAMETR:  3.8942E+00  3.6800E+00  1.4126E-01 -8.0653E-02  1.2572E-01  1.7183E-02  2.5046E-03
 PARAMETER:  1.0000E-01  1.0000E-01  1.0000E-01 -1.0000E-01  1.0000E-01  1.0000E-01  1.0000E-01
 GRADIENT:   1.2378E+04  1.5810E+04  1.0300E+01  3.5699E+01  9.3514E+00  2.6952E+01  1.5364E+00
 
0ITERATION NO.:    5    OBJECTIVE VALUE:  -19599.6216356547        NO. OF FUNC. EVALS.:  75
 CUMULATIVE NO. OF FUNC. EVALS.:       80
 NPARAMETR:  3.8949E+00  3.6813E+00  1.3890E-01 -8.1056E-02  1.2519E-01  1.6681E-02  2.5033E-03
 PARAMETER:  1.0002E-01  1.0003E-01  9.1585E-02 -1.0135E-01  8.8659E-02  8.5173E-02  9.9745E-02
 GRADIENT:  -1.0228E+04  1.2027E+04 -3.7972E+00 -2.2577E+01 -1.1050E+00 -2.1381E+00 -1.8381E+00
 
0ITERATION NO.:   10    OBJECTIVE VALUE:  -19599.6347430125        NO. OF FUNC. EVALS.: 102
 CUMULATIVE NO. OF FUNC. EVALS.:      182             RESET HESSIAN, TYPE I
 NPARAMETR:  3.8945E+00  3.6818E+00  1.3942E-01 -8.0641E-02  1.2472E-01  1.6717E-02  2.5047E-03
 PARAMETER:  1.0001E-01  1.0005E-01  9.3435E-02 -1.0064E-01  8.9866E-02  8.6235E-02  1.0001E-01
 GRADIENT:   1.2697E+04  1.6254E+04  4.8614E-01  1.7577E+00 -5.4022E-03  3.7421E-03  1.1533E+00
 
0ITERATION NO.:   15    OBJECTIVE VALUE:  -19599.6349196520        NO. OF FUNC. EVALS.:  79
 CUMULATIVE NO. OF FUNC. EVALS.:      261
 NPARAMETR:  3.8945E+00  3.6819E+00  1.3935E-01 -8.0643E-02  1.2475E-01  1.6717E-02  2.5044E-03
 PARAMETER:  1.0001E-01  1.0005E-01  9.3191E-02 -1.0067E-01  8.9872E-02  8.6235E-02  9.9964E-02
 GRADIENT:  -1.0242E+04  1.2101E+04  1.0558E-01  3.7499E-01  4.7934E-04  2.9131E-03  6.1973E-01
 
0ITERATION NO.:   16    OBJECTIVE VALUE:  -19599.6349196520        NO. OF FUNC. EVALS.:  19
 CUMULATIVE NO. OF FUNC. EVALS.:      280
 NPARAMETR:  3.8945E+00  3.6819E+00  1.3935E-01 -8.0643E-02  1.2475E-01  1.6717E-02  2.5044E-03
 PARAMETER:  1.0001E-01  1.0005E-01  9.3191E-02 -1.0067E-01  8.9872E-02  8.6235E-02  9.9964E-02
 GRADIENT:   5.3273E+00 -2.5999E+00  1.0681E-01  3.7472E-01  1.1661E-03  2.9073E-03  4.9343E-01
 
 #TERM:
0MINIMIZATION SUCCESSFUL
 HOWEVER, PROBLEMS OCCURRED WITH THE MINIMIZATION.
 REGARD THE RESULTS OF THE ESTIMATION STEP CAREFULLY, AND ACCEPT THEM ONLY
 AFTER CHECKING THAT THE COVARIANCE STEP PRODUCES REASONABLE OUTPUT.
 NO. OF FUNCTION EVALUATIONS USED:      280
 NO. OF SIG. DIGITS IN FINAL EST.:  3.0

 ETABAR IS THE ARITHMETIC MEAN OF THE ETA-ESTIMATES,
 AND THE P-VALUE IS GIVEN FOR THE NULL HYPOTHESIS THAT THE TRUE MEAN IS 0.
 
 ETABAR:        -6.2740E-04  1.0661E-04 -6.7122E-03  2.5885E-03  4.0688E-03
 SE:             2.3551E-02  2.1838E-02  6.6359E-03  6.6176E-03  6.8554E-03
 N:                     250         250         250         250         250
 
 P VAL.:         9.7875E-01  9.9610E-01  3.1178E-01  6.9568E-01  5.5283E-01
 
 ETASHRINKSD(%)  2.4530E-01  2.2382E+00  1.8683E+01  1.9048E+01  1.6106E+01
 ETASHRINKVR(%)  4.9000E-01  4.4263E+00  3.3876E+01  3.4468E+01  2.9618E+01
 EBVSHRINKSD(%)  1.3874E-01  2.1762E+00  1.7839E+01  1.7937E+01  1.7858E+01
 EBVSHRINKVR(%)  2.7729E-01  4.3051E+00  3.2496E+01  3.2656E+01  3.2527E+01
 RELATIVEINF(%)  1.0000E+02  0.0000E+00  0.0000E+00  0.0000E+00  0.0000E+00
 EPSSHRINKSD(%)  1.4071E+01
 EPSSHRINKVR(%)  2.6162E+01
 
  
 TOTAL DATA POINTS NORMALLY DISTRIBUTED (N):         3750
 N*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    6892.03899903504     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -19599.6349196520     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -12707.5959206169     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 TOTAL EFFECTIVE ETAS (NIND*NETA):                          1250
  
 #TERE:
 Elapsed estimation  time in seconds:    89.62
 Elapsed covariance  time in seconds:    15.41
 Elapsed postprocess time in seconds:     0.00
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************          FIRST ORDER CONDITIONAL ESTIMATION WITH INTERACTION (NO PRIOR)        ********************
 #OBJT:**************                       MINIMUM VALUE OF OBJECTIVE FUNCTION                      ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 





 #OBJV:********************************************        -19599.635       *********************************************
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************          FIRST ORDER CONDITIONAL ESTIMATION WITH INTERACTION (NO PRIOR)        ********************
 ********************                             FINAL PARAMETER ESTIMATE                           ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2     
 
         3.89E+00  3.68E+00
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4      ETA5     
 
 ETA1
+        1.39E-01
 
 ETA2
+       -8.06E-02  1.25E-01
 
 ETA3
+        0.00E+00  0.00E+00  1.67E-02
 
 ETA4
+        0.00E+00  0.00E+00  0.00E+00  1.67E-02
 
 ETA5
+        0.00E+00  0.00E+00  0.00E+00  0.00E+00  1.67E-02
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        2.50E-03
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4      ETA5     
 
 ETA1
+        3.73E-01
 
 ETA2
+       -6.12E-01  3.53E-01
 
 ETA3
+        0.00E+00  0.00E+00  1.29E-01
 
 ETA4
+        0.00E+00  0.00E+00  0.00E+00  1.29E-01
 
 ETA5
+        0.00E+00  0.00E+00  0.00E+00  0.00E+00  1.29E-01
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        5.00E-02
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************          FIRST ORDER CONDITIONAL ESTIMATION WITH INTERACTION (NO PRIOR)        ********************
 ********************                            STANDARD ERROR OF ESTIMATE                          ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2     
 
         2.36E-02  2.29E-02
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4      ETA5     
 
 ETA1
+        1.25E-02
 
 ETA2
+        9.95E-03  1.17E-02
 
 ETA3
+       ......... .........  1.08E-03
 
 ETA4
+       ......... ......... ......... .........
 
 ETA5
+       ......... ......... ......... ......... .........
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        6.76E-05
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4      ETA5     
 
 ETA1
+        1.67E-02
 
 ETA2
+        4.12E-02  1.65E-02
 
 ETA3
+       ......... .........  4.16E-03
 
 ETA4
+       ......... ......... ......... .........
 
 ETA5
+       ......... ......... ......... ......... .........
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        6.75E-04
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************          FIRST ORDER CONDITIONAL ESTIMATION WITH INTERACTION (NO PRIOR)        ********************
 ********************                          COVARIANCE MATRIX OF ESTIMATE                         ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      OM11      OM12      OM13      OM14      OM15      OM22      OM23      OM24      OM25      OM33  
             OM34      OM35      OM44      OM45      OM55      SG11  
 
 TH 1
+        5.59E-04
 
 TH 2
+       -3.22E-04  5.22E-04
 
 OM11
+        2.16E-07 -2.79E-07  1.56E-04
 
 OM12
+       -2.38E-07  1.62E-07 -9.00E-05  9.90E-05
 
 OM13
+       ......... ......... ......... ......... .........
 
 OM14
+       ......... ......... ......... ......... ......... .........
 
 OM15
+       ......... ......... ......... ......... ......... ......... .........
 
 OM22
+        2.53E-07 -4.17E-08  5.19E-05 -8.41E-05 ......... ......... .........  1.37E-04
 
 OM23
+       ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM24
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM25
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM33
+        2.89E-08 -8.86E-09  1.53E-08 -1.27E-08 ......... ......... ......... -3.76E-07 ......... ......... .........  1.16E-06
 
 OM34
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         .........
 
 OM35
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... .........
 
 OM44
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... .........
 
 OM45
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... .........
 
 OM55
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... .........
 
 SG11
+        4.40E-09  4.92E-09 -7.50E-10 -3.00E-10 ......... ......... ......... -5.00E-10 ......... ......... ......... -3.34E-10
         ......... ......... ......... ......... .........  4.57E-09
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************          FIRST ORDER CONDITIONAL ESTIMATION WITH INTERACTION (NO PRIOR)        ********************
 ********************                          CORRELATION MATRIX OF ESTIMATE                        ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      OM11      OM12      OM13      OM14      OM15      OM22      OM23      OM24      OM25      OM33  
             OM34      OM35      OM44      OM45      OM55      SG11  
 
 TH 1
+        2.36E-02
 
 TH 2
+       -5.96E-01  2.29E-02
 
 OM11
+        7.30E-04 -9.75E-04  1.25E-02
 
 OM12
+       -1.01E-03  7.13E-04 -7.24E-01  9.95E-03
 
 OM13
+       ......... ......... ......... ......... .........
 
 OM14
+       ......... ......... ......... ......... ......... .........
 
 OM15
+       ......... ......... ......... ......... ......... ......... .........
 
 OM22
+        9.15E-04 -1.56E-04  3.55E-01 -7.24E-01 ......... ......... .........  1.17E-02
 
 OM23
+       ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM24
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM25
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM33
+        1.14E-03 -3.61E-04  1.14E-03 -1.19E-03 ......... ......... ......... -2.99E-02 ......... ......... .........  1.08E-03
 
 OM34
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         .........
 
 OM35
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... .........
 
 OM44
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... .........
 
 OM45
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... .........
 
 OM55
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... .........
 
 SG11
+        2.76E-03  3.19E-03 -8.87E-04 -4.46E-04 ......... ......... ......... -6.33E-04 ......... ......... ......... -4.60E-03
         ......... ......... ......... ......... .........  6.76E-05
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************          FIRST ORDER CONDITIONAL ESTIMATION WITH INTERACTION (NO PRIOR)        ********************
 ********************                      INVERSE COVARIANCE MATRIX OF ESTIMATE                     ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      OM11      OM12      OM13      OM14      OM15      OM22      OM23      OM24      OM25      OM33  
             OM34      OM35      OM44      OM45      OM55      SG11  
 
 TH 1
+        2.77E+03
 
 TH 2
+        1.71E+03  2.97E+03
 
 OM11
+        1.51E+00  3.71E+00  1.54E+04
 
 OM12
+        1.41E+00 -1.20E+00  1.89E+04  4.46E+04
 
 OM13
+       ......... ......... ......... ......... .........
 
 OM14
+       ......... ......... ......... ......... ......... .........
 
 OM15
+       ......... ......... ......... ......... ......... ......... .........
 
 OM22
+       -4.49E+00 -4.49E+00  5.84E+03  2.03E+04 ......... ......... .........  1.76E+04
 
 OM23
+       ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM24
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM25
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 OM33
+       -5.89E+01 -2.28E+01  1.90E+03  6.83E+03 ......... ......... .........  5.87E+03 ......... ......... .........  8.67E+05
 
 OM34
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         .........
 
 OM35
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... .........
 
 OM44
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... .........
 
 OM45
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... .........
 
 OM55
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... .........
 
 SG11
+       -4.52E+03 -4.85E+03  4.54E+03  8.76E+03 ......... ......... .........  4.66E+03 ......... ......... .........  6.49E+04
         ......... ......... ......... ......... .........  2.19E+08
 
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************          FIRST ORDER CONDITIONAL ESTIMATION WITH INTERACTION (NO PRIOR)        ********************
 ********************                      EIGENVALUES OF COR MATRIX OF ESTIMATE                     ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

             1         2         3         4         5         6         7
 
         1.39E-01  4.04E-01  6.44E-01  9.96E-01  1.01E+00  1.60E+00  2.22E+00
 
 Elapsed finaloutput time in seconds:     0.02
 #CPUT: Total CPU Time in Seconds,     2377.469
Stop Time: 
Tue 10/22/2024 
06:27 PM
