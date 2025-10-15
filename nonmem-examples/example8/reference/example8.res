Tue 10/22/2024 
02:30 PM
;Model Desc: Two compartment Model, Using ADVAN3, TRANS4
;Project Name: nm7examples
;Project ID: NO PROJECT DESCRIPTION

$PROB RUN# Example 8 (from samp5l)
$INPUT C SET ID JID TIME  DV=CONC AMT=DOSE RATE EVID MDV CMT 
       CLX V1X QX V2X SDIX SDSX
$DATA example8.csv IGNORE=C

$SUBROUTINES ADVAN3 TRANS4


$PK
include nonmem_reserved_general
; Request extra information for Bayesian analysis.  
; An extra call will then be made for accepted samples
BAYES_EXTRA_REQUEST=1
MU_1=THETA(1)
MU_2=THETA(2)
MU_3=THETA(3)
MU_4=THETA(4)
CL=DEXP(MU_1+ETA(1))
V1=DEXP(MU_2+ETA(2))
Q=DEXP(MU_3+ETA(3))
V2=DEXP(MU_4+ETA(4))
S1=V1
; When Bayes_extra=1, then this particular set of individual 
; parameters were "accepted" So you may record them if you wish
  IF(BAYES_EXTRA==1 .AND. ITER_REPORT>=0 .AND. TIME==0.0) THEN
"  WRITE(51,98) ITER_REPORT,ID,CL,V1,Q,V2
" 98 FORMAT(I12,1X,F14.0,4(1X,1PG12.5))
ENDIF

$ERROR
include nonmem_reserved_general
BAYES_EXTRA_REQUEST=1
Y = F + F*EPS(1)
IF(BAYES_EXTRA==1 .AND. ITER_REPORT>=0 ) THEN
" WRITE(52,97) ITER_REPORT,ID,TIME,F
" 97 FORMAT(I12,1X,F14.0,2(1X,1PG12.5))
ENDIF

; Initial values of THETA
$THETA 
(2.0) ;[LN(CL)]
(2.0) ;[LN(V1)]
(2.0) ;[LN(Q)]
(2.0) ;[LN(V2)]
;INITIAL values of OMEGA
$OMEGA BLOCK(4)
0.15   ;[P]
0.01  ;[F]
0.15   ;[P]
0.01  ;[F]
0.01  ;[F]
0.15   ;[P]
0.01  ;[F]
0.01  ;[F]
0.01  ;[F]
0.15   ;[P]
;Initial value of SIGMA
$SIGMA 
(0.6 )   ;[P]


$PRIOR NWPRI
; Prior information to the Thetas.
$THETAP (2.0 FIX)x4
$THETAPV BLOCK(4) FIX VALUES(10000.0,0.0)

; Prior information to the OMEGAS.
$OMEGAP BLOCK(4)
0.2 FIX 
0.0  0.2 
0.0  0.0 0.2
0.0  0.0 0.0 0.2
$OMEGAPD (4 FIX)

$EST METHOD=BAYES INTERACTION FILE=example8.ext NBURN=10000 
     NITER=1000 PRINT=100 NOPRIOR=0 CTYPE=3 CINTERVAL=100
  
NM-TRAN MESSAGES 
  
 WARNINGS AND ERRORS (IF ANY) FOR PROBLEM    1
             
 (WARNING  2) NM-TRAN INFERS THAT THE DATA ARE POPULATION.

 (MU_WARNING 20) MU_001: MU_ VARIABLE SHOULD NOT BE DEFINED AFTER VERBATIM CODE.
  
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
 RUN# Example 8 (from samp5l)
0DATA CHECKOUT RUN:              NO
 DATA SET LOCATED ON UNIT NO.:    2
 THIS UNIT TO BE REWOUND:        NO
 CREATE/ADD TO FDATA.csv:        YES
 NO. OF DATA RECS IN DATA SET:      600
 NO. OF DATA ITEMS IN DATA SET:  17
 ID DATA ITEM IS DATA ITEM NO.:   3
 DEP VARIABLE IS DATA ITEM NO.:   6
 MDV DATA ITEM IS DATA ITEM NO.: 10
0INDICES PASSED TO SUBROUTINE PRED:
   9   5   7   8   0   0  11   0   0   0   0
0LABELS FOR DATA ITEMS:
 C SET ID JID TIME CONC DOSE RATE EVID MDV CMT CLX V1X QX V2X SDIX SDSX
0FORMAT FOR DATA:
 (2E2.0,3E4.0,E11.0,E4.0,4E2.0,2E7.0,E8.0,E7.0,E2.0,E5.0)

 TOT. NO. OF OBS RECS:      500
 TOT. NO. OF INDIVIDUALS:      100
0LENGTH OF THETA:   9
0DEFAULT THETA BOUNDARY TEST OMITTED:    NO
0OMEGA HAS BLOCK FORM:
  1
  1  1
  1  1  1
  1  1  1  1
  0  0  0  0  2
  0  0  0  0  2  2
  0  0  0  0  2  2  2
  0  0  0  0  2  2  2  2
  0  0  0  0  0  0  0  0  3
  0  0  0  0  0  0  0  0  3  3
  0  0  0  0  0  0  0  0  3  3  3
  0  0  0  0  0  0  0  0  3  3  3  3
0DEFAULT OMEGA BOUNDARY TEST OMITTED:    NO
0SIGMA HAS SIMPLE DIAGONAL FORM WITH DIMENSION:   1
0DEFAULT SIGMA BOUNDARY TEST OMITTED:    NO
0INITIAL ESTIMATE OF THETA:
 LOWER BOUND    INITIAL EST    UPPER BOUND
 -0.1000E+07     0.2000E+01     0.1000E+07
 -0.1000E+07     0.2000E+01     0.1000E+07
 -0.1000E+07     0.2000E+01     0.1000E+07
 -0.1000E+07     0.2000E+01     0.1000E+07
  0.2000E+01     0.2000E+01     0.2000E+01
  0.2000E+01     0.2000E+01     0.2000E+01
  0.2000E+01     0.2000E+01     0.2000E+01
  0.2000E+01     0.2000E+01     0.2000E+01
  0.4000E+01     0.4000E+01     0.4000E+01
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.1500E+00
                  0.1000E-01   0.1500E+00
                  0.1000E-01   0.1000E-01   0.1500E+00
                  0.1000E-01   0.1000E-01   0.1000E-01   0.1500E+00
        2                                                                                  YES
                  0.1000E+05
                  0.0000E+00   0.1000E+05
                  0.0000E+00   0.0000E+00   0.1000E+05
                  0.0000E+00   0.0000E+00   0.0000E+00   0.1000E+05
        3                                                                                  YES
                  0.2000E+00
                  0.0000E+00   0.2000E+00
                  0.0000E+00   0.0000E+00   0.2000E+00
                  0.0000E+00   0.0000E+00   0.0000E+00   0.2000E+00
0INITIAL ESTIMATE OF SIGMA:
 0.6000E+00
0
 PRIOR SUBROUTINE USER-SUPPLIED
1DOUBLE PRECISION PREDPP VERSION 7.6.0 beta 4 (nm76b4)

 TWO COMPARTMENT MODEL (ADVAN3)
0MAXIMUM NO. OF BASIC PK PARAMETERS:   4
0BASIC PK PARAMETERS (AFTER TRANSLATION):
   BASIC PK PARAMETER NO.  1: ELIMINATION RATE (K)
   BASIC PK PARAMETER NO.  2: CENTRAL-TO-PERIPH. RATE (K12)
   BASIC PK PARAMETER NO.  3: PERIPH.-TO-CENTRAL RATE (K21)
 TRANSLATOR WILL CONVERT PARAMETERS
 CL, V1, Q, V2 TO K, K12, K21 (TRANS4)
0COMPARTMENT ATTRIBUTES
 COMPT. NO.   FUNCTION   INITIAL    ON/OFF      DOSE      DEFAULT    DEFAULT
                         STATUS     ALLOWED    ALLOWED    FOR DOSE   FOR OBS.
    1         CENTRAL      ON         NO         YES        YES        YES
    2         PERIPH.      ON         NO         YES        NO         NO
    3         OUTPUT       OFF        YES        NO         NO         NO
1
 ADDITIONAL PK PARAMETERS - ASSIGNMENT OF ROWS IN GG
 COMPT. NO.                             INDICES
              SCALE      BIOAVAIL.   ZERO-ORDER  ZERO-ORDER  ABSORB
                         FRACTION    RATE        DURATION    LAG
    1            5           *           *           *           *
    2            *           *           *           *           *
    3            *           -           -           -           -
             - PARAMETER IS NOT ALLOWED FOR THIS MODEL
             * PARAMETER IS NOT SUPPLIED BY PK SUBROUTINE;
               WILL DEFAULT TO ONE IF APPLICABLE
0DATA ITEM INDICES USED BY PRED ARE:
   EVENT ID DATA ITEM IS DATA ITEM NO.:      9
   TIME DATA ITEM IS DATA ITEM NO.:          5
   DOSE AMOUNT DATA ITEM IS DATA ITEM NO.:   7
   DOSE RATE DATA ITEM IS DATA ITEM NO.:     8
   COMPT. NO. DATA ITEM IS DATA ITEM NO.:   11

0PK SUBROUTINE CALLED WITH EVERY EVENT RECORD.
 PK SUBROUTINE NOT CALLED AT NONEVENT (ADDITIONAL OR LAGGED) DOSE TIMES.
0ERROR SUBROUTINE CALLED WITH EVERY EVENT RECORD.
1
 
 
 #TBLN:      1
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
 NO. OF FUNCT. EVALS. ALLOWED:            2400
 NO. OF SIG. FIGURES REQUIRED:            3
 INTERMEDIATE PRINTOUT:                   YES
 ESTIMATE OUTPUT TO MSF:                  NO
 IND. OBJ. FUNC. VALUES SORTED:           NO
 NUMERICAL DERIVATIVE
       FILE REQUEST (NUMDER):               NONE
 MAP (ETAHAT) ESTIMATION METHOD (OPTMAP):   0
 ETA HESSIAN EVALUATION METHOD (ETADER):    0
 INITIAL ETA FOR MAP ESTIMATION (MCETA):    0
 SIGDIGITS FOR MAP ESTIMATION (SIGLO):      100
 GRADIENT SIGDIGITS OF
       FIXED EFFECTS PARAMETERS (SIGL):     100
 NOPRIOR SETTING (NOPRIOR):                 0
 NOCOV SETTING (NOCOV):                     OFF
 DERCONT SETTING (DERCONT):                 OFF
 FINAL ETA RE-EVALUATION (FNLETA):          1
 EXCLUDE NON-INFLUENTIAL (NON-INFL.) ETAS
       IN SHRINKAGE (ETASTYPE):             NO
 NON-INFL. ETA CORRECTION (NONINFETA):      0
 RAW OUTPUT FILE (FILE): example8.ext
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
 ITERATIONS (NITER):                        1000
 ANNEAL SETTING (CONSTRAIN):                 1
 STARTING SEED FOR MC METHODS (SEED):       11456
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
 SAMPLES FOR LOCAL SEARCH KERNEL (OSAMPLE_M2):           10
 SAMPLES FOR LOCAL UNIVARIATE SEARCH KERNEL (OSAMPLE_M3):10
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
   1   2   3   4
 THETAS THAT ARE GIBBS SAMPLED:
   1   2   3   4
 THETAS THAT ARE METROPOLIS-HASTINGS SAMPLED:
 
 SIGMAS THAT ARE GIBBS SAMPLED:
   1
 SIGMAS THAT ARE METROPOLIS-HASTINGS SAMPLED:
 
 OMEGAS ARE GIBBS SAMPLED
 
 MONITORING OF SEARCH:

 Burn-in Mode
 iteration       -10000 MCMCOBJ=    103422445.627986     
 iteration        -9900 MCMCOBJ=   -2360.02102010134     
 iteration        -9800 MCMCOBJ=   -2292.41715455974     
 iteration        -9700 MCMCOBJ=   -2337.53570894454     
 iteration        -9600 MCMCOBJ=   -2339.24557595695     
 iteration        -9500 MCMCOBJ=   -2279.03176216568     
 iteration        -9400 MCMCOBJ=   -2383.99680576389     
 iteration        -9300 MCMCOBJ=   -2317.61654795729     
 iteration        -9200 MCMCOBJ=   -2324.52360549265     
 iteration        -9100 MCMCOBJ=   -2344.50060592978     
 iteration        -9000 MCMCOBJ=   -2328.07826340985     
 Convergence achieved
 Elapsed burn-in time in seconds:    25.82
 Sampling Mode
 iteration            0 MCMCOBJ=   -2315.59263283576     
 iteration          100 MCMCOBJ=   -2299.99335211428     
 iteration          200 MCMCOBJ=   -2368.12768708073     
 iteration          300 MCMCOBJ=   -2280.67820741752     
 iteration          400 MCMCOBJ=   -2345.11823526185     
 iteration          500 MCMCOBJ=   -2322.25107877889     
 iteration          600 MCMCOBJ=   -2333.98888688602     
 iteration          700 MCMCOBJ=   -2326.33380200269     
 iteration          800 MCMCOBJ=   -2337.77769507568     
 iteration          900 MCMCOBJ=   -2245.42882750558     
 iteration         1000 MCMCOBJ=   -2287.69415024867     
 
 #TERM:
 BURN-IN WAS COMPLETED
 STATISTICAL PORTION WAS COMPLETED

 ETABAR IS THE ARITHMETIC MEAN OF THE ETA-ESTIMATES,
 AND THE P-VALUE IS GIVEN FOR THE NULL HYPOTHESIS THAT THE TRUE MEAN IS 0.
 
 ETABAR:        -3.9558E-03 -1.6248E-03 -7.8417E-04 -1.3187E-03
 SE:             3.8956E-02  2.7571E-02  2.9262E-02  3.1508E-02
 N:                     100         100         100         100
 
 P VAL.:         9.1912E-01  9.5300E-01  9.7862E-01  9.6661E-01
 
 ETASHRINKSD(%)  6.5661E+00  2.7148E+01  3.2353E+01  2.0259E+01
 ETASHRINKVR(%)  1.2701E+01  4.6927E+01  5.4239E+01  3.6414E+01
 EBVSHRINKSD(%)  3.5694E+00  2.4030E+01  2.9453E+01  1.6802E+01
 EBVSHRINKVR(%)  7.0113E+00  4.2285E+01  5.0231E+01  3.0781E+01
 RELATIVEINF(%)  8.7943E+01  5.6636E+01  4.5309E+01  6.0482E+01
 EPSSHRINKSD(%)  2.9974E+01
 EPSSHRINKVR(%)  5.0964E+01
 
  
 TOTAL DATA POINTS NORMALLY DISTRIBUTED (N):          500
 N*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    918.938533204673     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -2307.07065024074     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -1388.13211703607     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 TOTAL EFFECTIVE ETAS (NIND*NETA):                           400
 NIND*NETA*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    735.150826563738     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -2307.07065024074     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -1571.91982367701     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 PRIOR CONSTANT TO OBJECTIVE FUNCTION:    66.6250661892040     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -2307.07065024074     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -2240.44558405154     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 #TERE:
 Elapsed estimation  time in seconds:    63.58
 Elapsed covariance  time in seconds:     0.00
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 #OBJT:**************                       AVERAGE VALUE OF LIKELIHOOD FUNCTION                     ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 





 #OBJV:********************************************         -2307.071       *********************************************
 #OBJS:********************************************            44.418 (STD) *********************************************
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 ********************                             FINAL PARAMETER ESTIMATE                           ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4     
 
         1.64E+00  1.56E+00  7.52E-01  2.35E+00
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        1.74E-01
 
 ETA2
+       -4.95E-03  1.43E-01
 
 ETA3
+        7.89E-03 -7.57E-03  1.87E-01
 
 ETA4
+       -2.05E-02  9.10E-03  2.06E-02  1.56E-01
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        6.06E-02
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        4.16E-01
 
 ETA2
+       -3.49E-02  3.76E-01
 
 ETA3
+        3.49E-02 -5.14E-02  4.27E-01
 
 ETA4
+       -1.31E-01  5.46E-02  9.63E-02  3.92E-01
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        2.46E-01
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 ********************                STANDARD ERROR OF ESTIMATE (From Sample Variance)               ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4     
 
         4.46E-02  5.04E-02  7.13E-02  5.42E-02
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        2.82E-02
 
 ETA2
+        2.26E-02  3.16E-02
 
 ETA3
+        2.91E-02  3.31E-02  6.11E-02
 
 ETA4
+        2.31E-02  2.44E-02  3.38E-02  3.69E-02
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        7.48E-03
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        3.34E-02
 
 ETA2
+        1.41E-01  4.15E-02
 
 ETA3
+        1.55E-01  1.99E-01  6.93E-02
 
 ETA4
+        1.42E-01  1.57E-01  1.78E-01  4.60E-02
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        1.52E-02
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 ********************               COVARIANCE MATRIX OF ESTIMATE (From Sample Variance)             ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      OM11      OM12      OM13      OM14      OM22      OM23      OM24      OM33  
             OM34      OM44      SG11  
 
 TH 1
+        1.99E-03
 
 TH 2
+        2.00E-04  2.54E-03
 
 TH 3
+        4.70E-04  1.87E-04  5.09E-03
 
 TH 4
+        1.76E-04  3.74E-04  1.87E-03  2.94E-03
 
 OM11
+       -3.18E-05  8.85E-05  1.47E-04  8.80E-05  7.98E-04
 
 OM12
+        2.43E-05  1.43E-04  2.25E-04  8.82E-05  6.66E-05  5.11E-04
 
 OM13
+        1.01E-05  1.57E-04  4.69E-05  9.52E-05  2.04E-04  1.08E-04  8.49E-04
 
 OM14
+        5.22E-05  2.56E-05  2.58E-04  1.06E-04  5.99E-05  1.21E-04  3.20E-04  5.35E-04
 
 OM22
+        3.35E-05 -8.36E-05 -1.36E-04  3.59E-05 -1.76E-05  5.85E-05 -2.59E-05  1.06E-05  9.98E-04
 
 OM23
+       -1.80E-05  1.21E-04  5.39E-04  8.63E-05  2.56E-05  1.47E-04  7.18E-05  1.06E-04 -4.24E-05  1.10E-03
 
 OM24
+       -2.29E-05  3.07E-07  1.89E-04  9.65E-05 -1.10E-05  3.88E-05 -4.97E-06  6.03E-05  1.60E-04  2.71E-04  5.97E-04
 
 OM33
+        2.66E-05  3.16E-06  2.84E-04  1.06E-04  2.02E-04  2.87E-04  5.63E-04  3.37E-04  8.34E-05  2.43E-04  6.31E-05  3.74E-03
 
 OM34
+        5.02E-05 -5.37E-06  1.41E-04 -2.63E-05  4.63E-05  1.31E-04  1.75E-04  1.77E-04  3.40E-05  2.34E-04  9.85E-05  1.26E-03
          1.14E-03
 
 OM44
+        5.54E-05  1.15E-04  9.99E-05  1.23E-04  1.84E-05  1.20E-04  1.10E-04  9.62E-05  7.93E-05  1.69E-04  2.38E-04  6.42E-04
          6.79E-04  1.36E-03
 
 SG11
+       -2.44E-06  3.17E-05  1.12E-07 -5.00E-06 -1.65E-05 -1.81E-05 -2.36E-05 -1.77E-05 -4.62E-05 -7.64E-06 -2.09E-05 -1.50E-04
         -7.35E-05 -8.11E-05  5.60E-05
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 ********************              CORRELATION MATRIX OF ESTIMATE (From Sample Variance)             ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      OM11      OM12      OM13      OM14      OM22      OM23      OM24      OM33  
             OM34      OM44      SG11  
 
 TH 1
+        4.46E-02
 
 TH 2
+        8.89E-02  5.04E-02
 
 TH 3
+        1.48E-01  5.21E-02  7.13E-02
 
 TH 4
+        7.27E-02  1.37E-01  4.83E-01  5.42E-02
 
 OM11
+       -2.52E-02  6.22E-02  7.31E-02  5.74E-02  2.82E-02
 
 OM12
+        2.41E-02  1.25E-01  1.40E-01  7.19E-02  1.04E-01  2.26E-02
 
 OM13
+        7.78E-03  1.07E-01  2.26E-02  6.02E-02  2.48E-01  1.65E-01  2.91E-02
 
 OM14
+        5.06E-02  2.20E-02  1.57E-01  8.44E-02  9.17E-02  2.31E-01  4.75E-01  2.31E-02
 
 OM22
+        2.38E-02 -5.26E-02 -6.03E-02  2.10E-02 -1.97E-02  8.20E-02 -2.81E-02  1.45E-02  3.16E-02
 
 OM23
+       -1.21E-02  7.26E-02  2.28E-01  4.80E-02  2.73E-02  1.96E-01  7.44E-02  1.38E-01 -4.04E-02  3.31E-02
 
 OM24
+       -2.10E-02  2.49E-04  1.08E-01  7.28E-02 -1.59E-02  7.02E-02 -6.98E-03  1.07E-01  2.07E-01  3.34E-01  2.44E-02
 
 OM33
+        9.73E-03  1.03E-03  6.52E-02  3.20E-02  1.17E-01  2.08E-01  3.16E-01  2.38E-01  4.32E-02  1.20E-01  4.22E-02  6.11E-02
 
 OM34
+        3.33E-02 -3.16E-03  5.86E-02 -1.43E-02  4.85E-02  1.71E-01  1.78E-01  2.27E-01  3.18E-02  2.09E-01  1.19E-01  6.09E-01
          3.38E-02
 
 OM44
+        3.37E-02  6.17E-02  3.80E-02  6.13E-02  1.77E-02  1.44E-01  1.03E-01  1.13E-01  6.81E-02  1.38E-01  2.65E-01  2.85E-01
          5.44E-01  3.69E-02
 
 SG11
+       -7.31E-03  8.40E-02  2.10E-04 -1.23E-02 -7.79E-02 -1.07E-01 -1.08E-01 -1.02E-01 -1.96E-01 -3.08E-02 -1.14E-01 -3.27E-01
         -2.90E-01 -2.94E-01  7.48E-03
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 ********************           INVERSE COVARIANCE MATRIX OF ESTIMATE (From Sample Variance)         ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      OM11      OM12      OM13      OM14      OM22      OM23      OM24      OM33  
             OM34      OM44      SG11  
 
 TH 1
+        5.22E+02
 
 TH 2
+       -4.21E+01  4.25E+02
 
 TH 3
+       -5.42E+01  1.65E+01  2.87E+02
 
 TH 4
+        6.55E+00 -5.53E+01 -1.74E+02  4.61E+02
 
 OM11
+        3.37E+01 -2.64E+01 -3.76E+01 -6.17E+00  1.36E+03
 
 OM12
+        8.62E+00 -1.09E+02 -5.79E+01  1.19E+01 -1.02E+02  2.26E+03
 
 OM13
+        7.13E+00 -7.97E+01  6.78E+01 -3.84E+01 -3.25E+02 -1.99E+01  1.74E+03
 
 OM14
+       -3.98E+01  5.34E+01 -9.72E+01  1.73E+00  9.05E+01 -3.45E+02 -9.39E+02  2.64E+03
 
 OM22
+       -3.14E+01  2.85E+01  5.06E+01 -3.72E+01  2.25E+01 -1.51E+02  5.19E+01 -2.50E+00  1.12E+03
 
 OM23
+        3.44E+01 -4.11E+01 -1.08E+02  5.65E+01  1.12E+01 -2.22E+02 -3.63E+01 -1.96E+01  1.16E+02  1.16E+03
 
 OM24
+        4.25E+01  1.85E+01 -2.40E+01 -2.52E+01  2.18E+01  9.42E+01  1.13E+02 -2.03E+02 -3.38E+02 -5.07E+02  2.15E+03
 
 OM33
+        9.22E+00  5.99E+00 -7.60E+00 -1.25E+01 -2.20E+01 -1.01E+02 -1.98E+02  1.06E+01 -3.55E+00  1.66E+01  2.40E+01  4.88E+02
 
 OM34
+       -2.36E+01  2.94E+01 -9.34E+00  7.39E+01  1.79E+01  2.91E+01  1.31E+02 -2.52E+02  1.38E+01 -2.11E+02  1.33E+02 -5.09E+02
          1.91E+03
 
 OM44
+       -1.48E+01 -5.32E+01  2.30E+01 -5.30E+01  2.80E+01 -9.75E+01 -5.30E+01  8.54E+01  3.11E+01  5.58E+01 -3.64E+02  8.08E+01
         -7.09E+02  1.17E+03
 
 SG11
+        1.74E+01 -3.04E+02 -4.11E+01  1.41E+01  3.08E+02  1.56E+02  2.33E+01  6.26E+01  8.25E+02 -1.48E+02  1.75E+02  6.39E+02
          1.39E+02  8.90E+02  2.21E+04
 
 Elapsed postprocess time in seconds:     0.00
 Elapsed finaloutput time in seconds:     0.00
 #CPUT: Total CPU Time in Seconds,       63.453
Stop Time: 
Tue 10/22/2024 
02:31 PM
