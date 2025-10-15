Tue 10/22/2024 
12:35 PM
;Model Desc: Two compartment Model, Using ADVAN3, TRANS4
;Project Name: nm7examples
;Project ID: NO PROJECT DESCRIPTION

$PROB RUN# Example 1 (from samp5l)
$INPUT C SET ID JID TIME  DV=CONC AMT=DOSE RATE EVID MDV CMT CLX 
       V1X QX V2X SDIX SDSX
$DATA example1.csv IGNORE=C

$SUBROUTINES ADVAN3 TRANS4

$PK
; The thetas are MU modeled.  
; Best that there is a linear relationship between THETAs and Mus
; The linear MU modeling of THETAS allows them to be efficiently 
; Gibbs sampled.

MU_1=THETA(1)
MU_2=THETA(2)
MU_3=THETA(3)
MU_4=THETA(4)
CL=DEXP(MU_1+ETA(1))
V1=DEXP(MU_2+ETA(2))
Q=DEXP(MU_3+ETA(3))
V2=DEXP(MU_4+ETA(4))
S1=V1

$ERROR
Y = F + F*EPS(1)

; Initial values of THETA
$THETA 
(0.001, 2.0) ;[LN(CL)]
(0.001, 2.0) ;[LN(V1)]
(0.001, 2.0) ;[LN(Q)]
(0.001, 2.0) ;[LN(V2)]

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

;Prior information is important for MCMC Bayesian analysis,
;not necessary for maximization methods
;Note the syntax used for defining priors that is available 
;as of NONMEM 7.3
$PRIOR NWPRI

; Prior information of THETAS
$THETAP (2.0 FIX)X4

; Variance to prior information of THETAS.  
; Because variances are very large, this means that the prior 
; information to the THETAS is highly uninformative.
$THETAPV BLOCK(4) FIX VALUES(10000,0.0)

; Prior information to the OMEGAS.
$OMEGAP BLOCK(4) FIX VALUES(0.2,0.0)
; Degrees of freedom to prior OMEGA matrix.  
; Because degrees of freedom is very low, equal to the
; the dimension of the prior OMEGA, this means that the 
; prior information to the OMEGAS is highly uninformative
$OMEGAPD (4 FIX)

; Prior information to the SIGMAS
$SIGMAP 0.06 FIX
; Degrees of freedom to prior SIGMA matrix.  
; Because degrees of freedom is very low, equal to the
; the dimension of the prior SIGMA, this means that the 
; prior information to the SIGMA is highly uninformative
$SIGMAPD (1 FIX)

; The first analysis is iterative two-stage, 
; maximum of 500 iterations (NITER), iteration results
; are printed every 5 iterations, gradient precision (SIGL) is 4. 
; Termination is tested on all of 
; the population parameters (CTYPE=3), 
; and for less then 2 significant digits change (NSIG).
; Prior information is not necessary for ITS, so NOPRIOR=1.  
; The intermediate and final results of the ITS method will be 
; recoded in row/column format in example1.ext

$EST METHOD=ITS MAPITER=0 INTERACTION FILE=example1.ext NITER=500 
     PRINT=5 NOABORT SIGL=4 CTYPE=3 CITER=10 
     CALPHA=0.05 NOPRIOR=1 NSIG=2

; The results of ITS are used as the initial values for the 
; SAEM method. A maximum of 3000 ; stochastic iterations (NBURN) 
; is requested, but may end early if statistical test determines
; that variations in all parameters is stationary 
; (note that any settings from the previous $EST
; carries over to the next $EST statement, within a $PROB).  
; The SAEM is a Monte Carlo process,
; so setting the SEED assures repeatability of results.  
; Each iteration obtains only 2 Monte Carlo samples ISAMPLE),
;  so they are very fast. 
; But many iterations are needed, so PRINT only
; every 100th iteration.  
; After the stochastic phase, 500 accumulation iterations will be
; Performed (NITER), to obtain good parameters estimates with 
; little stochastic noise.
; As a new FILE has not been given, the SAEM results will append to 
; example1.ext.

$EST METHOD=SAEM INTERACTION NBURN=3000 NITER=500 PRINT=100 
     SEED=1556678 ISAMPLE=2

; After the SAEM method, obtain good estimates of the marginal 
; density (objective function),
; along with good estimates of the standard errors.  
; This is best done with importance sampling ; (IMP), 
; performing the expectation step only (EONLY=1), so that 
; final population parameters remain at the final SAEM result.  
; Five iterations (NITER) should allow the importance sampling
; proposal density to become stationary.  
; This is observed by the objective function settling 
; to a particular value (with some stochastic noise).  
; By using 3000 Monte Carlo samples
; (ISAMPLE), this assures a precise assessment of standard errors.

$EST METHOD=IMP  INTERACTION EONLY=1 NITER=5 ISAMPLE=3000 PRINT=1 
     SIGL=8 NOPRIOR=1

; The Bayesian analysis is performed.  
; While 10000 burn-in iterations are requested as a maximum, 
; because the termination test is on (CTYPE<>0, set at the
; first $EST statement), and because the initial parameters are at 
; the SAEM result, which is the maximum likelihood position, 
; the analysis should settle down to a stationary distribution in
; several hundred iterations.  
; Prior information is also used to facilitate Bayesian analysis.
; The individual Bayesian iteration results are important, 
; and may be need for post-processing analysis. 
; So specify a separate FILE for the Bayesian analysis. 

$EST METHOD=BAYES INTERACTION FILE=example1.txt NBURN=10000     
     NITER=10000 PRINT=100 NOPRIOR=0

; Just for old-times sake, let's see what the traditional 
; FOCE method will give us.  
; And, remember to introduce a new FILE, so its results won't 
; append to our Bayesian FILE. 
; Appending to example1.ext with the EM methods is fine.

$EST METHOD=COND INTERACTION MAXEVAL=9999 NSIG=3 SIGL=10 
     PRINT=5 NOABORT NOPRIOR=1
     FILE=example1.ext

; Time for the standard error results.  
; You may request a more precise gradient precision (SIGL)
; that differed from that used during estimation.

$COV MATRIX=R PRINT=E UNCONDITIONAL SIGL=12

; Print out results in tables. Include some of the new weighted 
; residual types

$TABLE ID TIME PRED RES WRES CPRED CWRES EPRED ERES EWRES NOAPPEND 
       ONEHEADER FILE=example1.TAB NOPRINT
$TABLE ID CL V1 Q V2 FIRSTONLY NOAPPEND NOPRINT FILE=example1.PAR
$TABLE ID ETA1 ETA2 ETA3 ETA4 FIRSTONLY NOAPPEND 
        NOPRINT FILE=example1.ETA
  
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
 RUN# Example 1 (from samp5l)
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
0(NONBLANK) LABELS FOR PRED-DEFINED ITEMS:
 CL V1 Q V2
0FORMAT FOR DATA:
 (2E2.0,3E4.0,E11.0,E4.0,4E2.0,2E7.0,E8.0,E7.0,E2.0,E5.0)

 TOT. NO. OF OBS RECS:      500
 TOT. NO. OF INDIVIDUALS:      100
0LENGTH OF THETA:  10
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
0SIGMA HAS BLOCK FORM:
  1
  0  2
0DEFAULT SIGMA BOUNDARY TEST OMITTED:    NO
0INITIAL ESTIMATE OF THETA:
 LOWER BOUND    INITIAL EST    UPPER BOUND
  0.1000E-02     0.2000E+01     0.1000E+07
  0.1000E-02     0.2000E+01     0.1000E+07
  0.1000E-02     0.2000E+01     0.1000E+07
  0.1000E-02     0.2000E+01     0.1000E+07
  0.2000E+01     0.2000E+01     0.2000E+01
  0.2000E+01     0.2000E+01     0.2000E+01
  0.2000E+01     0.2000E+01     0.2000E+01
  0.2000E+01     0.2000E+01     0.2000E+01
  0.4000E+01     0.4000E+01     0.4000E+01
  0.1000E+01     0.1000E+01     0.1000E+01
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
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.6000E+00
        2                                                                                  YES
                  0.6000E-01
0COVARIANCE STEP OMITTED:        NO
 R MATRIX SUBSTITUTED:          YES
 S MATRIX SUBSTITUTED:           NO
 EIGENVLS. PRINTED:             YES
 COMPRESSED FORMAT:              NO
 GRADIENT METHOD USED:     NOSLOW
 SIGDIGITS ETAHAT (SIGLO):                  -1
 SIGDIGITS GRADIENTS (SIGL):                12
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
0TABLES STEP OMITTED:    NO
 NO. OF TABLES:           3
 SEED NUMBER (SEED):    11456
 NPDTYPE:    0
 INTERPTYPE:    0
 RANMETHOD:             3U
 MC SAMPLES (ESAMPLE):    300
 WRES SQUARE ROOT TYPE (WRESCHOL): EIGENVALUE
0-- TABLE   1 --
0RECORDS ONLY:    ALL
04 COLUMNS APPENDED:    NO
 PRINTED:                NO
 HEADER:                YES
 FILE TO BE FORWARDED:   NO
 FORMAT:                S1PE11.4
 IDFORMAT:
 LFORMAT:
 RFORMAT:
 FIXED_EFFECT_ETAS:
0USER-CHOSEN ITEMS:
 ID TIME PRED RES WRES CPRED CWRES EPRED ERES EWRES
0-- TABLE   2 --
0RECORDS ONLY:    FIRSTONLY
04 COLUMNS APPENDED:    NO
 PRINTED:                NO
 HEADER:                YES
 FILE TO BE FORWARDED:   NO
 FORMAT:                S1PE11.4
 IDFORMAT:
 LFORMAT:
 RFORMAT:
 FIXED_EFFECT_ETAS:
0USER-CHOSEN ITEMS:
 ID CL V1 Q V2
0-- TABLE   3 --
0RECORDS ONLY:    FIRSTONLY
04 COLUMNS APPENDED:    NO
 PRINTED:                NO
 HEADER:                YES
 FILE TO BE FORWARDED:   NO
 FORMAT:                S1PE11.4
 IDFORMAT:
 LFORMAT:
 RFORMAT:
 FIXED_EFFECT_ETAS:
0USER-CHOSEN ITEMS:
 ID ETA1 ETA2 ETA3 ETA4
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
0ERROR IN LOG Y IS MODELED
0DATA ITEM INDICES USED BY PRED ARE:
   EVENT ID DATA ITEM IS DATA ITEM NO.:      9
   TIME DATA ITEM IS DATA ITEM NO.:          5
   DOSE AMOUNT DATA ITEM IS DATA ITEM NO.:   7
   DOSE RATE DATA ITEM IS DATA ITEM NO.:     8
   COMPT. NO. DATA ITEM IS DATA ITEM NO.:   11

0PK SUBROUTINE CALLED WITH EVERY EVENT RECORD.
 PK SUBROUTINE NOT CALLED AT NONEVENT (ADDITIONAL OR LAGGED) DOSE TIMES.
0DURING SIMULATION, ERROR SUBROUTINE CALLED WITH EVERY EVENT RECORD.
 OTHERWISE, ERROR SUBROUTINE CALLED ONCE IN THIS PROBLEM.
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
 NO. OF FUNCT. EVALS. ALLOWED:            2808
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
 SIGDIGITS FOR MAP ESTIMATION (SIGLO):      4
 GRADIENT SIGDIGITS OF
       FIXED EFFECTS PARAMETERS (SIGL):     4
 NOPRIOR SETTING (NOPRIOR):                 1
 NOCOV SETTING (NOCOV):                     OFF
 DERCONT SETTING (DERCONT):                 OFF
 FINAL ETA RE-EVALUATION (FNLETA):          1
 EXCLUDE NON-INFLUENTIAL (NON-INFL.) ETAS
       IN SHRINKAGE (ETASTYPE):             NO
 NON-INFL. ETA CORRECTION (NONINFETA):      0
 RAW OUTPUT FILE (FILE): example1.ext
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
 ITERATIONS (NITER):                        500
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
   1   2   3   4
 THETAS THAT ARE SIGMA-LIKE:
 
 
 MONITORING OF SEARCH:

 iteration            0  OBJ=  -234.362672489374
 iteration            5  OBJ=  -1112.78963281739
 iteration           10  OBJ=  -1119.86549175808
 iteration           15  OBJ=  -1120.24033306398
 iteration           20  OBJ=  -1120.33439998151
 iteration           25  OBJ=  -1120.34671977467
 iteration           30  OBJ=  -1120.34019310236
 iteration           35  OBJ=  -1120.33186417385
 iteration           40  OBJ=  -1120.32450362785
 iteration           45  OBJ=  -1120.31908984596
 iteration           50  OBJ=  -1120.31514403969
 iteration           55  OBJ=  -1120.31238796970
 iteration           60  OBJ=  -1120.31066238979
 iteration           65  OBJ=  -1120.30928402069
 iteration           70  OBJ=  -1120.30830933872
 iteration           75  OBJ=  -1120.30791757936
 iteration           80  OBJ=  -1120.30764709618
 iteration           85  OBJ=  -1120.30744676007
 iteration           90  OBJ=  -1120.30710651310
 iteration           95  OBJ=  -1120.30704889110
 iteration          100  OBJ=  -1120.30681376538
 iteration          105  OBJ=  -1120.30697529936
 iteration          110  OBJ=  -1120.30690218459
 iteration          115  OBJ=  -1120.30687621030
 Convergence achieved
 iteration          115  OBJ=  -1120.30678688238
 
 #TERM:
 OPTIMIZATION WAS COMPLETED


 ETABAR IS THE ARITHMETIC MEAN OF THE ETA-ESTIMATES,
 AND THE P-VALUE IS GIVEN FOR THE NULL HYPOTHESIS THAT THE TRUE MEAN IS 0.
 
 ETABAR:         1.5002E-06 -3.6302E-07  1.5491E-06  1.0229E-06
 SE:             3.9005E-02  2.8994E-02  3.4788E-02  3.3931E-02
 N:                     100         100         100         100
 
 P VAL.:         9.9997E-01  9.9999E-01  9.9996E-01  9.9998E-01
 
 ETASHRINKSD(%)  3.3268E+00  1.9755E+01  2.3051E+01  1.4612E+01
 ETASHRINKVR(%)  6.5430E+00  3.5608E+01  4.0789E+01  2.7089E+01
 EBVSHRINKSD(%)  3.3269E+00  1.9755E+01  2.3052E+01  1.4612E+01
 EBVSHRINKVR(%)  6.5431E+00  3.5607E+01  4.0790E+01  2.7089E+01
 RELATIVEINF(%)  9.0305E+01  6.4036E+01  5.8783E+01  7.0874E+01
 EPSSHRINKSD(%)  3.1329E+01
 EPSSHRINKVR(%)  5.2842E+01
 
  
 TOTAL DATA POINTS NORMALLY DISTRIBUTED (N):          500
 N*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    918.938533204673     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -1120.30678688238     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -201.368253677705     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 TOTAL EFFECTIVE ETAS (NIND*NETA):                           400
  
 #TERE:
 Elapsed estimation  time in seconds:    13.56
 Elapsed covariance  time in seconds:     0.12
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 #OBJT:**************                        FINAL VALUE OF OBJECTIVE FUNCTION                       ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 





 #OBJV:********************************************         -1120.307       *********************************************
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 ********************                             FINAL PARAMETER ESTIMATE                           ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4     
 
         1.68E+00  1.59E+00  8.12E-01  2.37E+00
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        1.63E-01
 
 ETA2
+        3.70E-03  1.31E-01
 
 ETA3
+        4.56E-03  1.73E-02  2.04E-01
 
 ETA4
+       -1.65E-02  1.20E-02  4.83E-02  1.58E-01
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        5.52E-02
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        4.03E-01
 
 ETA2
+        2.54E-02  3.61E-01
 
 ETA3
+        2.50E-02  1.06E-01  4.52E-01
 
 ETA4
+       -1.03E-01  8.37E-02  2.69E-01  3.97E-01
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        2.35E-01
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 ********************                          STANDARD ERROR OF ESTIMATE (S)                        ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4     
 
         4.56E-02  4.85E-02  6.44E-02  5.33E-02
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        2.86E-02
 
 ETA2
+        2.28E-02  3.45E-02
 
 ETA3
+        3.19E-02  3.77E-02  6.43E-02
 
 ETA4
+        2.74E-02  2.71E-02  4.49E-02  4.01E-02
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        7.79E-03
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        3.55E-02
 
 ETA2
+        1.55E-01  4.77E-02
 
 ETA3
+        1.73E-01  2.26E-01  7.11E-02
 
 ETA4
+        1.76E-01  1.83E-01  1.95E-01  5.05E-02
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        1.66E-02
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 ********************                        COVARIANCE MATRIX OF ESTIMATE (S)                       ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      OM11      OM12      OM13      OM14      OM22      OM23      OM24      OM33  
             OM34      OM44      SG11  
 
 TH 1
+        2.08E-03
 
 TH 2
+        3.55E-04  2.35E-03
 
 TH 3
+        3.46E-04  2.11E-04  4.15E-03
 
 TH 4
+       -3.40E-05  1.50E-04  1.57E-03  2.85E-03
 
 OM11
+       -2.37E-04  9.73E-05  8.40E-05 -7.40E-05  8.18E-04
 
 OM12
+        1.53E-05  1.75E-04  4.55E-05 -1.23E-04  3.03E-04  5.21E-04
 
 OM13
+       -1.58E-04  4.20E-06 -2.33E-04  2.33E-04  1.36E-04  2.37E-04  1.02E-03
 
 OM14
+       -1.72E-04 -2.06E-04  1.60E-04  1.84E-04  2.17E-04  1.80E-04  5.33E-04  7.51E-04
 
 OM22
+       -3.82E-05 -1.77E-05 -1.96E-04 -1.00E-04  1.37E-04  2.30E-04  8.55E-05  1.41E-04  1.19E-03
 
 OM23
+       -7.19E-05 -9.59E-05  6.49E-05  1.27E-04  1.84E-04  3.73E-05 -6.04E-05 -3.03E-05  2.76E-04  1.42E-03
 
 OM24
+       -2.52E-04 -1.02E-04  1.49E-04  1.61E-04  1.14E-04  8.49E-05  2.25E-05  9.08E-05  3.74E-04  5.70E-04  7.33E-04
 
 OM33
+       -3.17E-04 -3.27E-04 -1.39E-04  5.54E-04  2.24E-04  1.50E-04  7.91E-04  3.91E-04  5.45E-04  4.26E-04  4.02E-04  4.13E-03
 
 OM34
+       -1.98E-05 -9.20E-05  2.92E-04  2.93E-04  2.51E-04  1.18E-04  2.93E-04  2.25E-04  3.80E-04  3.58E-04  3.32E-04  2.18E-03
          2.01E-03
 
 OM44
+        3.07E-05 -1.57E-05  2.14E-04 -2.66E-04  2.68E-04  1.26E-04  7.08E-05  1.60E-04  3.26E-04  1.86E-04  2.28E-04  1.11E-03
          1.32E-03  1.61E-03
 
 SG11
+        4.55E-05  6.14E-05 -9.23E-06  2.30E-05 -6.01E-05 -4.31E-05 -5.90E-05 -8.04E-05 -8.13E-05  7.92E-06 -4.20E-05 -1.07E-04
         -1.16E-04 -1.21E-04  6.07E-05
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 ********************                        CORRELATION MATRIX OF ESTIMATE (S)                      ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      OM11      OM12      OM13      OM14      OM22      OM23      OM24      OM33  
             OM34      OM44      SG11  
 
 TH 1
+        4.56E-02
 
 TH 2
+        1.61E-01  4.85E-02
 
 TH 3
+        1.18E-01  6.75E-02  6.44E-02
 
 TH 4
+       -1.40E-02  5.80E-02  4.57E-01  5.33E-02
 
 OM11
+       -1.82E-01  7.02E-02  4.56E-02 -4.85E-02  2.86E-02
 
 OM12
+        1.47E-02  1.58E-01  3.09E-02 -1.01E-01  4.64E-01  2.28E-02
 
 OM13
+       -1.08E-01  2.71E-03 -1.13E-01  1.37E-01  1.49E-01  3.25E-01  3.19E-02
 
 OM14
+       -1.38E-01 -1.55E-01  9.09E-02  1.26E-01  2.77E-01  2.88E-01  6.09E-01  2.74E-02
 
 OM22
+       -2.43E-02 -1.06E-02 -8.84E-02 -5.45E-02  1.39E-01  2.93E-01  7.77E-02  1.49E-01  3.45E-02
 
 OM23
+       -4.19E-02 -5.25E-02  2.68E-02  6.33E-02  1.71E-01  4.34E-02 -5.02E-02 -2.93E-02  2.13E-01  3.77E-02
 
 OM24
+       -2.04E-01 -7.80E-02  8.54E-02  1.11E-01  1.47E-01  1.37E-01  2.61E-02  1.22E-01  4.01E-01  5.59E-01  2.71E-02
 
 OM33
+       -1.08E-01 -1.05E-01 -3.35E-02  1.62E-01  1.22E-01  1.02E-01  3.85E-01  2.22E-01  2.46E-01  1.76E-01  2.31E-01  6.43E-02
 
 OM34
+       -9.69E-03 -4.23E-02  1.01E-01  1.23E-01  1.95E-01  1.15E-01  2.04E-01  1.83E-01  2.46E-01  2.12E-01  2.74E-01  7.54E-01
          4.49E-02
 
 OM44
+        1.68E-02 -8.09E-03  8.29E-02 -1.24E-01  2.34E-01  1.38E-01  5.53E-02  1.46E-01  2.36E-01  1.23E-01  2.10E-01  4.29E-01
          7.34E-01  4.01E-02
 
 SG11
+        1.28E-01  1.62E-01 -1.84E-02  5.52E-02 -2.70E-01 -2.42E-01 -2.37E-01 -3.77E-01 -3.03E-01  2.70E-02 -1.99E-01 -2.14E-01
         -3.31E-01 -3.89E-01  7.79E-03
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 ********************                    INVERSE COVARIANCE MATRIX OF ESTIMATE (S)                   ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      OM11      OM12      OM13      OM14      OM22      OM23      OM24      OM33  
             OM34      OM44      SG11  
 
 TH 1
+        5.76E+02
 
 TH 2
+       -5.87E+01  5.03E+02
 
 TH 3
+       -5.66E+01 -1.19E+01  3.66E+02
 
 TH 4
+        1.96E+01 -5.05E+01 -2.23E+02  5.62E+02
 
 OM11
+        2.25E+02 -9.62E+01  1.15E+00  8.65E+00  1.86E+03
 
 OM12
+       -1.59E+02 -1.76E+02 -1.41E+02  2.35E+02 -9.90E+02  3.15E+03
 
 OM13
+        2.31E+01 -1.47E+02  2.20E+02 -1.56E+02  2.47E+02 -6.98E+02  2.17E+03
 
 OM14
+        3.20E+01  2.43E+02 -1.66E+02 -3.90E+01 -3.91E+02  1.20E+01 -1.36E+03  2.67E+03
 
 OM22
+       -6.54E+01 -3.65E+01  1.01E+02 -6.04E+00  1.19E+02 -4.55E+02  2.22E+02 -1.29E+02  1.21E+03
 
 OM23
+       -8.71E+01  5.18E+01  2.18E+01 -5.73E-01 -2.87E+02  1.16E+02  8.18E+00  1.25E+02 -2.38E+01  1.14E+03
 
 OM24
+        2.83E+02  1.00E+01 -6.84E+01 -8.93E+01  1.94E+02 -1.97E+02  1.46E+02 -1.43E+02 -4.87E+02 -8.95E+02  2.55E+03
 
 OM33
+        6.84E+01  7.38E+01  4.77E+01 -3.60E+01  1.35E+01  5.22E+01 -3.91E+02  1.10E+02 -1.29E+02 -3.93E+00  2.15E-01  7.45E+02
 
 OM34
+       -1.09E+02 -3.14E+01 -3.43E+01 -1.84E+02 -5.70E+01  1.78E+01  1.30E+02  3.45E+01  7.29E+01 -1.29E+02 -8.10E+01 -8.99E+02
          2.41E+03
 
 OM44
+       -2.13E+01 -5.74E+01 -8.27E+01  2.92E+02 -1.31E+02  8.72E+01  1.29E+02 -9.52E+01 -6.22E+01  6.34E+01 -5.48E+01  2.06E+02
         -1.32E+03  1.74E+03
 
 SG11
+       -2.23E+02 -5.82E+02  2.71E+01 -1.38E+02  7.69E+02  3.22E+02  4.95E+02  1.34E+03  1.01E+03 -9.37E+02  7.69E+02 -4.44E+02
          7.25E+02  1.08E+03  2.53E+04
 
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 ********************                    EIGENVALUES OF COR MATRIX OF ESTIMATE (S)                   ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

             1         2         3         4         5         6         7         8         9        10        11        12
             13        14        15
 
         1.27E-01  2.50E-01  3.16E-01  3.39E-01  4.50E-01  5.02E-01  6.10E-01  7.80E-01  9.46E-01  1.05E+00  1.37E+00  1.44E+00
          1.57E+00  1.75E+00  3.51E+00
 
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
 NO. OF FUNCT. EVALS. ALLOWED:            2808
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
 SIGDIGITS FOR MAP ESTIMATION (SIGLO):      4
 GRADIENT SIGDIGITS OF
       FIXED EFFECTS PARAMETERS (SIGL):     4
 NOPRIOR SETTING (NOPRIOR):                 1
 NOCOV SETTING (NOCOV):                     OFF
 DERCONT SETTING (DERCONT):                 OFF
 FINAL ETA RE-EVALUATION (FNLETA):          1
 EXCLUDE NON-INFLUENTIAL (NON-INFL.) ETAS
       IN SHRINKAGE (ETASTYPE):             NO
 NON-INFL. ETA CORRECTION (NONINFETA):      0
 RAW OUTPUT FILE (FILE): example1.ext
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
 CONVERGENCE INTERVAL (CINTERVAL):          100
 CONVERGENCE ITERATIONS (CITER):            10
 CONVERGENCE ALPHA ERROR (CALPHA):          5.000000000000000E-02
 BURN-IN ITERATIONS (NBURN):                3000
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
   1   2   3   4
 THETAS THAT ARE SIGMA-LIKE:
 
 
 MONITORING OF SEARCH:

 Stochastic/Burn-in Mode
 iteration        -3000  SAEMOBJ=  -2602.28072923062
 iteration        -2900  SAEMOBJ=  -2457.57363120941
 iteration        -2800  SAEMOBJ=  -2455.41475988551
 iteration        -2700  SAEMOBJ=  -2459.01189943552
 iteration        -2600  SAEMOBJ=  -2480.68753968814
 iteration        -2500  SAEMOBJ=  -2416.80559571603
 iteration        -2400  SAEMOBJ=  -2379.13932986121
 iteration        -2300  SAEMOBJ=  -2474.45163899019
 iteration        -2200  SAEMOBJ=  -2410.05112545377
 iteration        -2100  SAEMOBJ=  -2466.47073631892
 iteration        -2000  SAEMOBJ=  -2393.25781585732
 iteration        -1900  SAEMOBJ=  -2468.08542022404
 Convergence achieved
 Elapsed burn-in time in seconds:    54.09
 Reduced Stochastic/Accumulation Mode
 iteration            0  SAEMOBJ=  -2470.28644585330
 iteration          100  SAEMOBJ=  -2485.73230223923
 iteration          200  SAEMOBJ=  -2486.12683307251
 iteration          300  SAEMOBJ=  -2486.08489375104
 iteration          400  SAEMOBJ=  -2486.81732055626
 iteration          500  SAEMOBJ=  -2487.32251922320
 
 #TERM:
 STOCHASTIC PORTION WAS COMPLETED
 REDUCED STOCHASTIC PORTION WAS COMPLETED

 ETABAR IS THE ARITHMETIC MEAN OF THE ETA-ESTIMATES,
 AND THE P-VALUE IS GIVEN FOR THE NULL HYPOTHESIS THAT THE TRUE MEAN IS 0.
 
 ETABAR:         9.3484E-07 -3.8487E-05  2.3968E-05  4.0464E-06
 SE:             3.9080E-02  2.8495E-02  3.2810E-02  3.3269E-02
 N:                     100         100         100         100
 
 P VAL.:         9.9998E-01  9.9892E-01  9.9942E-01  9.9990E-01
 
 ETASHRINKSD(%)  3.5932E+00  2.3169E+01  2.7494E+01  1.6674E+01
 ETASHRINKVR(%)  7.0573E+00  4.0970E+01  4.7429E+01  3.0568E+01
 EBVSHRINKSD(%)  3.5918E+00  2.3173E+01  2.7493E+01  1.6680E+01
 EBVSHRINKVR(%)  7.0547E+00  4.0976E+01  4.7428E+01  3.0578E+01
 RELATIVEINF(%)  8.9381E+01  5.8137E+01  5.1589E+01  6.5762E+01
 EPSSHRINKSD(%)  2.9916E+01
 EPSSHRINKVR(%)  5.0882E+01
 
  
 TOTAL DATA POINTS NORMALLY DISTRIBUTED (N):          500
 N*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    918.938533204673     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -2487.32251922320     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -1568.38398601853     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 TOTAL EFFECTIVE ETAS (NIND*NETA):                           400
 NIND*NETA*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    735.150826563738     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -2487.32251922320     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -1752.17169265946     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 #TERE:
 Elapsed estimation  time in seconds:    79.08
 Elapsed covariance  time in seconds:     0.03
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************           STOCHASTIC APPROXIMATION EXPECTATION-MAXIMIZATION (NO PRIOR)         ********************
 #OBJT:**************                        FINAL VALUE OF LIKELIHOOD FUNCTION                      ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 





 #OBJV:********************************************         -2487.323       *********************************************
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************           STOCHASTIC APPROXIMATION EXPECTATION-MAXIMIZATION (NO PRIOR)         ********************
 ********************                             FINAL PARAMETER ESTIMATE                           ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4     
 
         1.63E+00  1.55E+00  7.45E-01  2.35E+00
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        1.64E-01
 
 ETA2
+       -3.98E-03  1.38E-01
 
 ETA3
+        1.49E-02 -3.93E-03  2.05E-01
 
 ETA4
+       -1.67E-02  1.22E-02  3.97E-02  1.59E-01
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        5.56E-02
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        4.05E-01
 
 ETA2
+       -2.65E-02  3.71E-01
 
 ETA3
+        8.12E-02 -2.34E-02  4.53E-01
 
 ETA4
+       -1.03E-01  8.24E-02  2.20E-01  3.99E-01
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        2.36E-01
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************           STOCHASTIC APPROXIMATION EXPECTATION-MAXIMIZATION (NO PRIOR)         ********************
 ********************                          STANDARD ERROR OF ESTIMATE (S)                        ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4     
 
         4.64E-02  5.33E-02  7.05E-02  5.47E-02
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        2.86E-02
 
 ETA2
+        2.38E-02  3.72E-02
 
 ETA3
+        3.36E-02  4.55E-02  7.12E-02
 
 ETA4
+        2.74E-02  3.00E-02  4.82E-02  4.11E-02
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        7.75E-03
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        3.53E-02
 
 ETA2
+        1.60E-01  5.01E-02
 
 ETA3
+        1.77E-01  2.72E-01  7.86E-02
 
 ETA4
+        1.74E-01  1.97E-01  2.21E-01  5.14E-02
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        1.64E-02
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************           STOCHASTIC APPROXIMATION EXPECTATION-MAXIMIZATION (NO PRIOR)         ********************
 ********************                        COVARIANCE MATRIX OF ESTIMATE (S)                       ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      OM11      OM12      OM13      OM14      OM22      OM23      OM24      OM33  
             OM34      OM44      SG11  
 
 TH 1
+        2.15E-03
 
 TH 2
+        3.59E-04  2.84E-03
 
 TH 3
+        5.46E-04 -7.31E-05  4.97E-03
 
 TH 4
+        6.45E-06  1.40E-04  1.75E-03  2.99E-03
 
 OM11
+       -2.83E-04  1.46E-04 -4.52E-06 -1.27E-04  8.19E-04
 
 OM12
+        2.76E-05  1.69E-04  2.80E-05 -1.23E-04  2.88E-04  5.68E-04
 
 OM13
+       -2.33E-04  3.87E-05 -5.37E-04  1.38E-04  1.92E-04  1.91E-04  1.13E-03
 
 OM14
+       -2.21E-04 -1.89E-04  4.49E-05  9.43E-05  2.19E-04  1.72E-04  5.11E-04  7.49E-04
 
 OM22
+       -1.11E-04 -3.35E-04 -3.16E-04 -2.67E-04  8.65E-05  1.67E-04 -6.58E-05  1.10E-04  1.38E-03
 
 OM23
+       -1.35E-04  1.84E-05  2.26E-04  1.69E-05  2.51E-04  6.81E-05 -1.39E-04 -3.33E-05  1.65E-04  2.07E-03
 
 OM24
+       -2.95E-04 -1.96E-04  1.93E-04  1.21E-04  1.31E-04  1.18E-04  9.95E-07  8.11E-05  3.80E-04  7.03E-04  9.02E-04
 
 OM33
+       -6.21E-04 -7.66E-04 -4.53E-04  4.00E-04  2.04E-04  7.94E-05  9.27E-04  3.84E-04  3.26E-04  1.22E-04  3.68E-04  5.07E-03
 
 OM34
+       -1.89E-04 -3.52E-04  1.81E-04  7.69E-05  2.25E-04  8.04E-05  2.55E-04  2.27E-04  2.84E-04  4.26E-04  2.99E-04  2.56E-03
          2.33E-03
 
 OM44
+       -8.95E-05 -1.71E-04  4.09E-05 -4.28E-04  2.31E-04  9.35E-05  2.66E-05  1.34E-04  3.12E-04  2.16E-04  2.61E-04  1.19E-03
          1.37E-03  1.69E-03
 
 SG11
+        4.41E-05  8.50E-05 -1.15E-05  3.03E-05 -5.95E-05 -3.92E-05 -4.64E-05 -7.40E-05 -7.72E-05 -2.94E-05 -5.23E-05 -1.06E-04
         -1.14E-04 -1.18E-04  6.00E-05
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************           STOCHASTIC APPROXIMATION EXPECTATION-MAXIMIZATION (NO PRIOR)         ********************
 ********************                        CORRELATION MATRIX OF ESTIMATE (S)                      ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      OM11      OM12      OM13      OM14      OM22      OM23      OM24      OM33  
             OM34      OM44      SG11  
 
 TH 1
+        4.64E-02
 
 TH 2
+        1.45E-01  5.33E-02
 
 TH 3
+        1.67E-01 -1.94E-02  7.05E-02
 
 TH 4
+        2.54E-03  4.79E-02  4.54E-01  5.47E-02
 
 OM11
+       -2.13E-01  9.57E-02 -2.24E-03 -8.12E-02  2.86E-02
 
 OM12
+        2.50E-02  1.33E-01  1.66E-02 -9.44E-02  4.23E-01  2.38E-02
 
 OM13
+       -1.49E-01  2.16E-02 -2.26E-01  7.49E-02  1.99E-01  2.38E-01  3.36E-02
 
 OM14
+       -1.74E-01 -1.30E-01  2.33E-02  6.30E-02  2.80E-01  2.63E-01  5.55E-01  2.74E-02
 
 OM22
+       -6.45E-02 -1.69E-01 -1.21E-01 -1.31E-01  8.13E-02  1.89E-01 -5.26E-02  1.08E-01  3.72E-02
 
 OM23
+       -6.37E-02  7.60E-03  7.04E-02  6.77E-03  1.92E-01  6.28E-02 -9.07E-02 -2.67E-02  9.75E-02  4.55E-02
 
 OM24
+       -2.12E-01 -1.22E-01  9.09E-02  7.35E-02  1.52E-01  1.65E-01  9.85E-04  9.87E-02  3.41E-01  5.14E-01  3.00E-02
 
 OM33
+       -1.88E-01 -2.02E-01 -9.04E-02  1.03E-01  1.00E-01  4.68E-02  3.87E-01  1.97E-01  1.23E-01  3.75E-02  1.72E-01  7.12E-02
 
 OM34
+       -8.44E-02 -1.37E-01  5.32E-02  2.91E-02  1.63E-01  7.00E-02  1.57E-01  1.72E-01  1.58E-01  1.94E-01  2.06E-01  7.46E-01
          4.82E-02
 
 OM44
+       -4.70E-02 -7.83E-02  1.41E-02 -1.90E-01  1.97E-01  9.55E-02  1.93E-02  1.20E-01  2.04E-01  1.15E-01  2.11E-01  4.06E-01
          6.94E-01  4.11E-02
 
 SG11
+        1.23E-01  2.06E-01 -2.11E-02  7.14E-02 -2.68E-01 -2.12E-01 -1.78E-01 -3.49E-01 -2.68E-01 -8.34E-02 -2.25E-01 -1.91E-01
         -3.05E-01 -3.72E-01  7.75E-03
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************           STOCHASTIC APPROXIMATION EXPECTATION-MAXIMIZATION (NO PRIOR)         ********************
 ********************                    INVERSE COVARIANCE MATRIX OF ESTIMATE (S)                   ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      OM11      OM12      OM13      OM14      OM22      OM23      OM24      OM33  
             OM34      OM44      SG11  
 
 TH 1
+        5.71E+02
 
 TH 2
+       -4.72E+01  4.33E+02
 
 TH 3
+       -7.57E+01  3.15E+01  3.23E+02
 
 TH 4
+        2.48E+01 -5.95E+01 -2.00E+02  5.15E+02
 
 OM11
+        2.29E+02 -1.08E+02 -3.06E+01  5.08E+01  1.78E+03
 
 OM12
+       -1.75E+02 -1.55E+02 -8.85E+01  1.47E+02 -7.31E+02  2.51E+03
 
 OM13
+       -3.03E+01 -1.17E+02  2.24E+02 -1.15E+02 -3.96E+01 -3.21E+02  1.88E+03
 
 OM14
+        1.14E+02  1.50E+02 -1.32E+02 -1.30E+01 -2.25E+02 -1.59E+02 -1.11E+03  2.39E+03
 
 OM22
+       -3.68E+01  5.99E+01  1.01E+02  1.74E+01  4.73E+01 -2.38E+02  2.54E+02 -1.42E+02  9.62E+02
 
 OM23
+       -3.36E+01 -1.85E+01  1.16E+01  2.81E+01 -2.01E+02  9.07E+01  3.79E+01  7.31E+01  7.26E+01  7.47E+02
 
 OM24
+        2.05E+02  4.77E+01 -7.01E+01 -1.06E+02  1.23E+02 -2.77E+02  2.14E+01 -7.19E-01 -3.85E+02 -6.25E+02  1.95E+03
 
 OM33
+        6.65E+01  9.70E+01  5.35E+01 -6.21E+01  5.34E+00  2.33E+01 -4.08E+02  1.86E+02 -3.31E+01  1.22E+02 -1.32E+02  6.52E+02
 
 OM34
+       -6.01E+01 -3.68E+01 -6.93E+01 -6.08E+01  3.62E+00 -1.22E+01  2.69E+02 -1.91E+02 -5.37E+00 -3.01E+02  2.48E+02 -7.82E+02
          1.90E+03
 
 OM44
+       -2.02E+01 -5.43E+01 -2.82E+01  2.18E+02 -1.25E+02  9.16E+01  7.72E+01  2.13E+01 -3.29E+01  1.59E+02 -2.39E+02  1.51E+02
         -9.73E+02  1.44E+03
 
 SG11
+       -7.23E+01 -5.53E+02  7.56E+01 -4.14E+01  7.70E+02  4.20E+02  4.23E+02  1.31E+03  6.69E+02 -1.12E+02  4.44E+02 -3.49E+02
          4.53E+02  1.08E+03  2.41E+04
 
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************           STOCHASTIC APPROXIMATION EXPECTATION-MAXIMIZATION (NO PRIOR)         ********************
 ********************                    EIGENVALUES OF COR MATRIX OF ESTIMATE (S)                   ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

             1         2         3         4         5         6         7         8         9        10        11        12
             13        14        15
 
         1.30E-01  2.80E-01  3.28E-01  3.80E-01  5.04E-01  5.17E-01  6.64E-01  7.60E-01  8.86E-01  1.13E+00  1.26E+00  1.54E+00
          1.59E+00  1.75E+00  3.27E+00
 
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
 NO. OF FUNCT. EVALS. ALLOWED:            2808
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
 RAW OUTPUT FILE (FILE): example1.ext
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
 ITERATIONS (NITER):                        5
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
   1   2   3   4
 THETAS THAT ARE SIGMA-LIKE:
 
 
 MONITORING OF SEARCH:

 iteration            0  OBJ=  -1145.09365836412 eff.=    2275. Smpl.=    3000. Fit.= 0.94042
 iteration            1  OBJ=  -1145.40936945063 eff.=    1312. Smpl.=    3000. Fit.= 0.91727
 iteration            2  OBJ=  -1145.61448062557 eff.=    1190. Smpl.=    3000. Fit.= 0.91200
 iteration            3  OBJ=  -1145.16880407627 eff.=    1182. Smpl.=    3000. Fit.= 0.91261
 iteration            4  OBJ=  -1145.74001404735 eff.=    1219. Smpl.=    3000. Fit.= 0.91425
 iteration            5  OBJ=  -1145.19432180534 eff.=    1207. Smpl.=    3000. Fit.= 0.91330
 
 #TERM:
 EXPECTATION ONLY PROCESS WAS NOT COMPLETED


 ETABAR IS THE ARITHMETIC MEAN OF THE ETA-ESTIMATES,
 AND THE P-VALUE IS GIVEN FOR THE NULL HYPOTHESIS THAT THE TRUE MEAN IS 0.
 
 ETABAR:         6.8960E-04  1.2432E-03  1.3169E-03  3.8609E-04
 SE:             3.9037E-02  2.8718E-02  3.2840E-02  3.3184E-02
 N:                     100         100         100         100
 
 P VAL.:         9.8591E-01  9.6547E-01  9.6801E-01  9.9072E-01
 
 ETASHRINKSD(%)  3.6979E+00  2.2569E+01  2.7428E+01  1.6888E+01
 ETASHRINKVR(%)  7.2591E+00  4.0044E+01  4.7334E+01  3.0924E+01
 EBVSHRINKSD(%)  3.5908E+00  2.3257E+01  2.6701E+01  1.6445E+01
 EBVSHRINKVR(%)  7.0527E+00  4.1104E+01  4.6273E+01  3.0186E+01
 RELATIVEINF(%)  8.9367E+01  5.7974E+01  5.2825E+01  6.6332E+01
 EPSSHRINKSD(%)  3.0518E+01
 EPSSHRINKVR(%)  5.1722E+01
 
  
 TOTAL DATA POINTS NORMALLY DISTRIBUTED (N):          500
 N*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    918.938533204673     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -1145.19432180534     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -226.255788600668     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 TOTAL EFFECTIVE ETAS (NIND*NETA):                           400
  
 #TERE:
 Elapsed estimation  time in seconds:    21.91
 Elapsed covariance  time in seconds:     5.49
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************         OBJECTIVE FUNCTION EVALUATION BY IMPORTANCE SAMPLING (NO PRIOR)        ********************
 #OBJT:**************                        FINAL VALUE OF OBJECTIVE FUNCTION                       ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 





 #OBJV:********************************************         -1145.194       *********************************************
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************         OBJECTIVE FUNCTION EVALUATION BY IMPORTANCE SAMPLING (NO PRIOR)        ********************
 ********************                             FINAL PARAMETER ESTIMATE                           ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4     
 
         1.63E+00  1.55E+00  7.45E-01  2.35E+00
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        1.64E-01
 
 ETA2
+       -3.98E-03  1.38E-01
 
 ETA3
+        1.49E-02 -3.93E-03  2.05E-01
 
 ETA4
+       -1.67E-02  1.22E-02  3.97E-02  1.59E-01
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        5.56E-02
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        4.05E-01
 
 ETA2
+       -2.65E-02  3.71E-01
 
 ETA3
+        8.12E-02 -2.34E-02  4.53E-01
 
 ETA4
+       -1.03E-01  8.24E-02  2.20E-01  3.99E-01
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        2.36E-01
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************         OBJECTIVE FUNCTION EVALUATION BY IMPORTANCE SAMPLING (NO PRIOR)        ********************
 ********************                          STANDARD ERROR OF ESTIMATE (R)                        ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4     
 
         4.30E-02  5.07E-02  6.80E-02  5.21E-02
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        2.70E-02
 
 ETA2
+        2.17E-02  3.35E-02
 
 ETA3
+        2.92E-02  3.57E-02  6.24E-02
 
 ETA4
+        2.34E-02  2.53E-02  3.71E-02  3.80E-02
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        6.63E-03
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        3.33E-02
 
 ETA2
+        1.45E-01  4.52E-02
 
 ETA3
+        1.54E-01  2.12E-01  6.89E-02
 
 ETA4
+        1.48E-01  1.68E-01  1.70E-01  4.76E-02
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        1.41E-02
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************         OBJECTIVE FUNCTION EVALUATION BY IMPORTANCE SAMPLING (NO PRIOR)        ********************
 ********************                        COVARIANCE MATRIX OF ESTIMATE (R)                       ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      OM11      OM12      OM13      OM14      OM22      OM23      OM24      OM33  
             OM34      OM44      SG11  
 
 TH 1
+        1.85E-03
 
 TH 2
+        2.40E-04  2.57E-03
 
 TH 3
+        5.41E-04  1.59E-04  4.63E-03
 
 TH 4
+        1.57E-04  3.61E-04  1.77E-03  2.72E-03
 
 OM11
+        5.18E-05  1.38E-04  1.35E-04  1.17E-04  7.27E-04
 
 OM12
+        5.80E-05  1.37E-04  1.92E-04  1.13E-04  1.04E-04  4.72E-04
 
 OM13
+        6.62E-05  1.63E-04 -2.42E-05  1.20E-04  2.57E-04  6.79E-05  8.54E-04
 
 OM14
+        5.93E-05  1.11E-04  1.31E-04  9.41E-05  1.08E-04  1.02E-04  3.63E-04  5.45E-04
 
 OM22
+       -7.64E-05 -2.42E-04 -3.86E-04 -2.00E-04 -3.99E-05  1.91E-05 -3.83E-05 -1.73E-05  1.12E-03
 
 OM23
+        5.58E-05  2.54E-04  5.87E-04  1.37E-04  5.72E-05  2.09E-04  2.26E-05  4.75E-05 -1.50E-04  1.27E-03
 
 OM24
+        5.27E-06 -3.27E-05  2.01E-04  1.42E-04  1.95E-05  5.41E-05  2.56E-05  5.99E-05  1.27E-04  3.58E-04  6.40E-04
 
 OM33
+        4.23E-05 -1.76E-04  2.14E-04  1.39E-04  1.64E-04  8.76E-05  5.91E-04  3.15E-04 -2.23E-05  1.18E-06  1.03E-04  3.89E-03
 
 OM34
+        2.90E-05 -4.36E-05 -4.09E-05 -7.31E-05  7.67E-05  6.27E-05  2.51E-04  2.48E-04  3.51E-05  6.25E-05  1.04E-04  1.53E-03
          1.38E-03
 
 OM44
+        1.86E-05  5.99E-05 -1.75E-04  6.48E-08  5.53E-05  4.66E-05  1.41E-04  1.35E-04  1.05E-04  2.71E-05  1.97E-04  6.32E-04
          8.77E-04  1.45E-03
 
 SG11
+        4.79E-06  2.49E-05  2.31E-05  1.27E-05 -1.45E-05 -1.45E-05 -2.79E-05 -2.60E-05 -4.88E-05  2.85E-06 -1.31E-05 -1.35E-04
         -8.33E-05 -7.13E-05  4.39E-05
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************         OBJECTIVE FUNCTION EVALUATION BY IMPORTANCE SAMPLING (NO PRIOR)        ********************
 ********************                        CORRELATION MATRIX OF ESTIMATE (R)                      ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      OM11      OM12      OM13      OM14      OM22      OM23      OM24      OM33  
             OM34      OM44      SG11  
 
 TH 1
+        4.30E-02
 
 TH 2
+        1.10E-01  5.07E-02
 
 TH 3
+        1.85E-01  4.62E-02  6.80E-02
 
 TH 4
+        7.03E-02  1.37E-01  4.98E-01  5.21E-02
 
 OM11
+        4.47E-02  1.01E-01  7.35E-02  8.33E-02  2.70E-02
 
 OM12
+        6.22E-02  1.24E-01  1.30E-01  9.95E-02  1.78E-01  2.17E-02
 
 OM13
+        5.27E-02  1.10E-01 -1.21E-02  7.89E-02  3.26E-01  1.07E-01  2.92E-02
 
 OM14
+        5.91E-02  9.33E-02  8.27E-02  7.73E-02  1.72E-01  2.01E-01  5.32E-01  2.34E-02
 
 OM22
+       -5.31E-02 -1.43E-01 -1.69E-01 -1.15E-01 -4.41E-02  2.63E-02 -3.91E-02 -2.21E-02  3.35E-02
 
 OM23
+        3.64E-02  1.41E-01  2.42E-01  7.35E-02  5.95E-02  2.70E-01  2.17E-02  5.71E-02 -1.25E-01  3.57E-02
 
 OM24
+        4.85E-03 -2.55E-02  1.17E-01  1.07E-01  2.85E-02  9.84E-02  3.46E-02  1.01E-01  1.50E-01  3.97E-01  2.53E-02
 
 OM33
+        1.58E-02 -5.56E-02  5.05E-02  4.28E-02  9.76E-02  6.46E-02  3.24E-01  2.17E-01 -1.07E-02  5.31E-04  6.55E-02  6.24E-02
 
 OM34
+        1.82E-02 -2.31E-02 -1.62E-02 -3.78E-02  7.65E-02  7.76E-02  2.31E-01  2.86E-01  2.82E-02  4.72E-02  1.11E-01  6.60E-01
          3.71E-02
 
 OM44
+        1.14E-02  3.11E-02 -6.75E-02  3.27E-05  5.39E-02  5.64E-02  1.27E-01  1.52E-01  8.26E-02  2.00E-02  2.04E-01  2.67E-01
          6.21E-01  3.80E-02
 
 SG11
+        1.68E-02  7.42E-02  5.13E-02  3.69E-02 -8.10E-02 -1.00E-01 -1.44E-01 -1.68E-01 -2.20E-01  1.21E-02 -7.84E-02 -3.27E-01
         -3.38E-01 -2.83E-01  6.63E-03
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************         OBJECTIVE FUNCTION EVALUATION BY IMPORTANCE SAMPLING (NO PRIOR)        ********************
 ********************                    INVERSE COVARIANCE MATRIX OF ESTIMATE (R)                   ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      OM11      OM12      OM13      OM14      OM22      OM23      OM24      OM33  
             OM34      OM44      SG11  
 
 TH 1
+        5.71E+02
 
 TH 2
+       -4.82E+01  4.32E+02
 
 TH 3
+       -7.60E+01  3.02E+01  3.28E+02
 
 TH 4
+        2.51E+01 -6.08E+01 -2.00E+02  5.17E+02
 
 OM11
+       -5.24E+00 -3.85E+01 -3.43E+01 -1.09E+01  1.59E+03
 
 OM12
+       -3.52E+01 -7.15E+01 -2.50E+01 -5.01E+01 -2.70E+02  2.48E+03
 
 OM13
+       -3.36E+01 -5.39E+01  9.84E+01 -6.59E+01 -4.84E+02  1.06E+02  1.95E+03
 
 OM14
+       -9.51E+00 -4.36E+01 -8.41E+01  1.13E+01  8.36E+01 -4.04E+02 -1.15E+03  2.84E+03
 
 OM22
+        7.83E+00  6.15E+01  5.74E+01  3.20E+01  4.07E+01 -1.34E+02  1.82E+01  4.99E+01  1.05E+03
 
 OM23
+        2.46E+01 -9.11E+01 -1.18E+02  8.54E+01  1.10E+01 -4.02E+02 -3.77E+01  1.10E+02  1.84E+02  1.12E+03
 
 OM24
+        3.79E+00  8.19E+01 -6.37E+00 -9.88E+01 -1.10E+01  1.16E+02  6.51E+01 -2.11E+02 -3.12E+02 -6.44E+02  2.10E+03
 
 OM33
+        4.93E+00  2.64E+01 -2.38E+01 -2.84E+01  2.17E+01 -2.50E+01 -2.62E+02  1.57E+02  4.44E+01  5.98E+01 -6.08E+01  5.48E+02
 
 OM34
+       -8.51E+00  2.20E+01 -3.20E-01  9.72E+01 -8.26E+00  3.95E+01  2.03E+02 -5.06E+02  7.73E+00 -1.36E+02  1.62E+02 -6.88E+02
          2.20E+03
 
 OM44
+       -1.01E+01 -4.88E+01  4.63E+01 -5.45E+01 -6.43E+00 -2.93E+01 -7.47E+01  1.43E+02 -4.56E+00  1.07E+02 -3.24E+02  2.23E+02
         -1.02E+03  1.30E+03
 
 SG11
+       -5.15E+01 -1.92E+02 -5.99E+01 -5.94E+01  3.04E+02  4.91E+02 -6.76E+01  6.53E+02  1.14E+03  3.67E+01 -1.47E+02  7.03E+02
          2.74E+02  8.00E+02  2.87E+04
 
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************         OBJECTIVE FUNCTION EVALUATION BY IMPORTANCE SAMPLING (NO PRIOR)        ********************
 ********************                    EIGENVALUES OF COR MATRIX OF ESTIMATE (R)                   ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

             1         2         3         4         5         6         7         8         9        10        11        12
             13        14        15
 
         1.92E-01  3.69E-01  4.47E-01  5.09E-01  6.32E-01  7.04E-01  8.20E-01  8.77E-01  8.98E-01  9.89E-01  1.08E+00  1.24E+00
          1.44E+00  2.01E+00  2.79E+00
 
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
 NO. OF FUNCT. EVALS. ALLOWED:            2808
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
 NOPRIOR SETTING (NOPRIOR):                 0
 NOCOV SETTING (NOCOV):                     OFF
 DERCONT SETTING (DERCONT):                 OFF
 FINAL ETA RE-EVALUATION (FNLETA):          1
 EXCLUDE NON-INFLUENTIAL (NON-INFL.) ETAS
       IN SHRINKAGE (ETASTYPE):             NO
 NON-INFL. ETA CORRECTION (NONINFETA):      0
 RAW OUTPUT FILE (FILE): example1.txt
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
 iteration       -10000 MCMCOBJ=   -2343.67061221172     
 iteration        -9900 MCMCOBJ=   -2315.86368845333     
 iteration        -9800 MCMCOBJ=   -2267.72668463462     
 iteration        -9700 MCMCOBJ=   -2342.39906051930     
 iteration        -9600 MCMCOBJ=   -2254.08908062694     
 iteration        -9500 MCMCOBJ=   -2398.32689317793     
 iteration        -9400 MCMCOBJ=   -2244.27643320589     
 iteration        -9300 MCMCOBJ=   -2332.09692913464     
 iteration        -9200 MCMCOBJ=   -2353.93291415776     
 iteration        -9100 MCMCOBJ=   -2357.51823315748     
 iteration        -9000 MCMCOBJ=   -2237.40298766950     
 Convergence achieved
 Elapsed burn-in time in seconds:    24.48
 Sampling Mode
 iteration            0 MCMCOBJ=   -2258.12269703080     
 iteration          100 MCMCOBJ=   -2247.78120042612     
 iteration          200 MCMCOBJ=   -2330.85002208016     
 iteration          300 MCMCOBJ=   -2357.75897193827     
 iteration          400 MCMCOBJ=   -2329.42871641528     
 iteration          500 MCMCOBJ=   -2186.50486250035     
 iteration          600 MCMCOBJ=   -2400.59039205568     
 iteration          700 MCMCOBJ=   -2284.19062809383     
 iteration          800 MCMCOBJ=   -2272.69947854739     
 iteration          900 MCMCOBJ=   -2270.48058891863     
 iteration         1000 MCMCOBJ=   -2336.13443153945     
 iteration         1100 MCMCOBJ=   -2283.76263213153     
 iteration         1200 MCMCOBJ=   -2307.02258522535     
 iteration         1300 MCMCOBJ=   -2318.64579583331     
 iteration         1400 MCMCOBJ=   -2299.54991254100     
 iteration         1500 MCMCOBJ=   -2351.93217579933     
 iteration         1600 MCMCOBJ=   -2286.63296507387     
 iteration         1700 MCMCOBJ=   -2410.99148013215     
 iteration         1800 MCMCOBJ=   -2321.75947738542     
 iteration         1900 MCMCOBJ=   -2364.40213698683     
 iteration         2000 MCMCOBJ=   -2267.72890071515     
 iteration         2100 MCMCOBJ=   -2281.70359411955     
 iteration         2200 MCMCOBJ=   -2278.97082841418     
 iteration         2300 MCMCOBJ=   -2277.75994567836     
 iteration         2400 MCMCOBJ=   -2292.09615549239     
 iteration         2500 MCMCOBJ=   -2331.10853650432     
 iteration         2600 MCMCOBJ=   -2268.99977716772     
 iteration         2700 MCMCOBJ=   -2302.22961617273     
 iteration         2800 MCMCOBJ=   -2375.00361901043     
 iteration         2900 MCMCOBJ=   -2270.00633456943     
 iteration         3000 MCMCOBJ=   -2289.76284247441     
 iteration         3100 MCMCOBJ=   -2339.34346628437     
 iteration         3200 MCMCOBJ=   -2317.13523935265     
 iteration         3300 MCMCOBJ=   -2363.29764597905     
 iteration         3400 MCMCOBJ=   -2243.85560962243     
 iteration         3500 MCMCOBJ=   -2271.22953558772     
 iteration         3600 MCMCOBJ=   -2325.79007162643     
 iteration         3700 MCMCOBJ=   -2337.21161303202     
 iteration         3800 MCMCOBJ=   -2342.08805294448     
 iteration         3900 MCMCOBJ=   -2323.09567557402     
 iteration         4000 MCMCOBJ=   -2319.68532294434     
 iteration         4100 MCMCOBJ=   -2333.74348527069     
 iteration         4200 MCMCOBJ=   -2288.90822482883     
 iteration         4300 MCMCOBJ=   -2357.45323399659     
 iteration         4400 MCMCOBJ=   -2309.30706103997     
 iteration         4500 MCMCOBJ=   -2387.28654350006     
 iteration         4600 MCMCOBJ=   -2303.81895263335     
 iteration         4700 MCMCOBJ=   -2312.92094627746     
 iteration         4800 MCMCOBJ=   -2295.99782166321     
 iteration         4900 MCMCOBJ=   -2262.26776579696     
 iteration         5000 MCMCOBJ=   -2262.20282620020     
 iteration         5100 MCMCOBJ=   -2348.27452955566     
 iteration         5200 MCMCOBJ=   -2256.77782433495     
 iteration         5300 MCMCOBJ=   -2305.04216533061     
 iteration         5400 MCMCOBJ=   -2333.51002273864     
 iteration         5500 MCMCOBJ=   -2340.41071229761     
 iteration         5600 MCMCOBJ=   -2240.03387851987     
 iteration         5700 MCMCOBJ=   -2321.92323642310     
 iteration         5800 MCMCOBJ=   -2305.80662035244     
 iteration         5900 MCMCOBJ=   -2293.85613210577     
 iteration         6000 MCMCOBJ=   -2314.36355927157     
 iteration         6100 MCMCOBJ=   -2212.92578496600     
 iteration         6200 MCMCOBJ=   -2211.13788580589     
 iteration         6300 MCMCOBJ=   -2358.09466620844     
 iteration         6400 MCMCOBJ=   -2293.35874386211     
 iteration         6500 MCMCOBJ=   -2250.83333756748     
 iteration         6600 MCMCOBJ=   -2297.96822709071     
 iteration         6700 MCMCOBJ=   -2289.12584916103     
 iteration         6800 MCMCOBJ=   -2393.66842355463     
 iteration         6900 MCMCOBJ=   -2339.91165993694     
 iteration         7000 MCMCOBJ=   -2317.83555515618     
 iteration         7100 MCMCOBJ=   -2266.61798364036     
 iteration         7200 MCMCOBJ=   -2388.42757115341     
 iteration         7300 MCMCOBJ=   -2328.77029012738     
 iteration         7400 MCMCOBJ=   -2262.04362477172     
 iteration         7500 MCMCOBJ=   -2260.17497829165     
 iteration         7600 MCMCOBJ=   -2264.27130617470     
 iteration         7700 MCMCOBJ=   -2281.30213647908     
 iteration         7800 MCMCOBJ=   -2293.16163803320     
 iteration         7900 MCMCOBJ=   -2284.25278664547     
 iteration         8000 MCMCOBJ=   -2324.00132113970     
 iteration         8100 MCMCOBJ=   -2320.06072536540     
 iteration         8200 MCMCOBJ=   -2258.32956678887     
 iteration         8300 MCMCOBJ=   -2247.34022317821     
 iteration         8400 MCMCOBJ=   -2264.99387425181     
 iteration         8500 MCMCOBJ=   -2369.41111264806     
 iteration         8600 MCMCOBJ=   -2297.83129707211     
 iteration         8700 MCMCOBJ=   -2284.34292076893     
 iteration         8800 MCMCOBJ=   -2347.99687465379     
 iteration         8900 MCMCOBJ=   -2310.87235783105     
 iteration         9000 MCMCOBJ=   -2254.73354852384     
 iteration         9100 MCMCOBJ=   -2356.56058508336     
 iteration         9200 MCMCOBJ=   -2206.68734936409     
 iteration         9300 MCMCOBJ=   -2336.63255937056     
 iteration         9400 MCMCOBJ=   -2209.82223581114     
 iteration         9500 MCMCOBJ=   -2363.73616794886     
 iteration         9600 MCMCOBJ=   -2253.18574827628     
 iteration         9700 MCMCOBJ=   -2316.74708116431     
 iteration         9800 MCMCOBJ=   -2326.23253777763     
 iteration         9900 MCMCOBJ=   -2317.90926912050     
 iteration        10000 MCMCOBJ=   -2317.07157043477     
 
 #TERM:
 BURN-IN WAS COMPLETED
 STATISTICAL PORTION WAS COMPLETED

 ETABAR IS THE ARITHMETIC MEAN OF THE ETA-ESTIMATES,
 AND THE P-VALUE IS GIVEN FOR THE NULL HYPOTHESIS THAT THE TRUE MEAN IS 0.
 
 ETABAR:        -8.6288E-04 -5.4136E-04 -5.0314E-04 -7.1990E-05
 SE:             3.8880E-02  2.8392E-02  2.9404E-02  3.1978E-02
 N:                     100         100         100         100
 
 P VAL.:         9.8229E-01  9.8479E-01  9.8635E-01  9.9820E-01
 
 ETASHRINKSD(%)  6.4782E+00  2.7098E+01  3.2399E+01  2.0087E+01
 ETASHRINKVR(%)  1.2537E+01  4.6853E+01  5.4301E+01  3.6139E+01
 EBVSHRINKSD(%)  3.6332E+00  2.3544E+01  2.9686E+01  1.6856E+01
 EBVSHRINKVR(%)  7.1344E+00  4.1546E+01  5.0559E+01  3.0871E+01
 RELATIVEINF(%)  8.8270E+01  5.7609E+01  4.5419E+01  6.1001E+01
 EPSSHRINKSD(%)  2.9992E+01
 EPSSHRINKVR(%)  5.0988E+01
 
  
 TOTAL DATA POINTS NORMALLY DISTRIBUTED (N):          500
 N*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    918.938533204673     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -2307.60369492595     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -1388.66516172128     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 TOTAL EFFECTIVE ETAS (NIND*NETA):                           400
 NIND*NETA*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    735.150826563738     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -2307.60369492595     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -1572.45286836221     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 PRIOR CONSTANT TO OBJECTIVE FUNCTION:    71.2763539723734     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -2307.60369492595     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -2236.32734095358     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 #TERE:
 Elapsed estimation  time in seconds:   273.74
 Elapsed covariance  time in seconds:     0.00
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 #OBJT:**************                       AVERAGE VALUE OF LIKELIHOOD FUNCTION                     ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 





 #OBJV:********************************************         -2307.604       *********************************************
 #OBJS:********************************************            43.175 (STD) *********************************************
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 ********************                             FINAL PARAMETER ESTIMATE                           ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4     
 
         1.63E+00  1.56E+00  7.45E-01  2.35E+00
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        1.73E-01
 
 ETA2
+       -2.84E-03  1.52E-01
 
 ETA3
+        9.28E-03 -5.48E-03  1.89E-01
 
 ETA4
+       -2.03E-02  1.40E-02  2.22E-02  1.60E-01
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        5.98E-02
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        4.14E-01
 
 ETA2
+       -2.02E-02  3.87E-01
 
 ETA3
+        4.47E-02 -3.23E-02  4.30E-01
 
 ETA4
+       -1.27E-01  8.63E-02  1.08E-01  3.98E-01
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        2.44E-01
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 ********************                STANDARD ERROR OF ESTIMATE (From Sample Variance)               ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4     
 
         4.46E-02  5.30E-02  6.96E-02  5.32E-02
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        2.81E-02
 
 ETA2
+        2.26E-02  3.47E-02
 
 ETA3
+        2.76E-02  3.35E-02  5.76E-02
 
 ETA4
+        2.28E-02  2.55E-02  3.28E-02  3.65E-02
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        7.12E-03
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        3.34E-02
 
 ETA2
+        1.37E-01  4.41E-02
 
 ETA3
+        1.49E-01  1.94E-01  6.55E-02
 
 ETA4
+        1.37E-01  1.57E-01  1.73E-01  4.51E-02
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        1.45E-02
 
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
+        2.54E-04  2.81E-03
 
 TH 3
+        5.94E-04  1.19E-04  4.84E-03
 
 TH 4
+        1.75E-04  3.51E-04  1.75E-03  2.83E-03
 
 OM11
+        5.90E-05  1.34E-04  1.94E-04  1.47E-04  7.91E-04
 
 OM12
+        5.67E-05  1.11E-04  1.84E-04  1.06E-04  1.01E-04  5.10E-04
 
 OM13
+        5.98E-05  1.22E-04  5.55E-05  1.34E-04  1.94E-04  6.05E-05  7.62E-04
 
 OM14
+        4.71E-05  8.43E-05  1.78E-04  9.65E-05  5.99E-05  9.86E-05  2.75E-04  5.22E-04
 
 OM22
+       -6.42E-05 -2.45E-04 -3.59E-04 -1.51E-04 -3.14E-05  1.94E-05 -4.71E-05 -1.20E-05  1.20E-03
 
 OM23
+        7.92E-05  2.23E-04  5.82E-04  1.35E-04  6.39E-05  1.63E-04  6.14E-05  4.64E-05 -1.53E-04  1.12E-03
 
 OM24
+        1.66E-05 -6.25E-05  2.00E-04  1.80E-04  9.47E-06  1.20E-05 -6.36E-07  4.18E-05  1.56E-04  2.41E-04  6.50E-04
 
 OM33
+        1.16E-04 -1.83E-04  4.86E-04  2.66E-04  1.27E-04  7.11E-05  3.90E-04  1.68E-04  2.92E-06  7.00E-05  5.13E-05  3.32E-03
 
 OM34
+        3.83E-05 -4.32E-05  4.56E-05 -5.85E-05  4.47E-05  5.34E-05  1.01E-04  1.39E-04  2.00E-05  7.56E-05  6.25E-05  1.06E-03
          1.08E-03
 
 OM44
+        1.98E-05  9.89E-06 -1.42E-04  3.13E-05  2.84E-05  2.63E-05  2.05E-05  3.19E-05  1.24E-04  5.78E-07  2.09E-04  3.44E-04
          6.09E-04  1.34E-03
 
 SG11
+        2.31E-06  3.81E-05  2.06E-05  1.37E-05 -1.09E-05 -1.25E-05 -1.34E-05 -1.60E-05 -4.94E-05  4.65E-06 -1.14E-05 -1.10E-04
         -6.44E-05 -6.16E-05  5.07E-05
 
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
+        1.08E-01  5.30E-02
 
 TH 3
+        1.91E-01  3.24E-02  6.96E-02
 
 TH 4
+        7.38E-02  1.25E-01  4.72E-01  5.32E-02
 
 OM11
+        4.70E-02  9.00E-02  9.90E-02  9.81E-02  2.81E-02
 
 OM12
+        5.63E-02  9.29E-02  1.17E-01  8.85E-02  1.59E-01  2.26E-02
 
 OM13
+        4.85E-02  8.36E-02  2.89E-02  9.12E-02  2.50E-01  9.72E-02  2.76E-02
 
 OM14
+        4.62E-02  6.96E-02  1.12E-01  7.94E-02  9.32E-02  1.91E-01  4.36E-01  2.28E-02
 
 OM22
+       -4.15E-02 -1.33E-01 -1.49E-01 -8.17E-02 -3.22E-02  2.48E-02 -4.92E-02 -1.51E-02  3.47E-02
 
 OM23
+        5.30E-02  1.26E-01  2.50E-01  7.56E-02  6.79E-02  2.15E-01  6.65E-02  6.07E-02 -1.32E-01  3.35E-02
 
 OM24
+        1.46E-02 -4.63E-02  1.13E-01  1.33E-01  1.32E-02  2.09E-02 -9.04E-04  7.17E-02  1.76E-01  2.82E-01  2.55E-02
 
 OM33
+        4.50E-02 -6.00E-02  1.21E-01  8.68E-02  7.85E-02  5.46E-02  2.45E-01  1.28E-01  1.46E-03  3.63E-02  3.49E-02  5.76E-02
 
 OM34
+        2.62E-02 -2.48E-02  2.00E-02 -3.35E-02  4.84E-02  7.20E-02  1.11E-01  1.85E-01  1.76E-02  6.88E-02  7.47E-02  5.60E-01
          3.28E-02
 
 OM44
+        1.21E-02  5.11E-03 -5.59E-02  1.61E-02  2.77E-02  3.19E-02  2.03E-02  3.83E-02  9.80E-02  4.72E-04  2.24E-01  1.63E-01
          5.08E-01  3.65E-02
 
 SG11
+        7.26E-03  1.01E-01  4.15E-02  3.62E-02 -5.45E-02 -7.75E-02 -6.83E-02 -9.86E-02 -2.00E-01  1.95E-02 -6.27E-02 -2.69E-01
         -2.75E-01 -2.37E-01  7.12E-03
 
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
+        5.29E+02
 
 TH 2
+       -4.61E+01  3.90E+02
 
 TH 3
+       -7.00E+01  2.94E+01  3.06E+02
 
 TH 4
+        1.94E+01 -5.59E+01 -1.73E+02  4.82E+02
 
 OM11
+       -7.97E+00 -4.23E+01 -3.78E+01 -2.26E+01  1.40E+03
 
 OM12
+       -2.59E+01 -4.53E+01 -2.05E+01 -4.46E+01 -2.19E+02  2.21E+03
 
 OM13
+       -2.55E+01 -2.97E+01  7.89E+01 -5.63E+01 -3.59E+02  6.03E+01  1.84E+03
 
 OM14
+        1.01E+00 -4.11E+01 -8.48E+01  6.58E+00  1.05E+02 -3.71E+02 -9.28E+02  2.58E+03
 
 OM22
+        2.34E+00  5.12E+01  5.22E+01  2.07E+01  1.58E+01 -1.09E+02  3.63E+01  1.34E+01  9.56E+02
 
 OM23
+        1.26E+01 -7.94E+01 -1.22E+02  7.46E+01  1.70E+00 -3.20E+02 -9.02E+01  1.05E+02  1.58E+02  1.14E+03
 
 OM24
+       -1.93E+00  6.92E+01 -1.44E+01 -1.09E+02 -5.48E+00  1.61E+02  9.00E+01 -1.94E+02 -2.87E+02 -4.74E+02  1.93E+03
 
 OM33
+       -8.14E+00  2.68E+01 -3.46E+01 -3.84E+01  7.01E+00  2.48E-03 -2.24E+02  1.20E+02  5.90E+00  3.24E+01 -2.12E+01  5.07E+02
 
 OM34
+        1.17E+00  1.56E+00  1.85E+00  1.02E+02 -1.53E+01 -9.66E+00  1.90E+02 -4.01E+02  4.42E+01 -1.27E+02  1.38E+02 -5.40E+02
          1.95E+03
 
 OM44
+       -1.33E+01 -2.83E+01  4.13E+01 -5.25E+01 -1.00E+01 -2.49E+01 -3.99E+01  1.51E+02 -2.26E+01  9.90E+01 -3.35E+02  1.43E+02
         -7.53E+02  1.15E+03
 
 SG11
+       -1.24E+01 -2.31E+02 -7.92E+01 -7.70E+01  2.37E+02  3.68E+02 -9.09E+01  4.59E+02  8.19E+02 -8.91E+00 -9.69E+01  5.73E+02
          3.57E+02  6.87E+02  2.35E+04
 
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 ********************           EIGENVALUES OF COR MATRIX OF ESTIMATE (From Sample Variance)         ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

             1         2         3         4         5         6         7         8         9        10        11        12
             13        14        15
 
         2.76E-01  4.11E-01  5.27E-01  5.59E-01  6.98E-01  7.58E-01  8.67E-01  9.31E-01  9.56E-01  1.00E+00  1.10E+00  1.20E+00
          1.40E+00  1.93E+00  2.39E+00
 
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
 RAW OUTPUT FILE (FILE): example1.ext
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

 
0ITERATION NO.:    0    OBJECTIVE VALUE:  -1116.01939148996        NO. OF FUNC. EVALS.:   7
 CUMULATIVE NO. OF FUNC. EVALS.:        7
 NPARAMETR:  1.6343E+00  1.5567E+00  7.4454E-01  2.3474E+00  1.7284E-01 -2.8448E-03  9.2803E-03 -2.0281E-02  1.5167E-01 -5.4774E-03
             1.3994E-02  1.8920E-01  2.2181E-02  1.6013E-01  5.9759E-02
 PARAMETER:  1.0000E-01  1.0000E-01  1.0000E-01  1.0000E-01  1.0000E-01 -1.0000E-01  1.0000E-01 -1.0000E-01  1.0000E-01 -1.0000E-01
             1.0000E-01  1.0000E-01  1.0000E-01  1.0000E-01  1.0000E-01
 GRADIENT:  -7.5100E+01 -6.3853E+01 -1.2972E+01 -3.6415E+01  1.2359E+01  2.5317E-01 -1.5502E-01 -2.7634E+00  9.2411E+00 -9.9711E-01
             9.3228E-01  1.0087E+01 -1.0187E+01  1.5863E+01  2.6204E+01
 
0ITERATION NO.:    5    OBJECTIVE VALUE:  -1119.60696757834        NO. OF FUNC. EVALS.:  46
 CUMULATIVE NO. OF FUNC. EVALS.:       53
 NPARAMETR:  1.6720E+00  1.5992E+00  8.3171E-01  2.3877E+00  1.5560E-01 -2.7128E-03  9.2053E-03 -1.4563E-02  1.3493E-01 -4.7342E-03
             1.1219E-02  1.7612E-01  3.4332E-02  1.4476E-01  5.4188E-02
 PARAMETER:  1.2282E-01  1.2694E-01  2.1086E-01  1.1700E-01  4.7479E-02 -1.0050E-01  1.0454E-01 -7.5678E-02  4.1507E-02 -9.1070E-02
             8.5105E-02  6.3997E-02  1.5524E-01  3.6786E-02  5.1074E-02
 GRADIENT:  -2.7916E+01 -4.6531E+00  1.7942E+01 -1.9585E+01 -9.2382E+00  5.0155E-02  4.6735E-01 -1.4170E+00 -3.4254E-02 -2.2120E+00
             1.0044E+00 -1.0022E+01  6.8699E+00 -7.9483E+00 -2.1304E+01
 
0ITERATION NO.:   10    OBJECTIVE VALUE:  -1120.05810379331        NO. OF FUNC. EVALS.:  42
 CUMULATIVE NO. OF FUNC. EVALS.:       95
 NPARAMETR:  1.6759E+00  1.6031E+00  8.4238E-01  2.3934E+00  1.6279E-01 -3.1652E-03  1.6542E-02 -7.0573E-03  1.2634E-01  6.9336E-03
             4.1487E-03  2.0236E-01  4.2543E-02  1.4678E-01  5.3908E-02
 PARAMETER:  1.2516E-01  1.2939E-01  2.2363E-01  1.1940E-01  7.0064E-02 -1.1464E-01  1.8367E-01 -3.5855E-02  8.5302E-03  1.4931E-01
             3.2179E-02  1.3023E-01  1.7579E-01  4.3416E-02  4.8477E-02
 GRADIENT:  -2.3612E+01 -7.4682E+00  1.5112E+01 -1.4066E+01 -4.1332E+00 -4.4976E-01  2.2303E-01  5.1949E+00 -3.3037E+00 -3.9655E-01
            -5.3161E+00 -3.3204E+00  3.1533E+00 -6.7984E+00 -1.7100E+01
 
0ITERATION NO.:   15    OBJECTIVE VALUE:  -1120.89317736233        NO. OF FUNC. EVALS.:  40
 CUMULATIVE NO. OF FUNC. EVALS.:      135
 NPARAMETR:  1.6828E+00  1.6106E+00  8.2678E-01  2.3910E+00  1.6481E-01 -2.1690E-04  8.3706E-03 -1.2514E-02  1.2838E-01  2.0849E-02
             1.2965E-02  1.8359E-01  3.4638E-02  1.4882E-01  5.6492E-02
 PARAMETER:  1.2924E-01  1.3406E-01  2.0491E-01  1.1838E-01  7.6209E-02 -7.8079E-03  9.2369E-02 -6.3190E-02  1.6805E-02  4.2574E-01
             1.0302E-01  7.6267E-02  1.4302E-01  5.5488E-02  7.1890E-02
 GRADIENT:  -7.4410E+00  3.3023E-01  2.7618E+00 -3.8410E+00 -5.1836E-02  3.9493E-02 -1.3037E+00  1.9591E+00 -1.6154E+00  4.6462E-01
            -1.3131E+00 -2.6481E+00  3.1765E+00 -2.1008E+00 -5.9323E+00
 
0ITERATION NO.:   20    OBJECTIVE VALUE:  -1121.02827378477        NO. OF FUNC. EVALS.:  46
 CUMULATIVE NO. OF FUNC. EVALS.:      181
 NPARAMETR:  1.6867E+00  1.6112E+00  8.1909E-01  2.3911E+00  1.6503E-01 -7.8162E-04  1.2363E-02 -1.2789E-02  1.3145E-01  1.5882E-02
             1.3861E-02  1.8740E-01  3.3201E-02  1.4986E-01  5.7166E-02
 PARAMETER:  1.3158E-01  1.3443E-01  1.9555E-01  1.1844E-01  7.6877E-02 -2.8118E-02  1.3633E-01 -6.4534E-02  2.8610E-02  3.2152E-01
             1.0850E-01  8.9352E-02  1.3825E-01  5.9916E-02  7.7822E-02
 GRADIENT:  -2.4339E-01  5.5599E-03 -5.8943E-02 -7.1293E-01  5.4060E-03  2.8891E-04 -2.0300E-03 -2.7183E-03  4.0614E-03 -1.0823E-04
            -1.8572E-03  4.0763E-03  5.2342E-04 -3.3321E-03  6.8149E-03
 
0ITERATION NO.:   22    OBJECTIVE VALUE:  -1121.02837127446        NO. OF FUNC. EVALS.:  26
 CUMULATIVE NO. OF FUNC. EVALS.:      207
 NPARAMETR:  1.6869E+00  1.6113E+00  8.1956E-01  2.3916E+00  1.6505E-01 -7.4300E-04  1.2403E-02 -1.2746E-02  1.3144E-01  1.5945E-02
             1.3897E-02  1.8755E-01  3.3273E-02  1.4991E-01  5.7162E-02
 PARAMETER:  1.3169E-01  1.3447E-01  1.9613E-01  1.1865E-01  7.6948E-02 -2.6727E-02  1.3676E-01 -6.4313E-02  2.8541E-02  3.2276E-01
             1.0882E-01  8.9685E-02  1.3846E-01  6.0024E-02  7.7783E-02
 GRADIENT:   1.0226E-02 -1.2626E-02 -1.6408E-02  3.9704E-02 -7.3057E-03  4.9908E-04 -1.0519E-03 -6.2291E-03  2.3929E-03  5.7806E-05
            -3.3352E-03 -1.2875E-03  4.4495E-03 -2.5222E-03 -6.5242E-03
 
 #TERM:
0MINIMIZATION SUCCESSFUL
 NO. OF FUNCTION EVALUATIONS USED:      207
 NO. OF SIG. DIGITS IN FINAL EST.:  3.2

 ETABAR IS THE ARITHMETIC MEAN OF THE ETA-ESTIMATES,
 AND THE P-VALUE IS GIVEN FOR THE NULL HYPOTHESIS THAT THE TRUE MEAN IS 0.
 
 ETABAR:        -3.4241E-06 -1.1911E-02 -3.4689E-03 -1.1603E-02
 SE:             3.9192E-02  2.8851E-02  3.2735E-02  3.3062E-02
 N:                     100         100         100         100
 
 P VAL.:         9.9993E-01  6.7972E-01  9.1561E-01  7.2562E-01
 
 ETASHRINKSD(%)  3.5305E+00  2.0353E+01  2.4407E+01  1.4554E+01
 ETASHRINKVR(%)  6.9364E+00  3.6564E+01  4.2857E+01  2.6990E+01
 EBVSHRINKSD(%)  3.3163E+00  1.9860E+01  2.4449E+01  1.5063E+01
 EBVSHRINKVR(%)  6.5227E+00  3.5775E+01  4.2921E+01  2.7857E+01
 RELATIVEINF(%)  9.0627E+01  6.3631E+01  5.5227E+01  6.8198E+01
 EPSSHRINKSD(%)  3.0941E+01
 EPSSHRINKVR(%)  5.2308E+01
 
  
 TOTAL DATA POINTS NORMALLY DISTRIBUTED (N):          500
 N*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    918.938533204673     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -1121.02837127446     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -202.089838069787     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 TOTAL EFFECTIVE ETAS (NIND*NETA):                           400
  
 #TERE:
 Elapsed estimation  time in seconds:     8.32
 Elapsed covariance  time in seconds:    11.84
 Elapsed postprocess time in seconds:     0.66
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************          FIRST ORDER CONDITIONAL ESTIMATION WITH INTERACTION (NO PRIOR)        ********************
 #OBJT:**************                       MINIMUM VALUE OF OBJECTIVE FUNCTION                      ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 





 #OBJV:********************************************         -1121.028       *********************************************
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************          FIRST ORDER CONDITIONAL ESTIMATION WITH INTERACTION (NO PRIOR)        ********************
 ********************                             FINAL PARAMETER ESTIMATE                           ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4     
 
         1.69E+00  1.61E+00  8.20E-01  2.39E+00
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        1.65E-01
 
 ETA2
+       -7.43E-04  1.31E-01
 
 ETA3
+        1.24E-02  1.59E-02  1.88E-01
 
 ETA4
+       -1.27E-02  1.39E-02  3.33E-02  1.50E-01
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        5.72E-02
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        4.06E-01
 
 ETA2
+       -5.04E-03  3.63E-01
 
 ETA3
+        7.05E-02  1.02E-01  4.33E-01
 
 ETA4
+       -8.10E-02  9.90E-02  1.98E-01  3.87E-01
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        2.39E-01
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************          FIRST ORDER CONDITIONAL ESTIMATION WITH INTERACTION (NO PRIOR)        ********************
 ********************                            STANDARD ERROR OF ESTIMATE                          ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4     
 
         4.33E-02  4.79E-02  7.20E-02  5.35E-02
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        3.04E-02
 
 ETA2
+        2.99E-02  3.53E-02
 
 ETA3
+        3.66E-02  3.54E-02  1.16E-01
 
 ETA4
+        2.98E-02  2.52E-02  6.91E-02  5.49E-02
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        1.43E-02
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        3.74E-02
 
 ETA2
+        2.03E-01  4.86E-02
 
 ETA3
+        1.91E-01  2.29E-01  1.33E-01
 
 ETA4
+        2.02E-01  1.69E-01  3.26E-01  7.09E-02
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        3.00E-02
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************          FIRST ORDER CONDITIONAL ESTIMATION WITH INTERACTION (NO PRIOR)        ********************
 ********************                          COVARIANCE MATRIX OF ESTIMATE                         ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      OM11      OM12      OM13      OM14      OM22      OM23      OM24      OM33  
             OM34      OM44      SG11  
 
 TH 1
+        1.87E-03
 
 TH 2
+        2.57E-04  2.29E-03
 
 TH 3
+        6.04E-04  2.79E-04  5.19E-03
 
 TH 4
+        2.73E-04  3.58E-04  2.16E-03  2.86E-03
 
 OM11
+        1.11E-04  1.81E-04  3.89E-04  3.41E-04  9.22E-04
 
 OM12
+        1.33E-04  1.92E-04  5.41E-04  4.36E-04  4.25E-04  8.92E-04
 
 OM13
+        1.72E-04  2.58E-04  3.29E-04  4.83E-04  5.58E-04  5.94E-04  1.34E-03
 
 OM14
+        1.45E-04  1.91E-04  4.86E-04  4.47E-04  3.94E-04  5.22E-04  7.92E-04  8.87E-04
 
 OM22
+        4.64E-05  9.56E-05 -1.71E-05  1.41E-04  2.44E-04  4.44E-04  4.30E-04  3.53E-04  1.24E-03
 
 OM23
+        5.33E-05  9.47E-05  8.38E-04  2.53E-04  6.55E-05  1.74E-04 -4.53E-05  3.31E-05 -3.62E-05  1.25E-03
 
 OM24
+        5.53E-05  1.49E-05  4.41E-04  3.02E-04  1.62E-04  2.48E-04  2.40E-04  2.62E-04  2.86E-04  3.40E-04  6.34E-04
 
 OM33
+        5.14E-04  3.21E-04  2.15E-03  1.84E-03  1.55E-03  2.19E-03  2.82E-03  2.20E-03  1.74E-03  6.28E-05  1.04E-03  1.34E-02
 
 OM34
+        3.12E-04  2.50E-04  1.09E-03  9.98E-04  9.20E-04  1.32E-03  1.63E-03  1.37E-03  1.08E-03  7.47E-06  6.95E-04  7.34E-03
          4.78E-03
 
 OM44
+        2.19E-04  2.70E-04  6.68E-04  7.24E-04  6.67E-04  9.58E-04  1.13E-03  9.70E-04  8.21E-04  1.30E-05  5.98E-04  4.78E-03
          3.27E-03  3.01E-03
 
 SG11
+       -3.36E-05 -3.68E-05 -1.89E-04 -1.83E-04 -2.06E-04 -3.07E-04 -3.25E-04 -2.77E-04 -2.63E-04  4.01E-06 -1.37E-04 -1.38E-03
         -8.31E-04 -6.05E-04  2.05E-04
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************          FIRST ORDER CONDITIONAL ESTIMATION WITH INTERACTION (NO PRIOR)        ********************
 ********************                          CORRELATION MATRIX OF ESTIMATE                        ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      OM11      OM12      OM13      OM14      OM22      OM23      OM24      OM33  
             OM34      OM44      SG11  
 
 TH 1
+        4.33E-02
 
 TH 2
+        1.24E-01  4.79E-02
 
 TH 3
+        1.94E-01  8.09E-02  7.20E-02
 
 TH 4
+        1.18E-01  1.40E-01  5.62E-01  5.35E-02
 
 OM11
+        8.42E-02  1.24E-01  1.78E-01  2.10E-01  3.04E-02
 
 OM12
+        1.03E-01  1.34E-01  2.52E-01  2.73E-01  4.69E-01  2.99E-02
 
 OM13
+        1.09E-01  1.47E-01  1.25E-01  2.47E-01  5.02E-01  5.43E-01  3.66E-02
 
 OM14
+        1.12E-01  1.34E-01  2.27E-01  2.81E-01  4.35E-01  5.86E-01  7.26E-01  2.98E-02
 
 OM22
+        3.04E-02  5.66E-02 -6.72E-03  7.50E-02  2.27E-01  4.21E-01  3.33E-01  3.36E-01  3.53E-02
 
 OM23
+        3.48E-02  5.59E-02  3.29E-01  1.34E-01  6.09E-02  1.64E-01 -3.49E-02  3.14E-02 -2.90E-02  3.54E-02
 
 OM24
+        5.07E-02  1.24E-02  2.43E-01  2.24E-01  2.12E-01  3.29E-01  2.60E-01  3.50E-01  3.22E-01  3.81E-01  2.52E-02
 
 OM33
+        1.03E-01  5.81E-02  2.58E-01  2.98E-01  4.41E-01  6.35E-01  6.66E-01  6.40E-01  4.27E-01  1.54E-02  3.56E-01  1.16E-01
 
 OM34
+        1.04E-01  7.54E-02  2.19E-01  2.70E-01  4.38E-01  6.39E-01  6.44E-01  6.67E-01  4.43E-01  3.06E-03  4.00E-01  9.18E-01
          6.91E-02
 
 OM44
+        9.20E-02  1.03E-01  1.69E-01  2.47E-01  4.00E-01  5.85E-01  5.61E-01  5.93E-01  4.24E-01  6.67E-03  4.32E-01  7.54E-01
          8.63E-01  5.49E-02
 
 SG11
+       -5.42E-02 -5.37E-02 -1.83E-01 -2.39E-01 -4.73E-01 -7.17E-01 -6.19E-01 -6.49E-01 -5.21E-01  7.91E-03 -3.81E-01 -8.34E-01
         -8.39E-01 -7.70E-01  1.43E-02
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************          FIRST ORDER CONDITIONAL ESTIMATION WITH INTERACTION (NO PRIOR)        ********************
 ********************                      INVERSE COVARIANCE MATRIX OF ESTIMATE                     ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      OM11      OM12      OM13      OM14      OM22      OM23      OM24      OM33  
             OM34      OM44      SG11  
 
 TH 1
+        5.70E+02
 
 TH 2
+       -4.80E+01  4.71E+02
 
 TH 3
+       -6.65E+01  5.08E+00  3.38E+02
 
 TH 4
+        1.21E+01 -4.63E+01 -2.16E+02  5.52E+02
 
 OM11
+       -1.16E+01 -4.03E+01 -3.27E+01 -1.63E+01  1.60E+03
 
 OM12
+       -4.42E+01 -7.52E+01 -4.16E+01 -5.17E+01 -2.89E+02  2.75E+03
 
 OM13
+       -3.65E+01 -6.88E+01  1.19E+02 -6.77E+01 -4.17E+02 -9.43E+01  2.04E+03
 
 OM14
+       -1.69E+01 -4.42E+01 -8.84E+01 -2.72E+01  1.07E+01 -3.67E+02 -1.18E+03  3.03E+03
 
 OM22
+       -1.35E+01 -3.24E+01  6.88E+01  1.36E+01  4.94E+01 -2.06E+02 -1.88E+01  6.67E+01  1.18E+03
 
 OM23
+        2.69E+01 -4.10E+01 -1.54E+02  6.72E+01 -3.20E+01 -3.81E+02  7.85E+01  7.04E+01  1.02E+02  1.14E+03
 
 OM24
+        6.11E+00  8.78E+01 -2.83E+01 -8.31E+01  4.55E-01  1.93E+02  5.51E+01 -2.95E+02 -3.45E+02 -6.89E+02  2.54E+03
 
 OM33
+       -5.27E+00  2.57E+01 -4.92E+01 -3.32E+01  2.17E+01  1.97E+00 -2.57E+02  1.36E+02  8.65E-01 -2.29E+01  5.56E+01  6.09E+02
 
 OM34
+       -2.48E+01  5.39E+00  8.12E+00  4.93E+01  4.41E+00 -7.13E+00  1.10E+02 -3.96E+02  3.11E+00  5.45E+01 -6.16E+01 -8.41E+02
          2.44E+03
 
 OM44
+       -1.67E+01 -6.07E+01  3.37E+01 -4.54E+01 -1.40E+01 -3.19E+01 -3.15E+01  4.79E+01  4.17E+00  6.30E+01 -3.39E+02  2.25E+02
         -1.11E+03  1.49E+03
 
 SG11
+       -3.23E+02 -2.48E+02 -4.27E+01 -1.47E+02  6.61E+02  2.83E+03 -2.55E+02  9.03E+02  1.19E+03 -5.69E+02  2.96E+02  1.13E+03
          5.82E+02  1.14E+03  2.53E+04
 
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************          FIRST ORDER CONDITIONAL ESTIMATION WITH INTERACTION (NO PRIOR)        ********************
 ********************                      EIGENVALUES OF COR MATRIX OF ESTIMATE                     ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

             1         2         3         4         5         6         7         8         9        10        11        12
             13        14        15
 
         5.46E-02  1.51E-01  2.15E-01  2.62E-01  3.67E-01  3.89E-01  5.28E-01  5.82E-01  6.65E-01  7.94E-01  9.00E-01  9.74E-01
          1.19E+00  1.73E+00  6.19E+00
 
 Elapsed finaloutput time in seconds:     0.05
 #CPUT: Total CPU Time in Seconds,      413.406
Stop Time: 
Tue 10/22/2024 
12:42 PM
