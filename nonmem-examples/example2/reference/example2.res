Tue 10/22/2024 
04:07 PM
;Model Desc: Two Compartment model with Clearance and 
; central volume modeled with covariates age and gender
;Project Name: nm7examples
;Project ID: NO PROJECT DESCRIPTION

$PROB RUN# example2 (from sampc)
$INPUT C SET ID JID TIME DV=CONC AMT=DOSE RATE EVID MDV CMT GNDR AGE
$DATA example2.csv IGNORE=C
$SUBROUTINES ADVAN3 TRANS4

$PK
; LCLM=log transformed clearance, male
LCLM=THETA(1)
;LCLF=log transformed clearance, female.
LCLF=THETA(2)
; CLAM=CL age slope, male
CLAM=THETA(3)
; CLAF=CL age slope, female
CLAF=THETA(4)
; LV1M=log transformed V1, male
LV1M=THETA(5)
; LV1F=log transformed V1, female
LV1F=THETA(6)
; V1AM=V1 age slope, male
V1AM=THETA(7)
; V1AF=V1 age slope, female
V1AF=THETA(8)
; LAGE=log transformed age
LAGE=DLOG(AGE)

;Mean of ETA1, the inter-subject deviation of Clearance,
; is ultimately modeled as linear function of THETA(1) to THETA(4).  
; Relating thetas to Mus by linear functions is not essential for 
; ITS, IMP, or IMPMAP methods, but is very helpful for MCMC methods 
; such as SAEM and BAYES.

MU_1=(1.0-GNDR)*(LCLM+LAGE*CLAM) + GNDR*(LCLF+LAGE*CLAF)

; Mean of ETA2, the inter-subject deviation of V1, 
; is ultimately modeled as linear function of THETA(5) to THETA(8)

MU_2=(1.0-GNDR)*(LV1M+LAGE*V1AM) + GNDR*(LV1F+LAGE*V1AF)
MU_3=THETA(9)
MU_4=THETA(10)
CL=DEXP(MU_1+ETA(1))
V1=DEXP(MU_2+ETA(2))
Q=DEXP(MU_3+ETA(3))
V2=DEXP(MU_4+ETA(4))
S1=V1

$ERROR
CALLFL=0
; Option to model the residual error coefficient in THETA(11), 
; rather than in SIGMA.
SDSL=THETA(11)
W=F*SDSL
Y = F + W*EPS(1)
IPRED=F
IWRES=(DV-F)/W

;Initial THETAs
$THETA
( 0.7 ) ;[LCLM]
( 0.7 ) ;[LCLF]
( 2 )   ;[CLAM]
( 2.0);[CLAF]
( 0.7 ) ;[LV1M]
( 0.7 ) ;[LV1F]
( 2.0 )   ;[V1AM]
( 2.0 )   ;[V1AF]
( 0.7 ) ;[MU_3]
(  0.7 );[MU_4]
(0.0, 0.3 )     ;[SDSL]

;Initial OMEGAs
$OMEGA BLOCK(4)
0.5  ;[p]
0.001  ;[f]
0.5  ;[p]
0.001 ;[f]
0.001 ;[f]
0.5  ;[p]
0.001 ;[f]
0.001 ;[f]
0.001 ;[f]
0.5 ;[p]

; SIGMA is 1.0 fixed, serves as unscaled variance for EPS(1).  
; THETA(11) takes up the residual error scaling.
$SIGMA 
(1.0 FIXED)

;Prior information is important for MCMC Bayesian analysis, 
; not necessary for maximization methods
; In this example, only the OMEGAs have a prior distribution,
; the THETAS do not.
; For Bayesian methods, it is most important for at least the 
; OMEGAs to have a prior, even an uninformative one, 
; to stabilize the analysis. Only if the number of subjects
; exceeds the OMEGA dimension number by at least 100, 
; then you may get away without priors on OMEGA for BAYES analysis.
$PRIOR NWPRI
; Prior OMEGA matrix
$OMEGAP BLOCK(4) FIX VALUES(0.01,0.0)
; Degrees of freedom to OMEGA prior matrix:
$OMEGAPD 4 FIX

; The first analysis is iterative two-stage.  
; Note that the GRD specification is THETA(11) is a 
; Sigma-like parameter.  This will allow NONMEM to make
; efficient gradient evaluations for THETA(11), which is useful 
; for later IMP,IMPMAP, and SAEM methods, but has no impact on 
; ITS and BAYES methods.

$EST METHOD=ITS INTERACTION FILE=example2.ext NITER=1000 NSIG=2 
     PRINT=5 NOABORT SIGL=8 NOPRIOR=1 CTYPE=3 GRD=TS(11)

; Results of ITS serve as initial parameters for the IMP method.

$EST METHOD=IMP INTERACTION EONLY=0 MAPITER=0 NITER=100 ISAMPLE=300 
     PRINT=1 SIGL=8

; The results of IMP are used as the initial values for the SAEM method.

$EST METHOD=SAEM NBURN=3000 NITER=2000 PRINT=10 ISAMPLE=3
     CTYPE=3 CITER=10 CALPHA=0.05

; After the SAEM method, obtain good estimates of the marginal density 
; (objective function),
; along with good estimates of the standard errors.

$EST METHOD=IMP INTERACTION EONLY=1 NITER=5 ISAMPLE=3000 
     PRINT=1 SIGL=8 SEED=123334
     CTYPE=3 CITER=10 CALPHA=0.05

; The Bayesian analysis is performed. 

$EST METHOD=BAYES INTERACTION FILE=example2.TXT NBURN=10000 
     NITER=3000 PRINT=100 NOPRIOR=0
     CTYPE=3 CITER=10 CALPHA=0.05

; Just for old-times sake, lets see what the traditional 
; FOCE method will give us.  
; And, remember to introduce a new FILE, so its results wont 
; append to our Bayesian FILE.

$EST  METHOD=COND INTERACTION MAXEVAL=9999 FILE=example2.ext NSIG=2 
  SIGL=14 PRINT=5 NOABORT NOPRIOR=1

$COV MATRIX=R UNCONDITIONAL
  
NM-TRAN MESSAGES 
  
 WARNINGS AND ERRORS (IF ANY) FOR PROBLEM    1
             
 (WARNING  2) NM-TRAN INFERS THAT THE DATA ARE POPULATION.

 (MU_WARNING 26) DATA ITEM(S) USED IN DEFINITION OF MU_(S) SHOULD BE CONSTANT FOR INDIV. REC.:
  GNDR AGE
  
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
 RUN# example2 (from sampc)
0DATA CHECKOUT RUN:              NO
 DATA SET LOCATED ON UNIT NO.:    2
 THIS UNIT TO BE REWOUND:        NO
 CREATE/ADD TO FDATA.csv:        YES
 NO. OF DATA RECS IN DATA SET:     2400
 NO. OF DATA ITEMS IN DATA SET:  13
 ID DATA ITEM IS DATA ITEM NO.:   3
 DEP VARIABLE IS DATA ITEM NO.:   6
 MDV DATA ITEM IS DATA ITEM NO.: 10
0INDICES PASSED TO SUBROUTINE PRED:
   9   5   7   8   0   0  11   0   0   0   0
0LABELS FOR DATA ITEMS:
 C SET ID JID TIME CONC DOSE RATE EVID MDV CMT GNDR AGE
0FORMAT FOR DATA:
 (2E2.0,3E4.0,E11.0,E4.0,5E2.0,E6.0)

 TOT. NO. OF OBS RECS:     2000
 TOT. NO. OF INDIVIDUALS:      400
0LENGTH OF THETA:  12
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
0DEFAULT OMEGA BOUNDARY TEST OMITTED:    NO
0SIGMA HAS SIMPLE DIAGONAL FORM WITH DIMENSION:   1
0DEFAULT SIGMA BOUNDARY TEST OMITTED:    NO
0INITIAL ESTIMATE OF THETA:
 LOWER BOUND    INITIAL EST    UPPER BOUND
 -0.1000E+07     0.7000E+00     0.1000E+07
 -0.1000E+07     0.7000E+00     0.1000E+07
 -0.1000E+07     0.2000E+01     0.1000E+07
 -0.1000E+07     0.2000E+01     0.1000E+07
 -0.1000E+07     0.7000E+00     0.1000E+07
 -0.1000E+07     0.7000E+00     0.1000E+07
 -0.1000E+07     0.2000E+01     0.1000E+07
 -0.1000E+07     0.2000E+01     0.1000E+07
 -0.1000E+07     0.7000E+00     0.1000E+07
 -0.1000E+07     0.7000E+00     0.1000E+07
  0.0000E+00     0.3000E+00     0.1000E+07
  0.4000E+01     0.4000E+01     0.4000E+01
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.5000E+00
                  0.1000E-02   0.5000E+00
                  0.1000E-02   0.1000E-02   0.5000E+00
                  0.1000E-02   0.1000E-02   0.1000E-02   0.5000E+00
        2                                                                                  YES
                  0.1000E-01
                  0.0000E+00   0.1000E-01
                  0.0000E+00   0.0000E+00   0.1000E-01
                  0.0000E+00   0.0000E+00   0.0000E+00   0.1000E-01
0INITIAL ESTIMATE OF SIGMA:
 0.1000E+01
0SIGMA CONSTRAINED TO BE THIS INITIAL ESTIMATE
0COVARIANCE STEP OMITTED:        NO
 R MATRIX SUBSTITUTED:          YES
 S MATRIX SUBSTITUTED:           NO
 EIGENVLS. PRINTED:              NO
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
0DURING SIMULATION, ERROR SUBROUTINE CALLED WITH EVERY EVENT RECORD.
 OTHERWISE, ERROR SUBROUTINE CALLED ONLY WITH OBSERVATION EVENTS.
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
 NO. OF FUNCT. EVALS. ALLOWED:            2208
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
 RAW OUTPUT FILE (FILE): example2.ext
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
 GRADIENT/GIBBS PATTERN (GRD):              DDDDDDDDDDS
 AUTOMATIC SETTING FEATURE (AUTO):          0
 CONVERGENCE TYPE (CTYPE):                  3
 CONVERGENCE INTERVAL (CINTERVAL):          5
 CONVERGENCE ITERATIONS (CITER):            10
 CONVERGENCE ALPHA ERROR (CALPHA):          5.000000000000000E-02
 ITERATIONS (NITER):                        1000
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
   1   2   3   4   5   6   7   8   9  10
 THETAS THAT ARE SIGMA-LIKE:
  11
 
 MONITORING OF SEARCH:

 iteration            0  OBJ=   43391.7048829661
 iteration            5  OBJ=  -10716.8268061578
 iteration           10  OBJ=  -10763.1673881633
 iteration           15  OBJ=  -10768.8282815921
 iteration           20  OBJ=  -10770.4608303307
 iteration           25  OBJ=  -10771.1256603381
 iteration           30  OBJ=  -10771.4490553266
 iteration           35  OBJ=  -10771.6227287039
 iteration           40  OBJ=  -10771.7216644551
 iteration           45  OBJ=  -10771.7801167724
 iteration           50  OBJ=  -10771.8154054351
 iteration           55  OBJ=  -10771.8369203821
 iteration           60  OBJ=  -10771.8500202462
 iteration           65  OBJ=  -10771.8578808674
 iteration           70  OBJ=  -10771.8624453689
 iteration           75  OBJ=  -10771.8649246382
 iteration           80  OBJ=  -10771.8660899660
 iteration           85  OBJ=  -10771.8664366482
 iteration           90  OBJ=  -10771.8662854312
 iteration           95  OBJ=  -10771.8658447599
 iteration          100  OBJ=  -10771.8652492476
 iteration          105  OBJ=  -10771.8645843299
 iteration          110  OBJ=  -10771.8639054775
 iteration          115  OBJ=  -10771.8632463696
 iteration          120  OBJ=  -10771.8626251004
 iteration          125  OBJ=  -10771.8620525132
 iteration          130  OBJ=  -10771.8615326632
 iteration          135  OBJ=  -10771.8610661119
 iteration          140  OBJ=  -10771.8606510410
 iteration          145  OBJ=  -10771.8602846779
 iteration          150  OBJ=  -10771.8599621110
 iteration          155  OBJ=  -10771.8596798670
 iteration          160  OBJ=  -10771.8594333965
 iteration          165  OBJ=  -10771.8592191466
 iteration          170  OBJ=  -10771.8590329843
 iteration          175  OBJ=  -10771.8588718327
 iteration          180  OBJ=  -10771.8587327910
 iteration          185  OBJ=  -10771.8586122742
 iteration          190  OBJ=  -10771.8585085388
 iteration          195  OBJ=  -10771.8584188779
 iteration          200  OBJ=  -10771.8583419220
 Convergence achieved
 
 #TERM:
 OPTIMIZATION WAS COMPLETED


 ETABAR IS THE ARITHMETIC MEAN OF THE ETA-ESTIMATES,
 AND THE P-VALUE IS GIVEN FOR THE NULL HYPOTHESIS THAT THE TRUE MEAN IS 0.
 
 ETABAR:        -4.7795E-08 -6.0847E-08 -1.1271E-07 -1.0175E-07
 SE:             4.7106E-03  2.9771E-03  2.9246E-03  3.6831E-03
 N:                     400         400         400         400
 
 P VAL.:         9.9999E-01  9.9998E-01  9.9997E-01  9.9998E-01
 
 ETASHRINKSD(%)  6.9160E+00  3.3019E+01  4.1116E+01  2.5038E+01
 ETASHRINKVR(%)  1.3354E+01  5.5135E+01  6.5326E+01  4.3807E+01
 EBVSHRINKSD(%)  6.9159E+00  3.3019E+01  4.1114E+01  2.5037E+01
 EBVSHRINKVR(%)  1.3354E+01  5.5135E+01  6.5324E+01  4.3806E+01
 RELATIVEINF(%)  7.7675E+01  4.1768E+01  2.7824E+01  4.1730E+01
 EPSSHRINKSD(%)  2.6119E+01
 EPSSHRINKVR(%)  4.5416E+01
 
  
 TOTAL DATA POINTS NORMALLY DISTRIBUTED (N):         2000
 N*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    3675.75413281869     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -10771.8583419220     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -7096.10420910333     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 TOTAL EFFECTIVE ETAS (NIND*NETA):                          1600
  
 #TERE:
 Elapsed estimation  time in seconds:    71.85
 Elapsed covariance  time in seconds:     0.06
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 #OBJT:**************                        FINAL VALUE OF OBJECTIVE FUNCTION                       ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 





 #OBJV:********************************************        -10771.858       *********************************************
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 ********************                             FINAL PARAMETER ESTIMATE                           ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11     
 
         3.30E+00  3.26E+00 -6.11E-01 -2.08E-01  7.29E-01  1.14E+00  3.37E-01  1.92E-01  6.92E-01  2.30E+00  1.00E-01
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        1.02E-02
 
 ETA2
+        1.53E-04  7.90E-03
 
 ETA3
+        1.18E-03 -3.76E-04  9.87E-03
 
 ETA4
+       -6.46E-04  4.42E-04  1.95E-03  9.66E-03
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        1.00E+00
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        1.01E-01
 
 ETA2
+        1.70E-02  8.89E-02
 
 ETA3
+        1.17E-01 -4.26E-02  9.93E-02
 
 ETA4
+       -6.50E-02  5.06E-02  2.00E-01  9.83E-02
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        1.00E+00
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 ********************                          STANDARD ERROR OF ESTIMATE (S)                        ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11     
 
         3.79E-02  2.88E-02  1.11E-02  8.40E-03  4.82E-02  4.09E-02  1.34E-02  1.18E-02  1.05E-02  8.92E-03  2.99E-03
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        1.00E-03
 
 ETA2
+        8.63E-04  1.49E-03
 
 ETA3
+        1.29E-03  1.43E-03  2.82E-03
 
 ETA4
+        1.12E-03  1.15E-03  1.98E-03  2.00E-03
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        0.00E+00
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        4.96E-03
 
 ETA2
+        9.50E-02  8.40E-03
 
 ETA3
+        1.21E-01  1.66E-01  1.42E-02
 
 ETA4
+        1.16E-01  1.28E-01  1.65E-01  1.02E-02
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+       .........
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 ********************                        COVARIANCE MATRIX OF ESTIMATE (S)                       ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 TH 1
+        1.44E-03
 
 TH 2
+        3.16E-06  8.28E-04
 
 TH 3
+       -4.10E-04  1.27E-06  1.22E-04
 
 TH 4
+       -4.26E-07 -2.31E-04 -1.24E-07  7.06E-05
 
 TH 5
+        6.37E-04  5.15E-05 -1.66E-04 -4.22E-06  2.33E-03
 
 TH 6
+       -1.85E-05  2.12E-04  6.18E-06 -5.18E-05  2.29E-05  1.67E-03
 
 TH 7
+       -1.66E-04 -1.54E-05  4.37E-05  1.60E-06 -6.32E-04 -9.98E-06  1.79E-04
 
 TH 8
+        6.08E-06 -5.22E-05 -1.62E-06  1.47E-05  3.97E-06 -4.68E-04 -6.15E-08  1.40E-04
 
 TH 9
+        1.76E-05  5.99E-05 -1.33E-07 -9.14E-06  1.15E-04  5.08E-05 -3.25E-05 -7.07E-06  1.10E-04
 
 TH10
+        1.39E-05  3.68E-05 -9.36E-08 -4.77E-06  1.07E-04  4.83E-05 -2.96E-05 -5.45E-06  6.57E-05  7.96E-05
 
 TH11
+       -7.57E-06 -3.62E-06  2.68E-06  1.28E-06 -8.73E-06  7.26E-07  2.49E-06 -7.66E-08  1.72E-06  1.46E-06  8.94E-06
 
 OM11
+        4.12E-06 -1.54E-07 -1.32E-06 -1.72E-07  1.27E-06  2.51E-07 -4.56E-07 -4.18E-08 -3.90E-07  1.26E-07 -4.29E-07  1.01E-06
 
 OM12
+        1.54E-06  1.60E-07 -6.64E-07 -5.05E-08  1.88E-06  7.39E-07 -3.71E-07  1.51E-08 -1.55E-07  3.36E-07 -5.64E-07  3.66E-07
          7.44E-07
 
 OM13
+        4.79E-07  1.83E-06 -4.23E-07 -5.80E-07  2.01E-06  4.11E-08 -5.63E-07  5.08E-08  7.48E-07  6.62E-07 -5.21E-07  5.31E-07
          3.31E-07  1.66E-06
 
 OM14
+        1.94E-06 -4.51E-07 -7.42E-07  2.93E-07  6.06E-06 -1.74E-07 -1.51E-06  1.34E-07  7.63E-07  9.25E-07 -3.45E-07  4.30E-07
          2.94E-07  1.03E-06  1.24E-06
 
 OM22
+        3.80E-06  6.65E-07 -1.07E-06 -8.81E-08  5.37E-06 -7.09E-06 -7.25E-07  2.12E-06 -3.23E-08 -4.76E-08 -1.08E-06  9.01E-08
          4.63E-07  6.30E-08  1.47E-07  2.23E-06
 
 OM23
+        3.07E-07  5.64E-07 -2.33E-07 -2.86E-07 -4.84E-07  1.27E-06  1.96E-07 -6.66E-08 -6.83E-07 -3.44E-07 -6.93E-07  2.97E-07
          4.24E-07  7.82E-07  5.02E-07  5.27E-07  2.06E-06
 
 OM24
+        2.54E-06 -3.94E-07 -6.50E-07  7.00E-08  4.96E-06 -2.15E-06 -1.24E-06  9.47E-07 -3.34E-08  8.15E-07 -3.67E-07  1.96E-07
          3.55E-07  4.62E-07  5.23E-07  5.18E-07  1.12E-06  1.33E-06
 
 OM33
+        4.38E-06  1.65E-06 -1.33E-06 -4.41E-07  1.01E-05 -5.16E-06 -2.94E-06  1.15E-06 -1.26E-06 -1.09E-06 -3.19E-06  4.44E-07
          4.59E-07  1.28E-06  6.61E-07  2.92E-07  1.53E-06  7.05E-07  7.97E-06
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 OM34
+        3.02E-06  2.73E-06 -9.62E-07 -7.23E-07  7.78E-06  2.22E-06 -2.43E-06 -5.46E-07 -2.77E-07  4.36E-07 -2.30E-06  4.17E-07
          3.93E-07  9.02E-07  8.44E-07  2.26E-07  1.21E-06  7.89E-07  4.53E-06  3.93E-06
 
 OM44
+       -1.44E-07  3.52E-06 -2.63E-08 -7.28E-07  8.55E-06  5.81E-06 -2.63E-06 -1.05E-06  1.19E-06  3.19E-06 -1.84E-06  4.12E-07
          3.76E-07  7.34E-07  9.61E-07  9.62E-08  8.18E-07  9.43E-07  2.40E-06  3.04E-06  3.98E-06
 
 SG11
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 ********************                        CORRELATION MATRIX OF ESTIMATE (S)                      ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 TH 1
+        3.79E-02
 
 TH 2
+        2.90E-03  2.88E-02
 
 TH 3
+       -9.77E-01  3.98E-03  1.11E-02
 
 TH 4
+       -1.34E-03 -9.55E-01 -1.34E-03  8.40E-03
 
 TH 5
+        3.48E-01  3.71E-02 -3.11E-01 -1.04E-02  4.82E-02
 
 TH 6
+       -1.19E-02  1.80E-01  1.36E-02 -1.51E-01  1.16E-02  4.09E-02
 
 TH 7
+       -3.28E-01 -4.00E-02  2.95E-01  1.42E-02 -9.79E-01 -1.82E-02  1.34E-02
 
 TH 8
+        1.36E-02 -1.54E-01 -1.24E-02  1.48E-01  6.96E-03 -9.68E-01 -3.89E-04  1.18E-02
 
 TH 9
+        4.41E-02  1.98E-01 -1.14E-03 -1.04E-01  2.28E-01  1.18E-01 -2.31E-01 -5.70E-02  1.05E-02
 
 TH10
+        4.09E-02  1.43E-01 -9.48E-04 -6.37E-02  2.49E-01  1.32E-01 -2.48E-01 -5.17E-02  7.02E-01  8.92E-03
 
 TH11
+       -6.68E-02 -4.21E-02  8.09E-02  5.09E-02 -6.05E-02  5.94E-03  6.22E-02 -2.17E-03  5.48E-02  5.48E-02  2.99E-03
 
 OM11
+        1.08E-01 -5.32E-03 -1.19E-01 -2.03E-02  2.63E-02  6.11E-03 -3.39E-02 -3.52E-03 -3.70E-02  1.40E-02 -1.43E-01  1.00E-03
 
 OM12
+        4.69E-02  6.45E-03 -6.95E-02 -6.97E-03  4.52E-02  2.09E-02 -3.22E-02  1.48E-03 -1.71E-02  4.37E-02 -2.19E-01  4.23E-01
          8.63E-04
 
 OM13
+        9.79E-03  4.94E-02 -2.96E-02 -5.35E-02  3.23E-02  7.79E-04 -3.27E-02  3.33E-03  5.53E-02  5.75E-02 -1.35E-01  4.10E-01
          2.97E-01  1.29E-03
 
 OM14
+        4.58E-02 -1.40E-02 -6.01E-02  3.13E-02  1.13E-01 -3.81E-03 -1.01E-01  1.02E-02  6.52E-02  9.30E-02 -1.03E-01  3.84E-01
          3.06E-01  7.15E-01  1.12E-03
 
 OM22
+        6.71E-02  1.55E-02 -6.46E-02 -7.02E-03  7.46E-02 -1.16E-01 -3.63E-02  1.20E-01 -2.06E-03 -3.58E-03 -2.42E-01  6.01E-02
          3.60E-01  3.27E-02  8.85E-02  1.49E-03
 
 OM23
+        5.64E-03  1.37E-02 -1.47E-02 -2.37E-02 -6.99E-03  2.16E-02  1.02E-02 -3.93E-03 -4.54E-02 -2.69E-02 -1.61E-01  2.06E-01
          3.43E-01  4.22E-01  3.14E-01  2.46E-01  1.43E-03
 
 OM24
+        5.80E-02 -1.19E-02 -5.10E-02  7.23E-03  8.93E-02 -4.57E-02 -8.03E-02  6.95E-02 -2.76E-03  7.93E-02 -1.07E-01  1.69E-01
          3.57E-01  3.11E-01  4.07E-01  3.01E-01  6.76E-01  1.15E-03
 
 OM33
+        4.09E-02  2.03E-02 -4.26E-02 -1.86E-02  7.42E-02 -4.47E-02 -7.80E-02  3.45E-02 -4.23E-02 -4.33E-02 -3.78E-01  1.57E-01
          1.89E-01  3.53E-01  2.10E-01  6.93E-02  3.78E-01  2.17E-01  2.82E-03
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 OM34
+        4.02E-02  4.80E-02 -4.39E-02 -4.35E-02  8.14E-02  2.74E-02 -9.15E-02 -2.33E-02 -1.33E-02  2.47E-02 -3.89E-01  2.10E-01
          2.30E-01  3.53E-01  3.82E-01  7.64E-02  4.24E-01  3.45E-01  8.10E-01  1.98E-03
 
 OM44
+       -1.90E-03  6.14E-02 -1.19E-03 -4.34E-02  8.88E-02  7.12E-02 -9.84E-02 -4.45E-02  5.67E-02  1.79E-01 -3.09E-01  2.05E-01
          2.18E-01  2.85E-01  4.32E-01  3.23E-02  2.86E-01  4.10E-01  4.27E-01  7.68E-01  2.00E-03
 
 SG11
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 ********************                    INVERSE COVARIANCE MATRIX OF ESTIMATE (S)                   ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 TH 1
+        1.72E+04
 
 TH 2
+        0.00E+00  1.72E+04
 
 TH 3
+        5.70E+04  0.00E+00  1.98E+05
 
 TH 4
+        0.00E+00  5.58E+04  0.00E+00  1.96E+05
 
 TH 5
+       -1.95E+03  0.00E+00 -5.24E+03  0.00E+00  1.16E+04
 
 TH 6
+        0.00E+00 -3.70E+03  0.00E+00 -1.19E+04  0.00E+00  1.16E+04
 
 TH 7
+       -5.24E+03  0.00E+00 -1.54E+04  0.00E+00  4.03E+04  0.00E+00  1.47E+05
 
 TH 8
+        0.00E+00 -1.19E+04  0.00E+00 -3.99E+04  0.00E+00  3.85E+04  0.00E+00  1.35E+05
 
 TH 9
+       -1.69E+03 -3.57E+03 -5.82E+03 -1.02E+04  5.64E+02  8.51E+01  2.83E+03  4.39E+02  1.96E+04
 
 TH10
+       -1.08E+03 -1.21E+02 -4.09E+03 -7.31E+02 -6.82E+02 -3.31E+03 -8.55E+02 -1.07E+04 -1.50E+04  2.83E+04
 
 TH11
+       -1.98E+03 -1.32E+03 -8.56E+03 -5.90E+03 -1.10E+03  2.04E+02 -4.99E+03  1.34E+02  2.83E+02 -3.92E+03  1.49E+05
 
 OM11
+       -1.03E+04  1.38E+04 -2.15E+04  4.71E+04  1.00E+04 -4.11E+02  3.32E+04 -2.49E+03  7.74E+03 -4.83E+03  1.70E+04  1.44E+06
 
 OM12
+        2.10E+04 -5.21E+02  6.96E+04 -4.16E+03 -5.37E+03 -1.18E+04 -1.89E+04 -3.30E+04  5.66E+03 -9.07E+03  3.84E+04 -5.91E+05
          2.11E+06
 
 OM13
+        7.66E+03  9.78E+03  2.39E+04  4.91E+04  6.20E+03 -1.44E+03  1.68E+04 -6.37E+03 -8.84E+03 -4.49E+03  6.11E+03 -2.44E+05
         -3.81E+04  1.68E+06
 
 OM14
+        8.96E+03 -1.87E+04  2.96E+04 -8.29E+04 -1.84E+04  1.03E+04 -5.48E+04  3.38E+04 -5.00E+03  8.32E+03 -3.11E+04 -2.02E+05
         -6.74E+04 -1.31E+06  2.30E+06
 
 OM22
+       -1.88E+03 -5.88E+03 -6.01E+03 -1.65E+04 -1.39E+04  4.63E+03 -4.87E+04  7.38E+03 -2.52E+03  8.78E+02  7.01E+04  5.94E+04
         -3.20E+05  9.13E+04 -2.92E+04  6.21E+05
 
 OM23
+        6.94E+03 -1.32E+00  1.94E+04 -1.38E+03 -1.07E+02 -8.86E+03 -5.65E+03 -2.16E+04 -1.20E+03  9.30E+03 -2.36E+03 -2.50E+04
         -8.61E+04 -4.09E+05  3.14E+05 -4.79E+04  1.21E+06
 
 OM24
+       -1.79E+04  1.14E+04 -5.39E+04  3.75E+04  3.40E+03 -6.15E+02  1.72E+04 -1.33E+04  8.94E+03 -1.07E+04 -4.75E+04  1.35E+05
         -2.15E+05  2.47E+05 -4.89E+05 -1.74E+05 -9.42E+05  1.90E+06
 
 OM33
+       -1.13E+03 -7.05E+03 -6.39E+03 -2.77E+04 -5.55E+03  5.34E+03 -1.62E+04  1.23E+04  3.59E+03 -4.49E+03  4.17E+04  2.05E+04
         -4.05E+04 -2.80E+05  3.00E+05  3.49E+04  3.09E+04 -4.79E+04  5.74E+05
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 OM34
+       -6.42E+03  8.83E+03 -1.31E+04  3.63E+04  1.14E+04 -9.93E+02  3.88E+04  3.03E+03 -9.11E+03  2.48E+04  6.08E+02 -2.10E+03
          5.16E+04  3.43E+05 -4.63E+05 -5.20E+04 -3.74E+05  3.17E+05 -9.10E+05  2.26E+06
 
 OM44
+        6.12E+03 -5.91E+03  1.24E+04 -1.85E+04 -1.55E+03 -4.65E+03 -5.38E+03 -1.46E+04  1.26E+04 -3.26E+04  5.62E+04 -3.26E+04
         -2.11E+04 -2.45E+04 -8.24E+04  9.64E+04  2.47E+05 -4.04E+05  3.52E+05 -1.14E+06  1.03E+06
 
 SG11
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
1
 
 
 #TBLN:      2
 #METH: Importance Sampling (No Prior)
 
 ESTIMATION STEP OMITTED:                 NO
 SHRINK INFO WITH EVALUATION (EVALSHRINK) NO
 ANALYSIS TYPE:                           POPULATION
 NUMBER OF SADDLE POINT RESET ITERATIONS:      0
 GRADIENT METHOD USED:               NOSLOW
 CONDITIONAL ESTIMATES USED:              YES
 CENTERED ETA:                            NO
 EPS-ETA INTERACTION:                     YES
 LAPLACIAN OBJ. FUNC.:                    NO
 NO. OF FUNCT. EVALS. ALLOWED:            2208
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
 RAW OUTPUT FILE (FILE): example2.ext
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
 GRADIENT/GIBBS PATTERN (GRD):              DDDDDDDDDDS
 AUTOMATIC SETTING FEATURE (AUTO):          0
 CONVERGENCE TYPE (CTYPE):                  3
 CONVERGENCE INTERVAL (CINTERVAL):          1
 CONVERGENCE ITERATIONS (CITER):            10
 CONVERGENCE ALPHA ERROR (CALPHA):          5.000000000000000E-02
 ITERATIONS (NITER):                        100
 ANNEAL SETTING (CONSTRAIN):                 1
 STARTING SEED FOR MC METHODS (SEED):       11456
 MC SAMPLES PER SUBJECT (ISAMPLE):          300
 RANDOM SAMPLING METHOD (RANMETHOD):        3U
 EXPECTATION ONLY (EONLY):                  0
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
   1   2   3   4   5   6   7   8   9  10
 THETAS THAT ARE SIGMA-LIKE:
  11
 
 MONITORING OF SEARCH:

 iteration            0  OBJ=  -10780.3039987426 eff.=     304. Smpl.=     300. Fit.= 0.98311
 iteration            1  OBJ=  -10780.6743906474 eff.=     124. Smpl.=     300. Fit.= 0.90173
 iteration            2  OBJ=  -10780.8306678229 eff.=     119. Smpl.=     300. Fit.= 0.89865
 iteration            3  OBJ=  -10782.4630991844 eff.=     119. Smpl.=     300. Fit.= 0.89851
 iteration            4  OBJ=  -10780.4846408482 eff.=     122. Smpl.=     300. Fit.= 0.90075
 iteration            5  OBJ=  -10782.2266439233 eff.=     120. Smpl.=     300. Fit.= 0.89950
 iteration            6  OBJ=  -10783.5249074368 eff.=     121. Smpl.=     300. Fit.= 0.89947
 iteration            7  OBJ=  -10781.2360527324 eff.=     121. Smpl.=     300. Fit.= 0.89999
 iteration            8  OBJ=  -10781.9367126987 eff.=     121. Smpl.=     300. Fit.= 0.89999
 iteration            9  OBJ=  -10782.5484011842 eff.=     120. Smpl.=     300. Fit.= 0.89892
 iteration           10  OBJ=  -10784.3187464706 eff.=     121. Smpl.=     300. Fit.= 0.89969
 iteration           11  OBJ=  -10781.8869225080 eff.=     122. Smpl.=     300. Fit.= 0.90121
 iteration           12  OBJ=  -10783.6365593954 eff.=     120. Smpl.=     300. Fit.= 0.89918
 iteration           13  OBJ=  -10783.3635888057 eff.=     121. Smpl.=     300. Fit.= 0.89952
 iteration           14  OBJ=  -10782.8298941250 eff.=     120. Smpl.=     300. Fit.= 0.89886
 iteration           15  OBJ=  -10782.2072336385 eff.=     121. Smpl.=     300. Fit.= 0.90008
 iteration           16  OBJ=  -10780.4080025582 eff.=     121. Smpl.=     300. Fit.= 0.90062
 iteration           17  OBJ=  -10781.8742825534 eff.=     119. Smpl.=     300. Fit.= 0.89827
 iteration           18  OBJ=  -10780.3313077154 eff.=     122. Smpl.=     300. Fit.= 0.90084
 iteration           19  OBJ=  -10779.9979100337 eff.=     121. Smpl.=     300. Fit.= 0.90029
 iteration           20  OBJ=  -10782.1940408415 eff.=     120. Smpl.=     300. Fit.= 0.89939
 iteration           21  OBJ=  -10783.1108175392 eff.=     120. Smpl.=     300. Fit.= 0.89887
 iteration           22  OBJ=  -10783.4032334777 eff.=     124. Smpl.=     300. Fit.= 0.90180
 iteration           23  OBJ=  -10779.1301696864 eff.=     118. Smpl.=     300. Fit.= 0.89863
 iteration           24  OBJ=  -10781.1048307838 eff.=     122. Smpl.=     300. Fit.= 0.90100
 iteration           25  OBJ=  -10779.8308732325 eff.=     120. Smpl.=     300. Fit.= 0.89958
 iteration           26  OBJ=  -10779.9901961336 eff.=     120. Smpl.=     300. Fit.= 0.90007
 iteration           27  OBJ=  -10785.9691747328 eff.=     122. Smpl.=     300. Fit.= 0.89979
 Convergence achieved
 iteration           27  OBJ=  -10779.7875948519 eff.=     121. Smpl.=     300. Fit.= 0.90059
 
 #TERM:
 OPTIMIZATION WAS COMPLETED


 ETABAR IS THE ARITHMETIC MEAN OF THE ETA-ESTIMATES,
 AND THE P-VALUE IS GIVEN FOR THE NULL HYPOTHESIS THAT THE TRUE MEAN IS 0.
 
 ETABAR:        -1.0080E-04 -1.0444E-04  5.7553E-05 -1.0926E-04
 SE:             4.7111E-03  2.9636E-03  2.8819E-03  3.6677E-03
 N:                     400         400         400         400
 
 P VAL.:         9.8293E-01  9.7189E-01  9.8407E-01  9.7623E-01
 
 ETASHRINKSD(%)  7.0460E+00  3.3603E+01  4.1539E+01  2.5292E+01
 ETASHRINKVR(%)  1.3596E+01  5.5914E+01  6.5824E+01  4.4187E+01
 EBVSHRINKSD(%)  7.0461E+00  3.3662E+01  4.1563E+01  2.5161E+01
 EBVSHRINKVR(%)  1.3596E+01  5.5992E+01  6.5851E+01  4.3991E+01
 RELATIVEINF(%)  7.7386E+01  4.1222E+01  2.7184E+01  4.1002E+01
 EPSSHRINKSD(%)  2.6320E+01
 EPSSHRINKVR(%)  4.5713E+01
 
  
 TOTAL DATA POINTS NORMALLY DISTRIBUTED (N):         2000
 N*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    3675.75413281869     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -10779.7875948519     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -7104.03346203324     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 TOTAL EFFECTIVE ETAS (NIND*NETA):                          1600
  
 #TERE:
 Elapsed estimation  time in seconds:   169.07
 Elapsed covariance  time in seconds:    17.32
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          IMPORTANCE SAMPLING (NO PRIOR)                        ********************
 #OBJT:**************                        FINAL VALUE OF OBJECTIVE FUNCTION                       ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 





 #OBJV:********************************************        -10779.788       *********************************************
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          IMPORTANCE SAMPLING (NO PRIOR)                        ********************
 ********************                             FINAL PARAMETER ESTIMATE                           ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11     
 
         3.30E+00  3.25E+00 -6.12E-01 -2.08E-01  7.32E-01  1.14E+00  3.36E-01  1.92E-01  6.90E-01  2.30E+00  1.00E-01
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        1.03E-02
 
 ETA2
+        1.73E-04  7.97E-03
 
 ETA3
+        1.19E-03 -1.99E-04  9.72E-03
 
 ETA4
+       -6.57E-04  4.91E-04  1.80E-03  9.64E-03
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        1.00E+00
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        1.01E-01
 
 ETA2
+        1.91E-02  8.93E-02
 
 ETA3
+        1.19E-01 -2.26E-02  9.86E-02
 
 ETA4
+       -6.60E-02  5.60E-02  1.86E-01  9.82E-02
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        1.00E+00
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          IMPORTANCE SAMPLING (NO PRIOR)                        ********************
 ********************                          STANDARD ERROR OF ESTIMATE (R)                        ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11     
 
         3.27E-02  2.88E-02  9.53E-03  8.35E-03  3.94E-02  3.64E-02  1.14E-02  1.05E-02  1.05E-02  8.61E-03  2.80E-03
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        9.71E-04
 
 ETA2
+        8.28E-04  1.35E-03
 
 ETA3
+        1.31E-03  1.40E-03  3.32E-03
 
 ETA4
+        1.02E-03  1.08E-03  2.29E-03  1.96E-03
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        0.00E+00
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        4.79E-03
 
 ETA2
+        9.07E-02  7.58E-03
 
 ETA3
+        1.19E-01  1.61E-01  1.68E-02
 
 ETA4
+        1.06E-01  1.21E-01  1.94E-01  9.99E-03
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+       .........
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          IMPORTANCE SAMPLING (NO PRIOR)                        ********************
 ********************                        COVARIANCE MATRIX OF ESTIMATE (R)                       ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 TH 1
+        1.07E-03
 
 TH 2
+        1.56E-05  8.29E-04
 
 TH 3
+       -3.02E-04 -2.90E-06  9.09E-05
 
 TH 4
+       -1.03E-06 -2.31E-04  1.12E-07  6.97E-05
 
 TH 5
+        2.93E-04  2.63E-05 -7.90E-05 -2.72E-06  1.55E-03
 
 TH 6
+        3.03E-05  3.10E-04 -6.05E-06 -8.10E-05  4.51E-05  1.33E-03
 
 TH 7
+       -8.15E-05 -8.40E-06  2.29E-05  9.16E-07 -4.34E-04 -1.41E-05  1.29E-04
 
 TH 8
+       -5.05E-06 -8.31E-05  1.05E-06  2.38E-05 -6.63E-06 -3.68E-04  2.19E-06  1.11E-04
 
 TH 9
+        4.35E-05  4.60E-05 -9.20E-06 -4.78E-06  5.89E-05  7.36E-05 -1.86E-05 -1.24E-05  1.10E-04
 
 TH10
+        2.71E-05  3.21E-05 -5.03E-06 -3.64E-06  5.32E-05  6.16E-05 -1.56E-05 -9.57E-06  6.33E-05  7.42E-05
 
 TH11
+        1.03E-06 -3.24E-07 -3.26E-07  1.29E-07  3.31E-06 -2.79E-06 -1.01E-06  8.47E-07  1.27E-06  9.13E-07  7.87E-06
 
 OM11
+        3.62E-07 -1.93E-07 -3.07E-08  8.90E-08  4.84E-07  4.23E-07 -3.87E-08 -9.52E-09  3.86E-07  3.37E-07 -4.17E-07  9.43E-07
 
 OM12
+        4.68E-08 -3.08E-07  5.17E-08  1.48E-07 -2.53E-08 -3.65E-07  1.58E-07  2.28E-07  3.04E-07  2.77E-07 -3.93E-07  2.78E-07
          6.86E-07
 
 OM13
+        9.51E-07 -8.87E-08 -8.44E-08  8.82E-08  2.14E-06  1.26E-06 -4.50E-07 -1.12E-07  8.35E-07  8.34E-07 -8.36E-07  5.58E-07
          2.23E-07  1.72E-06
 
 OM14
+        1.22E-06 -1.54E-06 -2.73E-07  5.44E-07  9.92E-07  8.33E-07 -1.79E-07 -8.57E-08  6.45E-07  4.38E-07 -6.43E-07  3.70E-07
          2.12E-07  9.47E-07  1.03E-06
 
 OM22
+       -7.40E-07 -4.11E-07  2.58E-07  1.18E-07 -2.24E-06 -1.73E-06  8.68E-07  5.57E-07 -5.97E-07 -4.13E-07 -1.08E-06  7.66E-08
          3.45E-07  5.43E-08  5.23E-08  1.83E-06
 
 OM23
+        8.91E-07 -8.18E-09 -1.67E-07  8.56E-08  2.64E-06 -9.71E-07 -4.75E-07  4.15E-07  1.14E-06  7.73E-07 -5.06E-07  1.80E-07
          3.28E-07  6.69E-07  3.40E-07  2.95E-07  1.96E-06
 
 OM24
+        2.05E-07 -4.80E-07 -1.55E-08  1.60E-07  1.38E-06 -2.95E-06 -2.63E-07  9.27E-07  4.36E-07  2.94E-07 -3.04E-07  9.53E-08
          2.13E-07  2.58E-07  2.98E-07  3.69E-07  9.10E-07  1.17E-06
 
 OM33
+        1.64E-06  1.72E-06 -1.99E-07 -1.68E-07  4.25E-06  6.55E-06 -1.00E-06 -1.27E-06  1.13E-06  1.25E-06 -3.88E-06  5.36E-07
          2.96E-07  2.31E-06  1.25E-06  2.05E-07  1.61E-06  4.70E-07  1.10E-05
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 OM34
+        8.79E-07  1.88E-06 -8.94E-08 -3.43E-07  1.98E-06  6.15E-06 -4.40E-07 -1.42E-06  7.59E-07  5.75E-07 -2.70E-06  3.46E-07
          2.02E-07  1.44E-06  1.01E-06  8.47E-08  1.10E-06  4.63E-07  6.65E-06  5.24E-06
 
 OM44
+        1.78E-07  1.69E-06  3.59E-08 -3.89E-07  1.63E-07  5.55E-06  2.11E-08 -1.43E-06  1.55E-07  1.99E-07 -1.96E-06  2.16E-07
          1.31E-07  8.17E-07  7.68E-07  4.41E-08  5.72E-07  5.11E-07  3.71E-06  3.62E-06  3.85E-06
 
 SG11
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          IMPORTANCE SAMPLING (NO PRIOR)                        ********************
 ********************                        CORRELATION MATRIX OF ESTIMATE (R)                      ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 TH 1
+        3.27E-02
 
 TH 2
+        1.66E-02  2.88E-02
 
 TH 3
+       -9.68E-01 -1.06E-02  9.53E-03
 
 TH 4
+       -3.79E-03 -9.59E-01  1.41E-03  8.35E-03
 
 TH 5
+        2.27E-01  2.32E-02 -2.10E-01 -8.27E-03  3.94E-02
 
 TH 6
+        2.54E-02  2.96E-01 -1.74E-02 -2.66E-01  3.14E-02  3.64E-02
 
 TH 7
+       -2.19E-01 -2.57E-02  2.11E-01  9.65E-03 -9.69E-01 -3.41E-02  1.14E-02
 
 TH 8
+       -1.47E-02 -2.74E-01  1.04E-02  2.70E-01 -1.60E-02 -9.59E-01  1.83E-02  1.05E-02
 
 TH 9
+        1.27E-01  1.52E-01 -9.20E-02 -5.46E-02  1.43E-01  1.92E-01 -1.56E-01 -1.12E-01  1.05E-02
 
 TH10
+        9.62E-02  1.30E-01 -6.13E-02 -5.05E-02  1.57E-01  1.96E-01 -1.59E-01 -1.06E-01  7.00E-01  8.61E-03
 
 TH11
+        1.13E-02 -4.01E-03 -1.22E-02  5.50E-03  3.00E-02 -2.73E-02 -3.17E-02  2.87E-02  4.30E-02  3.78E-02  2.80E-03
 
 OM11
+        1.14E-02 -6.90E-03 -3.31E-03  1.10E-02  1.26E-02  1.20E-02 -3.51E-03 -9.30E-04  3.79E-02  4.03E-02 -1.53E-01  9.71E-04
 
 OM12
+        1.73E-03 -1.29E-02  6.55E-03  2.15E-02 -7.76E-04 -1.21E-02  1.67E-02  2.62E-02  3.50E-02  3.88E-02 -1.69E-01  3.45E-01
          8.28E-04
 
 OM13
+        2.22E-02 -2.35E-03 -6.74E-03  8.05E-03  4.13E-02  2.64E-02 -3.02E-02 -8.10E-03  6.06E-02  7.38E-02 -2.27E-01  4.38E-01
          2.05E-01  1.31E-03
 
 OM14
+        3.66E-02 -5.27E-02 -2.82E-02  6.41E-02  2.48E-02  2.25E-02 -1.55E-02 -8.00E-03  6.05E-02  5.00E-02 -2.26E-01  3.74E-01
          2.51E-01  7.10E-01  1.02E-03
 
 OM22
+       -1.67E-02 -1.06E-02  2.00E-02  1.05E-02 -4.20E-02 -3.52E-02  5.64E-02  3.91E-02 -4.20E-02 -3.55E-02 -2.85E-01  5.83E-02
          3.07E-01  3.06E-02  3.80E-02  1.35E-03
 
 OM23
+        1.95E-02 -2.03E-04 -1.25E-02  7.32E-03  4.79E-02 -1.90E-02 -2.99E-02  2.82E-02  7.74E-02  6.41E-02 -1.29E-01  1.32E-01
          2.83E-01  3.64E-01  2.39E-01  1.56E-01  1.40E-03
 
 OM24
+        5.81E-03 -1.54E-02 -1.51E-03  1.77E-02  3.25E-02 -7.48E-02 -2.14E-02  8.14E-02  3.84E-02  3.16E-02 -1.00E-01  9.08E-02
          2.38E-01  1.82E-01  2.71E-01  2.52E-01  6.02E-01  1.08E-03
 
 OM33
+        1.51E-02  1.80E-02 -6.28E-03 -6.07E-03  3.25E-02  5.42E-02 -2.66E-02 -3.65E-02  3.23E-02  4.39E-02 -4.18E-01  1.66E-01
          1.08E-01  5.32E-01  3.69E-01  4.58E-02  3.47E-01  1.31E-01  3.32E-03
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 OM34
+        1.17E-02  2.86E-02 -4.10E-03 -1.80E-02  2.19E-02  7.37E-02 -1.69E-02 -5.89E-02  3.16E-02  2.92E-02 -4.20E-01  1.56E-01
          1.06E-01  4.80E-01  4.34E-01  2.73E-02  3.43E-01  1.87E-01  8.76E-01  2.29E-03
 
 OM44
+        2.77E-03  2.99E-02  1.92E-03 -2.37E-02  2.11E-03  7.76E-02  9.45E-04 -6.92E-02  7.51E-03  1.18E-02 -3.57E-01  1.13E-01
          8.07E-02  3.18E-01  3.85E-01  1.66E-02  2.08E-01  2.41E-01  5.70E-01  8.07E-01  1.96E-03
 
 SG11
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          IMPORTANCE SAMPLING (NO PRIOR)                        ********************
 ********************                    INVERSE COVARIANCE MATRIX OF ESTIMATE (R)                   ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 TH 1
+        1.58E+04
 
 TH 2
+        0.00E+00  1.84E+04
 
 TH 3
+        5.22E+04  1.94E-07  1.84E+05
 
 TH 4
+        2.49E-07  6.05E+04  4.36E-06  2.14E+05
 
 TH 5
+       -2.12E+03  3.65E-08 -6.69E+03  1.69E-06  1.11E+04
 
 TH 6
+       -2.72E-09 -3.52E+03  5.39E-07 -1.13E+04  9.21E-08  1.15E+04
 
 TH 7
+       -6.69E+03 -4.92E-07 -2.31E+04 -5.24E-06  3.74E+04 -1.44E-06  1.34E+05
 
 TH 8
+        3.59E-07 -1.13E+04  5.21E-06 -3.96E+04  2.24E-06  3.76E+04 -4.80E-06  1.33E+05
 
 TH 9
+       -1.31E+03 -3.84E+03 -3.49E+03 -1.24E+04  1.10E+03 -6.23E+02  4.26E+03 -1.41E+03  1.93E+04
 
 TH10
+       -9.67E+02 -2.95E+02 -3.53E+03 -9.09E+02 -6.94E+02 -3.16E+03 -1.30E+03 -9.75E+03 -1.46E+04  2.79E+04
 
 TH11
+       -2.69E+02 -2.71E+02 -7.68E+02 -1.19E+03 -1.06E+03 -8.94E+02 -2.96E+03 -3.69E+03 -1.06E+03 -1.08E+03  1.77E+05
 
 OM11
+        7.60E+01  4.44E+02  4.69E+02  1.60E+03 -1.30E+03 -5.67E+02 -4.69E+03 -1.58E+03 -1.57E+03  3.83E+02  3.44E+04  1.47E+06
 
 OM12
+       -1.98E+03 -1.39E+03 -8.15E+03 -4.73E+03 -3.51E+03 -3.32E+03 -1.33E+04 -1.28E+04  2.06E+02 -3.47E+03  2.27E+04 -4.85E+05
          1.97E+06
 
 OM13
+       -6.09E+03  5.42E+03 -2.41E+04  2.68E+04 -9.64E+01 -2.28E+03  1.35E+03 -1.04E+04  3.93E+03 -7.09E+03 -2.41E+04 -4.38E+05
          1.59E+05  1.71E+06
 
 OM14
+        3.26E+03 -3.30E+03  1.74E+04 -2.98E+04 -1.99E+03  2.98E+02 -8.46E+03  5.24E+03 -7.42E+03  5.73E+03  3.21E+04 -8.79E+04
         -3.01E+05 -1.27E+06  2.39E+06
 
 OM22
+       -9.10E+02 -1.14E+03 -3.17E+03 -3.96E+03 -3.03E+03 -1.71E+03 -1.35E+04 -6.33E+03  2.34E+03  1.83E+03  9.88E+04  4.86E+04
         -2.93E+05 -1.30E+04  8.22E+04  6.96E+05
 
 OM23
+        2.22E+03 -1.15E+02  8.93E+03 -2.71E+03 -5.45E+03  2.34E+03 -1.85E+04  9.88E+03 -6.02E+03  7.13E+02 -1.55E+04  7.68E+04
         -2.64E+05 -3.47E+05  3.22E+05  3.13E+04  1.06E+06
 
 OM24
+       -7.19E+02  1.91E+03 -4.51E+03  1.05E+04  1.36E+03  8.53E+02  7.74E+03 -8.62E+03 -8.41E+01 -1.89E+03 -1.92E+04  9.55E+02
          1.33E+04  2.58E+05 -4.64E+05 -2.13E+05 -7.81E+05  1.63E+06
 
 OM33
+       -1.16E+01 -1.76E+03  6.24E+02 -8.30E+03 -3.83E+02 -1.07E+03 -5.14E+02 -4.15E+03  3.37E+03 -4.43E+03  4.68E+04  3.39E+04
         -9.14E+03 -2.84E+05  2.22E+05 -1.19E+02  1.28E+04 -1.13E+04  5.71E+05
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 OM34
+       -5.29E+02  5.28E+02 -2.83E+03  5.88E+03  9.60E+02 -9.77E+02  3.59E+03 -2.66E+03 -6.80E+03  9.47E+03  5.44E+03  1.37E+04
          5.92E+04  1.89E+05 -3.62E+05  1.80E+04 -2.79E+05  2.49E+05 -9.02E+05  2.18E+06
 
 OM44
+       -1.34E+02 -5.81E+02 -5.18E+02 -1.58E+03 -3.25E+02 -3.99E+02 -2.30E+03  2.76E+03  5.04E+03 -4.39E+03  4.01E+04  4.95E+03
         -7.51E+03  1.09E+04 -3.82E+04  4.31E+04  2.03E+05 -2.99E+05  3.37E+05 -1.15E+06  1.05E+06
 
 SG11
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
1
 
 
 #TBLN:      3
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
 NO. OF FUNCT. EVALS. ALLOWED:            2208
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
 RAW OUTPUT FILE (FILE): example2.ext
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
 GRADIENT/GIBBS PATTERN (GRD):              DDDDDDDDDDS
 AUTOMATIC SETTING FEATURE (AUTO):          0
 CONVERGENCE TYPE (CTYPE):                  3
 CONVERGENCE INTERVAL (CINTERVAL):          10
 CONVERGENCE ITERATIONS (CITER):            10
 CONVERGENCE ALPHA ERROR (CALPHA):          5.000000000000000E-02
 BURN-IN ITERATIONS (NBURN):                3000
 FIRST ITERATION FOR MAP (MAPITERS):          NO
 ITERATIONS (NITER):                        2000
 ANNEAL SETTING (CONSTRAIN):                 1
 STARTING SEED FOR MC METHODS (SEED):       11456
 MC SAMPLES PER SUBJECT (ISAMPLE):          3
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
   1   2   3   4   5   6   7   8   9  10
 THETAS THAT ARE SIGMA-LIKE:
  11
 
 MONITORING OF SEARCH:

 Stochastic/Burn-in Mode
 iteration        -3000  SAEMOBJ=  -19860.8591080143
 iteration        -2990  SAEMOBJ=  -19853.1415635580
 iteration        -2980  SAEMOBJ=  -20014.8041840990
 iteration        -2970  SAEMOBJ=  -19969.4453353753
 iteration        -2960  SAEMOBJ=  -19993.9653119825
 iteration        -2950  SAEMOBJ=  -20008.9845288759
 iteration        -2940  SAEMOBJ=  -20114.5938443151
 iteration        -2930  SAEMOBJ=  -20122.3671439133
 iteration        -2920  SAEMOBJ=  -20074.2976160075
 iteration        -2910  SAEMOBJ=  -20005.2578108354
 iteration        -2900  SAEMOBJ=  -20025.7795116102
 iteration        -2890  SAEMOBJ=  -20092.6838958001
 Convergence achieved
 Elapsed burn-in time in seconds:    40.93
 Reduced Stochastic/Accumulation Mode
 iteration            0  SAEMOBJ=  -20084.4940658830
 iteration           10  SAEMOBJ=  -20172.0307261057
 iteration           20  SAEMOBJ=  -20175.6315285707
 iteration           30  SAEMOBJ=  -20177.0215372212
 iteration           40  SAEMOBJ=  -20178.8800041898
 iteration           50  SAEMOBJ=  -20180.1757897034
 iteration           60  SAEMOBJ=  -20183.6295353380
 iteration           70  SAEMOBJ=  -20182.7971789167
 iteration           80  SAEMOBJ=  -20183.7960061979
 iteration           90  SAEMOBJ=  -20183.9597603934
 iteration          100  SAEMOBJ=  -20182.2202058056
 iteration          110  SAEMOBJ=  -20181.6358620189
 iteration          120  SAEMOBJ=  -20181.1717236590
 iteration          130  SAEMOBJ=  -20181.9443712711
 iteration          140  SAEMOBJ=  -20181.9977872624
 iteration          150  SAEMOBJ=  -20182.0287127880
 iteration          160  SAEMOBJ=  -20182.3364728175
 iteration          170  SAEMOBJ=  -20181.9601359032
 iteration          180  SAEMOBJ=  -20182.2364353279
 iteration          190  SAEMOBJ=  -20181.7326816718
 iteration          200  SAEMOBJ=  -20182.0017843286
 iteration          210  SAEMOBJ=  -20181.5666440318
 iteration          220  SAEMOBJ=  -20181.2217960057
 iteration          230  SAEMOBJ=  -20180.5301106785
 iteration          240  SAEMOBJ=  -20180.5485059314
 iteration          250  SAEMOBJ=  -20180.4947621895
 iteration          260  SAEMOBJ=  -20179.9317837514
 iteration          270  SAEMOBJ=  -20179.5829344930
 iteration          280  SAEMOBJ=  -20179.3135676016
 iteration          290  SAEMOBJ=  -20179.1720808950
 iteration          300  SAEMOBJ=  -20179.0799766611
 iteration          310  SAEMOBJ=  -20178.7311576296
 iteration          320  SAEMOBJ=  -20178.4703677746
 iteration          330  SAEMOBJ=  -20178.2563988696
 iteration          340  SAEMOBJ=  -20178.1529743814
 iteration          350  SAEMOBJ=  -20177.7720674041
 iteration          360  SAEMOBJ=  -20177.7772768866
 iteration          370  SAEMOBJ=  -20177.6476826894
 iteration          380  SAEMOBJ=  -20177.4038172589
 iteration          390  SAEMOBJ=  -20177.3405388216
 iteration          400  SAEMOBJ=  -20177.0087249508
 iteration          410  SAEMOBJ=  -20176.6733698477
 iteration          420  SAEMOBJ=  -20176.7018247280
 iteration          430  SAEMOBJ=  -20176.5657172515
 iteration          440  SAEMOBJ=  -20176.4311253513
 iteration          450  SAEMOBJ=  -20176.3178384280
 iteration          460  SAEMOBJ=  -20176.0896858689
 iteration          470  SAEMOBJ=  -20175.9710225436
 iteration          480  SAEMOBJ=  -20175.6175565585
 iteration          490  SAEMOBJ=  -20175.4540362937
 iteration          500  SAEMOBJ=  -20175.5678648573
 iteration          510  SAEMOBJ=  -20175.5875749493
 iteration          520  SAEMOBJ=  -20175.7259354992
 iteration          530  SAEMOBJ=  -20175.7746750520
 iteration          540  SAEMOBJ=  -20175.7528375589
 iteration          550  SAEMOBJ=  -20175.4313779150
 iteration          560  SAEMOBJ=  -20175.3736766359
 iteration          570  SAEMOBJ=  -20175.2625408072
 iteration          580  SAEMOBJ=  -20175.1789270217
 iteration          590  SAEMOBJ=  -20174.9633531736
 iteration          600  SAEMOBJ=  -20174.8465971412
 iteration          610  SAEMOBJ=  -20174.7991179508
 iteration          620  SAEMOBJ=  -20174.6760067090
 iteration          630  SAEMOBJ=  -20174.6509373929
 iteration          640  SAEMOBJ=  -20174.5823524884
 iteration          650  SAEMOBJ=  -20174.4749961426
 iteration          660  SAEMOBJ=  -20174.3829487137
 iteration          670  SAEMOBJ=  -20174.1740844534
 iteration          680  SAEMOBJ=  -20174.0678516404
 iteration          690  SAEMOBJ=  -20174.0478068178
 iteration          700  SAEMOBJ=  -20174.0523214425
 iteration          710  SAEMOBJ=  -20174.0157973481
 iteration          720  SAEMOBJ=  -20173.8337859459
 iteration          730  SAEMOBJ=  -20173.8830480247
 iteration          740  SAEMOBJ=  -20173.9516467689
 iteration          750  SAEMOBJ=  -20173.8021577067
 iteration          760  SAEMOBJ=  -20173.7058243410
 iteration          770  SAEMOBJ=  -20173.6958996936
 iteration          780  SAEMOBJ=  -20173.7071320072
 iteration          790  SAEMOBJ=  -20173.6206175769
 iteration          800  SAEMOBJ=  -20173.6570964049
 iteration          810  SAEMOBJ=  -20173.6860766407
 iteration          820  SAEMOBJ=  -20173.5434930988
 iteration          830  SAEMOBJ=  -20173.4242501082
 iteration          840  SAEMOBJ=  -20173.1127288273
 iteration          850  SAEMOBJ=  -20173.0066049616
 iteration          860  SAEMOBJ=  -20172.9618010918
 iteration          870  SAEMOBJ=  -20172.9073083794
 iteration          880  SAEMOBJ=  -20172.9364080606
 iteration          890  SAEMOBJ=  -20172.9408037796
 iteration          900  SAEMOBJ=  -20172.9692403283
 iteration          910  SAEMOBJ=  -20172.8214559292
 iteration          920  SAEMOBJ=  -20172.7800625765
 iteration          930  SAEMOBJ=  -20172.7624383798
 iteration          940  SAEMOBJ=  -20172.6921293602
 iteration          950  SAEMOBJ=  -20172.6435260310
 iteration          960  SAEMOBJ=  -20172.7550554247
 iteration          970  SAEMOBJ=  -20172.4788878926
 iteration          980  SAEMOBJ=  -20172.3669715611
 iteration          990  SAEMOBJ=  -20172.3367419545
 iteration         1000  SAEMOBJ=  -20172.0886081787
 iteration         1010  SAEMOBJ=  -20172.0454636166
 iteration         1020  SAEMOBJ=  -20171.9929268194
 iteration         1030  SAEMOBJ=  -20171.8393884313
 iteration         1040  SAEMOBJ=  -20171.7474745407
 iteration         1050  SAEMOBJ=  -20171.6388553007
 iteration         1060  SAEMOBJ=  -20171.5731805187
 iteration         1070  SAEMOBJ=  -20171.5233589417
 iteration         1080  SAEMOBJ=  -20171.4668766913
 iteration         1090  SAEMOBJ=  -20171.4095303295
 iteration         1100  SAEMOBJ=  -20171.4447478101
 iteration         1110  SAEMOBJ=  -20171.4119008341
 iteration         1120  SAEMOBJ=  -20171.2949505655
 iteration         1130  SAEMOBJ=  -20171.2414779584
 iteration         1140  SAEMOBJ=  -20171.1319134163
 iteration         1150  SAEMOBJ=  -20171.2283432196
 iteration         1160  SAEMOBJ=  -20171.2190457880
 iteration         1170  SAEMOBJ=  -20171.2708966652
 iteration         1180  SAEMOBJ=  -20171.2814937264
 iteration         1190  SAEMOBJ=  -20171.1760172495
 iteration         1200  SAEMOBJ=  -20171.1199402129
 iteration         1210  SAEMOBJ=  -20171.1099315319
 iteration         1220  SAEMOBJ=  -20171.1618384496
 iteration         1230  SAEMOBJ=  -20171.1137188370
 iteration         1240  SAEMOBJ=  -20171.0882229466
 iteration         1250  SAEMOBJ=  -20171.0103530383
 iteration         1260  SAEMOBJ=  -20171.1241235325
 iteration         1270  SAEMOBJ=  -20171.0369570695
 iteration         1280  SAEMOBJ=  -20170.9875032038
 iteration         1290  SAEMOBJ=  -20170.9003995556
 iteration         1300  SAEMOBJ=  -20170.8446468765
 iteration         1310  SAEMOBJ=  -20170.8123246169
 iteration         1320  SAEMOBJ=  -20170.7408373807
 iteration         1330  SAEMOBJ=  -20170.7139458423
 iteration         1340  SAEMOBJ=  -20170.6674636266
 iteration         1350  SAEMOBJ=  -20170.6209761573
 iteration         1360  SAEMOBJ=  -20170.5769723502
 iteration         1370  SAEMOBJ=  -20170.5446178254
 iteration         1380  SAEMOBJ=  -20170.5298012790
 iteration         1390  SAEMOBJ=  -20170.5067509067
 iteration         1400  SAEMOBJ=  -20170.4464571033
 iteration         1410  SAEMOBJ=  -20170.3896393146
 iteration         1420  SAEMOBJ=  -20170.3421821631
 iteration         1430  SAEMOBJ=  -20170.2735456985
 iteration         1440  SAEMOBJ=  -20170.3099178274
 iteration         1450  SAEMOBJ=  -20170.2951265662
 iteration         1460  SAEMOBJ=  -20170.2498624731
 iteration         1470  SAEMOBJ=  -20170.1880838285
 iteration         1480  SAEMOBJ=  -20170.1172614046
 iteration         1490  SAEMOBJ=  -20170.1185993780
 iteration         1500  SAEMOBJ=  -20170.0504606760
 iteration         1510  SAEMOBJ=  -20170.0008908321
 iteration         1520  SAEMOBJ=  -20170.0127397294
 iteration         1530  SAEMOBJ=  -20170.0323213293
 iteration         1540  SAEMOBJ=  -20169.9974641400
 iteration         1550  SAEMOBJ=  -20169.9919503686
 iteration         1560  SAEMOBJ=  -20169.9518235762
 iteration         1570  SAEMOBJ=  -20169.8675234417
 iteration         1580  SAEMOBJ=  -20169.8740812277
 iteration         1590  SAEMOBJ=  -20169.9520368758
 iteration         1600  SAEMOBJ=  -20169.9232831045
 iteration         1610  SAEMOBJ=  -20169.8916174718
 iteration         1620  SAEMOBJ=  -20169.8564275164
 iteration         1630  SAEMOBJ=  -20169.8040301078
 iteration         1640  SAEMOBJ=  -20169.7234799606
 iteration         1650  SAEMOBJ=  -20169.6994468055
 iteration         1660  SAEMOBJ=  -20169.6774703930
 iteration         1670  SAEMOBJ=  -20169.6859695659
 iteration         1680  SAEMOBJ=  -20169.6538820079
 iteration         1690  SAEMOBJ=  -20169.6322662319
 iteration         1700  SAEMOBJ=  -20169.5992734706
 iteration         1710  SAEMOBJ=  -20169.5323437374
 iteration         1720  SAEMOBJ=  -20169.4912267267
 iteration         1730  SAEMOBJ=  -20169.4874885513
 iteration         1740  SAEMOBJ=  -20169.4787245547
 iteration         1750  SAEMOBJ=  -20169.5349303922
 iteration         1760  SAEMOBJ=  -20169.5583615850
 iteration         1770  SAEMOBJ=  -20169.4794204386
 iteration         1780  SAEMOBJ=  -20169.4408091435
 iteration         1790  SAEMOBJ=  -20169.4018001791
 iteration         1800  SAEMOBJ=  -20169.3894024660
 iteration         1810  SAEMOBJ=  -20169.3638522729
 iteration         1820  SAEMOBJ=  -20169.3239168833
 iteration         1830  SAEMOBJ=  -20169.1853715463
 iteration         1840  SAEMOBJ=  -20169.1657511125
 iteration         1850  SAEMOBJ=  -20169.1680347451
 iteration         1860  SAEMOBJ=  -20169.1063593209
 iteration         1870  SAEMOBJ=  -20169.0575225732
 iteration         1880  SAEMOBJ=  -20169.0479369155
 iteration         1890  SAEMOBJ=  -20169.0475068593
 iteration         1900  SAEMOBJ=  -20169.0179128637
 iteration         1910  SAEMOBJ=  -20168.9632327145
 iteration         1920  SAEMOBJ=  -20168.9255929875
 iteration         1930  SAEMOBJ=  -20168.8901825152
 iteration         1940  SAEMOBJ=  -20168.8405362664
 iteration         1950  SAEMOBJ=  -20168.8321127640
 iteration         1960  SAEMOBJ=  -20168.7796233921
 iteration         1970  SAEMOBJ=  -20168.7609803894
 iteration         1980  SAEMOBJ=  -20168.7928869463
 iteration         1990  SAEMOBJ=  -20168.7854487336
 iteration         2000  SAEMOBJ=  -20168.6981763053
 
 #TERM:
 STOCHASTIC PORTION WAS COMPLETED
 REDUCED STOCHASTIC PORTION WAS COMPLETED

 ETABAR IS THE ARITHMETIC MEAN OF THE ETA-ESTIMATES,
 AND THE P-VALUE IS GIVEN FOR THE NULL HYPOTHESIS THAT THE TRUE MEAN IS 0.
 
 ETABAR:        -2.5492E-07  4.8496E-07 -2.4302E-08  1.8552E-08
 SE:             4.6648E-03  2.9151E-03  2.3811E-03  3.3863E-03
 N:                     400         400         400         400
 
 P VAL.:         9.9996E-01  9.9987E-01  9.9999E-01  1.0000E+00
 
 ETASHRINKSD(%)  5.9345E+00  3.2675E+01  3.1139E+01  1.6682E+01
 ETASHRINKVR(%)  1.1517E+01  5.4674E+01  5.2581E+01  3.0581E+01
 EBVSHRINKSD(%)  5.9342E+00  3.2677E+01  3.1139E+01  1.6681E+01
 EBVSHRINKVR(%)  1.1516E+01  5.4676E+01  5.2581E+01  3.0579E+01
 RELATIVEINF(%)  5.7597E+01  3.0657E+01  8.8855E+00  1.5624E+01
 EPSSHRINKSD(%)  2.4296E+01
 EPSSHRINKVR(%)  4.2690E+01
 
  
 TOTAL DATA POINTS NORMALLY DISTRIBUTED (N):         2000
 N*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    3675.75413281869     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -20168.6981763053     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -16492.9440434866     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 TOTAL EFFECTIVE ETAS (NIND*NETA):                          1600
 NIND*NETA*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    2940.60330625495     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -20168.6981763053     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -17228.0948700503     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 #TERE:
 Elapsed estimation  time in seconds:   755.93
 Elapsed covariance  time in seconds:     0.04
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************           STOCHASTIC APPROXIMATION EXPECTATION-MAXIMIZATION (NO PRIOR)         ********************
 #OBJT:**************                        FINAL VALUE OF LIKELIHOOD FUNCTION                      ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 





 #OBJV:********************************************        -20168.698       *********************************************
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************           STOCHASTIC APPROXIMATION EXPECTATION-MAXIMIZATION (NO PRIOR)         ********************
 ********************                             FINAL PARAMETER ESTIMATE                           ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11     
 
         3.30E+00  3.25E+00 -6.12E-01 -2.08E-01  7.33E-01  1.13E+00  3.35E-01  1.93E-01  6.90E-01  2.30E+00  1.03E-01
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        9.84E-03
 
 ETA2
+       -2.12E-04  7.50E-03
 
 ETA3
+       -2.17E-04 -1.88E-03  4.78E-03
 
 ETA4
+       -1.72E-03 -5.53E-04 -2.10E-03  6.61E-03
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        1.00E+00
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        9.92E-02
 
 ETA2
+       -2.47E-02  8.66E-02
 
 ETA3
+       -3.16E-02 -3.14E-01  6.92E-02
 
 ETA4
+       -2.13E-01 -7.85E-02 -3.74E-01  8.13E-02
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        1.00E+00
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************           STOCHASTIC APPROXIMATION EXPECTATION-MAXIMIZATION (NO PRIOR)         ********************
 ********************                          STANDARD ERROR OF ESTIMATE (S)                        ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11     
 
         3.79E-02  2.89E-02  1.10E-02  8.45E-03  4.75E-02  4.22E-02  1.32E-02  1.22E-02  9.61E-03  8.34E-03  3.17E-03
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        9.70E-04
 
 ETA2
+        8.17E-04  1.43E-03
 
 ETA3
+        1.16E-03  1.26E-03  2.44E-03
 
 ETA4
+        9.95E-04  9.93E-04  1.66E-03  1.71E-03
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        0.00E+00
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        4.89E-03
 
 ETA2
+        9.63E-02  8.28E-03
 
 ETA3
+        1.72E-01  2.54E-01  1.77E-02
 
 ETA4
+        1.39E-01  1.46E-01  4.08E-01  1.05E-02
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+       .........
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************           STOCHASTIC APPROXIMATION EXPECTATION-MAXIMIZATION (NO PRIOR)         ********************
 ********************                        COVARIANCE MATRIX OF ESTIMATE (S)                       ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 TH 1
+        1.43E-03
 
 TH 2
+       -2.02E-06  8.36E-04
 
 TH 3
+       -4.09E-04  2.23E-06  1.22E-04
 
 TH 4
+        5.61E-07 -2.34E-04 -3.88E-07  7.14E-05
 
 TH 5
+        6.11E-04  4.57E-05 -1.60E-04 -4.19E-06  2.25E-03
 
 TH 6
+       -1.73E-05  2.63E-04  5.63E-06 -6.58E-05  1.83E-05  1.78E-03
 
 TH 7
+       -1.59E-04 -1.46E-05  4.15E-05  1.69E-06 -6.13E-04 -8.94E-06  1.74E-04
 
 TH 8
+        4.61E-06 -6.70E-05 -1.28E-06  1.87E-05  2.23E-06 -4.97E-04  3.19E-07  1.48E-04
 
 TH 9
+        1.12E-05  5.42E-05  1.80E-07 -9.25E-06  9.55E-05  5.09E-05 -2.82E-05 -8.95E-06  9.23E-05
 
 TH10
+        9.29E-06  3.24E-05 -3.29E-08 -4.70E-06  9.35E-05  4.89E-05 -2.72E-05 -6.50E-06  5.16E-05  6.96E-05
 
 TH11
+       -6.37E-06 -2.55E-06  2.21E-06  1.05E-06 -9.02E-06  5.97E-07  2.77E-06  5.49E-07  3.02E-06  2.99E-06  1.00E-05
 
 OM11
+        3.80E-06 -2.57E-07 -1.23E-06 -1.54E-07  9.04E-07  4.67E-07 -3.74E-07 -1.56E-07 -4.18E-07  1.02E-08 -4.55E-07  9.41E-07
 
 OM12
+        1.16E-06  2.09E-07 -5.51E-07 -8.29E-08  1.29E-06  9.30E-07 -2.83E-07 -1.29E-07 -2.61E-07  1.74E-07 -5.52E-07  2.98E-07
          6.67E-07
 
 OM13
+       -4.07E-07  1.67E-06 -1.06E-07 -5.56E-07  1.47E-06  1.64E-07 -4.48E-07 -6.01E-08  6.56E-07  5.05E-07 -6.32E-07  3.64E-07
          1.75E-07  1.34E-06
 
 OM14
+        1.39E-06 -6.45E-07 -5.73E-07  3.07E-07  5.32E-06 -4.44E-07 -1.35E-06  1.20E-07  5.99E-07  6.16E-07 -3.78E-07  2.87E-07
          1.59E-07  7.51E-07  9.90E-07
 
 OM22
+        3.31E-06  9.96E-08 -9.58E-07 -1.00E-08  6.08E-06 -7.00E-06 -1.17E-06  1.85E-06 -6.60E-07 -7.45E-07 -1.05E-06  2.86E-08
          3.42E-07 -3.85E-08  4.78E-08  2.05E-06
 
 OM23
+       -8.42E-07  5.09E-07  1.31E-07 -2.63E-07 -1.99E-06  2.06E-06  4.86E-07 -4.15E-07 -4.35E-07 -2.71E-07 -8.13E-07  1.93E-07
          2.42E-07  5.63E-07  2.98E-07  2.25E-07  1.59E-06
 
 OM24
+        1.95E-06 -5.17E-07 -4.79E-07  9.58E-08  4.03E-06 -1.75E-06 -1.10E-06  6.98E-07  1.34E-07  8.05E-07 -3.91E-07  1.06E-07
          2.02E-07  2.86E-07  3.41E-07  2.78E-07  7.02E-07  9.86E-07
 
 OM33
+        2.24E-06  2.47E-07 -5.99E-07 -6.04E-08  1.02E-05 -4.21E-06 -3.06E-06  6.05E-07 -1.70E-06 -1.67E-06 -3.73E-06  2.90E-07
          3.02E-07  8.16E-07  3.10E-07  1.30E-07  1.02E-06  3.72E-07  5.98E-06
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 OM34
+        1.58E-06  1.86E-06 -4.76E-07 -5.39E-07  6.76E-06  2.61E-06 -2.19E-06 -8.90E-07 -8.23E-07 -3.92E-07 -2.65E-06  2.71E-07
          2.39E-07  5.31E-07  4.64E-07  4.43E-08  7.98E-07  4.08E-07  3.16E-06  2.75E-06
 
 OM44
+       -9.25E-07  2.98E-06  2.03E-07 -6.77E-07  7.06E-06  5.73E-06 -2.26E-06 -1.25E-06  4.58E-07  2.23E-06 -2.11E-06  2.81E-07
          2.39E-07  4.44E-07  5.80E-07 -6.75E-08  5.02E-07  5.68E-07  1.57E-06  2.03E-06  2.94E-06
 
 SG11
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************           STOCHASTIC APPROXIMATION EXPECTATION-MAXIMIZATION (NO PRIOR)         ********************
 ********************                        CORRELATION MATRIX OF ESTIMATE (S)                      ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 TH 1
+        3.79E-02
 
 TH 2
+       -1.85E-03  2.89E-02
 
 TH 3
+       -9.77E-01  6.99E-03  1.10E-02
 
 TH 4
+        1.75E-03 -9.57E-01 -4.16E-03  8.45E-03
 
 TH 5
+        3.40E-01  3.33E-02 -3.04E-01 -1.04E-02  4.75E-02
 
 TH 6
+       -1.08E-02  2.15E-01  1.21E-02 -1.84E-01  9.11E-03  4.22E-02
 
 TH 7
+       -3.18E-01 -3.83E-02  2.85E-01  1.52E-02 -9.80E-01 -1.61E-02  1.32E-02
 
 TH 8
+        1.00E-02 -1.91E-01 -9.54E-03  1.82E-01  3.87E-03 -9.68E-01  1.99E-03  1.22E-02
 
 TH 9
+        3.08E-02  1.95E-01  1.70E-03 -1.14E-01  2.09E-01  1.25E-01 -2.23E-01 -7.67E-02  9.61E-03
 
 TH10
+        2.94E-02  1.34E-01 -3.56E-04 -6.66E-02  2.36E-01  1.39E-01 -2.47E-01 -6.41E-02  6.44E-01  8.34E-03
 
 TH11
+       -5.32E-02 -2.78E-02  6.32E-02  3.94E-02 -6.00E-02  4.46E-03  6.65E-02  1.43E-02  9.94E-02  1.13E-01  3.17E-03
 
 OM11
+        1.04E-01 -9.16E-03 -1.15E-01 -1.88E-02  1.96E-02  1.14E-02 -2.93E-02 -1.32E-02 -4.48E-02  1.26E-03 -1.48E-01  9.70E-04
 
 OM12
+        3.75E-02  8.85E-03 -6.11E-02 -1.20E-02  3.32E-02  2.70E-02 -2.63E-02 -1.30E-02 -3.33E-02  2.55E-02 -2.14E-01  3.76E-01
          8.17E-04
 
 OM13
+       -9.29E-03  4.99E-02 -8.31E-03 -5.69E-02  2.67E-02  3.35E-03 -2.94E-02 -4.27E-03  5.91E-02  5.23E-02 -1.73E-01  3.24E-01
          1.86E-01  1.16E-03
 
 OM14
+        3.68E-02 -2.24E-02 -5.21E-02  3.65E-02  1.13E-01 -1.06E-02 -1.03E-01  9.93E-03  6.27E-02  7.42E-02 -1.20E-01  2.98E-01
          1.96E-01  6.52E-01  9.95E-04
 
 OM22
+        6.10E-02  2.40E-03 -6.05E-02 -8.25E-04  8.93E-02 -1.16E-01 -6.17E-02  1.06E-01 -4.79E-02 -6.23E-02 -2.32E-01  2.05E-02
          2.92E-01 -2.32E-02  3.36E-02  1.43E-03
 
 OM23
+       -1.76E-02  1.39E-02  9.36E-03 -2.46E-02 -3.32E-02  3.86E-02  2.92E-02 -2.70E-02 -3.58E-02 -2.57E-02 -2.03E-01  1.58E-01
          2.34E-01  3.86E-01  2.37E-01  1.24E-01  1.26E-03
 
 OM24
+        5.19E-02 -1.80E-02 -4.37E-02  1.14E-02  8.56E-02 -4.18E-02 -8.41E-02  5.78E-02  1.41E-02  9.71E-02 -1.24E-01  1.10E-01
          2.49E-01  2.49E-01  3.46E-01  1.95E-01  5.60E-01  9.93E-04
 
 OM33
+        2.41E-02  3.50E-03 -2.22E-02 -2.92E-03  8.78E-02 -4.08E-02 -9.51E-02  2.04E-02 -7.22E-02 -8.20E-02 -4.82E-01  1.22E-01
          1.51E-01  2.89E-01  1.27E-01  3.71E-02  3.30E-01  1.53E-01  2.44E-03
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 OM34
+        2.51E-02  3.88E-02 -2.60E-02 -3.85E-02  8.59E-02  3.73E-02 -1.00E-01 -4.42E-02 -5.16E-02 -2.83E-02 -5.05E-01  1.68E-01
          1.76E-01  2.77E-01  2.81E-01  1.86E-02  3.81E-01  2.48E-01  7.81E-01  1.66E-03
 
 OM44
+       -1.42E-02  6.01E-02  1.07E-02 -4.67E-02  8.68E-02  7.92E-02 -1.00E-01 -5.98E-02  2.78E-02  1.56E-01 -3.89E-01  1.69E-01
          1.71E-01  2.24E-01  3.40E-01 -2.75E-02  2.32E-01  3.34E-01  3.75E-01  7.15E-01  1.71E-03
 
 SG11
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************           STOCHASTIC APPROXIMATION EXPECTATION-MAXIMIZATION (NO PRIOR)         ********************
 ********************                    INVERSE COVARIANCE MATRIX OF ESTIMATE (S)                   ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 TH 1
+        1.72E+04
 
 TH 2
+        0.00E+00  1.72E+04
 
 TH 3
+        5.69E+04  0.00E+00  1.97E+05
 
 TH 4
+        0.00E+00  5.57E+04  0.00E+00  1.95E+05
 
 TH 5
+       -1.73E+03  0.00E+00 -4.37E+03  0.00E+00  1.20E+04
 
 TH 6
+        0.00E+00 -3.75E+03  0.00E+00 -1.20E+04  0.00E+00  1.11E+04
 
 TH 7
+       -4.37E+03  0.00E+00 -1.20E+04  0.00E+00  4.18E+04  0.00E+00  1.53E+05
 
 TH 8
+        0.00E+00 -1.20E+04  0.00E+00 -4.01E+04  0.00E+00  3.68E+04  0.00E+00  1.29E+05
 
 TH 9
+       -1.44E+03 -3.47E+03 -5.01E+03 -9.72E+03  7.01E+02  4.50E+02  3.42E+03  1.79E+03  2.02E+04
 
 TH10
+       -8.07E+02 -1.78E+02 -3.11E+03 -9.08E+02 -1.48E+01 -3.43E+03  1.85E+03 -1.12E+04 -1.39E+04  2.85E+04
 
 TH11
+       -1.02E+03 -9.21E+02 -5.26E+03 -4.11E+03 -1.66E+03 -1.02E+03 -6.51E+03 -4.33E+03  1.68E+02 -6.10E+03  1.57E+05
 
 OM11
+       -7.76E+03  1.40E+04 -1.30E+04  4.71E+04  9.93E+03 -3.38E+02  3.34E+04 -1.36E+03  6.80E+03 -2.99E+03  1.53E+04  1.42E+06
 
 OM12
+        2.10E+04 -6.33E+02  7.02E+04 -2.70E+03 -5.28E+03 -1.03E+04 -1.98E+04 -2.91E+04  5.07E+03 -9.96E+03  3.26E+04 -5.72E+05
          2.07E+06
 
 OM13
+        5.91E+03  9.67E+03  1.70E+04  4.85E+04  4.96E+03 -1.42E+03  1.32E+04 -6.50E+03 -8.13E+03 -6.30E+03  2.13E+04 -2.15E+05
          1.90E+04  1.70E+06
 
 OM14
+        1.20E+04 -1.77E+04  4.01E+04 -7.86E+04 -1.80E+04  9.97E+03 -5.34E+04  3.25E+04 -6.67E+03  9.35E+03 -2.89E+04 -1.84E+05
         -3.88E+04 -1.23E+06  2.29E+06
 
 OM22
+       -1.30E+03 -5.16E+03 -4.73E+03 -1.39E+04 -1.10E+04  5.00E+03 -3.50E+04  9.64E+03  5.73E+01  3.13E+03  7.70E+04  6.42E+04
         -2.92E+05  8.35E+04 -2.83E+04  6.22E+05
 
 OM23
+        6.38E+03  6.50E+02  1.80E+04  3.94E+02  2.09E+02 -8.77E+03 -6.37E+03 -2.28E+04 -1.12E+03  7.24E+03 -1.67E+02 -2.42E+04
         -7.58E+04 -4.28E+05  2.95E+05 -2.56E+04  1.22E+06
 
 OM24
+       -1.74E+04  1.14E+04 -5.20E+04  3.70E+04  4.37E+03  1.35E+02  1.99E+04 -8.90E+03  6.48E+03 -1.42E+04 -3.40E+04  1.21E+05
         -1.83E+05  2.23E+05 -5.05E+05 -1.47E+05 -8.10E+05  1.89E+06
 
 OM33
+       -7.70E+02 -7.64E+03 -5.69E+03 -3.01E+04 -4.70E+03  6.26E+03 -1.21E+04  1.59E+04  3.74E+03 -4.38E+03  5.74E+04  1.61E+04
         -4.66E+04 -2.61E+05  2.80E+05  3.38E+04  4.40E+04 -7.41E+04  6.02E+05
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 OM34
+       -8.15E+03  7.72E+03 -2.05E+04  3.25E+04  1.03E+04 -1.42E+03  3.69E+04  1.24E+03 -7.09E+03  2.65E+04  2.88E+04  3.05E+02
          3.99E+04  3.28E+05 -4.28E+05 -8.79E+03 -4.09E+05  3.32E+05 -8.63E+05  2.24E+06
 
 OM44
+        6.46E+03 -5.30E+03  1.40E+04 -1.59E+04 -1.65E+03 -4.83E+03 -5.59E+03 -1.53E+04  1.29E+04 -3.47E+04  7.23E+04 -3.76E+04
         -3.28E+04 -2.75E+04 -8.05E+04  9.43E+04  2.19E+05 -3.70E+05  3.10E+05 -1.04E+06  1.04E+06
 
 SG11
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
1
 
 
 #TBLN:      4
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
 NO. OF FUNCT. EVALS. ALLOWED:            2208
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
 RAW OUTPUT FILE (FILE): example2.ext
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
 GRADIENT/GIBBS PATTERN (GRD):              DDDDDDDDDDS
 AUTOMATIC SETTING FEATURE (AUTO):          0
 CONVERGENCE TYPE (CTYPE):                  3
 CONVERGENCE INTERVAL (CINTERVAL):          1
 CONVERGENCE ITERATIONS (CITER):            10
 CONVERGENCE ALPHA ERROR (CALPHA):          5.000000000000000E-02
 ITERATIONS (NITER):                        5
 ANNEAL SETTING (CONSTRAIN):                 1
 STARTING SEED FOR MC METHODS (SEED):       123334
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
   1   2   3   4   5   6   7   8   9  10
 THETAS THAT ARE SIGMA-LIKE:
  11
 
 MONITORING OF SEARCH:

 iteration            0  OBJ=  -10778.3196047997 eff.=    2986. Smpl.=    3000. Fit.= 0.98053
 iteration            1  OBJ=  -10778.4459501732 eff.=    1202. Smpl.=    3000. Fit.= 0.90060
 iteration            2  OBJ=  -10778.9072981599 eff.=    1197. Smpl.=    3000. Fit.= 0.90009
 iteration            3  OBJ=  -10778.0171292960 eff.=    1201. Smpl.=    3000. Fit.= 0.90053
 iteration            4  OBJ=  -10777.8311275272 eff.=    1198. Smpl.=    3000. Fit.= 0.90055
 iteration            5  OBJ=  -10779.0711658918 eff.=    1201. Smpl.=    3000. Fit.= 0.90043
 
 #TERM:
 EXPECTATION ONLY PROCESS WAS NOT COMPLETED


 ETABAR IS THE ARITHMETIC MEAN OF THE ETA-ESTIMATES,
 AND THE P-VALUE IS GIVEN FOR THE NULL HYPOTHESIS THAT THE TRUE MEAN IS 0.
 
 ETABAR:        -4.8174E-05 -9.0748E-05 -2.0569E-05 -1.3944E-04
 SE:             4.6673E-03  2.9105E-03  2.3759E-03  3.3903E-03
 N:                     400         400         400         400
 
 P VAL.:         9.9176E-01  9.7513E-01  9.9309E-01  9.6719E-01
 
 ETASHRINKSD(%)  5.8847E+00  3.2782E+01  3.1290E+01  1.6585E+01
 ETASHRINKVR(%)  1.1423E+01  5.4818E+01  5.2789E+01  3.0420E+01
 EBVSHRINKSD(%)  5.9340E+00  3.2723E+01  3.1242E+01  1.6740E+01
 EBVSHRINKVR(%)  1.1516E+01  5.4738E+01  5.2723E+01  3.0677E+01
 RELATIVEINF(%)  5.5677E+01  2.9626E+01  8.1456E+00  1.4411E+01
 EPSSHRINKSD(%)  2.4301E+01
 EPSSHRINKVR(%)  4.2697E+01
 
  
 TOTAL DATA POINTS NORMALLY DISTRIBUTED (N):         2000
 N*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    3675.75413281869     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -10779.0711658918     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -7103.31703307310     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 TOTAL EFFECTIVE ETAS (NIND*NETA):                          1600
  
 #TERE:
 Elapsed estimation  time in seconds:    83.11
 Elapsed covariance  time in seconds:   160.93
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************         OBJECTIVE FUNCTION EVALUATION BY IMPORTANCE SAMPLING (NO PRIOR)        ********************
 #OBJT:**************                        FINAL VALUE OF OBJECTIVE FUNCTION                       ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 





 #OBJV:********************************************        -10779.071       *********************************************
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************         OBJECTIVE FUNCTION EVALUATION BY IMPORTANCE SAMPLING (NO PRIOR)        ********************
 ********************                             FINAL PARAMETER ESTIMATE                           ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11     
 
         3.30E+00  3.25E+00 -6.12E-01 -2.08E-01  7.33E-01  1.13E+00  3.35E-01  1.93E-01  6.90E-01  2.30E+00  1.03E-01
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        9.84E-03
 
 ETA2
+       -2.12E-04  7.50E-03
 
 ETA3
+       -2.17E-04 -1.88E-03  4.78E-03
 
 ETA4
+       -1.72E-03 -5.53E-04 -2.10E-03  6.61E-03
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        1.00E+00
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        9.92E-02
 
 ETA2
+       -2.47E-02  8.66E-02
 
 ETA3
+       -3.16E-02 -3.14E-01  6.92E-02
 
 ETA4
+       -2.13E-01 -7.85E-02 -3.74E-01  8.13E-02
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        1.00E+00
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************         OBJECTIVE FUNCTION EVALUATION BY IMPORTANCE SAMPLING (NO PRIOR)        ********************
 ********************                          STANDARD ERROR OF ESTIMATE (R)                        ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11     
 
         3.26E-02  2.87E-02  9.51E-03  8.32E-03  3.91E-02  3.60E-02  1.13E-02  1.04E-02  1.04E-02  8.48E-03  3.02E-03
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        9.25E-04
 
 ETA2
+        8.03E-04  1.32E-03
 
 ETA3
+        1.17E-03  1.41E-03  2.91E-03
 
 ETA4
+        9.03E-04  1.01E-03  2.03E-03  1.72E-03
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        0.00E+00
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        4.66E-03
 
 ETA2
+        9.45E-02  7.64E-03
 
 ETA3
+        1.76E-01  2.95E-01  2.10E-02
 
 ETA4
+        1.28E-01  1.49E-01  5.01E-01  1.06E-02
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+       .........
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************         OBJECTIVE FUNCTION EVALUATION BY IMPORTANCE SAMPLING (NO PRIOR)        ********************
 ********************                        COVARIANCE MATRIX OF ESTIMATE (R)                       ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 TH 1
+        1.06E-03
 
 TH 2
+        1.57E-05  8.25E-04
 
 TH 3
+       -3.00E-04 -2.93E-06  9.04E-05
 
 TH 4
+       -1.35E-06 -2.29E-04  1.76E-07  6.93E-05
 
 TH 5
+        2.87E-04  2.73E-05 -7.71E-05 -3.22E-06  1.53E-03
 
 TH 6
+        2.92E-05  3.03E-04 -5.78E-06 -7.88E-05  4.48E-05  1.30E-03
 
 TH 7
+       -7.96E-05 -8.87E-06  2.22E-05  1.08E-06 -4.29E-04 -1.45E-05  1.28E-04
 
 TH 8
+       -5.21E-06 -8.10E-05  1.06E-06  2.31E-05 -6.92E-06 -3.59E-04  2.37E-06  1.08E-04
 
 TH 9
+        4.09E-05  4.80E-05 -8.75E-06 -5.70E-06  5.69E-05  7.69E-05 -1.86E-05 -1.41E-05  1.07E-04
 
 TH10
+        2.55E-05  3.35E-05 -4.90E-06 -4.28E-06  5.26E-05  6.41E-05 -1.60E-05 -1.07E-05  6.07E-05  7.19E-05
 
 TH11
+        4.42E-07 -2.75E-06 -3.17E-07  4.40E-07  2.26E-06 -6.60E-06 -7.03E-07  1.44E-06 -2.97E-06 -2.20E-06  9.13E-06
 
 OM11
+        2.99E-07  1.55E-07 -2.00E-08  2.17E-08  2.45E-07  7.18E-07  5.76E-09 -9.54E-08  6.48E-07  5.04E-07 -4.37E-07  8.56E-07
 
 OM12
+        2.15E-07  9.94E-08  1.21E-08  7.02E-08  9.55E-08  3.82E-07  1.10E-07  4.31E-08  6.85E-07  5.33E-07 -4.98E-07  2.30E-07
          6.44E-07
 
 OM13
+        7.85E-07  1.17E-06  2.00E-09 -1.48E-07  1.31E-06  2.46E-06 -2.58E-07 -3.59E-07  2.21E-06  1.78E-06 -9.91E-07  3.92E-07
          1.46E-07  1.37E-06
 
 OM14
+        1.12E-06 -6.43E-07 -2.16E-07  3.77E-07  6.81E-07  1.66E-06 -1.34E-07 -2.50E-07  1.62E-06  1.14E-06 -7.52E-07  2.44E-07
          1.52E-07  6.86E-07  8.16E-07
 
 OM22
+       -3.50E-07  1.93E-07  1.64E-07  1.17E-08 -1.44E-06 -3.83E-07  6.33E-07  2.60E-07  7.71E-08  4.71E-08 -1.23E-06  7.22E-08
          3.02E-07  6.27E-08  6.29E-08  1.75E-06
 
 OM23
+        1.23E-06  1.82E-06 -1.42E-07 -2.06E-07  3.32E-06  2.57E-06 -6.36E-07 -2.63E-07  3.58E-06  2.58E-06 -1.13E-06  1.69E-07
          2.92E-07  7.24E-07  3.71E-07  1.89E-07  1.98E-06
 
 OM24
+        5.75E-07  6.74E-07 -4.98E-08 -2.07E-08  2.20E-06 -6.22E-07 -5.07E-07  4.79E-07  1.92E-06  1.43E-06 -6.31E-07  8.34E-08
          1.67E-07  2.93E-07  2.87E-07  2.60E-07  8.38E-07  1.02E-06
 
 OM33
+        1.64E-06  4.78E-06 -4.42E-09 -5.69E-07  3.20E-06  1.00E-05 -7.94E-07 -1.70E-06  6.90E-06  5.46E-06 -4.65E-06  3.52E-07
          3.39E-07  1.69E-06  9.03E-07  3.37E-07  1.84E-06  7.23E-07  8.47E-06
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 OM34
+        1.15E-06  4.35E-06  4.64E-09 -6.42E-07  1.86E-06  8.96E-06 -4.78E-07 -1.73E-06  5.60E-06  4.16E-06 -3.30E-06  2.28E-07
          2.40E-07  1.06E-06  7.29E-07  1.81E-07  1.35E-06  5.90E-07  5.13E-06  4.11E-06
 
 OM44
+        5.76E-07  3.49E-06  6.05E-08 -5.82E-07  6.52E-07  7.58E-06 -1.80E-07 -1.60E-06  3.92E-06  3.04E-06 -2.39E-06  1.41E-07
          1.58E-07  5.92E-07  5.32E-07  1.06E-07  7.72E-07  5.32E-07  2.89E-06  2.75E-06  2.95E-06
 
 SG11
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************         OBJECTIVE FUNCTION EVALUATION BY IMPORTANCE SAMPLING (NO PRIOR)        ********************
 ********************                        CORRELATION MATRIX OF ESTIMATE (R)                      ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 TH 1
+        3.26E-02
 
 TH 2
+        1.68E-02  2.87E-02
 
 TH 3
+       -9.68E-01 -1.07E-02  9.51E-03
 
 TH 4
+       -4.97E-03 -9.59E-01  2.22E-03  8.32E-03
 
 TH 5
+        2.25E-01  2.42E-02 -2.07E-01 -9.90E-03  3.91E-02
 
 TH 6
+        2.49E-02  2.93E-01 -1.69E-02 -2.63E-01  3.18E-02  3.60E-02
 
 TH 7
+       -2.16E-01 -2.73E-02  2.06E-01  1.15E-02 -9.70E-01 -3.56E-02  1.13E-02
 
 TH 8
+       -1.53E-02 -2.71E-01  1.08E-02  2.66E-01 -1.70E-02 -9.59E-01  2.01E-02  1.04E-02
 
 TH 9
+        1.21E-01  1.62E-01 -8.89E-02 -6.61E-02  1.40E-01  2.06E-01 -1.59E-01 -1.31E-01  1.04E-02
 
 TH10
+        9.23E-02  1.38E-01 -6.08E-02 -6.06E-02  1.59E-01  2.10E-01 -1.67E-01 -1.21E-01  6.91E-01  8.48E-03
 
 TH11
+        4.48E-03 -3.17E-02 -1.10E-02  1.75E-02  1.91E-02 -6.07E-02 -2.06E-02  4.59E-02 -9.51E-02 -8.59E-02  3.02E-03
 
 OM11
+        9.92E-03  5.84E-03 -2.28E-03  2.81E-03  6.77E-03  2.16E-02  5.51E-04 -9.91E-03  6.77E-02  6.43E-02 -1.56E-01  9.25E-04
 
 OM12
+        8.21E-03  4.31E-03  1.59E-03  1.05E-02  3.04E-03  1.32E-02  1.22E-02  5.16E-03  8.24E-02  7.83E-02 -2.06E-01  3.10E-01
          8.03E-04
 
 OM13
+        2.06E-02  3.48E-02  1.79E-04 -1.52E-02  2.86E-02  5.84E-02 -1.95E-02 -2.95E-02  1.82E-01  1.80E-01 -2.80E-01  3.62E-01
          1.56E-01  1.17E-03
 
 OM14
+        3.79E-02 -2.48E-02 -2.51E-02  5.01E-02  1.92E-02  5.11E-02 -1.31E-02 -2.66E-02  1.73E-01  1.49E-01 -2.76E-01  2.92E-01
          2.10E-01  6.49E-01  9.03E-04
 
 OM22
+       -8.10E-03  5.07E-03  1.30E-02  1.06E-03 -2.79E-02 -8.04E-03  4.23E-02  1.88E-02  5.63E-03  4.20E-03 -3.07E-01  5.89E-02
          2.84E-01  4.05E-02  5.26E-02  1.32E-03
 
 OM23
+        2.67E-02  4.51E-02 -1.06E-02 -1.75E-02  6.02E-02  5.07E-02 -3.99E-02 -1.79E-02  2.46E-01  2.16E-01 -2.64E-01  1.30E-01
          2.58E-01  4.39E-01  2.91E-01  1.01E-01  1.41E-03
 
 OM24
+        1.74E-02  2.32E-02 -5.18E-03 -2.46E-03  5.57E-02 -1.71E-02 -4.43E-02  4.55E-02  1.83E-01  1.66E-01 -2.07E-01  8.92E-02
          2.05E-01  2.47E-01  3.14E-01  1.94E-01  5.88E-01  1.01E-03
 
 OM33
+        1.73E-02  5.72E-02 -1.60E-04 -2.35E-02  2.81E-02  9.54E-02 -2.41E-02 -5.60E-02  2.29E-01  2.21E-01 -5.29E-01  1.31E-01
          1.45E-01  4.97E-01  3.43E-01  8.74E-02  4.48E-01  2.45E-01  2.91E-03
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 OM34
+        1.74E-02  7.47E-02  2.41E-04 -3.81E-02  2.34E-02  1.23E-01 -2.09E-02 -8.18E-02  2.67E-01  2.42E-01 -5.39E-01  1.21E-01
          1.48E-01  4.46E-01  3.98E-01  6.76E-02  4.73E-01  2.88E-01  8.69E-01  2.03E-03
 
 OM44
+        1.03E-02  7.08E-02  3.71E-03 -4.07E-02  9.71E-03  1.23E-01 -9.27E-03 -8.96E-02  2.21E-01  2.09E-01 -4.60E-01  8.91E-02
          1.15E-01  2.95E-01  3.43E-01  4.66E-02  3.19E-01  3.07E-01  5.77E-01  7.92E-01  1.72E-03
 
 SG11
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************         OBJECTIVE FUNCTION EVALUATION BY IMPORTANCE SAMPLING (NO PRIOR)        ********************
 ********************                    INVERSE COVARIANCE MATRIX OF ESTIMATE (R)                   ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 TH 1
+        1.59E+04
 
 TH 2
+        3.67E-09  1.85E+04
 
 TH 3
+        5.25E+04  1.58E-07  1.85E+05
 
 TH 4
+        3.01E-07  6.07E+04  3.78E-06  2.15E+05
 
 TH 5
+       -1.97E+03 -4.24E-08 -6.10E+03  4.34E-07  1.14E+04
 
 TH 6
+       -7.61E-08 -3.54E+03 -1.45E-08 -1.14E+04 -5.11E-07  1.15E+04
 
 TH 7
+       -6.10E+03  3.07E-07 -2.08E+04  7.19E-06  3.84E+04  2.80E-08  1.37E+05
 
 TH 8
+       -1.18E-07 -1.14E+04  1.68E-06 -3.98E+04 -1.52E-06  3.76E+04  3.39E-06  1.33E+05
 
 TH 9
+       -1.22E+03 -3.79E+03 -3.18E+03 -1.21E+04  1.30E+03 -3.20E+02  4.95E+03 -3.08E+02  1.97E+04
 
 TH10
+       -8.01E+02 -2.61E+02 -2.91E+03 -7.69E+02 -3.65E+02 -3.20E+03 -1.65E-01 -9.81E+03 -1.42E+04  2.83E+04
 
 TH11
+       -6.30E+02 -4.69E+02 -2.00E+03 -1.85E+03 -1.28E+03 -1.16E+03 -3.84E+03 -4.53E+03 -1.58E+03 -1.61E+03  1.83E+05
 
 OM11
+        5.26E+02 -2.42E+02  1.98E+03 -1.02E+03 -1.45E+03 -4.04E+02 -5.00E+03 -9.99E+02 -1.54E+03  4.88E+01  3.72E+04  1.49E+06
 
 OM12
+       -2.38E+03 -1.44E+03 -8.67E+03 -5.15E+03 -3.73E+03 -2.42E+03 -1.42E+04 -9.36E+03 -6.46E+01 -2.96E+03  2.45E+04 -4.74E+05
          2.00E+06
 
 OM13
+       -6.71E+03  4.65E+03 -2.59E+04  2.46E+04  3.96E+02 -1.65E+03  1.25E+03 -7.80E+03  2.53E+03 -6.93E+03 -1.94E+04 -4.18E+05
          2.16E+05  1.77E+06
 
 OM14
+        2.53E+03 -3.80E+03  1.44E+04 -3.14E+04 -1.44E+03  4.63E+02 -6.49E+03  6.06E+03 -7.68E+03  4.57E+03  4.19E+04 -6.92E+04
         -2.67E+05 -1.19E+06  2.48E+06
 
 OM22
+       -8.81E+02 -1.27E+03 -3.06E+03 -4.04E+03 -3.76E+03 -2.28E+03 -1.54E+04 -8.02E+03  4.39E+02  1.46E+03  1.06E+05  4.56E+04
         -2.78E+05 -1.10E+04  6.70E+04  7.05E+05
 
 OM23
+        1.90E+03 -3.53E+01  7.09E+03 -2.21E+03 -8.33E+03  3.71E+02 -2.84E+04  3.58E+03 -9.27E+03 -6.93E+02 -1.17E+03  7.59E+04
         -2.53E+05 -3.99E+05  2.93E+05  5.69E+04  1.08E+06
 
 OM24
+       -1.04E+03  1.34E+03 -5.62E+03  8.82E+03  9.46E+02 -6.23E+02  6.99E+03 -1.38E+04 -3.23E+03 -4.64E+03 -7.62E+03  7.26E+03
          1.88E+04  2.30E+05 -4.62E+05 -1.79E+05 -7.38E+05  1.73E+06
 
 OM33
+       -1.60E+02 -1.81E+03 -8.35E+01 -8.58E+03 -5.19E+02 -1.22E+03 -1.15E+03 -4.76E+03  3.06E+03 -4.63E+03  5.98E+04  3.88E+04
         -2.31E+04 -2.83E+05  2.07E+05  5.17E+03  3.75E+04 -4.44E+04  6.27E+05
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 OM34
+       -9.10E+02  1.65E+02 -4.38E+03  4.80E+03  1.61E+03 -1.79E+03  4.97E+03 -5.42E+03 -1.15E+04  5.13E+03  2.85E+04  1.70E+04
          5.52E+04  1.98E+05 -3.52E+05  2.29E+04 -3.53E+05  2.73E+05 -9.04E+05  2.23E+06
 
 OM44
+       -4.60E+02 -8.09E+02 -1.94E+03 -2.35E+03 -4.66E+02 -1.14E+03 -3.06E+03  4.96E+02  1.93E+03 -6.73E+03  5.85E+04  6.40E+03
         -1.02E+04  1.18E+04 -2.17E+04  5.42E+04  1.89E+05 -2.90E+05  2.97E+05 -1.10E+06  1.13E+06
 
 SG11
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
1
 
 
 #TBLN:      5
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
 NO. OF FUNCT. EVALS. ALLOWED:            2208
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
 RAW OUTPUT FILE (FILE): example2.TXT
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
 GRADIENT/GIBBS PATTERN (GRD):              DDDDDDDDDDS
 AUTOMATIC SETTING FEATURE (AUTO):          0
 CONVERGENCE TYPE (CTYPE):                  3
 KEEP ITERATIONS (THIN):            1
 CONVERGENCE INTERVAL (CINTERVAL):          100
 CONVERGENCE ITERATIONS (CITER):            10
 CONVERGENCE ALPHA ERROR (CALPHA):          5.000000000000000E-02
 BURN-IN ITERATIONS (NBURN):                10000
 FIRST ITERATION FOR MAP (MAPITERS):          NO
 ITERATIONS (NITER):                        3000
 ANNEAL SETTING (CONSTRAIN):                 1
 STARTING SEED FOR MC METHODS (SEED):       123334
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
 SAMPLES FOR LOCAL SEARCH KERNEL (PSAMPLE_M2):           1
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
   1   2   3   4   5   6   7   8   9  10
 THETAS THAT ARE GIBBS SAMPLED:
   1   2   3   4   5   6   7   8   9  10
 THETAS THAT ARE METROPOLIS-HASTINGS SAMPLED:
  11
 SIGMAS THAT ARE GIBBS SAMPLED:
 
 SIGMAS THAT ARE METROPOLIS-HASTINGS SAMPLED:
 
 OMEGAS ARE GIBBS SAMPLED
 
 MONITORING OF SEARCH:

 Burn-in Mode
 iteration       -10000 MCMCOBJ=   -19684.3042545325     
 iteration        -9900 MCMCOBJ=   -19401.0760534980     
 iteration        -9800 MCMCOBJ=   -19215.1439065075     
 iteration        -9700 MCMCOBJ=   -19466.1667133845     
 iteration        -9600 MCMCOBJ=   -19254.2390239666     
 iteration        -9500 MCMCOBJ=   -19306.0525783985     
 iteration        -9400 MCMCOBJ=   -19428.3615292182     
 iteration        -9300 MCMCOBJ=   -19296.1121033677     
 iteration        -9200 MCMCOBJ=   -19112.4384285972     
 iteration        -9100 MCMCOBJ=   -19175.8054464393     
 iteration        -9000 MCMCOBJ=   -19301.5861116927     
 Convergence achieved
 Elapsed burn-in time in seconds:   144.31
 Sampling Mode
 iteration            0 MCMCOBJ=   -19407.2048935221     
 iteration          100 MCMCOBJ=   -19469.9718102145     
 iteration          200 MCMCOBJ=   -19125.4762665275     
 iteration          300 MCMCOBJ=   -19387.2014699005     
 iteration          400 MCMCOBJ=   -19305.7577266543     
 iteration          500 MCMCOBJ=   -19112.3728190769     
 iteration          600 MCMCOBJ=   -19453.2432000226     
 iteration          700 MCMCOBJ=   -19380.5132919434     
 iteration          800 MCMCOBJ=   -19320.3734670029     
 iteration          900 MCMCOBJ=   -19378.6064273186     
 iteration         1000 MCMCOBJ=   -19336.2569929637     
 iteration         1100 MCMCOBJ=   -19306.6299930068     
 iteration         1200 MCMCOBJ=   -19207.6577978830     
 iteration         1300 MCMCOBJ=   -19360.4086873439     
 iteration         1400 MCMCOBJ=   -19399.5080103235     
 iteration         1500 MCMCOBJ=   -19256.4342548698     
 iteration         1600 MCMCOBJ=   -19182.2271781235     
 iteration         1700 MCMCOBJ=   -19230.7754497022     
 iteration         1800 MCMCOBJ=   -19136.5901107974     
 iteration         1900 MCMCOBJ=   -19358.1082226423     
 iteration         2000 MCMCOBJ=   -19124.0598595821     
 iteration         2100 MCMCOBJ=   -19216.9611044086     
 iteration         2200 MCMCOBJ=   -19322.7689638195     
 iteration         2300 MCMCOBJ=   -19405.6756442364     
 iteration         2400 MCMCOBJ=   -19152.9704768868     
 iteration         2500 MCMCOBJ=   -19340.1179781220     
 iteration         2600 MCMCOBJ=   -19100.6862572189     
 iteration         2700 MCMCOBJ=   -19077.5832013399     
 iteration         2800 MCMCOBJ=   -19118.5648037522     
 iteration         2900 MCMCOBJ=   -19183.0587180206     
 iteration         3000 MCMCOBJ=   -19202.4282433564     
 
 #TERM:
 BURN-IN WAS COMPLETED
 STATISTICAL PORTION WAS COMPLETED

 ETABAR IS THE ARITHMETIC MEAN OF THE ETA-ESTIMATES,
 AND THE P-VALUE IS GIVEN FOR THE NULL HYPOTHESIS THAT THE TRUE MEAN IS 0.
 
 ETABAR:        -1.7155E-04 -1.0375E-04 -3.3309E-05  1.2790E-04
 SE:             4.6716E-03  2.8571E-03  2.5378E-03  3.4906E-03
 N:                     400         400         400         400
 
 P VAL.:         9.7071E-01  9.7103E-01  9.8953E-01  9.7077E-01
 
 ETASHRINKSD(%)  7.3048E+00  3.4646E+01  4.1584E+01  2.3660E+01
 ETASHRINKVR(%)  1.4076E+01  5.7288E+01  6.5876E+01  4.1722E+01
 EBVSHRINKSD(%)  6.7569E+00  3.4657E+01  4.1421E+01  2.3212E+01
 EBVSHRINKVR(%)  1.3057E+01  5.7302E+01  6.5685E+01  4.1036E+01
 RELATIVEINF(%)  7.0422E+01  3.7128E+01  1.7707E+01  2.8690E+01
 EPSSHRINKSD(%)  2.5554E+01
 EPSSHRINKVR(%)  4.4578E+01
 
  
 TOTAL DATA POINTS NORMALLY DISTRIBUTED (N):         2000
 N*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    3675.75413281869     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -19293.0280437230     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -15617.2739109044     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 TOTAL EFFECTIVE ETAS (NIND*NETA):                          1600
 NIND*NETA*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    2940.60330625495     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -19293.0280437230     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -16352.4247374681     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 PRIOR CONSTANT TO OBJECTIVE FUNCTION:    70.3639128125257     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -19293.0280437230     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -19222.6641309105     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 #TERE:
 Elapsed estimation  time in seconds:   572.17
 Elapsed covariance  time in seconds:     0.00
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 #OBJT:**************                       AVERAGE VALUE OF LIKELIHOOD FUNCTION                     ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 





 #OBJV:********************************************        -19293.028       *********************************************
 #OBJS:********************************************           137.498 (STD) *********************************************
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 ********************                             FINAL PARAMETER ESTIMATE                           ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11     
 
         3.30E+00  3.25E+00 -6.12E-01 -2.08E-01  7.35E-01  1.14E+00  3.35E-01  1.92E-01  6.91E-01  2.30E+00  1.02E-01
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        1.02E-02
 
 ETA2
+       -3.02E-05  7.64E-03
 
 ETA3
+        5.79E-04 -6.86E-04  7.55E-03
 
 ETA4
+       -1.16E-03  1.84E-04  4.56E-05  8.36E-03
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        1.00E+00
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        1.01E-01
 
 ETA2
+       -7.09E-03  8.71E-02
 
 ETA3
+        6.02E-02 -9.88E-02  8.62E-02
 
 ETA4
+       -1.30E-01  2.02E-02 -1.68E-02  9.11E-02
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        1.00E+00
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 ********************                STANDARD ERROR OF ESTIMATE (From Sample Variance)               ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11     
 
         3.31E-02  2.91E-02  9.65E-03  8.41E-03  3.78E-02  3.55E-02  1.11E-02  1.03E-02  1.07E-02  9.00E-03  2.66E-03
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        9.74E-04
 
 ETA2
+        8.03E-04  1.26E-03
 
 ETA3
+        1.07E-03  1.19E-03  1.96E-03
 
 ETA4
+        8.73E-04  9.50E-04  1.29E-03  1.42E-03
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        0.00E+00
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        4.82E-03
 
 ETA2
+        9.20E-02  7.24E-03
 
 ETA3
+        1.20E-01  1.59E-01  1.12E-02
 
 ETA4
+        9.85E-02  1.17E-01  1.61E-01  7.70E-03
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        0.00E+00
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 ********************               COVARIANCE MATRIX OF ESTIMATE (From Sample Variance)             ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 TH 1
+        1.09E-03
 
 TH 2
+        1.98E-05  8.47E-04
 
 TH 3
+       -3.09E-04 -5.49E-06  9.31E-05
 
 TH 4
+       -2.10E-06 -2.35E-04  7.16E-07  7.08E-05
 
 TH 5
+        3.12E-04  2.69E-05 -8.52E-05 -5.57E-06  1.43E-03
 
 TH 6
+        5.66E-05  2.60E-04 -1.40E-05 -6.99E-05  6.76E-05  1.26E-03
 
 TH 7
+       -8.94E-05 -9.23E-06  2.54E-05  1.52E-06 -4.06E-04 -2.09E-05  1.23E-04
 
 TH 8
+       -1.25E-05 -7.03E-05  3.35E-06  2.11E-05 -9.76E-06 -3.51E-04  3.09E-06  1.07E-04
 
 TH 9
+        5.55E-05  4.98E-05 -1.31E-05 -5.29E-06  6.42E-05  7.14E-05 -2.13E-05 -1.19E-05  1.14E-04
 
 TH10
+        3.10E-05  3.39E-05 -5.96E-06 -4.15E-06  5.72E-05  6.29E-05 -1.70E-05 -1.00E-05  6.84E-05  8.10E-05
 
 TH11
+        2.54E-06  7.36E-08 -8.14E-07  6.46E-08  2.91E-06 -1.40E-06 -1.04E-06  4.25E-07  1.21E-06  4.87E-07  7.07E-06
 
 OM11
+        1.08E-07 -1.51E-06 -2.68E-08  4.78E-07  1.64E-06  1.12E-07 -3.79E-07  9.80E-08  2.59E-07  1.81E-07 -3.24E-07  9.48E-07
 
 OM12
+       -4.79E-08 -1.71E-07  5.93E-08  1.02E-07  5.50E-07  7.78E-07 -1.33E-07 -5.98E-08  5.88E-07  3.61E-07 -3.57E-07  2.61E-07
          6.45E-07
 
 OM13
+        1.21E-06 -1.05E-06 -3.15E-07  4.71E-07  9.56E-07  1.42E-07 -2.08E-07  3.46E-07  1.20E-06  8.64E-07 -2.04E-07  4.39E-07
          1.73E-07  1.14E-06
 
 OM14
+        3.71E-07 -1.62E-06 -7.98E-08  5.66E-07  7.89E-08 -9.65E-07  8.63E-08  4.32E-07  2.84E-07  1.42E-07 -3.00E-07  3.00E-07
          1.86E-07  5.71E-07  7.63E-07
 
 OM22
+        5.85E-07  1.09E-06 -1.22E-07 -3.07E-07 -3.65E-08  2.14E-06  1.87E-07 -6.22E-07 -1.72E-07 -2.92E-07 -8.74E-07  6.59E-08
          2.74E-07  5.14E-09  9.27E-09  1.59E-06
 
 OM23
+       -3.47E-07  1.93E-06  1.50E-07 -3.60E-07  1.97E-06  1.30E-06 -4.47E-07  4.89E-08  1.55E-06  1.08E-06  1.40E-08  8.36E-08
          2.25E-07  2.98E-07  1.15E-07  1.89E-07  1.42E-06
 
 OM24
+       -7.35E-07 -7.94E-07  2.36E-07  2.14E-07  2.78E-07 -9.89E-07 -4.16E-08  2.54E-07 -1.59E-07 -3.70E-07 -5.22E-08  2.51E-08
          1.23E-07  5.95E-08  1.41E-07  2.82E-07  5.89E-07  9.02E-07
 
 OM33
+        1.20E-06  4.20E-07 -7.04E-08  3.72E-07  1.58E-06  2.49E-06 -4.03E-07  1.98E-08  3.54E-06  2.73E-06 -1.14E-06  1.10E-07
          1.80E-07  7.05E-07  2.57E-07  7.61E-08  5.31E-07  5.83E-08  3.86E-06
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 OM34
+       -6.41E-07  1.62E-06  2.69E-07 -1.59E-07  1.30E-07  3.57E-06 -1.25E-07 -5.16E-07  2.30E-06  1.59E-06 -8.16E-07  6.28E-08
          8.84E-08  3.25E-07  2.75E-07 -4.83E-08  2.62E-07  3.59E-08  1.69E-06  1.67E-06
 
 OM44
+       -2.30E-06  1.84E-06  6.68E-07 -2.14E-07 -4.32E-07  4.38E-06 -4.10E-08 -9.62E-07  1.33E-06  9.94E-07 -8.91E-07  4.80E-08
          4.04E-08  1.19E-07  2.40E-07 -4.01E-08 -2.71E-08  1.34E-07  5.32E-07  1.18E-06  2.01E-06
 
 SG11
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 ********************              CORRELATION MATRIX OF ESTIMATE (From Sample Variance)             ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 TH 1
+        3.31E-02
 
 TH 2
+        2.06E-02  2.91E-02
 
 TH 3
+       -9.69E-01 -1.95E-02  9.65E-03
 
 TH 4
+       -7.53E-03 -9.60E-01  8.83E-03  8.41E-03
 
 TH 5
+        2.50E-01  2.45E-02 -2.34E-01 -1.75E-02  3.78E-02
 
 TH 6
+        4.81E-02  2.51E-01 -4.09E-02 -2.34E-01  5.03E-02  3.55E-02
 
 TH 7
+       -2.43E-01 -2.86E-02  2.37E-01  1.63E-02 -9.67E-01 -5.29E-02  1.11E-02
 
 TH 8
+       -3.67E-02 -2.34E-01  3.36E-02  2.42E-01 -2.50E-02 -9.57E-01  2.69E-02  1.03E-02
 
 TH 9
+        1.57E-01  1.60E-01 -1.27E-01 -5.88E-02  1.59E-01  1.88E-01 -1.79E-01 -1.08E-01  1.07E-02
 
 TH10
+        1.04E-01  1.30E-01 -6.86E-02 -5.48E-02  1.68E-01  1.97E-01 -1.70E-01 -1.08E-01  7.11E-01  9.00E-03
 
 TH11
+        2.88E-02  9.50E-04 -3.17E-02  2.89E-03  2.90E-02 -1.48E-02 -3.51E-02  1.55E-02  4.24E-02  2.03E-02  2.66E-03
 
 OM11
+        3.36E-03 -5.32E-02 -2.85E-03  5.84E-02  4.45E-02  3.25E-03 -3.51E-02  9.75E-03  2.49E-02  2.06E-02 -1.25E-01  9.74E-04
 
 OM12
+       -1.80E-03 -7.33E-03  7.66E-03  1.50E-02  1.81E-02  2.72E-02 -1.49E-02 -7.21E-03  6.85E-02  4.99E-02 -1.67E-01  3.34E-01
          8.03E-04
 
 OM13
+        3.43E-02 -3.39E-02 -3.06E-02  5.25E-02  2.37E-02  3.75E-03 -1.75E-02  3.14E-02  1.06E-01  9.00E-02 -7.18E-02  4.22E-01
          2.02E-01  1.07E-03
 
 OM14
+        1.28E-02 -6.38E-02 -9.47E-03  7.71E-02  2.39E-03 -3.11E-02  8.90E-03  4.79E-02  3.04E-02  1.81E-02 -1.29E-01  3.53E-01
          2.66E-01  6.12E-01  8.73E-04
 
 OM22
+        1.40E-02  2.97E-02 -1.00E-02 -2.89E-02 -7.64E-04  4.77E-02  1.34E-02 -4.77E-02 -1.27E-02 -2.57E-02 -2.60E-01  5.36E-02
          2.70E-01  3.82E-03  8.41E-03  1.26E-03
 
 OM23
+       -8.80E-03  5.58E-02  1.31E-02 -3.60E-02  4.37E-02  3.07E-02 -3.38E-02  3.98E-03  1.22E-01  1.01E-01  4.41E-03  7.21E-02
          2.36E-01  2.34E-01  1.11E-01  1.26E-01  1.19E-03
 
 OM24
+       -2.34E-02 -2.87E-02  2.57E-02  2.67E-02  7.73E-03 -2.93E-02 -3.94E-03  2.59E-02 -1.57E-02 -4.33E-02 -2.07E-02  2.72E-02
          1.62E-01  5.87E-02  1.70E-01  2.35E-01  5.21E-01  9.50E-04
 
 OM33
+        1.85E-02  7.34E-03 -3.71E-03  2.25E-02  2.12E-02  3.57E-02 -1.85E-02  9.74E-04  1.69E-01  1.54E-01 -2.18E-01  5.77E-02
          1.14E-01  3.36E-01  1.50E-01  3.07E-02  2.27E-01  3.12E-02  1.96E-03
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 OM34
+       -1.50E-02  4.32E-02  2.16E-02 -1.46E-02  2.67E-03  7.78E-02 -8.75E-03 -3.87E-02  1.66E-01  1.37E-01 -2.38E-01  5.00E-02
          8.52E-02  2.36E-01  2.44E-01 -2.96E-02  1.71E-01  2.93E-02  6.65E-01  1.29E-03
 
 OM44
+       -4.91E-02  4.45E-02  4.88E-02 -1.80E-02 -8.05E-03  8.69E-02 -2.61E-03 -6.57E-02  8.75E-02  7.79E-02 -2.36E-01  3.47E-02
          3.55E-02  7.86E-02  1.94E-01 -2.24E-02 -1.61E-02  9.97E-02  1.91E-01  6.45E-01  1.42E-03
 
 SG11
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 ********************           INVERSE COVARIANCE MATRIX OF ESTIMATE (From Sample Variance)         ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 TH 1
+        1.61E+04
 
 TH 2
+        1.42E+02  1.85E+04
 
 TH 3
+        5.31E+04  5.46E+02  1.87E+05
 
 TH 4
+        2.63E+02  6.11E+04  1.16E+03  2.17E+05
 
 TH 5
+       -2.21E+03  7.98E+02 -7.01E+03  2.82E+03  1.15E+04
 
 TH 6
+       -1.86E+02 -3.27E+03 -6.84E+02 -1.09E+04 -1.51E+02  1.13E+04
 
 TH 7
+       -6.90E+03  2.24E+03 -2.41E+04  7.95E+03  3.78E+04 -8.75E+01  1.33E+05
 
 TH 8
+       -5.57E+02 -1.12E+04 -2.33E+03 -4.01E+04 -5.38E+02  3.69E+04 -4.13E+02  1.30E+05
 
 TH 9
+       -1.04E+03 -4.46E+03 -1.99E+03 -1.44E+04  1.39E+03 -2.66E+02  5.37E+03 -2.53E+02  1.98E+04
 
 TH10
+       -1.25E+03  3.63E+02 -4.90E+03  1.35E+03 -1.15E+03 -2.76E+03 -2.87E+03 -8.33E+03 -1.47E+04  2.63E+04
 
 TH11
+        2.14E+02 -1.28E+03  1.20E+03 -4.79E+03  1.43E+02 -6.20E+02  1.29E+03 -1.84E+03 -3.35E+03  5.64E+02  1.74E+05
 
 OM11
+        1.84E+03 -2.78E+01  4.01E+03 -5.61E+03 -4.40E+03  4.43E+01 -9.80E+03  1.85E+03  2.98E+02  9.17E+02  4.24E+04  1.45E+06
 
 OM12
+       -1.70E+03  4.60E+03 -7.70E+03  1.45E+04  4.20E+03 -4.43E+03  1.38E+04 -1.40E+04 -7.03E+03  5.40E+02  3.46E+04 -4.55E+05
          2.05E+06
 
 OM13
+        1.64E+03  5.00E+03  9.01E+03  1.63E+04  3.50E+03 -6.99E+03  1.01E+04 -2.39E+04 -4.98E+03  1.84E+02 -4.06E+04 -4.82E+05
          1.60E+05  1.85E+06
 
 OM14
+       -1.98E+03 -6.86E+03 -4.50E+03 -3.04E+04 -8.31E+03  3.40E+03 -3.00E+04  6.47E+03  4.08E+03  9.05E+02  4.70E+04 -1.23E+05
         -3.93E+05 -1.22E+06  2.50E+06
 
 OM22
+       -7.38E+02 -3.22E+03 -2.68E+02 -8.01E+03 -4.10E+03 -1.33E+03 -1.47E+04  2.92E+01 -7.84E+02  3.34E+03  9.43E+04  3.06E+04
         -2.87E+05 -2.28E+04  1.15E+05  7.74E+05
 
 OM23
+        3.67E+03 -6.02E+03  8.84E+03 -1.19E+04 -6.09E+03 -7.28E+03 -1.87E+04 -2.43E+04 -3.70E+03 -5.53E+03 -1.51E+04  5.26E+04
         -2.67E+05 -3.25E+05  2.86E+05  3.52E+04  1.21E+06
 
 OM24
+       -2.94E+03  7.11E+03 -1.09E+04  1.63E+04  3.71E+03  6.61E+03  1.35E+04  1.87E+04 -5.08E+02  1.13E+04 -3.43E+04  2.81E+04
          4.99E+04  2.91E+05 -4.70E+05 -2.47E+05 -8.20E+05  1.81E+06
 
 OM33
+       -3.58E+03 -5.42E+03 -1.23E+04 -2.23E+04 -1.90E+03  1.17E+03 -5.92E+03  2.89E+03 -3.69E+02 -5.72E+03  5.12E+04  7.06E+04
         -5.26E+04 -3.13E+05  2.37E+05 -2.93E+03  8.70E+03 -7.33E+04  6.44E+05
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 OM34
+        2.01E+03  1.07E+04  7.21E+03  4.05E+04  3.62E+03 -6.46E+03  9.90E+03 -2.08E+04 -1.12E+04  9.39E+03 -8.36E+03  2.03E+04
          6.21E+04  2.36E+05 -4.24E+05  4.06E+04 -3.15E+05  3.64E+05 -8.42E+05  2.32E+06
 
 OM44
+        1.73E+03 -1.12E+04  9.16E+02 -3.75E+04  1.16E+03 -9.00E+02  5.39E+03  2.27E+03  2.17E+03 -6.25E+03  7.06E+04  4.68E+03
         -6.42E+03 -5.09E+04  2.17E+04  4.57E+04  2.50E+05 -3.24E+05  3.46E+05 -1.14E+06  1.14E+06
 
 SG11
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
1
 
 
 #TBLN:      6
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
 SIGDIGITS FOR MAP ESTIMATION (SIGLO):      14
 GRADIENT SIGDIGITS OF
       FIXED EFFECTS PARAMETERS (SIGL):     14
 NOPRIOR SETTING (NOPRIOR):                 1
 NOCOV SETTING (NOCOV):                     OFF
 DERCONT SETTING (DERCONT):                 OFF
 FINAL ETA RE-EVALUATION (FNLETA):          1
 EXCLUDE NON-INFLUENTIAL (NON-INFL.) ETAS
       IN SHRINKAGE (ETASTYPE):             NO
 NON-INFL. ETA CORRECTION (NONINFETA):      0
 RAW OUTPUT FILE (FILE): example2.ext
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

 
0ITERATION NO.:    0    OBJECTIVE VALUE:  -10769.9630279780        NO. OF FUNC. EVALS.:  13
 CUMULATIVE NO. OF FUNC. EVALS.:       13
 NPARAMETR:  3.2996E+00  3.2531E+00 -6.1205E-01 -2.0833E-01  7.3545E-01  1.1363E+00  3.3499E-01  1.9241E-01  6.9127E-01  2.2988E+00
             1.0234E-01  1.0160E-02 -3.0167E-05  5.7891E-04 -1.1583E-03  7.6448E-03 -6.8557E-04  1.8359E-04  7.5494E-03  4.5634E-05
             8.3629E-03
 PARAMETER:  1.0000E-01  1.0000E-01 -1.0000E-01 -1.0000E-01  1.0000E-01  1.0000E-01  1.0000E-01  1.0000E-01  1.0000E-01  1.0000E-01
             1.0000E-01  1.0000E-01 -1.0000E-01  1.0000E-01 -1.0000E-01  1.0000E-01 -1.0000E-01  1.0000E-01  1.0000E-01  1.0000E-01
             1.0000E-01
 GRADIENT:  -5.4456E+03 -4.8304E+03 -3.3674E+03 -1.0306E+03 -4.9755E+02  3.4086E+02 -8.6677E+02  1.8425E+02  2.6108E+01 -5.3653E+02
             4.0072E+01  6.8412E+00 -4.5714E-02 -8.7901E-01 -3.6111E+00  1.7778E+00  1.4848E+00 -4.0331E-01  1.8392E+00 -1.1627E+00
             5.3055E+00
 
0ITERATION NO.:    5    OBJECTIVE VALUE:  -10770.9621583751        NO. OF FUNC. EVALS.:  96
 CUMULATIVE NO. OF FUNC. EVALS.:      109
 NPARAMETR:  3.3040E+00  3.2572E+00 -6.1176E-01 -2.0825E-01  7.3651E-01  1.1369E+00  3.3593E-01  1.9246E-01  6.9351E-01  2.3012E+00
             1.0234E-01  1.0160E-02 -3.0167E-05  5.7891E-04 -1.1582E-03  7.6448E-03 -6.8558E-04  1.8359E-04  7.5494E-03  4.5638E-05
             8.3628E-03
 PARAMETER:  1.0013E-01  1.0013E-01 -9.9954E-02 -9.9960E-02  1.0014E-01  1.0006E-01  1.0028E-01  1.0003E-01  1.0032E-01  1.0011E-01
             9.9955E-02  9.9991E-02 -1.0000E-01  1.0000E-01 -9.9996E-02  9.9999E-02 -1.0000E-01  1.0000E-01  9.9998E-02  1.0000E-01
             9.9994E-02
 GRADIENT:  -8.1189E+02 -4.3889E+02 -4.8141E+02 -9.9541E+01  5.4862E+01 -1.6516E+00  6.7507E+01  1.0120E+01 -1.2209E+02  6.9325E+02
             3.5434E+01  7.5730E+00 -6.6963E-02 -1.1063E+00 -3.7597E+00  4.8218E-01  1.1683E+00 -3.5680E-01  1.2864E+00 -1.1570E+00
             4.6629E+00
 
0ITERATION NO.:   10    OBJECTIVE VALUE:  -10771.0724921213        NO. OF FUNC. EVALS.:  90
 CUMULATIVE NO. OF FUNC. EVALS.:      199
 NPARAMETR:  3.3046E+00  3.2639E+00 -6.1201E-01 -2.1029E-01  7.2944E-01  1.1416E+00  3.3788E-01  1.9102E-01  6.9367E-01  2.3008E+00
             1.0125E-01  1.0112E-02 -3.0090E-05  5.7946E-04 -1.1412E-03  7.6434E-03 -6.8806E-04  1.8373E-04  7.5441E-03  4.6588E-05
             8.3358E-03
 PARAMETER:  1.0015E-01  1.0033E-01 -9.9994E-02 -1.0094E-01  9.9184E-02  1.0047E-01  1.0086E-01  9.9277E-02  1.0035E-01  1.0009E-01
             8.9300E-02  9.7655E-02 -9.9979E-02  1.0033E-01 -9.8756E-02  9.9909E-02 -1.0037E-01  1.0011E-01  9.9600E-02  1.0037E-01
             9.8543E-02
 GRADIENT:  -8.3471E+02 -4.0438E+02 -5.7434E+02 -2.0605E+02 -1.1914E-01  3.8785E+01  9.2978E+01  1.1860E+00  2.6584E+01  9.8533E+01
            -3.2192E+00  3.0301E+00 -7.2321E-02 -9.4107E-01 -2.5818E+00 -3.2573E+00  1.4674E+00 -2.0788E-01 -4.3729E-01 -1.1750E+00
             1.6965E+00
 
0ITERATION NO.:   15    OBJECTIVE VALUE:  -10771.1708575757        NO. OF FUNC. EVALS.:  81
 CUMULATIVE NO. OF FUNC. EVALS.:      280
 NPARAMETR:  3.3053E+00  3.2608E+00 -6.1222E-01 -2.0935E-01  7.3355E-01  1.1378E+00  3.3673E-01  1.9219E-01  6.9358E-01  2.3007E+00
             1.0085E-01  1.0080E-02 -2.9796E-05  6.5126E-04 -1.0622E-03  7.8998E-03 -7.8749E-04  1.9236E-04  7.6674E-03  5.7043E-05
             8.2624E-03
 PARAMETER:  1.0017E-01  1.0023E-01 -1.0003E-01 -1.0049E-01  9.9742E-02  1.0013E-01  1.0052E-01  9.9884E-02  1.0033E-01  1.0009E-01
             8.5286E-02  9.6084E-02 -9.9156E-02  1.1294E-01 -9.2070E-02  1.1640E-01 -1.1300E-01  1.0332E-01  1.0613E-01  1.1241E-01
             9.5035E-02
 GRADIENT:  -8.4063E+02 -4.0333E+02 -5.7929E+02 -1.6398E+02  3.0758E+01  2.3286E+01  7.0005E+01  2.4952E+01  2.5888E+01  9.8818E+01
            -1.1773E+01 -7.4240E-01 -1.1091E-01 -3.0128E-01 -1.3573E-01  1.1526E+00 -2.3919E-01 -8.3741E-02 -1.6858E-01 -1.1455E+00
            -8.3993E-01
 
0ITERATION NO.:   20    OBJECTIVE VALUE:  -10772.0079281468        NO. OF FUNC. EVALS.:  72
 CUMULATIVE NO. OF FUNC. EVALS.:      352
 NPARAMETR:  3.3052E+00  3.2574E+00 -6.1181E-01 -2.0809E-01  7.3562E-01  1.1395E+00  3.3616E-01  1.9188E-01  6.9439E-01  2.3014E+00
             1.0028E-01  1.0233E-02  1.5743E-06  1.2783E-03 -6.5523E-04  7.8342E-03 -2.7567E-04  4.2539E-04  9.9875E-03  1.8626E-03
             9.7482E-03
 PARAMETER:  1.0017E-01  1.0013E-01 -9.9961E-02 -9.9882E-02  1.0002E-01  1.0028E-01  1.0035E-01  9.9725E-02  1.0045E-01  1.0012E-01
             7.9636E-02  1.0361E-01  5.1998E-03  2.2001E-01 -5.6366E-02  1.1224E-01 -3.9849E-02  2.3331E-01  2.3765E-01  1.3366E+00
             1.6103E-01
 GRADIENT:   3.9874E+01 -2.0791E+01  2.6971E+01  4.6652E+00 -4.4044E+00 -1.8171E+01 -1.3759E+01 -3.7369E+00  1.0304E+01 -6.7162E+01
             5.9053E+00  8.0965E-01 -1.5958E-01  1.0948E+00 -2.1961E+00  4.4591E-01  4.5014E-01 -3.0936E-01  3.9679E+00 -7.4384E-01
             9.5401E+00
 
0ITERATION NO.:   25    OBJECTIVE VALUE:  -10772.1428695273        NO. OF FUNC. EVALS.:  71
 CUMULATIVE NO. OF FUNC. EVALS.:      423
 NPARAMETR:  3.3052E+00  3.2580E+00 -6.1182E-01 -2.0825E-01  7.3530E-01  1.1400E+00  3.3630E-01  1.9179E-01  6.9455E-01  2.3016E+00
             1.0001E-01  1.0283E-02  1.8992E-04  1.2877E-03 -5.8911E-04  7.9496E-03 -1.6682E-04  5.4607E-04  1.0022E-02  2.0178E-03
             9.6791E-03
 PARAMETER:  1.0017E-01  1.0015E-01 -9.9964E-02 -9.9959E-02  9.9980E-02  1.0032E-01  1.0039E-01  9.9677E-02  1.0048E-01  1.0012E-01
             7.6981E-02  1.0601E-01  6.2578E-01  2.2110E-01 -5.0557E-02  1.1933E-01 -2.7339E-02  3.0324E-01  2.3957E-01  1.4331E+00
             1.5369E-01
 GRADIENT:  -9.7800E+00 -3.2918E+00 -6.5915E+00 -1.8246E+00  3.3124E-01  1.0012E+00  1.0020E+00  2.9540E-01 -6.7709E-02  3.8701E+00
            -3.5652E-01 -4.5666E-02 -3.7910E-03 -1.5021E-02  1.6062E-01 -1.3308E-02 -4.2029E-03  7.5327E-03 -1.1180E-01  2.6664E-02
            -2.7417E-01
 
0ITERATION NO.:   30    OBJECTIVE VALUE:  -10772.1430246384        NO. OF FUNC. EVALS.: 176
 CUMULATIVE NO. OF FUNC. EVALS.:      599
 NPARAMETR:  3.3052E+00  3.2580E+00 -6.1182E-01 -2.0824E-01  7.3530E-01  1.1400E+00  3.3630E-01  1.9179E-01  6.9456E-01  2.3016E+00
             1.0002E-01  1.0283E-02  1.9214E-04  1.2829E-03 -5.9443E-04  7.9511E-03 -1.7007E-04  5.4305E-04  1.0011E-02  2.0053E-03
             9.6753E-03
 PARAMETER:  1.0017E-01  1.0015E-01 -9.9963E-02 -9.9957E-02  9.9980E-02  1.0032E-01  1.0039E-01  9.9676E-02  1.0048E-01  1.0012E-01
             7.7113E-02  1.0601E-01  6.3309E-01  2.2027E-01 -5.1014E-02  1.1942E-01 -2.7829E-02  3.0169E-01  2.3906E-01  1.4256E+00
             1.5372E-01
 GRADIENT:   1.2170E+01  9.0697E+00  8.1438E+00  1.8788E+00 -1.6811E+00 -1.8978E+00 -2.1485E+00 -1.3474E+00 -1.4031E+00 -3.8774E+00
             5.2980E-04  1.0865E-02 -3.4125E-04 -1.5422E-03  5.8143E-03  3.3749E-02 -3.7663E-03 -4.0094E-04  8.9480E-03  1.1957E-03
             1.7704E-02
 
0ITERATION NO.:   31    OBJECTIVE VALUE:  -10772.1430246384        NO. OF FUNC. EVALS.:  22
 CUMULATIVE NO. OF FUNC. EVALS.:      621
 NPARAMETR:  3.3052E+00  3.2580E+00 -6.1182E-01 -2.0824E-01  7.3530E-01  1.1400E+00  3.3630E-01  1.9179E-01  6.9456E-01  2.3016E+00
             1.0002E-01  1.0283E-02  1.9214E-04  1.2829E-03 -5.9443E-04  7.9511E-03 -1.7007E-04  5.4305E-04  1.0011E-02  2.0053E-03
             9.6753E-03
 PARAMETER:  1.0017E-01  1.0015E-01 -9.9963E-02 -9.9957E-02  9.9980E-02  1.0032E-01  1.0039E-01  9.9676E-02  1.0048E-01  1.0012E-01
             7.7113E-02  1.0601E-01  6.3309E-01  2.2027E-01 -5.1014E-02  1.1942E-01 -2.7829E-02  3.0169E-01  2.3906E-01  1.4256E+00
             1.5372E-01
 GRADIENT:   1.2170E+01  9.0697E+00  8.1438E+00  1.8788E+00 -1.6811E+00 -1.8978E+00 -2.1485E+00 -1.3474E+00 -1.4031E+00 -3.8774E+00
             5.2980E-04  1.0865E-02 -3.4125E-04 -1.5422E-03  5.8143E-03  3.3749E-02 -3.7663E-03 -4.0094E-04  8.9480E-03  1.1957E-03
             1.7704E-02
 
 #TERM:
0MINIMIZATION SUCCESSFUL
 NO. OF FUNCTION EVALUATIONS USED:      621
 NO. OF SIG. DIGITS IN FINAL EST.:  2.8

 ETABAR IS THE ARITHMETIC MEAN OF THE ETA-ESTIMATES,
 AND THE P-VALUE IS GIVEN FOR THE NULL HYPOTHESIS THAT THE TRUE MEAN IS 0.
 
 ETABAR:         5.5379E-05 -1.0209E-03 -3.7853E-05 -9.6794E-04
 SE:             4.7163E-03  2.9851E-03  2.9488E-03  3.6732E-03
 N:                     400         400         400         400
 
 P VAL.:         9.9063E-01  7.3235E-01  9.8976E-01  7.9215E-01
 
 ETASHRINKSD(%)  6.9790E+00  3.3037E+01  4.1056E+01  2.5307E+01
 ETASHRINKVR(%)  1.3471E+01  5.5159E+01  6.5256E+01  4.4209E+01
 EBVSHRINKSD(%)  6.9643E+00  3.3076E+01  4.1035E+01  2.5321E+01
 EBVSHRINKVR(%)  1.3444E+01  5.5211E+01  6.5231E+01  4.4231E+01
 RELATIVEINF(%)  7.8113E+01  4.2166E+01  2.8300E+01  4.1876E+01
 EPSSHRINKSD(%)  2.6155E+01
 EPSSHRINKVR(%)  4.5470E+01
 
  
 TOTAL DATA POINTS NORMALLY DISTRIBUTED (N):         2000
 N*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    3675.75413281869     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -10772.1430246384     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -7096.38889181968     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 TOTAL EFFECTIVE ETAS (NIND*NETA):                          1600
  
 #TERE:
 Elapsed estimation  time in seconds:    84.32
 Elapsed covariance  time in seconds:    80.12
 Elapsed postprocess time in seconds:     0.00
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************          FIRST ORDER CONDITIONAL ESTIMATION WITH INTERACTION (NO PRIOR)        ********************
 #OBJT:**************                       MINIMUM VALUE OF OBJECTIVE FUNCTION                      ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 





 #OBJV:********************************************        -10772.143       *********************************************
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************          FIRST ORDER CONDITIONAL ESTIMATION WITH INTERACTION (NO PRIOR)        ********************
 ********************                             FINAL PARAMETER ESTIMATE                           ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11     
 
         3.31E+00  3.26E+00 -6.12E-01 -2.08E-01  7.35E-01  1.14E+00  3.36E-01  1.92E-01  6.95E-01  2.30E+00  1.00E-01
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        1.03E-02
 
 ETA2
+        1.92E-04  7.95E-03
 
 ETA3
+        1.28E-03 -1.70E-04  1.00E-02
 
 ETA4
+       -5.94E-04  5.43E-04  2.01E-03  9.68E-03
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        1.00E+00
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        1.01E-01
 
 ETA2
+        2.12E-02  8.92E-02
 
 ETA3
+        1.26E-01 -1.91E-02  1.00E-01
 
 ETA4
+       -5.96E-02  6.19E-02  2.04E-01  9.84E-02
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        1.00E+00
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************          FIRST ORDER CONDITIONAL ESTIMATION WITH INTERACTION (NO PRIOR)        ********************
 ********************                            STANDARD ERROR OF ESTIMATE                          ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11     
 
         3.27E-02  2.86E-02  9.52E-03  8.31E-03  3.91E-02  3.58E-02  1.13E-02  1.04E-02  1.05E-02  8.57E-03  2.78E-03
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        9.68E-04
 
 ETA2
+        8.28E-04  1.37E-03
 
 ETA3
+        1.28E-03  1.43E-03  3.04E-03
 
 ETA4
+        1.00E-03  1.10E-03  2.14E-03  1.92E-03
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+       .........
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        4.77E-03
 
 ETA2
+        9.06E-02  7.70E-03
 
 ETA3
+        1.15E-01  1.62E-01  1.52E-02
 
 ETA4
+        1.04E-01  1.22E-01  1.75E-01  9.77E-03
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+       .........
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************          FIRST ORDER CONDITIONAL ESTIMATION WITH INTERACTION (NO PRIOR)        ********************
 ********************                          COVARIANCE MATRIX OF ESTIMATE                         ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 TH 1
+        1.07E-03
 
 TH 2
+        1.55E-05  8.21E-04
 
 TH 3
+       -3.01E-04 -2.84E-06  9.06E-05
 
 TH 4
+       -9.74E-07 -2.28E-04  9.86E-08  6.91E-05
 
 TH 5
+        2.90E-04  2.48E-05 -7.84E-05 -2.41E-06  1.53E-03
 
 TH 6
+        2.82E-05  2.93E-04 -5.48E-06 -7.62E-05  3.97E-05  1.28E-03
 
 TH 7
+       -8.09E-05 -7.81E-06  2.28E-05  8.12E-07 -4.29E-04 -1.24E-05  1.28E-04
 
 TH 8
+       -4.54E-06 -7.82E-05  9.14E-07  2.24E-05 -5.44E-06 -3.55E-04  1.84E-06  1.07E-04
 
 TH 9
+        4.38E-05  4.50E-05 -9.14E-06 -4.53E-06  5.73E-05  7.03E-05 -1.80E-05 -1.17E-05  1.10E-04
 
 TH10
+        2.72E-05  3.15E-05 -4.96E-06 -3.49E-06  5.17E-05  5.97E-05 -1.50E-05 -9.21E-06  6.31E-05  7.35E-05
 
 TH11
+        1.20E-06 -3.26E-07 -2.78E-07  1.25E-07  2.98E-06 -2.78E-06 -9.73E-07  6.98E-07  4.99E-07  9.18E-08  7.72E-06
 
 OM11
+        2.76E-07 -9.56E-08 -9.30E-09  6.97E-08  3.63E-07  4.66E-07 -2.23E-09 -2.98E-08  4.28E-07  3.99E-07 -3.87E-07  9.37E-07
 
 OM12
+        2.27E-07 -1.93E-07  1.28E-09  1.22E-07  1.53E-07 -1.89E-07  1.42E-07  2.03E-07  3.42E-07  3.21E-07 -4.20E-07  2.73E-07
          6.85E-07
 
 OM13
+        4.97E-07  1.38E-07  3.87E-08  3.65E-08  1.15E-06  1.17E-06 -1.69E-07 -1.21E-07  1.01E-06  1.00E-06 -7.06E-07  5.40E-07
          2.18E-07  1.63E-06
 
 OM14
+        9.73E-07 -1.32E-06 -1.97E-07  5.03E-07  6.48E-07  9.52E-07 -9.61E-08 -1.17E-07  8.95E-07  6.88E-07 -5.91E-07  3.62E-07
          2.11E-07  8.98E-07  1.01E-06
 
 OM22
+       -2.29E-07 -8.96E-08  1.43E-07  1.10E-07 -1.24E-06 -1.21E-06  7.01E-07  5.50E-07  1.31E-07  1.68E-07 -1.20E-06  8.40E-08
          3.69E-07  7.49E-08  7.63E-08  1.89E-06
 
 OM23
+        6.73E-07  1.02E-07 -7.41E-08  8.04E-08  2.07E-06 -5.90E-07 -2.47E-07  3.88E-07  1.20E-06  9.21E-07 -5.61E-07  1.74E-07
          3.53E-07  6.73E-07  3.58E-07  3.80E-07  2.06E-06
 
 OM24
+        2.97E-07 -2.84E-07 -1.60E-08  1.45E-07  1.69E-06 -2.50E-06 -3.23E-07  8.98E-07  6.29E-07  4.77E-07 -3.80E-07  9.34E-08
          2.29E-07  2.86E-07  3.20E-07  4.14E-07  9.96E-07  1.20E-06
 
 OM33
+        8.37E-07  1.76E-06  3.44E-08 -1.22E-07  1.47E-06  5.41E-06 -1.08E-07 -9.46E-07  1.53E-06  1.93E-06 -3.38E-06  4.51E-07
          3.33E-07  1.95E-06  1.07E-06  3.68E-07  1.61E-06  6.22E-07  9.24E-06
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 OM34
+        5.09E-07  2.14E-06  4.52E-08 -3.18E-07  4.74E-07  5.76E-06  2.10E-08 -1.23E-06  1.69E-06  1.52E-06 -2.41E-06  3.01E-07
          2.39E-07  1.22E-06  9.30E-07  2.14E-07  1.16E-06  6.15E-07  5.54E-06  4.57E-06
 
 OM44
+        9.56E-08  2.06E-06  1.18E-07 -3.77E-07 -3.18E-07  5.69E-06  1.51E-07 -1.35E-06  1.33E-06  1.20E-06 -1.84E-06  2.00E-07
          1.65E-07  7.17E-07  7.57E-07  1.37E-07  6.63E-07  6.44E-07  3.12E-06  3.31E-06  3.70E-06
 
 SG11
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************          FIRST ORDER CONDITIONAL ESTIMATION WITH INTERACTION (NO PRIOR)        ********************
 ********************                          CORRELATION MATRIX OF ESTIMATE                        ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 TH 1
+        3.27E-02
 
 TH 2
+        1.66E-02  2.86E-02
 
 TH 3
+       -9.68E-01 -1.04E-02  9.52E-03
 
 TH 4
+       -3.59E-03 -9.59E-01  1.25E-03  8.31E-03
 
 TH 5
+        2.27E-01  2.21E-02 -2.10E-01 -7.42E-03  3.91E-02
 
 TH 6
+        2.41E-02  2.85E-01 -1.61E-02 -2.56E-01  2.84E-02  3.58E-02
 
 TH 7
+       -2.19E-01 -2.41E-02  2.11E-01  8.63E-03 -9.69E-01 -3.06E-02  1.13E-02
 
 TH 8
+       -1.34E-02 -2.63E-01  9.26E-03  2.60E-01 -1.34E-02 -9.59E-01  1.57E-02  1.04E-02
 
 TH 9
+        1.28E-01  1.50E-01 -9.17E-02 -5.21E-02  1.40E-01  1.88E-01 -1.52E-01 -1.08E-01  1.05E-02
 
 TH10
+        9.71E-02  1.28E-01 -6.08E-02 -4.90E-02  1.54E-01  1.95E-01 -1.54E-01 -1.04E-01  7.03E-01  8.57E-03
 
 TH11
+        1.33E-02 -4.10E-03 -1.05E-02  5.43E-03  2.74E-02 -2.79E-02 -3.10E-02  2.43E-02  1.72E-02  3.86E-03  2.78E-03
 
 OM11
+        8.72E-03 -3.45E-03 -1.01E-03  8.67E-03  9.58E-03  1.35E-02 -2.04E-04 -2.97E-03  4.23E-02  4.81E-02 -1.44E-01  9.68E-04
 
 OM12
+        8.40E-03 -8.15E-03  1.63E-04  1.78E-02  4.72E-03 -6.39E-03  1.51E-02  2.36E-02  3.95E-02  4.52E-02 -1.83E-01  3.41E-01
          8.28E-04
 
 OM13
+        1.19E-02  3.76E-03  3.19E-03  3.44E-03  2.29E-02  2.55E-02 -1.17E-02 -9.12E-03  7.55E-02  9.15E-02 -1.99E-01  4.37E-01
          2.06E-01  1.28E-03
 
 OM14
+        2.97E-02 -4.58E-02 -2.06E-02  6.03E-02  1.65E-02  2.65E-02 -8.47E-03 -1.13E-02  8.52E-02  7.99E-02 -2.12E-01  3.73E-01
          2.54E-01  7.01E-01  1.00E-03
 
 OM22
+       -5.11E-03 -2.28E-03  1.09E-02  9.60E-03 -2.30E-02 -2.46E-02  4.51E-02  3.87E-02  9.10E-03  1.43E-02 -3.15E-01  6.32E-02
          3.24E-01  4.27E-02  5.54E-02  1.37E-03
 
 OM23
+        1.44E-02  2.48E-03 -5.43E-03  6.75E-03  3.69E-02 -1.15E-02 -1.52E-02  2.61E-02  7.98E-02  7.49E-02 -1.41E-01  1.26E-01
          2.97E-01  3.68E-01  2.49E-01  1.93E-01  1.43E-03
 
 OM24
+        8.29E-03 -9.03E-03 -1.54E-03  1.59E-02  3.93E-02 -6.36E-02 -2.60E-02  7.91E-02  5.48E-02  5.07E-02 -1.25E-01  8.80E-02
          2.52E-01  2.04E-01  2.91E-01  2.75E-01  6.33E-01  1.10E-03
 
 OM33
+        8.43E-03  2.02E-02  1.19E-03 -4.82E-03  1.24E-02  4.97E-02 -3.15E-03 -3.00E-02  4.82E-02  7.40E-02 -4.00E-01  1.53E-01
          1.32E-01  5.04E-01  3.51E-01  8.82E-02  3.70E-01  1.87E-01  3.04E-03
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 OM34
+        7.29E-03  3.49E-02  2.22E-03 -1.79E-02  5.66E-03  7.53E-02  8.69E-04 -5.55E-02  7.53E-02  8.32E-02 -4.06E-01  1.46E-01
          1.35E-01  4.48E-01  4.33E-01  7.27E-02  3.77E-01  2.62E-01  8.52E-01  2.14E-03
 
 OM44
+        1.52E-03  3.75E-02  6.44E-03 -2.36E-02 -4.23E-03  8.27E-02  6.96E-03 -6.76E-02  6.59E-02  7.27E-02 -3.45E-01  1.07E-01
          1.04E-01  2.92E-01  3.92E-01  5.21E-02  2.40E-01  3.05E-01  5.34E-01  8.04E-01  1.92E-03
 
 SG11
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************          FIRST ORDER CONDITIONAL ESTIMATION WITH INTERACTION (NO PRIOR)        ********************
 ********************                      INVERSE COVARIANCE MATRIX OF ESTIMATE                     ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 TH 1
+        1.59E+04
 
 TH 2
+        8.45E-07  1.85E+04
 
 TH 3
+        5.24E+04  0.00E+00  1.84E+05
 
 TH 4
+        0.00E+00  6.08E+04 -1.69E-21  2.15E+05
 
 TH 5
+       -2.14E+03  0.00E+00 -6.77E+03  0.00E+00  1.12E+04
 
 TH 6
+        0.00E+00 -3.51E+03  1.06E-22 -1.13E+04  0.00E+00  1.17E+04
 
 TH 7
+       -6.77E+03 -8.30E-06 -2.34E+04  0.00E+00  3.76E+04 -4.24E-22  1.34E+05
 
 TH 8
+        0.00E+00 -1.13E+04 -2.12E-22 -3.94E+04 -6.43E-05  3.83E+04 -1.41E-04  1.35E+05
 
 TH 9
+       -1.36E+03 -3.88E+03 -3.63E+03 -1.25E+04  1.09E+03 -5.65E+02  4.23E+03 -1.28E+03  1.95E+04
 
 TH10
+       -9.76E+02 -2.73E+02 -3.56E+03 -8.21E+02 -7.37E+02 -3.20E+03 -1.43E+03 -9.86E+03 -1.49E+04  2.83E+04
 
 TH11
+       -1.45E+03 -1.36E+03 -4.74E+03 -4.80E+03 -1.10E+03 -4.99E+02 -3.42E+03 -2.28E+03 -1.11E+03 -7.12E+02  1.80E+05
 
 OM11
+       -9.58E+01 -2.71E+02 -2.45E+02 -8.33E+02 -1.36E+03 -5.05E+02 -4.66E+03 -1.39E+03 -8.05E+02 -4.25E+02  3.48E+04  1.48E+06
 
 OM12
+       -1.87E+03 -1.01E+03 -6.61E+03 -3.03E+03 -4.36E+03 -3.70E+03 -1.58E+04 -1.37E+04  1.02E+03 -1.63E+03  2.18E+04 -4.88E+05
          1.99E+06
 
 OM13
+       -5.14E+03  5.46E+03 -2.12E+04  2.77E+04  2.87E+02 -9.56E+02  1.79E+03 -6.02E+03  1.55E+03 -5.58E+03 -2.71E+04 -4.50E+05
          1.72E+05  1.73E+06
 
 OM14
+        2.63E+03 -3.48E+03  1.47E+04 -3.08E+04 -1.18E+03  7.46E+02 -5.36E+03  6.86E+03 -6.20E+03  3.22E+03  3.14E+04 -1.00E+05
         -2.98E+05 -1.28E+06  2.44E+06
 
 OM22
+       -1.21E+03 -2.00E+03 -4.12E+03 -6.90E+03 -5.31E+03 -2.21E+03 -2.13E+04 -8.43E+03 -4.04E+02 -5.82E+01  1.05E+05  4.63E+04
         -2.96E+05  7.32E+02  7.28E+04  6.94E+05
 
 OM23
+        1.61E+03  3.46E+02  6.42E+03 -1.52E+03 -5.92E+03  1.42E+03 -2.09E+04  7.61E+03 -4.87E+03  3.26E+02 -1.40E+04  8.02E+04
         -2.62E+05 -3.57E+05  3.23E+05  2.29E+04  1.06E+06
 
 OM24
+       -6.03E+02  1.65E+03 -3.82E+03  9.71E+03  1.86E+03 -7.12E+02  1.08E+04 -1.48E+04  6.59E+02 -2.77E+02 -2.51E+04  1.19E+04
          8.76E+03  2.62E+05 -4.60E+05 -2.19E+05 -8.16E+05  1.70E+06
 
 OM33
+       -5.95E+02 -2.29E+03 -1.30E+03 -1.02E+04 -1.02E+03 -1.04E+03 -3.04E+03 -3.92E+03  5.23E+03 -4.08E+03  4.80E+04  3.98E+04
         -1.64E+04 -2.97E+05  2.36E+05 -1.45E+03  1.56E+04 -1.38E+04  5.87E+05
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      TH 9      TH10      TH11      OM11  
             OM12      OM13      OM14      OM22      OM23      OM24      OM33      OM34      OM44      SG11  
 
 OM34
+        4.03E+02  3.15E+02  5.57E+02  5.33E+03  1.53E+03 -7.26E+02  5.04E+03 -2.19E+03 -8.07E+03  6.40E+03  5.89E+03  1.18E+04
          5.78E+04  2.13E+05 -3.85E+05  1.49E+04 -2.97E+05  2.42E+05 -9.11E+05  2.25E+06
 
 OM44
+       -1.36E+03 -9.54E+02 -5.09E+03 -2.86E+03 -1.16E+02 -6.16E+02 -1.68E+03  2.31E+03  3.21E+03 -4.40E+03  4.39E+04  5.54E+03
         -3.74E+03  9.90E+03 -5.07E+04  4.55E+04  2.08E+05 -3.20E+05  3.51E+05 -1.19E+06  1.09E+06
 
 SG11
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 Elapsed finaloutput time in seconds:     0.02
 #CPUT: Total CPU Time in Seconds,     1992.703
Stop Time: 
Tue 10/22/2024 
04:41 PM
