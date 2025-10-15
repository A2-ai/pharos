Tue 10/22/2024 
01:27 PM
;Model Desc: Receptor Mediated Clearance model with Dynamic Change 
;            in Receptors
;Project Name: nm7examples
;Project ID: NO PROJECT DESCRIPTION

$PROB RUN# example6 (from r2compl)
$INPUT C SET ID JID TIME DV=CONC DOSE=AMT RATE EVID MDV CMT
$DATA example6.csv IGNORE=C

; The new numerical integration solver is used, although ADVAN=9 
; is also efficient for this problem.

$SUBROUTINES ADVAN13 TRANS1 TOL=4
$MODEL NCOMPARTMENTS=3

$PK
MU_1=THETA(1)
MU_2=THETA(2)
MU_3=THETA(3)
MU_4=THETA(4)
MU_5=THETA(5)
MU_6=THETA(6)
MU_7=THETA(7)
MU_8=THETA(8)
VC=EXP(MU_1+ETA(1))
K10=EXP(MU_2+ETA(2))
K12=EXP(MU_3+ETA(3))
K21=EXP(MU_4+ETA(4))
VM=EXP(MU_5+ETA(5))
KMC=EXP(MU_6+ETA(6))
K03=EXP(MU_7+ETA(7))
K30=EXP(MU_8+ETA(8))
S3=VC
S1=VC
KM=KMC*S1
F3=K03/K30

$DES
DADT(1) = -(K10+K12)*A(1) + K21*A(2) - VM*A(1)*A(3)/(A(1)+KM)
DADT(2) = K12*A(1) - K21*A(2)
DADT(3) =  -VM*A(1)*A(3)/(A(1)+KM) - K30*A(3) + K03

$ERROR
CALLFL=0
ETYPE=1
IF(CMT.NE.1) ETYPE=0
IPRED=F
Y = F + F*ETYPE*EPS(1) + F*(1.0-ETYPE)*EPS(2)


$THETA 
;Initial Thetas
( 4.0 )  ;[MU_1]
( -2.1 ) ;[MU_2]
( 0.7 )  ;[MU_3]
( -0.17 );[MU_4]      
( 2.2 ) ;[MU_5]
( 0.14 )  ;[MU_6]
( 3.7 )  ;[MU_7]
( -0.7) ;[MU_8]


;Initial Omegas
$OMEGA BLOCK(8)
0.2 ;[p]
-0.0043  ;[f]
0.2 ;[p]
0.0048   ;[f]    
-0.0023  ;[f]     
0.2 ;[p]
0.0032   ;[f]   
0.0059   ;[f]  
-0.0014  ;[f]   
0.2 ;[p]
0.0029   ;[f]   
0.0027 ;[f]  
-0.00026 ;[f]  
-0.0032  ;[f]    
0.2 ;[p]
-0.0025  ;[f]  
0.00097  ;[f]   
0.0024   ;[f]  
0.00197  ;[f]  
-0.0080  ;[f]   
0.2 ;[p]
0.0031   ;[f]  
-0.00571 ;[f]    
0.0030   ;[f]   
-0.0074  ;[f]    
0.0025   ;[f]   
0.0034   ;[f]  
0.2 ;[p]
0.00973  ;[f]  
0.00862  ;[f]  
0.0041   ;[f]  
0.0046   ;[f]   
0.00061  ;[f] 
-0.0056  ;[f]   
0.0056   ;[f]  
0.2 ;[p]

$SIGMA  
0.1 ;[p]
0.1 ;[p]

$PRIOR NWPRI
; Omega prior
$OMEGAP BLOCK(8)
0.2 FIX
0.0 0.2
0.0 0.0 0.2
0.0 0.0 0.0 0.2
0.0 0.0 0.0 0.0 0.2
0.0 0.0 0.0 0.0 0.0 0.2
0.0 0.0 0.0 0.0 0.0 0.0 0.2
0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.2
; degrees of freedom for OMEGA prior
$OMEGAPD
(8 FIXED)           ;[dfo]

; Starting with a short iterative two stage analysis brings the 
; results closer so less time needs to be spent during the 
; burn-in of the BAYES analysis

$EST METHOD=ITS INTERACTION SIGL=4 NITER=15 PRINT=1 
     FILE=example6.ext NOABORT NOPRIOR=1

$EST METHOD=BAYES INTERACTION NBURN=4000 SIGL=4 NITER=10000
     PRINT=10 CTYPE=3 FILE=example6.txt NOABORT NOPRIOR=0

; By default, ISAMPLE_M* are 2.  Since there are many data points 
; per subject, setting these to 1 is enough, and it reduces the 
; time of the analysis

     ISAMPLE_M1=1 ISAMPLE_M2=1 ISAMPLE_M3=1 IACCEPT=0.4

$COV MATRIX=R UNCONDITIONAL
  
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
 RUN# example6 (from r2compl)
0DATA CHECKOUT RUN:              NO
 DATA SET LOCATED ON UNIT NO.:    2
 THIS UNIT TO BE REWOUND:        NO
 CREATE/ADD TO FDATA.csv:        YES
 NO. OF DATA RECS IN DATA SET:     1750
 NO. OF DATA ITEMS IN DATA SET:  11
 ID DATA ITEM IS DATA ITEM NO.:   3
 DEP VARIABLE IS DATA ITEM NO.:   6
 MDV DATA ITEM IS DATA ITEM NO.: 10
0INDICES PASSED TO SUBROUTINE PRED:
   9   5   7   8   0   0  11   0   0   0   0
0LABELS FOR DATA ITEMS:
 C SET ID JID TIME CONC DOSE RATE EVID MDV CMT
0FORMAT FOR DATA:
 (2E2.0,2E3.0,E5.0,E10.0,2E5.0,3E2.0)

 TOT. NO. OF OBS RECS:     1568
 TOT. NO. OF INDIVIDUALS:       50
0LENGTH OF THETA:   9
0DEFAULT THETA BOUNDARY TEST OMITTED:    NO
0OMEGA HAS BLOCK FORM:
  1
  1  1
  1  1  1
  1  1  1  1
  1  1  1  1  1
  1  1  1  1  1  1
  1  1  1  1  1  1  1
  1  1  1  1  1  1  1  1
  0  0  0  0  0  0  0  0  2
  0  0  0  0  0  0  0  0  2  2
  0  0  0  0  0  0  0  0  2  2  2
  0  0  0  0  0  0  0  0  2  2  2  2
  0  0  0  0  0  0  0  0  2  2  2  2  2
  0  0  0  0  0  0  0  0  2  2  2  2  2  2
  0  0  0  0  0  0  0  0  2  2  2  2  2  2  2
  0  0  0  0  0  0  0  0  2  2  2  2  2  2  2  2
0DEFAULT OMEGA BOUNDARY TEST OMITTED:    NO
0SIGMA HAS SIMPLE DIAGONAL FORM WITH DIMENSION:   2
0DEFAULT SIGMA BOUNDARY TEST OMITTED:    NO
0INITIAL ESTIMATE OF THETA:
 LOWER BOUND    INITIAL EST    UPPER BOUND
 -0.1000E+07     0.4000E+01     0.1000E+07
 -0.1000E+07    -0.2100E+01     0.1000E+07
 -0.1000E+07     0.7000E+00     0.1000E+07
 -0.1000E+07    -0.1700E+00     0.1000E+07
 -0.1000E+07     0.2200E+01     0.1000E+07
 -0.1000E+07     0.1400E+00     0.1000E+07
 -0.1000E+07     0.3700E+01     0.1000E+07
 -0.1000E+07    -0.7000E+00     0.1000E+07
  0.8000E+01     0.8000E+01     0.8000E+01
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.2000E+00
                 -0.4300E-02   0.2000E+00
                  0.4800E-02  -0.2300E-02   0.2000E+00
                  0.3200E-02   0.5900E-02  -0.1400E-02   0.2000E+00
                  0.2900E-02   0.2700E-02  -0.2600E-03  -0.3200E-02   0.2000E+00
                 -0.2500E-02   0.9700E-03   0.2400E-02   0.1970E-02  -0.8000E-02   0.2000E+00
                  0.3100E-02  -0.5710E-02   0.3000E-02  -0.7400E-02   0.2500E-02   0.3400E-02   0.2000E+00
                  0.9730E-02   0.8620E-02   0.4100E-02   0.4600E-02   0.6100E-03  -0.5600E-02   0.5600E-02   0.2000E+00
        2                                                                                  YES
                  0.2000E+00
                  0.0000E+00   0.2000E+00
                  0.0000E+00   0.0000E+00   0.2000E+00
                  0.0000E+00   0.0000E+00   0.0000E+00   0.2000E+00
                  0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.2000E+00
                  0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.2000E+00
                  0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.2000E+00
                  0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.2000E+00
0INITIAL ESTIMATE OF SIGMA:
 0.1000E+00
 0.0000E+00   0.1000E+00
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

 GENERAL NONLINEAR KINETICS MODEL WITH STIFF/NONSTIFF EQUATIONS (LSODA, ADVAN13)
0MODEL SUBROUTINE USER-SUPPLIED - ID NO. 9999
0MAXIMUM NO. OF BASIC PK PARAMETERS:   7
0COMPARTMENT ATTRIBUTES
 COMPT. NO.   FUNCTION   INITIAL    ON/OFF      DOSE      DEFAULT    DEFAULT
                         STATUS     ALLOWED    ALLOWED    FOR DOSE   FOR OBS.
    1         COMP 1       ON         YES        YES        YES        YES
    2         COMP 2       ON         YES        YES        NO         NO
    3         COMP 3       ON         YES        YES        NO         NO
    4         OUTPUT       OFF        YES        NO         NO         NO
 INITIAL (BASE) TOLERANCE SETTINGS:
 NRD (RELATIVE) VALUE(S) OF TOLERANCE:   4
 ANRD (ABSOLUTE) VALUE(S) OF TOLERANCE:  12
1
 ADDITIONAL PK PARAMETERS - ASSIGNMENT OF ROWS IN GG
 COMPT. NO.                             INDICES
              SCALE      BIOAVAIL.   ZERO-ORDER  ZERO-ORDER  ABSORB
                         FRACTION    RATE        DURATION    LAG
    1            9           *           *           *           *
    2            *           *           *           *           *
    3            8          10           *           *           *
    4            *           -           -           -           -
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
0DES SUBROUTINE USES COMPACT STORAGE MODE.
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
 NO. OF FUNCT. EVALS. ALLOWED:            3480
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
 RAW OUTPUT FILE (FILE): example6.ext
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
 CONVERGENCE TYPE (CTYPE):                  0
 ITERATIONS (NITER):                        15
 ANNEAL SETTING (CONSTRAIN):                 1

 TOLERANCES FOR ESTIMATION/EVALUATION STEP:
 NRD (RELATIVE) VALUE(S) OF TOLERANCE:   4
 ANRD (ABSOLUTE) VALUE(S) OF TOLERANCE:  12
 TOLERANCES FOR COVARIANCE STEP:
 NRD (RELATIVE) VALUE(S) OF TOLERANCE:   4
 ANRD (ABSOLUTE) VALUE(S) OF TOLERANCE:  12
 
 THE FOLLOWING LABELS ARE EQUIVALENT
 PRED=PREDI
 RES=RESI
 WRES=WRESI
 IWRS=IWRESI
 IPRD=IPREDI
 IRS=IRESI
 
 EM/BAYES SETUP:
 THETAS THAT ARE MU MODELED:
   1   2   3   4   5   6   7   8
 THETAS THAT ARE SIGMA-LIKE:
 
 
 MONITORING OF SEARCH:

 iteration            0  OBJ=  -3444.74507957977
 iteration            1  OBJ=  -3598.45263320042
 iteration            2  OBJ=  -3712.26435407253
 iteration            3  OBJ=  -3819.61181223329
 iteration            4  OBJ=  -3924.00713821958
 iteration            5  OBJ=  -4026.58071830191
 iteration            6  OBJ=  -4127.62999207525
 iteration            7  OBJ=  -4227.11841490832
 iteration            8  OBJ=  -4324.61184665705
 iteration            9  OBJ=  -4419.27493684135
 iteration           10  OBJ=  -4509.46541865128
 iteration           11  OBJ=  -4591.76071639688
 iteration           12  OBJ=  -4659.24969620808
 iteration           13  OBJ=  -4699.15883451159
 iteration           14  OBJ=  -4708.49014405059
 iteration           15  OBJ=  -4709.72446726038
 
 #TERM:
 OPTIMIZATION WAS NOT TESTED FOR CONVERGENCE


 ETABAR IS THE ARITHMETIC MEAN OF THE ETA-ESTIMATES,
 AND THE P-VALUE IS GIVEN FOR THE NULL HYPOTHESIS THAT THE TRUE MEAN IS 0.
 
 ETABAR:        -8.4109E-04 -2.9968E-03  2.4673E-03  1.4617E-03  1.4889E-03  2.1020E-03  5.6130E-04  1.1675E-03
 SE:             6.9294E-02  5.2546E-02  3.7560E-02  6.5052E-02  5.6737E-02  5.7053E-02  6.4169E-02  6.1367E-02
 N:                      50          50          50          50          50          50          50          50
 
 P VAL.:         9.9032E-01  9.5452E-01  9.4762E-01  9.8207E-01  9.7906E-01  9.7061E-01  9.9302E-01  9.8482E-01
 
 ETASHRINKSD(%)  6.4526E-01  4.2851E+00  8.1602E+00  1.6353E+00  1.5118E+00  5.8484E+00  3.5762E-01  1.5906E+00
 ETASHRINKVR(%)  1.2863E+00  8.3866E+00  1.5655E+01  3.2439E+00  3.0008E+00  1.1355E+01  7.1396E-01  3.1560E+00
 EBVSHRINKSD(%)  6.4061E-01  5.5298E+00  1.0005E+01  2.1309E+00  1.5846E+00  6.4035E+00  4.6109E-01  1.7930E+00
 EBVSHRINKVR(%)  1.2771E+00  1.0754E+01  1.9009E+01  4.2164E+00  3.1441E+00  1.2397E+01  9.2006E-01  3.5539E+00
 RELATIVEINF(%)  1.0000E+02  3.8254E+01  6.2214E+01  9.1079E+01  7.3138E+01  4.7695E+01  1.0000E+02  6.8700E+01
 EPSSHRINKSD(%)  1.5654E+01  7.2063E+00
 EPSSHRINKVR(%)  2.8858E+01  1.3893E+01
 
  
 TOTAL DATA POINTS NORMALLY DISTRIBUTED (N):         1568
 N*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    2881.79124012985     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -4709.72446726038     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -1827.93322713053     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 TOTAL EFFECTIVE ETAS (NIND*NETA):                           400
  
 #TERE:
 Elapsed estimation  time in seconds:   210.19
 Elapsed covariance  time in seconds:     0.04
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 #OBJT:**************                        FINAL VALUE OF OBJECTIVE FUNCTION                       ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 





 #OBJV:********************************************         -4709.724       *********************************************
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 ********************                             FINAL PARAMETER ESTIMATE                           ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8     
 
         3.91E+00 -2.19E+00  5.58E-01 -1.87E-01  2.26E+00  2.10E-01  3.71E+00 -7.09E-01
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4      ETA5      ETA6      ETA7      ETA8     
 
 ETA1
+        2.43E-01
 
 ETA2
+       -3.36E-02  1.51E-01
 
 ETA3
+        4.63E-02 -1.38E-02  8.36E-02
 
 ETA4
+        3.12E-02  4.59E-02 -2.11E-02  2.19E-01
 
 ETA5
+        2.60E-02  2.68E-02 -2.79E-03 -3.26E-02  1.66E-01
 
 ETA6
+       -2.84E-02  1.06E-02  2.63E-02  1.85E-02 -7.91E-02  1.84E-01
 
 ETA7
+        2.83E-02 -3.27E-02  3.11E-02 -7.06E-02  2.32E-02  3.50E-03  2.07E-01
 
 ETA8
+        9.59E-02  8.01E-02  3.36E-02  4.32E-02  9.82E-04 -4.96E-02  5.40E-02  1.94E-01
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1      EPS2     
 
 EPS1
+        9.29E-03
 
 EPS2
+        0.00E+00  2.25E-02
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4      ETA5      ETA6      ETA7      ETA8     
 
 ETA1
+        4.93E-01
 
 ETA2
+       -1.76E-01  3.88E-01
 
 ETA3
+        3.25E-01 -1.23E-01  2.89E-01
 
 ETA4
+        1.35E-01  2.53E-01 -1.56E-01  4.68E-01
 
 ETA5
+        1.29E-01  1.69E-01 -2.37E-02 -1.71E-01  4.07E-01
 
 ETA6
+       -1.34E-01  6.39E-02  2.12E-01  9.23E-02 -4.53E-01  4.28E-01
 
 ETA7
+        1.26E-01 -1.85E-01  2.36E-01 -3.31E-01  1.25E-01  1.79E-02  4.55E-01
 
 ETA8
+        4.41E-01  4.68E-01  2.64E-01  2.10E-01  5.47E-03 -2.63E-01  2.69E-01  4.41E-01
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1      EPS2     
 
 EPS1
+        9.64E-02
 
 EPS2
+        0.00E+00  1.50E-01
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 ********************                          STANDARD ERROR OF ESTIMATE (S)                        ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8     
 
         3.35E-01  2.21E-01  2.20E-01  3.04E-01  2.10E-01  2.57E-01  1.48E-01  3.71E-01
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4      ETA5      ETA6      ETA7      ETA8     
 
 ETA1
+        1.46E-01
 
 ETA2
+        2.18E-01  2.75E-01
 
 ETA3
+        1.03E-01  1.19E-01  1.76E-01
 
 ETA4
+        8.48E-02  1.26E-01  9.97E-02  1.64E-01
 
 ETA5
+        1.37E-01  8.87E-02  8.42E-02  1.61E-01  3.00E-01
 
 ETA6
+        1.10E-01  1.58E-01  1.31E-01  1.50E-01  8.02E-02  1.49E-01
 
 ETA7
+        1.57E-01  1.05E-01  1.13E-01  9.28E-02  9.82E-02  1.01E-01  1.41E-01
 
 ETA8
+        1.93E-01  1.43E-01  5.78E-02  8.96E-02  1.38E-01  1.94E-01  1.27E-01  2.53E-01
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1      EPS2     
 
 EPS1
+        3.43E-03
 
 EPS2
+        0.00E+00  4.29E-03
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4      ETA5      ETA6      ETA7      ETA8     
 
 ETA1
+        1.48E-01
 
 ETA2
+        1.03E+00  3.54E-01
 
 ETA3
+        7.08E-01  9.34E-01  3.04E-01
 
 ETA4
+        3.53E-01  6.22E-01  8.41E-01  1.75E-01
 
 ETA5
+        7.07E-01  5.61E-01  7.14E-01  9.33E-01  3.68E-01
 
 ETA6
+        5.46E-01  9.41E-01  1.03E+00  7.36E-01  8.03E-01  1.74E-01
 
 ETA7
+        6.81E-01  5.45E-01  9.47E-01  4.11E-01  5.92E-01  5.21E-01  1.55E-01
 
 ETA8
+        5.83E-01  7.49E-01  3.40E-01  3.58E-01  7.74E-01  9.10E-01  5.71E-01  2.87E-01
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1      EPS2     
 
 EPS1
+        1.78E-02
 
 EPS2
+       .........  1.43E-02
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 ********************                        COVARIANCE MATRIX OF ESTIMATE (S)                       ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      OM11      OM12      OM13      OM14  
             OM15      OM16      OM17      OM18      OM22      OM23      OM24      OM25      OM26      OM27      OM28      OM33  
            OM34      OM35      OM36      OM37      OM38      OM44      OM45      OM46      OM47      OM48      OM55      OM56  
             OM57      OM58      OM66      OM67      OM68      OM77      OM78      OM88      SG11      SG12      SG22  
 
 TH 1
+        1.12E-01
 
 TH 2
+       -3.03E-02  4.88E-02
 
 TH 3
+        2.64E-02 -1.32E-02  4.86E-02
 
 TH 4
+        7.64E-02 -2.42E-02 -3.07E-03  9.25E-02
 
 TH 5
+       -5.13E-02  2.52E-02 -8.32E-04 -4.74E-02  4.43E-02
 
 TH 6
+       -5.51E-02  1.96E-02 -1.60E-02 -3.60E-02  3.06E-02  6.60E-02
 
 TH 7
+       -1.52E-02 -3.76E-03  8.31E-03 -2.17E-02  1.16E-02  1.10E-02  2.18E-02
 
 TH 8
+        9.72E-02 -3.85E-02  1.59E-02  8.91E-02 -5.58E-02 -5.14E-02 -1.06E-02  1.37E-01
 
 OM11
+       -2.77E-02  5.15E-03 -1.88E-02 -9.34E-03  1.15E-02  1.40E-02  8.52E-04 -1.72E-02  2.12E-02
 
 OM12
+       -1.95E-02  2.96E-02  2.23E-03 -2.66E-02  2.14E-02  1.57E-02 -2.86E-03 -5.17E-02 -4.02E-03  4.74E-02
 
 OM13
+       -1.02E-02 -8.62E-03  2.98E-03 -7.63E-03  3.25E-03  2.61E-03  4.45E-03  1.24E-04  6.42E-03 -1.26E-02  1.06E-02
 
 OM14
+       -8.17E-03 -2.99E-03  3.03E-03 -1.18E-02  3.87E-03 -3.02E-03  6.45E-03 -8.49E-03  7.76E-04 -3.28E-03  3.13E-03  7.20E-03
 
 OM15
+        1.26E-02 -6.12E-03 -3.20E-03  1.65E-02 -6.92E-03  6.67E-03 -6.29E-03  1.02E-02 -1.02E-04  7.69E-04 -1.64E-03 -8.01E-03
          1.87E-02
 
 OM16
+       -1.61E-02  1.40E-02 -1.21E-03 -1.70E-02  1.25E-02  5.06E-03  2.08E-03 -1.40E-02  2.99E-03  8.32E-03 -2.75E-04  1.90E-03
         -8.80E-03  1.22E-02
 
 OM17
+        3.84E-03 -2.98E-03 -1.99E-02  2.31E-02 -7.08E-03  4.68E-04 -7.06E-03  2.42E-02  1.30E-02 -1.70E-02  2.90E-03 -4.64E-03
          6.10E-03 -2.72E-03  2.45E-02
 
 OM18
+       -2.71E-02  1.59E-02 -3.21E-02 -3.80E-04  1.14E-02  2.54E-02 -4.91E-03 -1.68E-02  2.00E-02  6.28E-03 -2.09E-03 -5.35E-03
          6.36E-03  1.73E-03  2.02E-02  3.73E-02
 
 OM22
+       -2.47E-02 -2.29E-02 -3.20E-02  1.75E-03 -9.08E-03  1.60E-02  3.29E-03  1.40E-02  1.89E-02 -4.16E-02  1.49E-02  2.14E-03
          4.20E-03 -7.32E-03  2.18E-02  1.56E-02  7.54E-02
 
 OM23
+        5.16E-03  5.88E-03  1.33E-02 -1.00E-02  5.30E-03 -4.75E-03  3.62E-03 -1.21E-02 -8.67E-03  1.40E-02 -6.02E-03  2.48E-03
         -5.36E-03  2.86E-03 -1.24E-02 -9.68E-03 -2.44E-02  1.41E-02
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      OM11      OM12      OM13      OM14  
             OM15      OM16      OM17      OM18      OM22      OM23      OM24      OM25      OM26      OM27      OM28      OM33  
            OM34      OM35      OM36      OM37      OM38      OM44      OM45      OM46      OM47      OM48      OM55      OM56  
             OM57      OM58      OM66      OM67      OM68      OM77      OM78      OM88      SG11      SG12      SG22  
 
 OM24
+       -9.60E-03 -3.89E-03 -1.16E-02  3.04E-03 -2.03E-03  1.13E-02 -4.96E-03 -3.89E-03  4.98E-03  1.51E-03  1.29E-04 -5.63E-03
          1.11E-02 -3.94E-03  5.85E-03  1.12E-02  1.54E-02 -8.25E-03  1.59E-02
 
 OM25
+        2.28E-03  3.17E-04 -1.32E-03 -2.25E-03 -4.91E-04 -1.78E-03  3.05E-03  1.01E-03  8.56E-04 -3.91E-03  4.82E-04  2.45E-03
         -4.41E-03  1.93E-03 -2.63E-04 -3.41E-03  1.87E-03  2.24E-03 -6.02E-03  7.87E-03
 
 OM26
+       -2.16E-02  2.27E-02 -1.73E-02 -1.32E-02  1.57E-02  2.50E-02 -2.45E-03 -2.64E-02  8.55E-03  1.88E-02 -6.07E-03 -5.67E-03
          4.05E-03  7.06E-03  3.37E-03  2.00E-02 -4.01E-03  5.69E-04  6.82E-03 -1.38E-03  2.48E-02
 
 OM27
+       -1.32E-02  1.65E-02 -3.20E-03 -1.43E-02  1.32E-02  5.66E-03  1.32E-03 -2.31E-02  3.30E-03  1.46E-02 -3.64E-03  7.08E-04
         -5.09E-03  6.86E-03 -3.32E-03  5.39E-03 -1.47E-02  6.12E-03 -4.53E-03  2.17E-03  9.50E-03  1.11E-02
 
 OM28
+       -3.48E-02  1.55E-02 -1.54E-02 -2.57E-02  1.67E-02  1.76E-02  9.94E-04 -4.41E-02  1.05E-02  1.79E-02 -1.08E-03  4.23E-04
         -1.12E-03  6.01E-03 -2.95E-03  1.22E-02  1.92E-03 -2.83E-05  6.12E-03  2.72E-04  1.30E-02  8.35E-03  2.04E-02
 
 OM33
+        4.12E-02 -2.47E-02  1.39E-03  3.74E-02 -2.94E-02 -2.28E-02 -6.23E-03  5.77E-02 -5.97E-03 -2.93E-02  2.61E-03 -2.95E-03
          6.97E-03 -9.99E-03  1.21E-02 -7.39E-03  1.98E-02 -8.38E-03  1.75E-03  5.92E-04 -1.37E-02 -1.35E-02 -1.85E-02  3.09E-02
 
 OM34
+        1.15E-02 -1.29E-02  7.78E-03  9.68E-03 -7.67E-03 -1.42E-02  3.60E-03  2.00E-02 -2.19E-03 -1.44E-02  4.19E-03  3.74E-03
         -4.33E-03 -2.04E-03  1.54E-03 -9.52E-03  4.33E-03 -4.31E-04 -5.62E-03  2.92E-03 -1.19E-02 -4.14E-03 -8.35E-03  8.74E-03
         9.95E-03
 
 OM35
+        7.75E-04  5.24E-03 -7.97E-03  3.41E-03 -7.26E-04  8.69E-03 -4.06E-03 -3.82E-03  7.32E-04  3.69E-03 -3.22E-03 -3.31E-03
          6.38E-03 -2.61E-03  2.84E-03  8.00E-03  2.25E-03 -1.45E-03  4.00E-03 -1.96E-03  6.37E-03 -2.46E-05  1.69E-03 -3.95E-05
        -5.88E-03  7.10E-03
 
 OM36
+        1.41E-02  2.91E-03 -1.19E-02  1.47E-02 -1.21E-02 -1.48E-02 -1.09E-02  1.29E-02  3.24E-05  1.46E-03 -6.18E-03 -4.14E-03
          2.46E-03  1.65E-03  6.64E-03  4.30E-03 -1.18E-03 -1.11E-03  3.75E-03  2.49E-04  4.89E-03  1.26E-03  1.65E-03  6.81E-03
        -1.70E-03  1.01E-03  1.72E-02
 
 OM37
+       -1.98E-02  1.15E-02 -1.60E-02 -9.12E-03  6.77E-03  1.35E-02 -4.40E-03 -2.05E-02  9.32E-03  8.21E-03 -2.20E-04 -3.10E-03
          4.34E-03  2.70E-03  4.21E-03  1.40E-02  7.07E-03 -5.67E-03  8.36E-03 -2.93E-03  1.17E-02  3.10E-03  1.16E-02 -7.13E-03
        -8.50E-03  4.92E-03  4.08E-03  1.28E-02
 
 OM38
+        9.15E-03 -7.64E-03  1.29E-03  7.77E-03 -6.49E-03 -3.15E-03 -8.35E-04  1.31E-02 -3.74E-04 -8.61E-03  2.48E-03 -6.81E-04
          2.89E-03 -2.86E-03  3.37E-03 -1.56E-03  6.46E-03 -2.80E-03  6.38E-04 -4.73E-05 -3.29E-03 -3.60E-03 -4.76E-03  8.00E-03
         1.86E-03  6.29E-04  1.40E-04 -9.00E-04  3.34E-03
 
 OM44
+       -2.34E-02 -9.95E-03  1.33E-02 -2.34E-02  1.06E-02  3.18E-03  8.31E-03 -1.83E-02  3.37E-03 -2.75E-03  1.07E-02  5.00E-03
         -1.97E-03 -5.73E-04 -6.84E-03 -7.93E-03  6.89E-03 -1.80E-03  3.72E-03 -3.06E-03 -8.83E-03 -3.05E-03  4.05E-03 -6.09E-03
         3.20E-03 -6.40E-03 -9.25E-03 -5.56E-04 -4.96E-04  2.69E-02
 
 OM45
+       -1.98E-02  2.29E-02 -1.09E-02 -1.64E-02  1.62E-02  2.44E-02 -1.55E-03 -3.67E-02  3.98E-03  2.64E-02 -6.95E-03 -3.49E-03
          3.74E-03  5.22E-03 -5.04E-03  1.37E-02 -1.47E-02  3.74E-03  3.62E-03 -1.38E-03  2.05E-02  9.98E-03  1.38E-02 -1.89E-02
        -1.32E-02  7.20E-03  1.12E-04  1.11E-02 -3.95E-03 -8.07E-03  2.58E-02
 
 OM46
+       -1.91E-02 -5.36E-03  1.96E-05 -1.55E-02  8.44E-03  1.24E-02  1.25E-02 -1.14E-03  6.84E-03 -1.65E-02  9.96E-03  5.31E-03
         -8.00E-03  4.23E-03  1.06E-03 -2.73E-03  2.21E-02 -4.47E-03 -3.87E-03  5.84E-03 -2.87E-03 -1.76E-03 -4.52E-04  7.32E-04
         5.51E-03 -4.47E-03 -9.80E-03 -2.65E-03  1.52E-03  7.20E-03 -6.57E-03  2.26E-02
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      OM11      OM12      OM13      OM14  
             OM15      OM16      OM17      OM18      OM22      OM23      OM24      OM25      OM26      OM27      OM28      OM33  
            OM34      OM35      OM36      OM37      OM38      OM44      OM45      OM46      OM47      OM48      OM55      OM56  
             OM57      OM58      OM66      OM67      OM68      OM77      OM78      OM88      SG11      SG12      SG22  
 
 OM47
+       -1.03E-02  8.65E-03 -2.14E-03 -6.21E-03  9.20E-03  9.79E-03  5.68E-03 -7.31E-03  4.58E-03  1.79E-03  7.93E-04  1.65E-03
         -4.66E-03  4.47E-03  9.29E-04  5.03E-03 -2.30E-03  2.79E-04 -4.17E-03  1.79E-03  4.39E-03  4.62E-03  2.55E-03 -7.04E-03
        -4.82E-04 -2.15E-04 -4.59E-03  8.72E-04 -1.34E-03 -2.96E-03  5.13E-03  5.81E-03  8.61E-03
 
 OM48
+       -1.92E-02  2.12E-03 -1.17E-02 -7.27E-03  5.37E-03  9.49E-03  1.83E-03 -1.47E-02  7.47E-03 -8.63E-04  2.18E-03  1.34E-03
          7.73E-04 -9.70E-04  3.75E-03  9.50E-03  1.28E-02 -4.87E-03  5.61E-03 -1.79E-03  3.36E-03  5.95E-04  7.00E-03 -4.42E-03
        -1.99E-03  1.51E-03 -1.09E-03  5.26E-03 -1.06E-03  4.41E-03  2.46E-03  1.59E-03  1.41E-03  8.02E-03
 
 OM55
+       -5.92E-02  3.32E-02 -1.23E-02 -5.67E-02  3.88E-02  3.07E-02  7.75E-03 -9.15E-02  1.60E-02  4.62E-02 -3.68E-03  1.52E-03
         -4.07E-03  1.42E-02 -1.50E-02  1.46E-02 -2.33E-02  1.00E-02  4.14E-03  3.17E-03  2.58E-02  2.23E-02  3.74E-02 -4.39E-02
        -1.47E-02 -9.83E-04  3.49E-04  1.68E-02 -1.15E-02  1.06E-02  2.96E-02 -3.25E-03  6.72E-03  8.00E-03  8.98E-02
 
 OM56
+       -7.33E-03  1.05E-02 -1.03E-02 -5.12E-03  3.17E-03  5.17E-03 -4.55E-03 -1.19E-02  2.71E-03  7.64E-03 -3.56E-03 -1.85E-03
          4.24E-05  3.26E-03  1.69E-03  7.68E-03 -1.02E-03  2.86E-05  2.20E-03  4.08E-04  8.30E-03  4.38E-03  6.26E-03 -4.87E-03
        -5.32E-03  3.05E-03  5.27E-03  5.65E-03 -1.47E-03 -5.13E-03  7.52E-03 -3.85E-03  2.17E-04  1.57E-03  1.01E-02  6.44E-03
 
 OM57
+        1.04E-03 -2.12E-03 -1.30E-02  1.04E-02 -3.31E-03  7.49E-04 -3.09E-03  1.23E-02  6.95E-03 -1.19E-02  2.63E-03 -1.58E-03
          2.23E-03 -1.47E-03  1.20E-02  9.47E-03  1.63E-02 -8.26E-03  2.15E-03  1.24E-03  1.24E-03 -1.53E-03 -1.59E-03  7.75E-03
         9.12E-04  1.90E-03  2.53E-03  2.71E-03  2.61E-03 -4.27E-03 -2.63E-03  3.73E-03  6.78E-04  2.53E-03 -9.67E-03  1.41E-03
          9.64E-03
 
 OM58
+        2.39E-02 -1.26E-02 -8.51E-03  2.93E-02 -1.74E-02 -8.67E-03 -5.09E-03  3.70E-02  2.13E-03 -2.12E-02  2.10E-03 -3.24E-03
          7.91E-03 -7.14E-03  1.49E-02  3.12E-03  1.86E-02 -8.84E-03  1.72E-03  3.67E-03 -5.43E-03 -7.53E-03 -9.40E-03  1.95E-02
         5.27E-03  1.89E-03  4.83E-03 -2.16E-03  5.38E-03 -7.74E-03 -9.26E-03  2.26E-03 -2.12E-03 -9.89E-04 -2.50E-02 -2.03E-03
          9.34E-03  1.91E-02
 
 OM66
+       -2.87E-02  1.33E-02 -7.45E-03 -2.44E-02  1.65E-02  2.49E-02  4.54E-03 -3.04E-02  3.45E-03  1.15E-02 -7.95E-04  2.33E-04
          8.64E-04  3.56E-03 -5.20E-03  9.12E-03  4.40E-03  8.31E-04  5.06E-03 -4.21E-03  1.28E-02  3.11E-03  9.12E-03 -1.20E-02
        -9.40E-03  5.98E-03 -5.11E-03  8.58E-03 -1.76E-03  9.44E-04  1.39E-02  3.17E-03  3.17E-03  4.49E-03  1.22E-02  2.32E-03
         -2.17E-03 -8.79E-03  2.23E-02
 
 OM67
+        1.40E-02 -1.65E-02  5.80E-03  9.57E-03 -1.33E-02 -1.11E-02 -8.16E-04  1.62E-02 -5.75E-03 -9.35E-03  1.60E-03  4.68E-04
          2.84E-03 -5.96E-03 -1.82E-03 -9.67E-03  7.55E-03 -1.09E-03  2.42E-03 -9.89E-04 -9.44E-03 -7.69E-03 -6.15E-03  1.08E-02
         4.28E-03 -1.73E-03  1.74E-03 -4.25E-03  2.30E-03  4.00E-03 -9.45E-03 -1.19E-03 -6.38E-03 -1.48E-03 -1.41E-02 -3.75E-03
         -1.55E-03  3.82E-03 -4.04E-03  1.02E-02
 
 OM68
+        5.47E-02 -1.25E-02  7.48E-03  4.47E-02 -2.90E-02 -2.93E-02 -1.21E-02  5.95E-02 -1.23E-02 -1.10E-02 -7.02E-03 -7.56E-03
          7.37E-03 -4.40E-03  8.12E-03 -8.46E-03 -8.93E-03  5.42E-05 -3.91E-04 -6.45E-04 -5.39E-03 -7.44E-03 -1.78E-02  2.46E-02
         5.46E-03 -8.88E-05  1.46E-02 -7.91E-03  4.48E-03 -1.46E-02 -1.14E-02 -1.08E-02 -6.96E-03 -1.02E-02 -3.38E-02 -2.08E-03
          1.97E-03  1.39E-02 -1.53E-02  7.99E-03  3.78E-02
 
 OM77
+       -7.62E-03  9.71E-03 -4.64E-03 -2.17E-03  4.81E-03  1.11E-03 -7.94E-03 -2.26E-02  1.06E-03  1.84E-02 -6.17E-03 -2.62E-03
          4.04E-03 -1.67E-03 -1.70E-03  7.40E-03 -1.36E-02  3.81E-03  4.78E-03 -3.21E-03  5.45E-03  4.04E-03  8.71E-03 -1.04E-02
        -5.76E-03  2.73E-03  3.81E-03  5.16E-03 -4.00E-03  9.34E-04  7.95E-03 -1.37E-02 -3.93E-03  2.56E-03  1.86E-02  3.97E-03
         -3.51E-03 -5.82E-03  1.64E-03 -1.77E-03 -4.25E-03  1.99E-02
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      OM11      OM12      OM13      OM14  
             OM15      OM16      OM17      OM18      OM22      OM23      OM24      OM25      OM26      OM27      OM28      OM33  
            OM34      OM35      OM36      OM37      OM38      OM44      OM45      OM46      OM47      OM48      OM55      OM56  
             OM57      OM58      OM66      OM67      OM68      OM77      OM78      OM88      SG11      SG12      SG22  
 
 OM78
+        1.40E-02  4.82E-03 -8.44E-03  2.30E-02 -5.11E-03  6.70E-04 -9.13E-03  1.55E-02  2.86E-03  2.26E-03 -5.56E-03 -6.68E-03
          7.72E-03 -2.59E-03  1.26E-02  1.47E-02 -5.09E-03 -2.50E-03  3.56E-03 -2.47E-03  6.81E-03  1.05E-03 -2.77E-03  4.41E-03
        -3.30E-03  5.19E-03  5.58E-03  2.73E-03  1.10E-03 -1.15E-02  4.29E-03 -9.06E-03  1.12E-03 -6.31E-04 -7.31E-03  2.64E-03
          4.70E-03  7.13E-03 -2.20E-03 -4.13E-03  1.02E-02  5.65E-03  1.62E-02
 
 OM88
+       -5.07E-02  3.43E-02 -3.27E-02 -2.46E-02  3.11E-02  4.23E-02 -2.02E-03 -5.81E-02  2.19E-02  2.94E-02 -5.85E-03 -4.89E-03
          4.54E-03  8.13E-03  9.91E-03  4.09E-02 -1.85E-03 -2.28E-03  1.03E-02 -3.20E-03  3.21E-02  1.61E-02  2.61E-02 -2.86E-02
        -1.81E-02  1.04E-02  8.59E-04  2.08E-02 -5.91E-03 -6.90E-03  3.12E-02 -5.28E-03  9.88E-03  1.17E-02  4.65E-02  1.24E-02
          4.30E-03 -9.42E-03  1.96E-02 -1.82E-02 -2.46E-02  1.51E-02  1.33E-02  6.42E-02
 
 SG11
+        9.45E-04 -4.47E-04  1.15E-04  8.98E-04 -5.73E-04 -4.91E-04 -1.74E-04  1.11E-03 -1.68E-04 -3.92E-04 -2.09E-05 -1.00E-04
          1.71E-04 -1.90E-04  1.82E-04 -1.67E-04  7.84E-05 -1.18E-04  1.72E-05 -8.33E-07 -2.59E-04 -2.14E-04 -3.48E-04  5.07E-04
         1.55E-04 -9.36E-06  1.32E-04 -1.56E-04  1.23E-04 -1.66E-04 -2.86E-04 -8.68E-05 -1.06E-04 -1.22E-04 -7.14E-04 -9.88E-05
          1.11E-04  3.38E-04 -3.17E-04  1.65E-04  5.21E-04 -1.38E-04  1.49E-04 -4.99E-04  1.18E-05
 
 SG12
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
        ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 SG22
+        1.19E-04 -5.80E-04  2.34E-04  3.46E-05 -2.14E-04 -1.78E-04  1.90E-04  6.00E-04 -5.21E-05 -7.14E-04  2.90E-04  1.30E-04
         -6.02E-05 -1.61E-04  6.46E-05 -2.80E-04  6.35E-04 -1.77E-04 -3.48E-05 -3.19E-06 -4.47E-04 -2.85E-04 -3.25E-04  4.03E-04
         2.47E-04 -1.02E-04 -2.26E-04 -1.94E-04  1.44E-04  2.77E-04 -4.87E-04  3.33E-04 -5.69E-05  1.28E-05 -7.88E-04 -1.92E-04
          1.04E-04  1.86E-04 -7.87E-05  1.94E-04 -2.45E-05 -3.15E-04 -1.88E-04 -6.31E-04  3.92E-06  0.00E+00  1.84E-05
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 ********************                        CORRELATION MATRIX OF ESTIMATE (S)                      ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      OM11      OM12      OM13      OM14  
             OM15      OM16      OM17      OM18      OM22      OM23      OM24      OM25      OM26      OM27      OM28      OM33  
            OM34      OM35      OM36      OM37      OM38      OM44      OM45      OM46      OM47      OM48      OM55      OM56  
             OM57      OM58      OM66      OM67      OM68      OM77      OM78      OM88      SG11      SG12      SG22  
 
 TH 1
+        3.35E-01
 
 TH 2
+       -4.08E-01  2.21E-01
 
 TH 3
+        3.58E-01 -2.71E-01  2.20E-01
 
 TH 4
+        7.49E-01 -3.61E-01 -4.57E-02  3.04E-01
 
 TH 5
+       -7.27E-01  5.43E-01 -1.79E-02 -7.41E-01  2.10E-01
 
 TH 6
+       -6.39E-01  3.46E-01 -2.83E-01 -4.60E-01  5.67E-01  2.57E-01
 
 TH 7
+       -3.08E-01 -1.15E-01  2.56E-01 -4.83E-01  3.72E-01  2.89E-01  1.48E-01
 
 TH 8
+        7.82E-01 -4.70E-01  1.95E-01  7.90E-01 -7.16E-01 -5.39E-01 -1.94E-01  3.71E-01
 
 OM11
+       -5.68E-01  1.60E-01 -5.87E-01 -2.11E-01  3.75E-01  3.76E-01  3.97E-02 -3.18E-01  1.46E-01
 
 OM12
+       -2.67E-01  6.14E-01  4.64E-02 -4.01E-01  4.67E-01  2.81E-01 -8.90E-02 -6.40E-01 -1.27E-01  2.18E-01
 
 OM13
+       -2.94E-01 -3.79E-01  1.31E-01 -2.44E-01  1.50E-01  9.85E-02  2.93E-01  3.25E-03  4.28E-01 -5.64E-01  1.03E-01
 
 OM14
+       -2.87E-01 -1.60E-01  1.62E-01 -4.59E-01  2.17E-01 -1.38E-01  5.15E-01 -2.70E-01  6.28E-02 -1.78E-01  3.59E-01  8.48E-02
 
 OM15
+        2.76E-01 -2.02E-01 -1.06E-01  3.96E-01 -2.40E-01  1.90E-01 -3.12E-01  2.01E-01 -5.11E-03  2.58E-02 -1.16E-01 -6.89E-01
          1.37E-01
 
 OM16
+       -4.36E-01  5.73E-01 -4.98E-02 -5.08E-01  5.38E-01  1.79E-01  1.28E-01 -3.43E-01  1.86E-01  3.46E-01 -2.42E-02  2.03E-01
         -5.82E-01  1.10E-01
 
 OM17
+        7.32E-02 -8.61E-02 -5.76E-01  4.85E-01 -2.15E-01  1.16E-02 -3.05E-01  4.17E-01  5.72E-01 -4.97E-01  1.80E-01 -3.49E-01
          2.84E-01 -1.57E-01  1.57E-01
 
 OM18
+       -4.18E-01  3.74E-01 -7.54E-01 -6.48E-03  2.81E-01  5.13E-01 -1.72E-01 -2.35E-01  7.10E-01  1.49E-01 -1.05E-01 -3.27E-01
          2.41E-01  8.10E-02  6.68E-01  1.93E-01
 
 OM22
+       -2.68E-01 -3.77E-01 -5.28E-01  2.09E-02 -1.57E-01  2.27E-01  8.12E-02  1.37E-01  4.73E-01 -6.95E-01  5.26E-01  9.18E-02
          1.12E-01 -2.42E-01  5.08E-01  2.95E-01  2.75E-01
 
 OM23
+        1.30E-01  2.24E-01  5.07E-01 -2.78E-01  2.12E-01 -1.56E-01  2.07E-01 -2.74E-01 -5.02E-01  5.42E-01 -4.92E-01  2.46E-01
         -3.30E-01  2.18E-01 -6.66E-01 -4.22E-01 -7.47E-01  1.19E-01
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      OM11      OM12      OM13      OM14  
             OM15      OM16      OM17      OM18      OM22      OM23      OM24      OM25      OM26      OM27      OM28      OM33  
            OM34      OM35      OM36      OM37      OM38      OM44      OM45      OM46      OM47      OM48      OM55      OM56  
             OM57      OM58      OM66      OM67      OM68      OM77      OM78      OM88      SG11      SG12      SG22  
 
 OM24
+       -2.27E-01 -1.40E-01 -4.16E-01  7.92E-02 -7.66E-02  3.49E-01 -2.66E-01 -8.30E-02  2.71E-01  5.49E-02  9.94E-03 -5.26E-01
          6.43E-01 -2.83E-01  2.96E-01  4.61E-01  4.43E-01 -5.50E-01  1.26E-01
 
 OM25
+        7.68E-02  1.62E-02 -6.74E-02 -8.33E-02 -2.63E-02 -7.79E-02  2.33E-01  3.08E-02  6.63E-02 -2.02E-01  5.27E-02  3.25E-01
         -3.63E-01  1.98E-01 -1.90E-02 -1.99E-01  7.67E-02  2.13E-01 -5.38E-01  8.87E-02
 
 OM26
+       -4.09E-01  6.51E-01 -4.99E-01 -2.75E-01  4.74E-01  6.17E-01 -1.05E-01 -4.51E-01  3.73E-01  5.48E-01 -3.74E-01 -4.24E-01
          1.88E-01  4.06E-01  1.37E-01  6.56E-01 -9.27E-02  3.04E-02  3.43E-01 -9.85E-02  1.58E-01
 
 OM27
+       -3.75E-01  7.12E-01 -1.38E-01 -4.48E-01  5.97E-01  2.10E-01  8.52E-02 -5.93E-01  2.16E-01  6.38E-01 -3.36E-01  7.93E-02
         -3.53E-01  5.91E-01 -2.02E-01  2.65E-01 -5.08E-01  4.90E-01 -3.41E-01  2.33E-01  5.73E-01  1.05E-01
 
 OM28
+       -7.26E-01  4.93E-01 -4.89E-01 -5.93E-01  5.57E-01  4.81E-01  4.72E-02 -8.33E-01  5.06E-01  5.77E-01 -7.35E-02  3.49E-02
         -5.74E-02  3.82E-01 -1.32E-01  4.41E-01  4.90E-02 -1.67E-03  3.40E-01  2.15E-02  5.78E-01  5.56E-01  1.43E-01
 
 OM33
+        7.00E-01 -6.35E-01  3.58E-02  6.99E-01 -7.95E-01 -5.04E-01 -2.40E-01  8.85E-01 -2.34E-01 -7.65E-01  1.44E-01 -1.98E-01
          2.90E-01 -5.15E-01  4.39E-01 -2.18E-01  4.11E-01 -4.02E-01  7.90E-02  3.80E-02 -4.96E-01 -7.33E-01 -7.40E-01  1.76E-01
 
 OM34
+        3.45E-01 -5.84E-01  3.54E-01  3.19E-01 -3.66E-01 -5.55E-01  2.45E-01  5.41E-01 -1.51E-01 -6.63E-01  4.08E-01  4.42E-01
         -3.17E-01 -1.85E-01  9.85E-02 -4.95E-01  1.58E-01 -3.64E-02 -4.46E-01  3.30E-01 -7.60E-01 -3.95E-01 -5.87E-01  4.99E-01
         9.97E-02
 
 OM35
+        2.74E-02  2.81E-01 -4.29E-01  1.33E-01 -4.09E-02  4.02E-01 -3.27E-01 -1.22E-01  5.97E-02  2.01E-01 -3.71E-01 -4.63E-01
          5.54E-01 -2.81E-01  2.16E-01  4.92E-01  9.71E-02 -1.45E-01  3.76E-01 -2.63E-01  4.80E-01 -2.78E-03  1.40E-01 -2.67E-03
        -7.00E-01  8.42E-02
 
 OM36
+        3.21E-01  1.01E-01 -4.12E-01  3.68E-01 -4.37E-01 -4.40E-01 -5.64E-01  2.65E-01  1.70E-03  5.11E-02 -4.57E-01 -3.72E-01
          1.37E-01  1.14E-01  3.23E-01  1.70E-01 -3.28E-02 -7.14E-02  2.26E-01  2.14E-02  2.37E-01  9.10E-02  8.83E-02  2.95E-01
        -1.30E-01  9.12E-02  1.31E-01
 
 OM37
+       -5.21E-01  4.62E-01 -6.43E-01 -2.65E-01  2.85E-01  4.63E-01 -2.64E-01 -4.88E-01  5.66E-01  3.33E-01 -1.89E-02 -3.23E-01
          2.80E-01  2.16E-01  2.38E-01  6.43E-01  2.28E-01 -4.23E-01  5.86E-01 -2.92E-01  6.59E-01  2.61E-01  7.17E-01 -3.59E-01
        -7.54E-01  5.16E-01  2.75E-01  1.13E-01
 
 OM38
+        4.72E-01 -5.99E-01  1.01E-01  4.42E-01 -5.34E-01 -2.12E-01 -9.79E-02  6.14E-01 -4.44E-02 -6.84E-01  4.17E-01 -1.39E-01
          3.65E-01 -4.49E-01  3.72E-01 -1.40E-01  4.07E-01 -4.09E-01  8.74E-02 -9.23E-03 -3.61E-01 -5.92E-01 -5.77E-01  7.88E-01
         3.22E-01  1.29E-01  1.85E-02 -1.38E-01  5.78E-02
 
 OM44
+       -4.26E-01 -2.75E-01  3.67E-01 -4.70E-01  3.08E-01  7.55E-02  3.43E-01 -3.01E-01  1.41E-01 -7.69E-02  6.33E-01  3.59E-01
         -8.79E-02 -3.17E-02 -2.66E-01 -2.50E-01  1.53E-01 -9.23E-02  1.79E-01 -2.10E-01 -3.41E-01 -1.77E-01  1.73E-01 -2.11E-01
         1.95E-01 -4.63E-01 -4.30E-01 -3.00E-02 -5.24E-02  1.64E-01
 
 OM45
+       -3.68E-01  6.46E-01 -3.09E-01 -3.37E-01  4.78E-01  5.90E-01 -6.55E-02 -6.17E-01  1.70E-01  7.55E-01 -4.20E-01 -2.57E-01
          1.70E-01  2.94E-01 -2.01E-01  4.42E-01 -3.33E-01  1.96E-01  1.78E-01 -9.70E-02  8.10E-01  5.91E-01  6.04E-01 -6.69E-01
        -8.23E-01  5.33E-01  5.33E-03  6.12E-01 -4.25E-01 -3.06E-01  1.61E-01
 
 OM46
+       -3.80E-01 -1.62E-01  5.92E-04 -3.39E-01  2.67E-01  3.22E-01  5.65E-01 -2.05E-02  3.13E-01 -5.04E-01  6.44E-01  4.17E-01
         -3.89E-01  2.55E-01  4.51E-02 -9.42E-02  5.36E-01 -2.51E-01 -2.04E-01  4.38E-01 -1.21E-01 -1.11E-01 -2.11E-02  2.77E-02
         3.68E-01 -3.53E-01 -4.97E-01 -1.56E-01  1.75E-01  2.92E-01 -2.72E-01  1.50E-01
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      OM11      OM12      OM13      OM14  
             OM15      OM16      OM17      OM18      OM22      OM23      OM24      OM25      OM26      OM27      OM28      OM33  
            OM34      OM35      OM36      OM37      OM38      OM44      OM45      OM46      OM47      OM48      OM55      OM56  
             OM57      OM58      OM66      OM67      OM68      OM77      OM78      OM88      SG11      SG12      SG22  
 
 OM47
+       -3.32E-01  4.22E-01 -1.05E-01 -2.20E-01  4.71E-01  4.11E-01  4.15E-01 -2.13E-01  3.39E-01  8.85E-02  8.30E-02  2.09E-01
         -3.67E-01  4.36E-01  6.40E-02  2.81E-01 -9.04E-02  2.53E-02 -3.56E-01  2.18E-01  3.00E-01  4.73E-01  1.93E-01 -4.32E-01
        -5.21E-02 -2.75E-02 -3.77E-01  8.31E-02 -2.49E-01 -1.95E-01  3.44E-01  4.17E-01  9.28E-02
 
 OM48
+       -6.40E-01  1.07E-01 -5.93E-01 -2.67E-01  2.85E-01  4.13E-01  1.39E-01 -4.41E-01  5.73E-01 -4.42E-02  2.36E-01  1.76E-01
          6.30E-02 -9.82E-02  2.68E-01  5.49E-01  5.23E-01 -4.58E-01  4.96E-01 -2.26E-01  2.38E-01  6.31E-02  5.48E-01 -2.81E-01
        -2.23E-01  2.00E-01 -9.29E-02  5.19E-01 -2.04E-01  3.00E-01  1.71E-01  1.18E-01  1.70E-01  8.96E-02
 
 OM55
+       -5.89E-01  5.02E-01 -1.86E-01 -6.22E-01  6.15E-01  3.99E-01  1.75E-01 -8.24E-01  3.67E-01  7.08E-01 -1.19E-01  5.99E-02
         -9.92E-02  4.28E-01 -3.20E-01  2.53E-01 -2.84E-01  2.82E-01  1.09E-01  1.19E-01  5.46E-01  7.07E-01  8.75E-01 -8.35E-01
        -4.93E-01 -3.89E-02  8.88E-03  4.97E-01 -6.62E-01  2.17E-01  6.16E-01 -7.21E-02  2.42E-01  2.98E-01  3.00E-01
 
 OM56
+       -2.72E-01  5.92E-01 -5.81E-01 -2.10E-01  1.88E-01  2.51E-01 -3.84E-01 -4.01E-01  2.32E-01  4.37E-01 -4.31E-01 -2.71E-01
          3.86E-03  3.68E-01  1.35E-01  4.96E-01 -4.63E-02  3.00E-03  2.17E-01  5.73E-02  6.56E-01  5.19E-01  5.47E-01 -3.46E-01
        -6.65E-01  4.52E-01  5.00E-01  6.23E-01 -3.17E-01 -3.89E-01  5.84E-01 -3.19E-01  2.91E-02  2.18E-01  4.18E-01  8.02E-02
 
 OM57
+        3.17E-02 -9.78E-02 -6.02E-01  3.48E-01 -1.60E-01  2.97E-02 -2.13E-01  3.39E-01  4.86E-01 -5.56E-01  2.60E-01 -1.89E-01
          1.66E-01 -1.35E-01  7.82E-01  5.00E-01  6.06E-01 -7.08E-01  1.73E-01  1.42E-01  7.98E-02 -1.49E-01 -1.13E-01  4.49E-01
         9.31E-02  2.30E-01  1.96E-01  2.44E-01  4.59E-01 -2.65E-01 -1.67E-01  2.53E-01  7.44E-02  2.87E-01 -3.29E-01  1.79E-01
          9.82E-02
 
 OM58
+        5.15E-01 -4.13E-01 -2.79E-01  6.96E-01 -5.98E-01 -2.44E-01 -2.49E-01  7.22E-01  1.06E-01 -7.04E-01  1.48E-01 -2.76E-01
          4.18E-01 -4.68E-01  6.86E-01  1.17E-01  4.89E-01 -5.38E-01  9.83E-02  2.99E-01 -2.49E-01 -5.18E-01 -4.76E-01  8.03E-01
         3.82E-01  1.62E-01  2.66E-01 -1.38E-01  6.73E-01 -3.41E-01 -4.17E-01  1.09E-01 -1.65E-01 -7.98E-02 -6.02E-01 -1.83E-01
          6.88E-01  1.38E-01
 
 OM66
+       -5.73E-01  4.04E-01 -2.26E-01 -5.36E-01  5.24E-01  6.49E-01  2.06E-01 -5.48E-01  1.59E-01  3.54E-01 -5.16E-02  1.84E-02
          4.22E-02  2.16E-01 -2.22E-01  3.16E-01  1.07E-01  4.68E-02  2.68E-01 -3.18E-01  5.45E-01  1.98E-01  4.28E-01 -4.58E-01
        -6.31E-01  4.75E-01 -2.61E-01  5.08E-01 -2.03E-01  3.85E-02  5.80E-01  1.41E-01  2.29E-01  3.35E-01  2.72E-01  1.94E-01
         -1.48E-01 -4.25E-01  1.49E-01
 
 OM67
+        4.12E-01 -7.37E-01  2.60E-01  3.11E-01 -6.25E-01 -4.26E-01 -5.46E-02  4.33E-01 -3.91E-01 -4.24E-01  1.53E-01  5.45E-02
          2.05E-01 -5.34E-01 -1.15E-01 -4.95E-01  2.72E-01 -9.10E-02  1.89E-01 -1.10E-01 -5.92E-01 -7.23E-01 -4.26E-01  6.06E-01
         4.24E-01 -2.03E-01  1.31E-01 -3.72E-01  3.94E-01  2.41E-01 -5.81E-01 -7.83E-02 -6.79E-01 -1.63E-01 -4.66E-01 -4.62E-01
         -1.56E-01  2.73E-01 -2.67E-01  1.01E-01
 
 OM68
+        8.40E-01 -2.91E-01  1.74E-01  7.56E-01 -7.09E-01 -5.87E-01 -4.22E-01  8.25E-01 -4.34E-01 -2.60E-01 -3.51E-01 -4.58E-01
          2.77E-01 -2.05E-01  2.67E-01 -2.25E-01 -1.67E-01  2.35E-03 -1.59E-02 -3.74E-02 -1.76E-01 -3.64E-01 -6.43E-01  7.19E-01
         2.82E-01 -5.42E-03  5.73E-01 -3.60E-01  3.98E-01 -4.58E-01 -3.65E-01 -3.71E-01 -3.86E-01 -5.85E-01 -5.80E-01 -1.33E-01
          1.03E-01  5.18E-01 -5.25E-01  4.06E-01  1.94E-01
 
 OM77
+       -1.61E-01  3.11E-01 -1.49E-01 -5.06E-02  1.62E-01  3.06E-02 -3.81E-01 -4.31E-01  5.14E-02  5.99E-01 -4.24E-01 -2.18E-01
          2.09E-01 -1.07E-01 -7.67E-02  2.72E-01 -3.52E-01  2.27E-01  2.68E-01 -2.57E-01  2.45E-01  2.72E-01  4.32E-01 -4.20E-01
        -4.09E-01  2.30E-01  2.06E-01  3.23E-01 -4.91E-01  4.03E-02  3.50E-01 -6.45E-01 -3.00E-01  2.02E-01  4.39E-01  3.50E-01
         -2.53E-01 -2.98E-01  7.78E-02 -1.24E-01 -1.55E-01  1.41E-01
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      OM11      OM12      OM13      OM14  
             OM15      OM16      OM17      OM18      OM22      OM23      OM24      OM25      OM26      OM27      OM28      OM33  
            OM34      OM35      OM36      OM37      OM38      OM44      OM45      OM46      OM47      OM48      OM55      OM56  
             OM57      OM58      OM66      OM67      OM68      OM77      OM78      OM88      SG11      SG12      SG22  
 
 OM78
+        3.28E-01  1.72E-01 -3.01E-01  5.94E-01 -1.91E-01  2.05E-02 -4.87E-01  3.28E-01  1.55E-01  8.15E-02 -4.24E-01 -6.19E-01
          4.43E-01 -1.85E-01  6.33E-01  5.97E-01 -1.46E-01 -1.65E-01  2.22E-01 -2.19E-01  3.40E-01  7.87E-02 -1.53E-01  1.97E-01
        -2.60E-01  4.84E-01  3.34E-01  1.90E-01  1.50E-01 -5.53E-01  2.10E-01 -4.74E-01  9.47E-02 -5.54E-02 -1.92E-01  2.58E-01
          3.77E-01  4.05E-01 -1.16E-01 -3.21E-01  4.13E-01  3.15E-01  1.27E-01
 
 OM88
+       -5.97E-01  6.13E-01 -5.86E-01 -3.19E-01  5.83E-01  6.50E-01 -5.41E-02 -6.18E-01  5.94E-01  5.33E-01 -2.24E-01 -2.27E-01
          1.31E-01  2.91E-01  2.50E-01  8.36E-01 -2.65E-02 -7.58E-02  3.23E-01 -1.42E-01  8.04E-01  6.04E-01  7.22E-01 -6.42E-01
        -7.18E-01  4.87E-01  2.58E-02  7.26E-01 -4.04E-01 -1.66E-01  7.67E-01 -1.39E-01  4.20E-01  5.16E-01  6.13E-01  6.12E-01
          1.73E-01 -2.69E-01  5.17E-01 -7.10E-01 -4.99E-01  4.22E-01  4.13E-01  2.53E-01
 
 SG11
+        8.20E-01 -5.89E-01  1.52E-01  8.60E-01 -7.94E-01 -5.57E-01 -3.44E-01  8.70E-01 -3.37E-01 -5.24E-01 -5.91E-02 -3.44E-01
          3.64E-01 -5.01E-01  3.38E-01 -2.51E-01  8.31E-02 -2.90E-01  3.96E-02 -2.74E-03 -4.78E-01 -5.92E-01 -7.10E-01  8.40E-01
         4.51E-01 -3.24E-02  2.93E-01 -4.02E-01  6.18E-01 -2.95E-01 -5.19E-01 -1.68E-01 -3.33E-01 -3.97E-01 -6.94E-01 -3.59E-01
          3.28E-01  7.11E-01 -6.18E-01  4.74E-01  7.80E-01 -2.84E-01  3.42E-01 -5.73E-01  3.43E-03
 
 SG12
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
        ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 SG22
+        8.25E-02 -6.11E-01  2.48E-01  2.65E-02 -2.37E-01 -1.62E-01  2.99E-01  3.77E-01 -8.33E-02 -7.64E-01  6.56E-01  3.58E-01
         -1.02E-01 -3.40E-01  9.60E-02 -3.38E-01  5.39E-01 -3.48E-01 -6.42E-02 -8.38E-03 -6.60E-01 -6.31E-01 -5.31E-01  5.35E-01
         5.76E-01 -2.82E-01 -4.02E-01 -3.99E-01  5.80E-01  3.93E-01 -7.06E-01  5.17E-01 -1.43E-01  3.34E-02 -6.13E-01 -5.57E-01
          2.46E-01  3.13E-01 -1.23E-01  4.48E-01 -2.94E-02 -5.19E-01 -3.44E-01 -5.80E-01  2.66E-01  0.00E+00  4.29E-03
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                          ITERATIVE TWO STAGE (NO PRIOR)                        ********************
 ********************                    INVERSE COVARIANCE MATRIX OF ESTIMATE (S)                   ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      OM11      OM12      OM13      OM14  
             OM15      OM16      OM17      OM18      OM22      OM23      OM24      OM25      OM26      OM27      OM28      OM33  
            OM34      OM35      OM36      OM37      OM38      OM44      OM45      OM46      OM47      OM48      OM55      OM56  
             OM57      OM58      OM66      OM67      OM68      OM77      OM78      OM88      SG11      SG12      SG22  
 
 TH 1
+        3.20E+02
 
 TH 2
+        1.68E+02  4.65E+02
 
 TH 3
+       -6.38E+01  1.63E+01  5.69E+02
 
 TH 4
+       -3.21E+01 -1.37E+01  6.43E+01  2.96E+02
 
 TH 5
+       -8.29E+01 -1.31E+02 -4.41E+00  2.34E+01  3.77E+02
 
 TH 6
+       -1.89E+01 -7.44E+01 -8.26E+01 -5.77E+01  1.23E+02  2.99E+02
 
 TH 7
+        5.46E+01  1.56E+02 -2.10E+01  1.17E+02 -8.80E+01 -8.39E+01  3.77E+02
 
 TH 8
+       -2.05E+02 -2.92E+02 -9.88E+01 -1.02E+02  1.25E+02  1.50E+02 -2.25E+02  5.72E+02
 
 OM11
+        8.57E+01  3.37E+01 -1.06E+02  1.43E+01  2.30E+01  1.21E+02  1.14E+02 -4.24E+01  1.03E+03
 
 OM12
+        8.12E+01  4.84E+01  5.34E+01  4.46E+02  1.06E+02  5.27E+01  3.65E+02 -3.39E+02  8.93E+02  3.60E+03
 
 OM13
+       -2.39E+02 -3.09E+01 -2.95E+02  5.89E+01 -1.57E+02  7.87E+00  1.97E+02  2.12E+02 -9.69E+02 -8.59E+02  3.76E+03
 
 OM14
+        3.78E+01  4.95E+02  1.16E+02  9.52E+01 -9.92E+01  8.04E+01  1.18E+02 -1.53E+02 -3.45E+02  3.63E+02  4.05E+02  2.75E+03
 
 OM15
+        8.06E+01  1.71E+02 -1.92E+02 -9.38E+01 -4.50E+02 -3.86E+02 -1.48E+01 -1.22E+02 -7.19E+02 -1.80E+03  1.12E+03 -1.88E+02
          2.86E+03
 
 OM16
+        2.51E+02  6.44E+01 -3.35E+01  8.97E+01 -3.58E+02 -3.31E+02  1.30E+02 -2.54E+02  2.24E+02 -3.89E+02 -6.94E+02 -8.69E+02
          8.82E+02  1.95E+03
 
 OM17
+        1.69E+02  1.98E+02  2.02E+02  1.02E+02  1.86E+01  1.72E+02  1.11E+02 -2.19E+02  5.31E+02  1.98E+03 -1.55E+03  1.08E+03
         -1.64E+03 -3.84E+02  2.92E+03
 
 OM18
+       -7.62E+01 -2.81E+02  1.44E+02 -1.11E+02 -6.29E+01 -2.49E+02 -3.29E+02  2.38E+02 -1.14E+03 -2.93E+03  1.05E+03 -9.07E+02
          2.03E+03  6.06E+02 -2.44E+03  4.15E+03
 
 OM22
+        9.76E+01  1.83E+02  2.31E+01  2.75E+02 -4.43E+01 -3.74E+01  2.86E+02 -3.39E+02  3.64E+02  2.18E+03 -2.33E+02  1.57E+02
         -8.10E+02 -2.02E+02  1.00E+03 -1.78E+03  2.05E+03
 
 OM23
+        2.34E+01 -1.29E+02 -2.10E+02 -1.19E+02 -1.81E+02  1.91E+02 -2.26E+02  2.77E+02 -5.76E+02 -1.15E+03  1.83E+03 -4.28E+02
          9.07E+02 -6.47E+01 -8.70E+02  1.61E+03 -3.48E+02  3.89E+03
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      OM11      OM12      OM13      OM14  
             OM15      OM16      OM17      OM18      OM22      OM23      OM24      OM25      OM26      OM27      OM28      OM33  
            OM34      OM35      OM36      OM37      OM38      OM44      OM45      OM46      OM47      OM48      OM55      OM56  
             OM57      OM58      OM66      OM67      OM68      OM77      OM78      OM88      SG11      SG12      SG22  
 
 OM24
+        5.01E+02  6.78E+02 -1.27E+02 -6.82E+01 -8.68E+01  1.27E+02  1.87E+02 -4.98E+02  2.23E+02  3.50E+02 -4.32E+02  1.98E+03
         -8.56E+02 -2.98E+02  1.51E+03 -1.13E+03 -1.64E+02 -3.51E+02  4.67E+03
 
 OM25
+        1.13E+02  8.61E+01 -2.17E+02 -5.35E+01 -2.35E+02 -4.63E+02  1.41E+01 -2.31E+02 -6.49E+02 -1.85E+03  6.32E+02 -8.61E+02
          2.96E+03  1.05E+03 -1.96E+03  2.33E+03 -1.13E+03 -2.43E+02 -6.41E+02  6.31E+03
 
 OM26
+        1.39E+01 -3.99E+01  1.24E+02  1.38E+02 -3.86E+02 -4.69E+02  2.45E+02 -5.83E+01 -2.69E+01 -3.87E+02 -3.00E+02 -3.04E+02
          9.97E+02  1.32E+03 -6.11E+02  7.41E+02 -5.21E+02 -6.93E+02 -1.15E+03  1.95E+03  3.71E+03
 
 OM27
+        2.48E+02  1.61E+02 -2.11E+02  1.59E+02  1.11E+02  3.50E+02  2.33E+02 -1.69E+02  6.27E+02  2.02E+03 -6.35E+02  1.49E+03
         -1.90E+03 -6.34E+02  2.15E+03 -2.38E+03  1.15E+03 -1.01E+03  2.91E+03 -2.48E+03 -1.52E+03  4.55E+03
 
 OM28
+       -3.84E+02 -4.81E+02  1.53E+02 -4.06E+02 -1.29E+02 -7.60E+01 -4.18E+02  9.22E+02 -9.51E+02 -4.37E+03  1.10E+03 -1.06E+03
          2.39E+03  8.55E+02 -2.63E+03  4.26E+03 -3.24E+03  1.53E+03 -2.24E+03  2.67E+03  2.37E+03 -3.61E+03  8.64E+03
 
 OM33
+       -1.20E+02 -6.15E+01  6.05E+01  1.57E+02 -3.96E+00  1.12E+01 -3.37E+00 -1.37E+02  4.00E+01  6.69E+00  2.04E+02  5.83E+01
          7.99E+01  4.72E+02  1.78E+02 -4.91E+02 -3.23E+02 -1.81E+02  2.28E+02 -1.42E+02  1.54E+02  4.18E+02  1.49E+02  3.02E+03
 
 OM34
+        1.27E+02 -1.14E+02  2.24E+01 -2.88E+02  8.21E+01  1.22E+02 -2.06E+02  1.72E+02  3.21E+01 -4.97E+02 -2.19E+02 -8.86E+02
          1.56E+02  2.62E+02 -3.84E+02  1.77E+02 -3.56E+02  2.21E+02 -2.01E+02  9.57E+02  1.96E+02 -3.53E+02  8.84E+02  7.01E+02
         3.17E+03
 
 OM35
+       -1.87E+02 -1.42E+02  9.59E+01  1.07E+02  2.63E+02  6.23E+01  1.08E+02  3.04E+02  6.55E+02  9.56E+02 -9.82E+02  3.69E+01
         -1.32E+03  3.79E+02  8.08E+02 -1.37E+03 -7.71E+01 -2.06E+03  7.57E+02 -6.03E+02  7.53E+02  1.30E+03 -2.76E+02  9.77E+02
         1.04E+03  4.42E+03
 
 OM36
+       -3.81E+01  1.70E+02  1.24E+02  7.97E+01  1.12E+02  2.09E+01  1.68E+02 -1.19E+02 -3.70E+01  1.54E+02  3.32E+02  1.27E+02
          2.30E+02 -1.95E+02 -3.69E+02 -5.73E+00  2.40E+01 -7.31E+02  7.17E+01  4.99E+02  7.91E+01 -2.81E+02 -1.39E+02 -8.30E+01
        -2.63E+02  1.15E+03  1.96E+03
 
 OM37
+        1.90E+02 -2.81E+02 -3.95E+01 -1.77E+02  1.30E+02  1.92E+02 -2.66E+02  7.92E+01 -7.42E+02 -7.19E+02  1.02E+03 -3.56E+02
          7.62E+02 -4.57E+02 -6.85E+02  1.26E+03 -3.11E+02  2.55E+03 -2.49E+02  1.21E+03 -5.62E+02 -5.56E+02  4.84E+02 -1.54E+02
         1.43E+03 -1.92E+03 -9.26E+02  4.40E+03
 
 OM38
+        1.61E+02  2.76E+02 -1.45E+02  1.45E+02  3.36E+02 -1.43E+02  5.73E+01 -1.70E+02  9.50E+02  1.56E+03 -3.71E+03 -1.44E+02
         -1.39E+03  3.88E+02  1.38E+03 -8.82E+02  8.14E+02 -3.05E+03  6.75E+02 -1.82E+02  4.50E+02  7.55E+02 -2.06E+03 -2.50E+03
        -1.79E+03  8.69E+02  4.14E+02 -3.14E+03  9.74E+03
 
 OM44
+        7.09E+01 -1.29E+01 -1.98E+02  1.45E+02 -1.14E+02 -5.85E+01  1.23E+02 -1.15E+02  2.26E+02  5.22E+02 -3.47E+02 -3.22E+02
          1.69E+01  3.86E+02  9.54E+01 -2.75E+02  4.96E+02 -1.50E+02 -3.88E+02  2.33E+02  5.45E+02  9.08E+01 -3.61E+02  1.57E+02
         1.50E+02  3.15E+02 -1.13E+01 -3.38E+02  3.62E+02  7.84E+02
 
 OM45
+       -1.05E+02 -8.03E+01  1.02E+02 -1.86E+02  1.22E+02  1.40E+02 -2.29E+02  2.67E+02 -1.50E+02 -9.23E+02  9.66E+01 -4.31E+02
          8.88E+00 -7.13E+01 -1.55E+02  6.38E+02 -4.18E+02  8.12E+02 -2.87E+02 -6.46E+02 -1.07E+03 -2.51E+02  1.14E+03  3.31E+02
         5.58E+02  7.58E+01  8.15E+01  1.77E+02 -1.08E+03 -1.56E+02  1.61E+03
 
 OM46
+        6.47E+01  1.22E+02  1.55E+02 -9.90E+01  1.57E+02  2.62E+01 -1.41E+02  7.32E+00 -2.55E+02 -1.81E+02  1.47E+02  5.79E+02
         -1.65E+02 -6.06E+02  2.05E+02  7.13E+01 -3.77E+02  1.16E+02  6.47E+02 -1.21E+03 -1.28E+03  4.08E+02 -3.53E+02  3.45E+01
        -2.89E+02  5.62E+01  4.58E+02  1.69E+02 -4.42E+02 -5.99E+02  6.37E+02  1.79E+03
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      OM11      OM12      OM13      OM14  
             OM15      OM16      OM17      OM18      OM22      OM23      OM24      OM25      OM26      OM27      OM28      OM33  
            OM34      OM35      OM36      OM37      OM38      OM44      OM45      OM46      OM47      OM48      OM55      OM56  
             OM57      OM58      OM66      OM67      OM68      OM77      OM78      OM88      SG11      SG12      SG22  
 
 OM47
+        1.35E+02  2.16E+02 -2.01E+02  2.47E+02 -2.51E+02 -1.64E+02  2.26E+02 -4.66E+02  1.58E+02  1.06E+03 -2.75E+02  5.62E+02
          9.73E+01  2.62E+02  6.15E+02 -6.27E+02  8.27E+02 -3.24E+02  1.10E+03  1.88E+02  5.09E+02  8.89E+02 -1.66E+03  2.48E+02
        -8.69E+02  1.64E+02  2.69E+02 -9.11E+02  1.35E+03  7.17E+02 -8.84E+02 -7.22E+02  2.68E+03
 
 OM48
+       -1.74E+02 -5.14E+02  2.14E+02 -2.01E+02  2.83E+02  5.00E+00 -4.34E+02  3.78E+02 -9.83E+01 -7.85E+02 -1.35E+02 -1.83E+03
          3.77E+02  2.12E+02 -1.06E+03  1.32E+03 -6.38E+02  7.53E+02 -2.39E+03  7.48E+02 -2.92E+02 -2.12E+03  1.59E+03 -6.25E+02
         2.92E+02 -1.10E+03 -3.98E+02  1.12E+03  2.27E+01 -4.85E+02  6.26E+02  3.42E+02 -1.79E+03  3.50E+03
 
 OM55
+       -2.10E+02 -1.32E+02  1.42E+02  5.96E+01  2.37E+02  1.49E+02 -1.72E+02  1.77E+02  1.36E+02  8.17E+02 -4.16E+02  1.09E+02
         -1.20E+03 -6.02E+02  7.07E+02 -1.00E+03  6.25E+02 -3.75E+02 -2.81E+02 -1.78E+03 -1.12E+03  5.00E+02 -1.56E+03 -8.56E+00
        -1.90E+02  1.79E+02 -3.39E+02 -4.77E+02  5.08E+02 -1.39E+02  2.13E+02  2.68E+02 -2.99E+02  2.85E+02  1.24E+03
 
 OM56
+       -3.38E+02 -4.43E+02  9.83E+01  1.51E+02  2.38E+02  1.95E+02 -7.78E+01  3.40E+02  9.07E+01  7.36E+02  6.12E+02  8.51E+00
         -1.26E+03 -1.18E+03  3.55E+02 -5.15E+02  5.46E+02  7.71E+02 -1.06E+03 -2.71E+03 -1.34E+03  3.58E+02 -8.81E+02 -2.23E+02
        -1.69E+02 -1.34E+03 -1.51E+03  3.52E+02 -9.99E+02 -4.71E+01  1.04E+02  1.63E+02 -4.26E+02  6.22E+02  1.35E+03  4.46E+03
 
 OM57
+        1.37E+01  1.63E+02  1.11E+02 -2.25E+02 -3.72E+02 -1.16E+02 -1.19E+02 -1.81E+01 -5.78E+02 -1.75E+03  7.01E+02 -1.47E+02
          1.70E+03  3.66E+02 -1.14E+03  1.70E+03 -9.44E+02  1.38E+03 -1.83E+02  2.00E+03  4.49E+02 -2.02E+03  2.38E+03 -4.34E+02
         2.57E+02 -1.14E+03  1.18E+02  7.87E+02 -1.08E+03 -1.58E+02  3.05E+02 -2.41E+02  3.09E+01  4.06E+02 -7.51E+02 -9.81E+02
          2.77E+03
 
 OM58
+       -7.52E+01 -2.70E+02  3.49E+02  2.48E+02  3.14E+02  3.59E+02  7.30E+01  2.74E-01  8.27E+02  2.53E+03 -1.06E+03  5.92E+02
         -3.58E+03 -9.60E+02  2.04E+03 -2.28E+03  1.32E+03 -1.45E+02  1.03E+03 -5.67E+03 -1.50E+03  2.83E+03 -3.56E+03 -4.24E+02
        -1.30E+03  3.21E+02 -9.22E+02 -8.70E+02  1.10E+03 -6.85E+01 -1.96E+02  4.99E+02  2.64E+02 -6.23E+02  1.71E+03  3.06E+03
         -2.47E+03  6.94E+03
 
 OM66
+       -1.38E+02 -2.16E+02  8.85E+00  2.54E+01  4.67E+01  9.81E+01 -9.63E+01  1.25E+02  2.09E+01  2.24E+02  1.74E+02 -1.91E+02
         -3.72E+02 -5.10E+02  1.50E+02 -1.59E+02  3.17E+02  1.13E+02 -5.87E+02 -3.69E+02 -7.00E+02  1.22E+01 -4.33E+02 -3.56E+02
         2.62E+01 -8.78E+02 -7.20E+02  1.17E+02 -9.87E+01  4.26E+01 -4.84E+01 -3.30E+02 -1.58E+02  4.39E+02  6.29E+02  1.67E+03
         -1.29E+02  7.38E+02  1.19E+03
 
 OM67
+        1.51E+02  3.64E+02  1.75E+02 -1.51E+02 -7.90E+01 -2.16E+02  5.40E+01 -1.90E+02  4.02E+01 -4.79E+02 -5.66E+02  1.48E+02
          3.39E+02  5.51E+02 -2.65E+01  7.48E+01 -5.48E+02 -5.38E+02  2.69E+02  5.26E+02  8.26E+02 -5.05E+02  7.49E+02 -7.22E+01
         2.35E+02  3.34E+02  5.73E+01 -2.65E+02  3.41E+02 -1.83E+02 -1.05E+02  2.49E+02 -1.88E+02  6.59E+01 -3.23E+02 -7.14E+02
          5.16E+02 -6.80E+02 -5.27E+02  1.68E+03
 
 OM68
+       -2.04E+02 -7.74E+01 -1.19E+02  3.06E+00  2.65E+02  2.80E+02 -1.25E+02  2.91E+01 -1.06E+02  6.20E+02  6.66E+02  3.51E+02
         -7.50E+02 -1.43E+03  2.35E+02 -6.77E+02  8.39E+02  3.59E+02 -1.17E+02 -1.25E+03 -2.33E+03  7.49E+02 -1.89E+03 -6.06E+02
        -5.21E+02 -1.16E+03 -4.64E+02  4.21E+02 -1.95E+02 -1.93E+02  3.11E+02  4.31E+02 -1.27E+02  3.44E+02  1.02E+03  1.82E+03
         -4.39E+02  1.30E+03  1.04E+03 -9.61E+02  2.65E+03
 
 OM77
+        5.01E+01  7.26E+01 -1.28E+02  1.07E+02 -6.33E+01  2.48E+01  1.44E+02 -5.41E+01  1.55E+02  5.55E+02 -1.49E+02  4.75E+02
         -3.30E+02  3.37E+01  5.52E+02 -5.15E+02  4.10E+02 -3.98E+02  6.98E+02 -6.94E+02 -8.48E+01  1.07E+03 -9.34E+02  1.06E+02
        -6.74E+02  4.12E+02  1.56E+02 -8.67E+02  1.03E+03  1.86E+02 -1.89E+02  4.01E+00  9.78E+02 -9.49E+02  4.03E+01 -9.27E+01
         -4.94E+02  6.57E+02 -9.84E+01 -3.22E+02  1.21E+02  9.11E+02
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      OM11      OM12      OM13      OM14  
             OM15      OM16      OM17      OM18      OM22      OM23      OM24      OM25      OM26      OM27      OM28      OM33  
            OM34      OM35      OM36      OM37      OM38      OM44      OM45      OM46      OM47      OM48      OM55      OM56  
             OM57      OM58      OM66      OM67      OM68      OM77      OM78      OM88      SG11      SG12      SG22  
 
 OM78
+       -2.49E+02 -1.12E+02  4.94E+01 -4.19E+02  2.28E+01 -1.91E+02 -1.43E+02  2.85E+02 -6.51E+02 -2.23E+03  1.14E+03 -1.20E+03
          1.80E+03  2.66E+02 -2.28E+03  1.95E+03 -1.20E+03  7.50E+02 -2.49E+03  2.51E+03  8.10E+02 -3.27E+03  3.67E+03 -5.09E+02
         1.25E+03 -9.50E+02  1.20E+02  1.31E+03 -1.97E+03 -3.85E+02  6.99E+02  4.08E+01 -2.11E+03  2.33E+03 -4.96E+02 -3.32E+02
          1.73E+03 -3.21E+03  3.60E+01  9.94E+02 -5.45E+02 -1.58E+03  5.14E+03
 
 OM88
+        1.56E+02  4.22E+02 -5.40E+01  1.51E+02 -7.22E+01  1.28E+01  2.36E+02 -4.20E+02  3.63E+02  1.80E+03  6.72E+01  9.87E+02
         -9.03E+02 -5.01E+02  1.12E+03 -2.58E+03  1.34E+03 -1.10E+03  1.13E+03 -1.34E+03 -9.25E+02  1.64E+03 -3.63E+03  8.61E+02
        -2.12E+02  5.94E+02  3.37E+02 -7.81E+02 -4.58E+02  9.70E+01 -5.60E+02  3.26E+02  6.70E+02 -1.45E+03  6.03E+02  1.01E+02
         -1.24E+03  1.19E+03 -7.66E+01 -1.28E+02  8.38E+02  4.82E+02 -1.90E+03  2.97E+03
 
 SG11
+       -1.79E+03  6.68E+03 -2.08E+03 -1.15E+03 -2.84E+03  1.81E+03  9.35E+03 -2.67E+03  5.60E+03  1.80E+04  1.81E+04  6.97E+03
         -8.55E+03 -5.23E+03  9.75E+03 -1.93E+04  2.35E+04  1.29E+04 -2.01E+04 -1.54E+04  1.96E+04  4.70E+03 -8.29E+03 -1.03E+04
        -3.41E+02 -9.49E+03 -8.87E+03 -2.86E+02 -1.50E+04  1.07E+04 -9.85E+03 -2.11E+04  6.76E+03 -1.34E+04  9.60E+01  2.52E+04
         -1.58E+04  1.67E+04  1.16E+04 -2.90E+03  5.04E+03  6.27E+03 -4.26E+03  8.88E+03  2.40E+06
 
 SG12
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
        ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 SG22
+       -2.70E+03 -3.31E+03  2.75E+03  4.41E+03  2.11E+03  8.46E+02 -1.46E+03  4.04E+03  6.49E+03  6.64E+03 -6.79E+03 -8.94E+03
         -6.35E+03  5.49E+03  7.29E+02 -2.49E+03 -2.80E+01 -2.92E+03 -1.97E+04 -6.29E+03  1.01E+04 -5.97E+03  1.41E+04  2.46E+03
         2.32E+03  1.65E+04  4.29E+03 -8.49E+03 -5.44E+03  3.09E+03  6.37E+03 -2.97E+03 -6.90E+03  7.37E+03  3.93E+03  8.87E+02
         -9.80E+03  7.30E+03 -2.20E+03  2.39E+02 -1.89E+03 -9.83E+02 -1.32E+03 -2.81E+02  1.80E+05  0.00E+00  8.37E+05
 
1
 
 
 #TBLN:      2
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
 NO. OF FUNCT. EVALS. ALLOWED:            3480
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
 SIGDIGITS FOR MAP ESTIMATION (SIGLO):      4
 GRADIENT SIGDIGITS OF
       FIXED EFFECTS PARAMETERS (SIGL):     4
 NOPRIOR SETTING (NOPRIOR):                 0
 NOCOV SETTING (NOCOV):                     OFF
 DERCONT SETTING (DERCONT):                 OFF
 FINAL ETA RE-EVALUATION (FNLETA):          1
 EXCLUDE NON-INFLUENTIAL (NON-INFL.) ETAS
       IN SHRINKAGE (ETASTYPE):             NO
 NON-INFL. ETA CORRECTION (NONINFETA):      0
 RAW OUTPUT FILE (FILE): example6.txt
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
 CONVERGENCE INTERVAL (CINTERVAL):          10
 CONVERGENCE ITERATIONS (CITER):            10
 CONVERGENCE ALPHA ERROR (CALPHA):          5.000000000000000E-02
 BURN-IN ITERATIONS (NBURN):                4000
 FIRST ITERATION FOR MAP (MAPITERS):          NO
 ITERATIONS (NITER):                        10000
 ANNEAL SETTING (CONSTRAIN):                 1
 STARTING SEED FOR MC METHODS (SEED):       11456
 MC SAMPLES PER SUBJECT (ISAMPLE):          1
 RANDOM SAMPLING METHOD (RANMETHOD):        3U
 PROPOSAL DENSITY SCALING RANGE
              (ISCALE_MIN, ISCALE_MAX):     1.000000000000000E-06   ,1000000.00000000
 SAMPLE ACCEPTANCE RATE (IACCEPT):          0.400000000000000
 METROPOLIS HASTINGS SAMPLING FOR INDIVIDUAL ETAS:
 SAMPLES FOR GLOBAL SEARCH KERNEL (ISAMPLE_M1):          1
 SAMPLES FOR NEIGHBOR SEARCH KERNEL (ISAMPLE_M1A):       0
 SAMPLES FOR MASS/IMP/POST. MATRIX SEARCH (ISAMPLE_M1B): 2
 SAMPLES FOR LOCAL SEARCH KERNEL (ISAMPLE_M2):           1
 SAMPLES FOR LOCAL UNIVARIATE KERNEL (ISAMPLE_M3):       1
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
 SAMPLES FOR LOCAL SEARCH KERNEL (OSAMPLE_M2):           36
 SAMPLES FOR LOCAL UNIVARIATE SEARCH KERNEL (OSAMPLE_M3):36
 USER DEFINED PRIOR SETTING FOR THETAS: (TPU):        0.00000000000000
 WEIGHT FACTOR FOR STD PRIOR FOR SIGMAS (SVARF): -1.000000000000000+300

 TOLERANCES FOR ESTIMATION/EVALUATION STEP:
 NRD (RELATIVE) VALUE(S) OF TOLERANCE:   4
 ANRD (ABSOLUTE) VALUE(S) OF TOLERANCE:  12
 TOLERANCES FOR COVARIANCE STEP:
 NRD (RELATIVE) VALUE(S) OF TOLERANCE:   4
 ANRD (ABSOLUTE) VALUE(S) OF TOLERANCE:  12
 
 THE FOLLOWING LABELS ARE EQUIVALENT
 PRED=PREDI
 RES=RESI
 WRES=WRESI
 IWRS=IWRESI
 IPRD=IPREDI
 IRS=IRESI
 
 EM/BAYES SETUP:
 THETAS THAT ARE MU MODELED:
   1   2   3   4   5   6   7   8
 THETAS THAT ARE GIBBS SAMPLED:
   1   2   3   4   5   6   7   8
 THETAS THAT ARE METROPOLIS-HASTINGS SAMPLED:
 
 SIGMAS THAT ARE GIBBS SAMPLED:
   1   2
 SIGMAS THAT ARE METROPOLIS-HASTINGS SAMPLED:
 
 OMEGAS ARE GIBBS SAMPLED
 
 MONITORING OF SEARCH:

 Burn-in Mode
 iteration        -4000 MCMCOBJ=   -6809.15323707710     
 iteration        -3990 MCMCOBJ=   -6678.45157837653     
 iteration        -3980 MCMCOBJ=   -6713.20991866405     
 iteration        -3970 MCMCOBJ=   -6699.44345518765     
 iteration        -3960 MCMCOBJ=   -6670.15506850953     
 iteration        -3950 MCMCOBJ=   -6675.03101069385     
 iteration        -3940 MCMCOBJ=   -6661.23233535758     
 iteration        -3930 MCMCOBJ=   -6658.34484092227     
 iteration        -3920 MCMCOBJ=   -6663.45790864111     
 iteration        -3910 MCMCOBJ=   -6650.22915713170     
 iteration        -3900 MCMCOBJ=   -6631.60339698252     
 iteration        -3890 MCMCOBJ=   -6623.36576032406     
 iteration        -3880 MCMCOBJ=   -6650.78339505639     
 iteration        -3870 MCMCOBJ=   -6622.41205379151     
 iteration        -3860 MCMCOBJ=   -6664.66049955954     
 Convergence achieved
 Elapsed burn-in time in seconds:    41.01
 Sampling Mode
 iteration            0 MCMCOBJ=   -6657.28966571115     
 iteration           10 MCMCOBJ=   -6595.35134978585     
 iteration           20 MCMCOBJ=   -6577.26109269382     
 iteration           30 MCMCOBJ=   -6574.13931036770     
 iteration           40 MCMCOBJ=   -6614.78848474318     
 iteration           50 MCMCOBJ=   -6599.34192148739     
 iteration           60 MCMCOBJ=   -6602.52532774645     
 iteration           70 MCMCOBJ=   -6666.02434187438     
 iteration           80 MCMCOBJ=   -6616.12569895891     
 iteration           90 MCMCOBJ=   -6518.93175562296     
 iteration          100 MCMCOBJ=   -6630.39879167448     
 iteration          110 MCMCOBJ=   -6553.32984610635     
 iteration          120 MCMCOBJ=   -6575.01741536598     
 iteration          130 MCMCOBJ=   -6562.13869571504     
 iteration          140 MCMCOBJ=   -6555.81998130097     
 iteration          150 MCMCOBJ=   -6597.20400194599     
 iteration          160 MCMCOBJ=   -6606.20857636629     
 iteration          170 MCMCOBJ=   -6546.74557418766     
 iteration          180 MCMCOBJ=   -6589.18475224052     
 iteration          190 MCMCOBJ=   -6564.24953538050     
 iteration          200 MCMCOBJ=   -6495.64356248289     
 iteration          210 MCMCOBJ=   -6592.51759975789     
 iteration          220 MCMCOBJ=   -6645.09983954443     
 iteration          230 MCMCOBJ=   -6568.04419201765     
 iteration          240 MCMCOBJ=   -6593.10656057589     
 iteration          250 MCMCOBJ=   -6557.30071375494     
 iteration          260 MCMCOBJ=   -6590.17265878258     
 iteration          270 MCMCOBJ=   -6509.11488834634     
 iteration          280 MCMCOBJ=   -6541.92357384804     
 iteration          290 MCMCOBJ=   -6541.71268966459     
 iteration          300 MCMCOBJ=   -6583.29262710193     
 iteration          310 MCMCOBJ=   -6528.55594294352     
 iteration          320 MCMCOBJ=   -6554.32774076846     
 iteration          330 MCMCOBJ=   -6545.45852484286     
 iteration          340 MCMCOBJ=   -6580.29170462034     
 iteration          350 MCMCOBJ=   -6512.95093617146     
 iteration          360 MCMCOBJ=   -6547.36316126970     
 iteration          370 MCMCOBJ=   -6556.38085644336     
 iteration          380 MCMCOBJ=   -6544.00361607882     
 iteration          390 MCMCOBJ=   -6537.52047138694     
 iteration          400 MCMCOBJ=   -6504.21780425783     
 iteration          410 MCMCOBJ=   -6480.94678148995     
 iteration          420 MCMCOBJ=   -6493.16854089389     
 iteration          430 MCMCOBJ=   -6575.01729594288     
 iteration          440 MCMCOBJ=   -6514.03666768020     
 iteration          450 MCMCOBJ=   -6616.22021559276     
 iteration          460 MCMCOBJ=   -6542.44644711152     
 iteration          470 MCMCOBJ=   -6555.54116297387     
 iteration          480 MCMCOBJ=   -6504.92434041019     
 iteration          490 MCMCOBJ=   -6571.39121446872     
 iteration          500 MCMCOBJ=   -6587.31483154330     
 iteration          510 MCMCOBJ=   -6532.53804217703     
 iteration          520 MCMCOBJ=   -6514.91928329926     
 iteration          530 MCMCOBJ=   -6500.70675320155     
 iteration          540 MCMCOBJ=   -6485.17907360470     
 iteration          550 MCMCOBJ=   -6540.70478940015     
 iteration          560 MCMCOBJ=   -6531.83069453970     
 iteration          570 MCMCOBJ=   -6563.77348182931     
 iteration          580 MCMCOBJ=   -6554.22303107637     
 iteration          590 MCMCOBJ=   -6543.80924338418     
 iteration          600 MCMCOBJ=   -6516.96495520005     
 iteration          610 MCMCOBJ=   -6535.83843574869     
 iteration          620 MCMCOBJ=   -6532.64592186169     
 iteration          630 MCMCOBJ=   -6450.73445264422     
 iteration          640 MCMCOBJ=   -6553.84311065766     
 iteration          650 MCMCOBJ=   -6549.30968851335     
 iteration          660 MCMCOBJ=   -6502.99580180108     
 iteration          670 MCMCOBJ=   -6496.12228214882     
 iteration          680 MCMCOBJ=   -6534.91858188417     
 iteration          690 MCMCOBJ=   -6482.78136887851     
 iteration          700 MCMCOBJ=   -6473.61215513617     
 iteration          710 MCMCOBJ=   -6533.48638847712     
 iteration          720 MCMCOBJ=   -6511.67327341092     
 iteration          730 MCMCOBJ=   -6484.30619870352     
 iteration          740 MCMCOBJ=   -6493.63699842253     
 iteration          750 MCMCOBJ=   -6515.19399605091     
 iteration          760 MCMCOBJ=   -6512.93449633376     
 iteration          770 MCMCOBJ=   -6569.46533745718     
 iteration          780 MCMCOBJ=   -6547.57131524922     
 iteration          790 MCMCOBJ=   -6521.48550687327     
 iteration          800 MCMCOBJ=   -6529.66743698898     
 iteration          810 MCMCOBJ=   -6532.75134263337     
 iteration          820 MCMCOBJ=   -6547.46811673404     
 iteration          830 MCMCOBJ=   -6494.49410009754     
 iteration          840 MCMCOBJ=   -6522.29990957970     
 iteration          850 MCMCOBJ=   -6529.12152337728     
 iteration          860 MCMCOBJ=   -6502.54782419213     
 iteration          870 MCMCOBJ=   -6480.97149507881     
 iteration          880 MCMCOBJ=   -6536.82887285828     
 iteration          890 MCMCOBJ=   -6445.81512387200     
 iteration          900 MCMCOBJ=   -6507.05357006244     
 iteration          910 MCMCOBJ=   -6464.05561251933     
 iteration          920 MCMCOBJ=   -6514.82707919353     
 iteration          930 MCMCOBJ=   -6491.22103284070     
 iteration          940 MCMCOBJ=   -6550.00806396034     
 iteration          950 MCMCOBJ=   -6473.13485881214     
 iteration          960 MCMCOBJ=   -6486.51196956072     
 iteration          970 MCMCOBJ=   -6526.11354992386     
 iteration          980 MCMCOBJ=   -6483.00200670101     
 iteration          990 MCMCOBJ=   -6471.14293600899     
 iteration         1000 MCMCOBJ=   -6495.44636144734     
 iteration         1010 MCMCOBJ=   -6512.80558500460     
 iteration         1020 MCMCOBJ=   -6480.03790110116     
 iteration         1030 MCMCOBJ=   -6504.14932269103     
 iteration         1040 MCMCOBJ=   -6484.43828866609     
 iteration         1050 MCMCOBJ=   -6535.86218239978     
 iteration         1060 MCMCOBJ=   -6466.64260051075     
 iteration         1070 MCMCOBJ=   -6512.11310967974     
 iteration         1080 MCMCOBJ=   -6491.79554946944     
 iteration         1090 MCMCOBJ=   -6519.71958928507     
 iteration         1100 MCMCOBJ=   -6579.95718488020     
 iteration         1110 MCMCOBJ=   -6532.19125605231     
 iteration         1120 MCMCOBJ=   -6494.92003785638     
 iteration         1130 MCMCOBJ=   -6514.42695885343     
 iteration         1140 MCMCOBJ=   -6538.81856430892     
 iteration         1150 MCMCOBJ=   -6496.11653959765     
 iteration         1160 MCMCOBJ=   -6459.10852134906     
 iteration         1170 MCMCOBJ=   -6503.53461953605     
 iteration         1180 MCMCOBJ=   -6536.45307940461     
 iteration         1190 MCMCOBJ=   -6441.13382777422     
 iteration         1200 MCMCOBJ=   -6473.56534788612     
 iteration         1210 MCMCOBJ=   -6575.61097065084     
 iteration         1220 MCMCOBJ=   -6496.28013623041     
 iteration         1230 MCMCOBJ=   -6555.48606660337     
 iteration         1240 MCMCOBJ=   -6466.76287074438     
 iteration         1250 MCMCOBJ=   -6514.28353639961     
 iteration         1260 MCMCOBJ=   -6484.80778309047     
 iteration         1270 MCMCOBJ=   -6542.68937061016     
 iteration         1280 MCMCOBJ=   -6524.12724813300     
 iteration         1290 MCMCOBJ=   -6495.74407205201     
 iteration         1300 MCMCOBJ=   -6531.44673679855     
 iteration         1310 MCMCOBJ=   -6464.79432695215     
 iteration         1320 MCMCOBJ=   -6467.12803231368     
 iteration         1330 MCMCOBJ=   -6470.13877493377     
 iteration         1340 MCMCOBJ=   -6538.95767698667     
 iteration         1350 MCMCOBJ=   -6492.61089065678     
 iteration         1360 MCMCOBJ=   -6472.28430706053     
 iteration         1370 MCMCOBJ=   -6482.80903591680     
 iteration         1380 MCMCOBJ=   -6421.11589313836     
 iteration         1390 MCMCOBJ=   -6481.06300133489     
 iteration         1400 MCMCOBJ=   -6563.15841695988     
 iteration         1410 MCMCOBJ=   -6491.71532671436     
 iteration         1420 MCMCOBJ=   -6503.27763879786     
 iteration         1430 MCMCOBJ=   -6484.50012213269     
 iteration         1440 MCMCOBJ=   -6485.94245148810     
 iteration         1450 MCMCOBJ=   -6467.40084499681     
 iteration         1460 MCMCOBJ=   -6545.64193334626     
 iteration         1470 MCMCOBJ=   -6517.40413608457     
 iteration         1480 MCMCOBJ=   -6478.45118286301     
 iteration         1490 MCMCOBJ=   -6460.69355237325     
 iteration         1500 MCMCOBJ=   -6472.96926934792     
 iteration         1510 MCMCOBJ=   -6549.52441833871     
 iteration         1520 MCMCOBJ=   -6540.21119634375     
 iteration         1530 MCMCOBJ=   -6502.51456657619     
 iteration         1540 MCMCOBJ=   -6497.03030414188     
 iteration         1550 MCMCOBJ=   -6513.67731791771     
 iteration         1560 MCMCOBJ=   -6468.11599330605     
 iteration         1570 MCMCOBJ=   -6491.01206097600     
 iteration         1580 MCMCOBJ=   -6473.49564797930     
 iteration         1590 MCMCOBJ=   -6518.02276469621     
 iteration         1600 MCMCOBJ=   -6508.63063267853     
 iteration         1610 MCMCOBJ=   -6497.53220622712     
 iteration         1620 MCMCOBJ=   -6496.44247219008     
 iteration         1630 MCMCOBJ=   -6507.31189531471     
 iteration         1640 MCMCOBJ=   -6474.52073008956     
 iteration         1650 MCMCOBJ=   -6524.88744843547     
 iteration         1660 MCMCOBJ=   -6459.36166517631     
 iteration         1670 MCMCOBJ=   -6490.18435298103     
 iteration         1680 MCMCOBJ=   -6514.93835808623     
 iteration         1690 MCMCOBJ=   -6459.65998360114     
 iteration         1700 MCMCOBJ=   -6524.72784548031     
 iteration         1710 MCMCOBJ=   -6441.19360038053     
 iteration         1720 MCMCOBJ=   -6502.66809891053     
 iteration         1730 MCMCOBJ=   -6414.05578539361     
 iteration         1740 MCMCOBJ=   -6553.39116490551     
 iteration         1750 MCMCOBJ=   -6530.95970075071     
 iteration         1760 MCMCOBJ=   -6547.89232244858     
 iteration         1770 MCMCOBJ=   -6524.75603379865     
 iteration         1780 MCMCOBJ=   -6501.10679558750     
 iteration         1790 MCMCOBJ=   -6489.14936359065     
 iteration         1800 MCMCOBJ=   -6512.94163139410     
 iteration         1810 MCMCOBJ=   -6452.23735499021     
 iteration         1820 MCMCOBJ=   -6533.07587810268     
 iteration         1830 MCMCOBJ=   -6449.31425541514     
 iteration         1840 MCMCOBJ=   -6466.93845417751     
 iteration         1850 MCMCOBJ=   -6562.84263209579     
 iteration         1860 MCMCOBJ=   -6515.57937306591     
 iteration         1870 MCMCOBJ=   -6503.14991546673     
 iteration         1880 MCMCOBJ=   -6468.48906720637     
 iteration         1890 MCMCOBJ=   -6525.65789266497     
 iteration         1900 MCMCOBJ=   -6494.16576929524     
 iteration         1910 MCMCOBJ=   -6492.57254739816     
 iteration         1920 MCMCOBJ=   -6505.70196884813     
 iteration         1930 MCMCOBJ=   -6489.01004109815     
 iteration         1940 MCMCOBJ=   -6477.32116199779     
 iteration         1950 MCMCOBJ=   -6526.65672130850     
 iteration         1960 MCMCOBJ=   -6515.36192128380     
 iteration         1970 MCMCOBJ=   -6495.87695018178     
 iteration         1980 MCMCOBJ=   -6512.14400044805     
 iteration         1990 MCMCOBJ=   -6496.77674054199     
 iteration         2000 MCMCOBJ=   -6431.77751893770     
 iteration         2010 MCMCOBJ=   -6473.09823303830     
 iteration         2020 MCMCOBJ=   -6484.25119463148     
 iteration         2030 MCMCOBJ=   -6502.23465705063     
 iteration         2040 MCMCOBJ=   -6501.14715688281     
 iteration         2050 MCMCOBJ=   -6483.49721031334     
 iteration         2060 MCMCOBJ=   -6460.09485072116     
 iteration         2070 MCMCOBJ=   -6542.66914882045     
 iteration         2080 MCMCOBJ=   -6524.54266282797     
 iteration         2090 MCMCOBJ=   -6510.76202636091     
 iteration         2100 MCMCOBJ=   -6515.53228693481     
 iteration         2110 MCMCOBJ=   -6479.65709451093     
 iteration         2120 MCMCOBJ=   -6506.72770364543     
 iteration         2130 MCMCOBJ=   -6481.67107520621     
 iteration         2140 MCMCOBJ=   -6523.76955713708     
 iteration         2150 MCMCOBJ=   -6568.08877379020     
 iteration         2160 MCMCOBJ=   -6508.64312611380     
 iteration         2170 MCMCOBJ=   -6468.80370802824     
 iteration         2180 MCMCOBJ=   -6510.16197365710     
 iteration         2190 MCMCOBJ=   -6424.75106423602     
 iteration         2200 MCMCOBJ=   -6526.64133382354     
 iteration         2210 MCMCOBJ=   -6425.08357252035     
 iteration         2220 MCMCOBJ=   -6498.72514077758     
 iteration         2230 MCMCOBJ=   -6493.17392529395     
 iteration         2240 MCMCOBJ=   -6472.68857581701     
 iteration         2250 MCMCOBJ=   -6615.15590872793     
 iteration         2260 MCMCOBJ=   -6401.47606216075     
 iteration         2270 MCMCOBJ=   -6487.37045024710     
 iteration         2280 MCMCOBJ=   -6488.44423498893     
 iteration         2290 MCMCOBJ=   -6492.51915517026     
 iteration         2300 MCMCOBJ=   -6471.49746541329     
 iteration         2310 MCMCOBJ=   -6474.62850980256     
 iteration         2320 MCMCOBJ=   -6521.18419907926     
 iteration         2330 MCMCOBJ=   -6550.07157343667     
 iteration         2340 MCMCOBJ=   -6439.54955594458     
 iteration         2350 MCMCOBJ=   -6486.96540702253     
 iteration         2360 MCMCOBJ=   -6490.32432789373     
 iteration         2370 MCMCOBJ=   -6521.27008749810     
 iteration         2380 MCMCOBJ=   -6474.23768877801     
 iteration         2390 MCMCOBJ=   -6487.83518666917     
 iteration         2400 MCMCOBJ=   -6506.30096818373     
 iteration         2410 MCMCOBJ=   -6450.94938401709     
 iteration         2420 MCMCOBJ=   -6522.69908106517     
 iteration         2430 MCMCOBJ=   -6504.00625839294     
 iteration         2440 MCMCOBJ=   -6459.83753455901     
 iteration         2450 MCMCOBJ=   -6448.98299158640     
 iteration         2460 MCMCOBJ=   -6518.22850203353     
 iteration         2470 MCMCOBJ=   -6462.45541488396     
 iteration         2480 MCMCOBJ=   -6472.00458494395     
 iteration         2490 MCMCOBJ=   -6454.55020419304     
 iteration         2500 MCMCOBJ=   -6496.88350926379     
 iteration         2510 MCMCOBJ=   -6528.50557156317     
 iteration         2520 MCMCOBJ=   -6534.67548925352     
 iteration         2530 MCMCOBJ=   -6494.08537027743     
 iteration         2540 MCMCOBJ=   -6506.36884614304     
 iteration         2550 MCMCOBJ=   -6471.23765081469     
 iteration         2560 MCMCOBJ=   -6505.68404833095     
 iteration         2570 MCMCOBJ=   -6433.75398673984     
 iteration         2580 MCMCOBJ=   -6498.92784144497     
 iteration         2590 MCMCOBJ=   -6465.23879659524     
 iteration         2600 MCMCOBJ=   -6457.37058950368     
 iteration         2610 MCMCOBJ=   -6512.43657066832     
 iteration         2620 MCMCOBJ=   -6484.75342230410     
 iteration         2630 MCMCOBJ=   -6475.26333516857     
 iteration         2640 MCMCOBJ=   -6514.04501648168     
 iteration         2650 MCMCOBJ=   -6494.16400389742     
 iteration         2660 MCMCOBJ=   -6495.81741189216     
 iteration         2670 MCMCOBJ=   -6500.26923958638     
 iteration         2680 MCMCOBJ=   -6456.52395826424     
 iteration         2690 MCMCOBJ=   -6489.41176719166     
 iteration         2700 MCMCOBJ=   -6539.99990741847     
 iteration         2710 MCMCOBJ=   -6424.85490142000     
 iteration         2720 MCMCOBJ=   -6469.50357710955     
 iteration         2730 MCMCOBJ=   -6477.11541640708     
 iteration         2740 MCMCOBJ=   -6470.38165656630     
 iteration         2750 MCMCOBJ=   -6519.33709801288     
 iteration         2760 MCMCOBJ=   -6436.07933354767     
 iteration         2770 MCMCOBJ=   -6488.79595685479     
 iteration         2780 MCMCOBJ=   -6458.16524279296     
 iteration         2790 MCMCOBJ=   -6468.27084128396     
 iteration         2800 MCMCOBJ=   -6518.10318845666     
 iteration         2810 MCMCOBJ=   -6472.87865837986     
 iteration         2820 MCMCOBJ=   -6509.07079101012     
 iteration         2830 MCMCOBJ=   -6496.85934114912     
 iteration         2840 MCMCOBJ=   -6543.65648623445     
 iteration         2850 MCMCOBJ=   -6476.34358658179     
 iteration         2860 MCMCOBJ=   -6534.10444068738     
 iteration         2870 MCMCOBJ=   -6559.05156574592     
 iteration         2880 MCMCOBJ=   -6503.18360646036     
 iteration         2890 MCMCOBJ=   -6519.57427538393     
 iteration         2900 MCMCOBJ=   -6477.90782580483     
 iteration         2910 MCMCOBJ=   -6414.49627848087     
 iteration         2920 MCMCOBJ=   -6510.24422548955     
 iteration         2930 MCMCOBJ=   -6548.09805367099     
 iteration         2940 MCMCOBJ=   -6497.22229115190     
 iteration         2950 MCMCOBJ=   -6504.90527218094     
 iteration         2960 MCMCOBJ=   -6429.12107223082     
 iteration         2970 MCMCOBJ=   -6472.74056846706     
 iteration         2980 MCMCOBJ=   -6457.36796122377     
 iteration         2990 MCMCOBJ=   -6441.87279936711     
 iteration         3000 MCMCOBJ=   -6452.95455739692     
 iteration         3010 MCMCOBJ=   -6485.16363666746     
 iteration         3020 MCMCOBJ=   -6518.80448207239     
 iteration         3030 MCMCOBJ=   -6471.96461658860     
 iteration         3040 MCMCOBJ=   -6519.77536629855     
 iteration         3050 MCMCOBJ=   -6472.16184316376     
 iteration         3060 MCMCOBJ=   -6501.46444325211     
 iteration         3070 MCMCOBJ=   -6469.61348375704     
 iteration         3080 MCMCOBJ=   -6535.37427015836     
 iteration         3090 MCMCOBJ=   -6508.48771941087     
 iteration         3100 MCMCOBJ=   -6550.83489968052     
 iteration         3110 MCMCOBJ=   -6506.66087988632     
 iteration         3120 MCMCOBJ=   -6520.19781123105     
 iteration         3130 MCMCOBJ=   -6545.77343774182     
 iteration         3140 MCMCOBJ=   -6487.17309315676     
 iteration         3150 MCMCOBJ=   -6525.13911304374     
 iteration         3160 MCMCOBJ=   -6488.37589360941     
 iteration         3170 MCMCOBJ=   -6507.87600395230     
 iteration         3180 MCMCOBJ=   -6414.25862517870     
 iteration         3190 MCMCOBJ=   -6509.18316426587     
 iteration         3200 MCMCOBJ=   -6496.99749861583     
 iteration         3210 MCMCOBJ=   -6415.53972532466     
 iteration         3220 MCMCOBJ=   -6525.34008872142     
 iteration         3230 MCMCOBJ=   -6540.62115654701     
 iteration         3240 MCMCOBJ=   -6483.57337366438     
 iteration         3250 MCMCOBJ=   -6470.16930606324     
 iteration         3260 MCMCOBJ=   -6547.33216547020     
 iteration         3270 MCMCOBJ=   -6496.45313286223     
 iteration         3280 MCMCOBJ=   -6489.84455768549     
 iteration         3290 MCMCOBJ=   -6457.77045522965     
 iteration         3300 MCMCOBJ=   -6510.45959811710     
 iteration         3310 MCMCOBJ=   -6450.19035576147     
 iteration         3320 MCMCOBJ=   -6500.13687912885     
 iteration         3330 MCMCOBJ=   -6507.95308859097     
 iteration         3340 MCMCOBJ=   -6487.80233778243     
 iteration         3350 MCMCOBJ=   -6453.38996096765     
 iteration         3360 MCMCOBJ=   -6502.71124451826     
 iteration         3370 MCMCOBJ=   -6493.32531478052     
 iteration         3380 MCMCOBJ=   -6424.96991727447     
 iteration         3390 MCMCOBJ=   -6462.72508459122     
 iteration         3400 MCMCOBJ=   -6438.10092027007     
 iteration         3410 MCMCOBJ=   -6475.05349023959     
 iteration         3420 MCMCOBJ=   -6535.91036818777     
 iteration         3430 MCMCOBJ=   -6475.72153346142     
 iteration         3440 MCMCOBJ=   -6481.07325086282     
 iteration         3450 MCMCOBJ=   -6465.07166984599     
 iteration         3460 MCMCOBJ=   -6483.44393142803     
 iteration         3470 MCMCOBJ=   -6499.97344061470     
 iteration         3480 MCMCOBJ=   -6483.82958316449     
 iteration         3490 MCMCOBJ=   -6448.81702278387     
 iteration         3500 MCMCOBJ=   -6527.00552568362     
 iteration         3510 MCMCOBJ=   -6463.54244710908     
 iteration         3520 MCMCOBJ=   -6531.99383755155     
 iteration         3530 MCMCOBJ=   -6501.35643916455     
 iteration         3540 MCMCOBJ=   -6502.27787323600     
 iteration         3550 MCMCOBJ=   -6510.69747810539     
 iteration         3560 MCMCOBJ=   -6496.11400063724     
 iteration         3570 MCMCOBJ=   -6443.94643830010     
 iteration         3580 MCMCOBJ=   -6486.03142120411     
 iteration         3590 MCMCOBJ=   -6493.99424636081     
 iteration         3600 MCMCOBJ=   -6463.49412330205     
 iteration         3610 MCMCOBJ=   -6501.70868530232     
 iteration         3620 MCMCOBJ=   -6442.72884541573     
 iteration         3630 MCMCOBJ=   -6421.64196858050     
 iteration         3640 MCMCOBJ=   -6469.26248669779     
 iteration         3650 MCMCOBJ=   -6524.22040835826     
 iteration         3660 MCMCOBJ=   -6490.02360561492     
 iteration         3670 MCMCOBJ=   -6512.24913807674     
 iteration         3680 MCMCOBJ=   -6494.34853308892     
 iteration         3690 MCMCOBJ=   -6487.44486362025     
 iteration         3700 MCMCOBJ=   -6470.10667349849     
 iteration         3710 MCMCOBJ=   -6451.52108309022     
 iteration         3720 MCMCOBJ=   -6476.26247705863     
 iteration         3730 MCMCOBJ=   -6514.89171929535     
 iteration         3740 MCMCOBJ=   -6509.60769835997     
 iteration         3750 MCMCOBJ=   -6499.84327267022     
 iteration         3760 MCMCOBJ=   -6491.47069948556     
 iteration         3770 MCMCOBJ=   -6501.72964460635     
 iteration         3780 MCMCOBJ=   -6432.10345144945     
 iteration         3790 MCMCOBJ=   -6527.25286513685     
 iteration         3800 MCMCOBJ=   -6504.71950917285     
 iteration         3810 MCMCOBJ=   -6535.13999299844     
 iteration         3820 MCMCOBJ=   -6516.12107870864     
 iteration         3830 MCMCOBJ=   -6496.16658785338     
 iteration         3840 MCMCOBJ=   -6512.86628540056     
 iteration         3850 MCMCOBJ=   -6518.77365117790     
 iteration         3860 MCMCOBJ=   -6509.37750483695     
 iteration         3870 MCMCOBJ=   -6501.95499474393     
 iteration         3880 MCMCOBJ=   -6535.92992486158     
 iteration         3890 MCMCOBJ=   -6509.81592032860     
 iteration         3900 MCMCOBJ=   -6532.30772073105     
 iteration         3910 MCMCOBJ=   -6490.16801167160     
 iteration         3920 MCMCOBJ=   -6517.35677080691     
 iteration         3930 MCMCOBJ=   -6527.83609139132     
 iteration         3940 MCMCOBJ=   -6480.17423770503     
 iteration         3950 MCMCOBJ=   -6507.00898090960     
 iteration         3960 MCMCOBJ=   -6529.63493261346     
 iteration         3970 MCMCOBJ=   -6535.21478195688     
 iteration         3980 MCMCOBJ=   -6561.14082402537     
 iteration         3990 MCMCOBJ=   -6476.06867173008     
 iteration         4000 MCMCOBJ=   -6536.65609660608     
 iteration         4010 MCMCOBJ=   -6458.73323174370     
 iteration         4020 MCMCOBJ=   -6502.25889158601     
 iteration         4030 MCMCOBJ=   -6505.15760499165     
 iteration         4040 MCMCOBJ=   -6548.30197834727     
 iteration         4050 MCMCOBJ=   -6513.88632236175     
 iteration         4060 MCMCOBJ=   -6498.69035534997     
 iteration         4070 MCMCOBJ=   -6476.36015276833     
 iteration         4080 MCMCOBJ=   -6479.07129193511     
 iteration         4090 MCMCOBJ=   -6515.09661950798     
 iteration         4100 MCMCOBJ=   -6516.51669108417     
 iteration         4110 MCMCOBJ=   -6467.04746886517     
 iteration         4120 MCMCOBJ=   -6440.42373476954     
 iteration         4130 MCMCOBJ=   -6440.06012536887     
 iteration         4140 MCMCOBJ=   -6502.00452931788     
 iteration         4150 MCMCOBJ=   -6532.34824947265     
 iteration         4160 MCMCOBJ=   -6501.75152418458     
 iteration         4170 MCMCOBJ=   -6507.34275899227     
 iteration         4180 MCMCOBJ=   -6471.78486891168     
 iteration         4190 MCMCOBJ=   -6515.91652993658     
 iteration         4200 MCMCOBJ=   -6486.59534451488     
 iteration         4210 MCMCOBJ=   -6467.84722736962     
 iteration         4220 MCMCOBJ=   -6517.77867116457     
 iteration         4230 MCMCOBJ=   -6485.48413305313     
 iteration         4240 MCMCOBJ=   -6449.11544842757     
 iteration         4250 MCMCOBJ=   -6482.58230578859     
 iteration         4260 MCMCOBJ=   -6487.54856285651     
 iteration         4270 MCMCOBJ=   -6495.47433995700     
 iteration         4280 MCMCOBJ=   -6429.18590035282     
 iteration         4290 MCMCOBJ=   -6456.80420620762     
 iteration         4300 MCMCOBJ=   -6427.75673142086     
 iteration         4310 MCMCOBJ=   -6499.12342638798     
 iteration         4320 MCMCOBJ=   -6475.88798806810     
 iteration         4330 MCMCOBJ=   -6519.45286971895     
 iteration         4340 MCMCOBJ=   -6396.35275930080     
 iteration         4350 MCMCOBJ=   -6513.93673722440     
 iteration         4360 MCMCOBJ=   -6510.16850059146     
 iteration         4370 MCMCOBJ=   -6427.81487379221     
 iteration         4380 MCMCOBJ=   -6505.81254136825     
 iteration         4390 MCMCOBJ=   -6469.11432699787     
 iteration         4400 MCMCOBJ=   -6466.48451657567     
 iteration         4410 MCMCOBJ=   -6478.55632626936     
 iteration         4420 MCMCOBJ=   -6535.02093775099     
 iteration         4430 MCMCOBJ=   -6527.03125611107     
 iteration         4440 MCMCOBJ=   -6465.03041630976     
 iteration         4450 MCMCOBJ=   -6480.67264229885     
 iteration         4460 MCMCOBJ=   -6471.58363385902     
 iteration         4470 MCMCOBJ=   -6478.33428542302     
 iteration         4480 MCMCOBJ=   -6470.39042202225     
 iteration         4490 MCMCOBJ=   -6503.40756321973     
 iteration         4500 MCMCOBJ=   -6544.76474060442     
 iteration         4510 MCMCOBJ=   -6505.91990414823     
 iteration         4520 MCMCOBJ=   -6557.60883499304     
 iteration         4530 MCMCOBJ=   -6493.28592397952     
 iteration         4540 MCMCOBJ=   -6517.16757772019     
 iteration         4550 MCMCOBJ=   -6446.66595714723     
 iteration         4560 MCMCOBJ=   -6466.07171579890     
 iteration         4570 MCMCOBJ=   -6502.23156060265     
 iteration         4580 MCMCOBJ=   -6467.04603317193     
 iteration         4590 MCMCOBJ=   -6460.71618322863     
 iteration         4600 MCMCOBJ=   -6508.13521041817     
 iteration         4610 MCMCOBJ=   -6444.15593632753     
 iteration         4620 MCMCOBJ=   -6442.45024028008     
 iteration         4630 MCMCOBJ=   -6462.81513143286     
 iteration         4640 MCMCOBJ=   -6516.44735980774     
 iteration         4650 MCMCOBJ=   -6445.17871228106     
 iteration         4660 MCMCOBJ=   -6430.36026518425     
 iteration         4670 MCMCOBJ=   -6495.58287992275     
 iteration         4680 MCMCOBJ=   -6524.93583956557     
 iteration         4690 MCMCOBJ=   -6490.02019015937     
 iteration         4700 MCMCOBJ=   -6473.41263577562     
 iteration         4710 MCMCOBJ=   -6481.57823007621     
 iteration         4720 MCMCOBJ=   -6475.99439547187     
 iteration         4730 MCMCOBJ=   -6551.01612695918     
 iteration         4740 MCMCOBJ=   -6469.58438300718     
 iteration         4750 MCMCOBJ=   -6486.01334970035     
 iteration         4760 MCMCOBJ=   -6540.23221032383     
 iteration         4770 MCMCOBJ=   -6524.57596644062     
 iteration         4780 MCMCOBJ=   -6459.63955671442     
 iteration         4790 MCMCOBJ=   -6386.09886092591     
 iteration         4800 MCMCOBJ=   -6463.29805171565     
 iteration         4810 MCMCOBJ=   -6478.73505453154     
 iteration         4820 MCMCOBJ=   -6484.86803562726     
 iteration         4830 MCMCOBJ=   -6505.09871690568     
 iteration         4840 MCMCOBJ=   -6487.31551015493     
 iteration         4850 MCMCOBJ=   -6476.12137055748     
 iteration         4860 MCMCOBJ=   -6478.36025122806     
 iteration         4870 MCMCOBJ=   -6531.35228933152     
 iteration         4880 MCMCOBJ=   -6540.66802325709     
 iteration         4890 MCMCOBJ=   -6498.19222078302     
 iteration         4900 MCMCOBJ=   -6429.07782034840     
 iteration         4910 MCMCOBJ=   -6463.57804836462     
 iteration         4920 MCMCOBJ=   -6499.69354229804     
 iteration         4930 MCMCOBJ=   -6435.07191234193     
 iteration         4940 MCMCOBJ=   -6521.69393545193     
 iteration         4950 MCMCOBJ=   -6531.75174446888     
 iteration         4960 MCMCOBJ=   -6477.58467220643     
 iteration         4970 MCMCOBJ=   -6488.24918077971     
 iteration         4980 MCMCOBJ=   -6509.77962922858     
 iteration         4990 MCMCOBJ=   -6471.72835085069     
 iteration         5000 MCMCOBJ=   -6503.14092691525     
 iteration         5010 MCMCOBJ=   -6516.30576389527     
 iteration         5020 MCMCOBJ=   -6465.74741170097     
 iteration         5030 MCMCOBJ=   -6488.26835139330     
 iteration         5040 MCMCOBJ=   -6446.00551874142     
 iteration         5050 MCMCOBJ=   -6413.38136796658     
 iteration         5060 MCMCOBJ=   -6539.27180522292     
 iteration         5070 MCMCOBJ=   -6464.90788713108     
 iteration         5080 MCMCOBJ=   -6496.50243197000     
 iteration         5090 MCMCOBJ=   -6510.00630124962     
 iteration         5100 MCMCOBJ=   -6405.79209896399     
 iteration         5110 MCMCOBJ=   -6488.73987642793     
 iteration         5120 MCMCOBJ=   -6520.21423925819     
 iteration         5130 MCMCOBJ=   -6489.61399339880     
 iteration         5140 MCMCOBJ=   -6472.23248511607     
 iteration         5150 MCMCOBJ=   -6504.24716475158     
 iteration         5160 MCMCOBJ=   -6525.13480281934     
 iteration         5170 MCMCOBJ=   -6454.95596807131     
 iteration         5180 MCMCOBJ=   -6513.74494004113     
 iteration         5190 MCMCOBJ=   -6481.56947337563     
 iteration         5200 MCMCOBJ=   -6521.13065990093     
 iteration         5210 MCMCOBJ=   -6419.10645998933     
 iteration         5220 MCMCOBJ=   -6423.19483096386     
 iteration         5230 MCMCOBJ=   -6510.76874140138     
 iteration         5240 MCMCOBJ=   -6528.30823524131     
 iteration         5250 MCMCOBJ=   -6494.21026839146     
 iteration         5260 MCMCOBJ=   -6522.62135132357     
 iteration         5270 MCMCOBJ=   -6442.39475524256     
 iteration         5280 MCMCOBJ=   -6434.80336121243     
 iteration         5290 MCMCOBJ=   -6425.84544534552     
 iteration         5300 MCMCOBJ=   -6431.31253996665     
 iteration         5310 MCMCOBJ=   -6517.51669359687     
 iteration         5320 MCMCOBJ=   -6495.25282647459     
 iteration         5330 MCMCOBJ=   -6473.19407493527     
 iteration         5340 MCMCOBJ=   -6445.82178991438     
 iteration         5350 MCMCOBJ=   -6511.66537721424     
 iteration         5360 MCMCOBJ=   -6509.92878067438     
 iteration         5370 MCMCOBJ=   -6490.98552836965     
 iteration         5380 MCMCOBJ=   -6438.52589866075     
 iteration         5390 MCMCOBJ=   -6472.18046267573     
 iteration         5400 MCMCOBJ=   -6452.17885752190     
 iteration         5410 MCMCOBJ=   -6501.33122465060     
 iteration         5420 MCMCOBJ=   -6444.41192503689     
 iteration         5430 MCMCOBJ=   -6469.17702931175     
 iteration         5440 MCMCOBJ=   -6484.94623899005     
 iteration         5450 MCMCOBJ=   -6520.43572609933     
 iteration         5460 MCMCOBJ=   -6448.09063578170     
 iteration         5470 MCMCOBJ=   -6516.55040138269     
 iteration         5480 MCMCOBJ=   -6406.02395862233     
 iteration         5490 MCMCOBJ=   -6502.08886512900     
 iteration         5500 MCMCOBJ=   -6439.01631645554     
 iteration         5510 MCMCOBJ=   -6499.62024116706     
 iteration         5520 MCMCOBJ=   -6434.79458754724     
 iteration         5530 MCMCOBJ=   -6489.70490158807     
 iteration         5540 MCMCOBJ=   -6495.70928206784     
 iteration         5550 MCMCOBJ=   -6488.87169129553     
 iteration         5560 MCMCOBJ=   -6540.26143819145     
 iteration         5570 MCMCOBJ=   -6442.16227837328     
 iteration         5580 MCMCOBJ=   -6502.08751242763     
 iteration         5590 MCMCOBJ=   -6489.29427371245     
 iteration         5600 MCMCOBJ=   -6498.04071423747     
 iteration         5610 MCMCOBJ=   -6480.82788582996     
 iteration         5620 MCMCOBJ=   -6494.42218949958     
 iteration         5630 MCMCOBJ=   -6516.26015095003     
 iteration         5640 MCMCOBJ=   -6539.34141151531     
 iteration         5650 MCMCOBJ=   -6490.17830215826     
 iteration         5660 MCMCOBJ=   -6467.38643121818     
 iteration         5670 MCMCOBJ=   -6402.37772690109     
 iteration         5680 MCMCOBJ=   -6507.50971932346     
 iteration         5690 MCMCOBJ=   -6452.54618100950     
 iteration         5700 MCMCOBJ=   -6434.98718483126     
 iteration         5710 MCMCOBJ=   -6495.73713666338     
 iteration         5720 MCMCOBJ=   -6400.36123295678     
 iteration         5730 MCMCOBJ=   -6424.54532532797     
 iteration         5740 MCMCOBJ=   -6445.77772436575     
 iteration         5750 MCMCOBJ=   -6458.76647902790     
 iteration         5760 MCMCOBJ=   -6499.77702432102     
 iteration         5770 MCMCOBJ=   -6461.59850311436     
 iteration         5780 MCMCOBJ=   -6461.80861343314     
 iteration         5790 MCMCOBJ=   -6464.85906154688     
 iteration         5800 MCMCOBJ=   -6467.69611384478     
 iteration         5810 MCMCOBJ=   -6497.51557200902     
 iteration         5820 MCMCOBJ=   -6449.99676749116     
 iteration         5830 MCMCOBJ=   -6413.39997281547     
 iteration         5840 MCMCOBJ=   -6462.27556411919     
 iteration         5850 MCMCOBJ=   -6535.36289862338     
 iteration         5860 MCMCOBJ=   -6445.36301002385     
 iteration         5870 MCMCOBJ=   -6488.51737434028     
 iteration         5880 MCMCOBJ=   -6473.46574536404     
 iteration         5890 MCMCOBJ=   -6519.42146851334     
 iteration         5900 MCMCOBJ=   -6526.03027693554     
 iteration         5910 MCMCOBJ=   -6468.48320861329     
 iteration         5920 MCMCOBJ=   -6512.71461376810     
 iteration         5930 MCMCOBJ=   -6480.99326820466     
 iteration         5940 MCMCOBJ=   -6429.14842479700     
 iteration         5950 MCMCOBJ=   -6464.35316812061     
 iteration         5960 MCMCOBJ=   -6511.40264910418     
 iteration         5970 MCMCOBJ=   -6464.11027617902     
 iteration         5980 MCMCOBJ=   -6449.04351985734     
 iteration         5990 MCMCOBJ=   -6493.65955546727     
 iteration         6000 MCMCOBJ=   -6494.21427571669     
 iteration         6010 MCMCOBJ=   -6468.80334832120     
 iteration         6020 MCMCOBJ=   -6501.18462501343     
 iteration         6030 MCMCOBJ=   -6484.31755180777     
 iteration         6040 MCMCOBJ=   -6464.50663071501     
 iteration         6050 MCMCOBJ=   -6472.19516551510     
 iteration         6060 MCMCOBJ=   -6524.71653616453     
 iteration         6070 MCMCOBJ=   -6488.55027453350     
 iteration         6080 MCMCOBJ=   -6482.79008270240     
 iteration         6090 MCMCOBJ=   -6431.80388009190     
 iteration         6100 MCMCOBJ=   -6543.60066111501     
 iteration         6110 MCMCOBJ=   -6475.98884845434     
 iteration         6120 MCMCOBJ=   -6423.51475832578     
 iteration         6130 MCMCOBJ=   -6413.36076409032     
 iteration         6140 MCMCOBJ=   -6543.80572500923     
 iteration         6150 MCMCOBJ=   -6497.30507393238     
 iteration         6160 MCMCOBJ=   -6470.59597476181     
 iteration         6170 MCMCOBJ=   -6526.42258446842     
 iteration         6180 MCMCOBJ=   -6503.48679322177     
 iteration         6190 MCMCOBJ=   -6491.15643761145     
 iteration         6200 MCMCOBJ=   -6470.54599825305     
 iteration         6210 MCMCOBJ=   -6464.47368149526     
 iteration         6220 MCMCOBJ=   -6450.62544930952     
 iteration         6230 MCMCOBJ=   -6481.84052891239     
 iteration         6240 MCMCOBJ=   -6475.85646592265     
 iteration         6250 MCMCOBJ=   -6472.62177950772     
 iteration         6260 MCMCOBJ=   -6482.97121072767     
 iteration         6270 MCMCOBJ=   -6487.40175197918     
 iteration         6280 MCMCOBJ=   -6469.11933831418     
 iteration         6290 MCMCOBJ=   -6477.18479418789     
 iteration         6300 MCMCOBJ=   -6482.53629411525     
 iteration         6310 MCMCOBJ=   -6461.28368680946     
 iteration         6320 MCMCOBJ=   -6464.62199212101     
 iteration         6330 MCMCOBJ=   -6481.64240384251     
 iteration         6340 MCMCOBJ=   -6474.03213363366     
 iteration         6350 MCMCOBJ=   -6508.85956260121     
 iteration         6360 MCMCOBJ=   -6454.44440227799     
 iteration         6370 MCMCOBJ=   -6503.32747899358     
 iteration         6380 MCMCOBJ=   -6466.47209297912     
 iteration         6390 MCMCOBJ=   -6553.18242110449     
 iteration         6400 MCMCOBJ=   -6483.98276816147     
 iteration         6410 MCMCOBJ=   -6447.78784481991     
 iteration         6420 MCMCOBJ=   -6489.34428839555     
 iteration         6430 MCMCOBJ=   -6530.35554730019     
 iteration         6440 MCMCOBJ=   -6488.62655197276     
 iteration         6450 MCMCOBJ=   -6522.58328499391     
 iteration         6460 MCMCOBJ=   -6476.98049567823     
 iteration         6470 MCMCOBJ=   -6489.50111674808     
 iteration         6480 MCMCOBJ=   -6558.55100968876     
 iteration         6490 MCMCOBJ=   -6467.25275348568     
 iteration         6500 MCMCOBJ=   -6441.25029287007     
 iteration         6510 MCMCOBJ=   -6445.18014253546     
 iteration         6520 MCMCOBJ=   -6461.62374369956     
 iteration         6530 MCMCOBJ=   -6427.01680508631     
 iteration         6540 MCMCOBJ=   -6528.91695348142     
 iteration         6550 MCMCOBJ=   -6476.73981649736     
 iteration         6560 MCMCOBJ=   -6475.96541982068     
 iteration         6570 MCMCOBJ=   -6493.68824784909     
 iteration         6580 MCMCOBJ=   -6512.61162636575     
 iteration         6590 MCMCOBJ=   -6555.68453738801     
 iteration         6600 MCMCOBJ=   -6484.09170344025     
 iteration         6610 MCMCOBJ=   -6497.53511728872     
 iteration         6620 MCMCOBJ=   -6439.09268637401     
 iteration         6630 MCMCOBJ=   -6499.43844275323     
 iteration         6640 MCMCOBJ=   -6515.57685798451     
 iteration         6650 MCMCOBJ=   -6442.82014862133     
 iteration         6660 MCMCOBJ=   -6496.18184826761     
 iteration         6670 MCMCOBJ=   -6552.61884699612     
 iteration         6680 MCMCOBJ=   -6465.35059547888     
 iteration         6690 MCMCOBJ=   -6456.87263696512     
 iteration         6700 MCMCOBJ=   -6551.40459686262     
 iteration         6710 MCMCOBJ=   -6453.51136227379     
 iteration         6720 MCMCOBJ=   -6550.51926493741     
 iteration         6730 MCMCOBJ=   -6486.78982431813     
 iteration         6740 MCMCOBJ=   -6500.22083240329     
 iteration         6750 MCMCOBJ=   -6447.72186905501     
 iteration         6760 MCMCOBJ=   -6490.42520877455     
 iteration         6770 MCMCOBJ=   -6537.38073296120     
 iteration         6780 MCMCOBJ=   -6437.98482830260     
 iteration         6790 MCMCOBJ=   -6476.71028366592     
 iteration         6800 MCMCOBJ=   -6555.79121733858     
 iteration         6810 MCMCOBJ=   -6505.79753335390     
 iteration         6820 MCMCOBJ=   -6448.89023169985     
 iteration         6830 MCMCOBJ=   -6475.90898800477     
 iteration         6840 MCMCOBJ=   -6494.29594572732     
 iteration         6850 MCMCOBJ=   -6519.53816134970     
 iteration         6860 MCMCOBJ=   -6431.81306975040     
 iteration         6870 MCMCOBJ=   -6484.18534064996     
 iteration         6880 MCMCOBJ=   -6497.64650891283     
 iteration         6890 MCMCOBJ=   -6548.58723711774     
 iteration         6900 MCMCOBJ=   -6414.29913242364     
 iteration         6910 MCMCOBJ=   -6471.94995386715     
 iteration         6920 MCMCOBJ=   -6480.66445023297     
 iteration         6930 MCMCOBJ=   -6508.14179505705     
 iteration         6940 MCMCOBJ=   -6497.15302779794     
 iteration         6950 MCMCOBJ=   -6468.66576935430     
 iteration         6960 MCMCOBJ=   -6486.17523593288     
 iteration         6970 MCMCOBJ=   -6441.74771974237     
 iteration         6980 MCMCOBJ=   -6538.66595893460     
 iteration         6990 MCMCOBJ=   -6478.66056313115     
 iteration         7000 MCMCOBJ=   -6473.47402430867     
 iteration         7010 MCMCOBJ=   -6476.01678395955     
 iteration         7020 MCMCOBJ=   -6495.46152127153     
 iteration         7030 MCMCOBJ=   -6494.04714510341     
 iteration         7040 MCMCOBJ=   -6492.79697140594     
 iteration         7050 MCMCOBJ=   -6529.49914217843     
 iteration         7060 MCMCOBJ=   -6420.44898050140     
 iteration         7070 MCMCOBJ=   -6472.90774450884     
 iteration         7080 MCMCOBJ=   -6507.54577608714     
 iteration         7090 MCMCOBJ=   -6511.13904262921     
 iteration         7100 MCMCOBJ=   -6498.69175567305     
 iteration         7110 MCMCOBJ=   -6468.51376548234     
 iteration         7120 MCMCOBJ=   -6458.22189555583     
 iteration         7130 MCMCOBJ=   -6517.20478815443     
 iteration         7140 MCMCOBJ=   -6493.63343634334     
 iteration         7150 MCMCOBJ=   -6524.69286205522     
 iteration         7160 MCMCOBJ=   -6428.52406018082     
 iteration         7170 MCMCOBJ=   -6388.10305167795     
 iteration         7180 MCMCOBJ=   -6575.71739255464     
 iteration         7190 MCMCOBJ=   -6472.19354665431     
 iteration         7200 MCMCOBJ=   -6456.14226559899     
 iteration         7210 MCMCOBJ=   -6477.03395127749     
 iteration         7220 MCMCOBJ=   -6427.58197648353     
 iteration         7230 MCMCOBJ=   -6467.98180462099     
 iteration         7240 MCMCOBJ=   -6484.87282138617     
 iteration         7250 MCMCOBJ=   -6410.46194480890     
 iteration         7260 MCMCOBJ=   -6468.10102085469     
 iteration         7270 MCMCOBJ=   -6481.06596605087     
 iteration         7280 MCMCOBJ=   -6472.34635147131     
 iteration         7290 MCMCOBJ=   -6456.87832095534     
 iteration         7300 MCMCOBJ=   -6441.09218002779     
 iteration         7310 MCMCOBJ=   -6489.38750844313     
 iteration         7320 MCMCOBJ=   -6514.18720533691     
 iteration         7330 MCMCOBJ=   -6459.15660354156     
 iteration         7340 MCMCOBJ=   -6527.17888665593     
 iteration         7350 MCMCOBJ=   -6495.38884213406     
 iteration         7360 MCMCOBJ=   -6509.79354580617     
 iteration         7370 MCMCOBJ=   -6445.53862995120     
 iteration         7380 MCMCOBJ=   -6504.86804777920     
 iteration         7390 MCMCOBJ=   -6489.73668649586     
 iteration         7400 MCMCOBJ=   -6516.92499057127     
 iteration         7410 MCMCOBJ=   -6509.82219350557     
 iteration         7420 MCMCOBJ=   -6519.74959768275     
 iteration         7430 MCMCOBJ=   -6370.67095224390     
 iteration         7440 MCMCOBJ=   -6510.60843209874     
 iteration         7450 MCMCOBJ=   -6520.80453168154     
 iteration         7460 MCMCOBJ=   -6499.22710116061     
 iteration         7470 MCMCOBJ=   -6499.13606372511     
 iteration         7480 MCMCOBJ=   -6476.55031963468     
 iteration         7490 MCMCOBJ=   -6468.86723207000     
 iteration         7500 MCMCOBJ=   -6450.66219627735     
 iteration         7510 MCMCOBJ=   -6541.82551349270     
 iteration         7520 MCMCOBJ=   -6512.23867510598     
 iteration         7530 MCMCOBJ=   -6446.84570995083     
 iteration         7540 MCMCOBJ=   -6464.80499662580     
 iteration         7550 MCMCOBJ=   -6454.79572427096     
 iteration         7560 MCMCOBJ=   -6456.27073534608     
 iteration         7570 MCMCOBJ=   -6494.79566267273     
 iteration         7580 MCMCOBJ=   -6445.91970360537     
 iteration         7590 MCMCOBJ=   -6439.57254770729     
 iteration         7600 MCMCOBJ=   -6524.36071267023     
 iteration         7610 MCMCOBJ=   -6470.68249776182     
 iteration         7620 MCMCOBJ=   -6487.98853948766     
 iteration         7630 MCMCOBJ=   -6516.19146463575     
 iteration         7640 MCMCOBJ=   -6492.97922837108     
 iteration         7650 MCMCOBJ=   -6549.19396405635     
 iteration         7660 MCMCOBJ=   -6521.90756067014     
 iteration         7670 MCMCOBJ=   -6477.66587216038     
 iteration         7680 MCMCOBJ=   -6464.21645656640     
 iteration         7690 MCMCOBJ=   -6470.07324341745     
 iteration         7700 MCMCOBJ=   -6492.86209844584     
 iteration         7710 MCMCOBJ=   -6502.36361132400     
 iteration         7720 MCMCOBJ=   -6508.90237695535     
 iteration         7730 MCMCOBJ=   -6467.55291227360     
 iteration         7740 MCMCOBJ=   -6526.31975520574     
 iteration         7750 MCMCOBJ=   -6445.68976819008     
 iteration         7760 MCMCOBJ=   -6444.21022161955     
 iteration         7770 MCMCOBJ=   -6471.20535735589     
 iteration         7780 MCMCOBJ=   -6500.69115315428     
 iteration         7790 MCMCOBJ=   -6524.86701622425     
 iteration         7800 MCMCOBJ=   -6517.73578795355     
 iteration         7810 MCMCOBJ=   -6484.44773561333     
 iteration         7820 MCMCOBJ=   -6457.51354204929     
 iteration         7830 MCMCOBJ=   -6514.88574229940     
 iteration         7840 MCMCOBJ=   -6461.72463166451     
 iteration         7850 MCMCOBJ=   -6433.43237131738     
 iteration         7860 MCMCOBJ=   -6465.49758114875     
 iteration         7870 MCMCOBJ=   -6502.82224660845     
 iteration         7880 MCMCOBJ=   -6499.84271704251     
 iteration         7890 MCMCOBJ=   -6508.77222795646     
 iteration         7900 MCMCOBJ=   -6517.56777036475     
 iteration         7910 MCMCOBJ=   -6523.16016289739     
 iteration         7920 MCMCOBJ=   -6484.01834306148     
 iteration         7930 MCMCOBJ=   -6515.74445284403     
 iteration         7940 MCMCOBJ=   -6436.33656474977     
 iteration         7950 MCMCOBJ=   -6489.68841012857     
 iteration         7960 MCMCOBJ=   -6449.04830710888     
 iteration         7970 MCMCOBJ=   -6497.64950946287     
 iteration         7980 MCMCOBJ=   -6468.63119323364     
 iteration         7990 MCMCOBJ=   -6489.32478239972     
 iteration         8000 MCMCOBJ=   -6533.14853970280     
 iteration         8010 MCMCOBJ=   -6505.34096330758     
 iteration         8020 MCMCOBJ=   -6514.24089653783     
 iteration         8030 MCMCOBJ=   -6427.92319948193     
 iteration         8040 MCMCOBJ=   -6552.27020202557     
 iteration         8050 MCMCOBJ=   -6448.30019031714     
 iteration         8060 MCMCOBJ=   -6475.39327190090     
 iteration         8070 MCMCOBJ=   -6488.69792988074     
 iteration         8080 MCMCOBJ=   -6493.59466807090     
 iteration         8090 MCMCOBJ=   -6472.72630952073     
 iteration         8100 MCMCOBJ=   -6503.99128463175     
 iteration         8110 MCMCOBJ=   -6515.94327521671     
 iteration         8120 MCMCOBJ=   -6476.87760664762     
 iteration         8130 MCMCOBJ=   -6492.64147209630     
 iteration         8140 MCMCOBJ=   -6536.18206774909     
 iteration         8150 MCMCOBJ=   -6435.60110481605     
 iteration         8160 MCMCOBJ=   -6475.51607688790     
 iteration         8170 MCMCOBJ=   -6374.12768707191     
 iteration         8180 MCMCOBJ=   -6493.44361802407     
 iteration         8190 MCMCOBJ=   -6466.76568245652     
 iteration         8200 MCMCOBJ=   -6503.46864088352     
 iteration         8210 MCMCOBJ=   -6471.84845061808     
 iteration         8220 MCMCOBJ=   -6518.51661031616     
 iteration         8230 MCMCOBJ=   -6522.79248577111     
 iteration         8240 MCMCOBJ=   -6506.43501488781     
 iteration         8250 MCMCOBJ=   -6534.08496180625     
 iteration         8260 MCMCOBJ=   -6455.94997247789     
 iteration         8270 MCMCOBJ=   -6501.28770669596     
 iteration         8280 MCMCOBJ=   -6473.29862153323     
 iteration         8290 MCMCOBJ=   -6478.09486337676     
 iteration         8300 MCMCOBJ=   -6517.57687480706     
 iteration         8310 MCMCOBJ=   -6489.63345077162     
 iteration         8320 MCMCOBJ=   -6474.97103558374     
 iteration         8330 MCMCOBJ=   -6424.66024876840     
 iteration         8340 MCMCOBJ=   -6487.67117213934     
 iteration         8350 MCMCOBJ=   -6471.53117563040     
 iteration         8360 MCMCOBJ=   -6482.18663538454     
 iteration         8370 MCMCOBJ=   -6534.45496539059     
 iteration         8380 MCMCOBJ=   -6444.83147284922     
 iteration         8390 MCMCOBJ=   -6508.84399624943     
 iteration         8400 MCMCOBJ=   -6491.15582509740     
 iteration         8410 MCMCOBJ=   -6494.47656218015     
 iteration         8420 MCMCOBJ=   -6512.25190435877     
 iteration         8430 MCMCOBJ=   -6548.14070988274     
 iteration         8440 MCMCOBJ=   -6447.85547818895     
 iteration         8450 MCMCOBJ=   -6488.69479900246     
 iteration         8460 MCMCOBJ=   -6524.64828911376     
 iteration         8470 MCMCOBJ=   -6497.60283546028     
 iteration         8480 MCMCOBJ=   -6450.79286730188     
 iteration         8490 MCMCOBJ=   -6493.98651946722     
 iteration         8500 MCMCOBJ=   -6537.08691020817     
 iteration         8510 MCMCOBJ=   -6495.34833709085     
 iteration         8520 MCMCOBJ=   -6419.94063662313     
 iteration         8530 MCMCOBJ=   -6504.73206732173     
 iteration         8540 MCMCOBJ=   -6420.84728794006     
 iteration         8550 MCMCOBJ=   -6523.24013754444     
 iteration         8560 MCMCOBJ=   -6450.41716090571     
 iteration         8570 MCMCOBJ=   -6438.26638614703     
 iteration         8580 MCMCOBJ=   -6466.83394048592     
 iteration         8590 MCMCOBJ=   -6474.78222162215     
 iteration         8600 MCMCOBJ=   -6470.28515690140     
 iteration         8610 MCMCOBJ=   -6445.58922733420     
 iteration         8620 MCMCOBJ=   -6477.24821421376     
 iteration         8630 MCMCOBJ=   -6521.08535108178     
 iteration         8640 MCMCOBJ=   -6508.24156897315     
 iteration         8650 MCMCOBJ=   -6514.29607327896     
 iteration         8660 MCMCOBJ=   -6468.15240808882     
 iteration         8670 MCMCOBJ=   -6504.56402475427     
 iteration         8680 MCMCOBJ=   -6437.91057496625     
 iteration         8690 MCMCOBJ=   -6416.90786553187     
 iteration         8700 MCMCOBJ=   -6492.73380519287     
 iteration         8710 MCMCOBJ=   -6498.32284679305     
 iteration         8720 MCMCOBJ=   -6408.50502117499     
 iteration         8730 MCMCOBJ=   -6414.06426010563     
 iteration         8740 MCMCOBJ=   -6496.46048945750     
 iteration         8750 MCMCOBJ=   -6520.75932876453     
 iteration         8760 MCMCOBJ=   -6504.74509893461     
 iteration         8770 MCMCOBJ=   -6509.19541365200     
 iteration         8780 MCMCOBJ=   -6480.06556809989     
 iteration         8790 MCMCOBJ=   -6508.39028723415     
 iteration         8800 MCMCOBJ=   -6548.55657624331     
 iteration         8810 MCMCOBJ=   -6487.07932684229     
 iteration         8820 MCMCOBJ=   -6477.45308494117     
 iteration         8830 MCMCOBJ=   -6472.94371644975     
 iteration         8840 MCMCOBJ=   -6470.66399952260     
 iteration         8850 MCMCOBJ=   -6479.89475428905     
 iteration         8860 MCMCOBJ=   -6423.73290854998     
 iteration         8870 MCMCOBJ=   -6454.06625513226     
 iteration         8880 MCMCOBJ=   -6510.59477177475     
 iteration         8890 MCMCOBJ=   -6486.99609169343     
 iteration         8900 MCMCOBJ=   -6440.50338321561     
 iteration         8910 MCMCOBJ=   -6492.99904050563     
 iteration         8920 MCMCOBJ=   -6476.84378929261     
 iteration         8930 MCMCOBJ=   -6476.40609967679     
 iteration         8940 MCMCOBJ=   -6494.44623722276     
 iteration         8950 MCMCOBJ=   -6448.62068359266     
 iteration         8960 MCMCOBJ=   -6439.68851723434     
 iteration         8970 MCMCOBJ=   -6438.72540402988     
 iteration         8980 MCMCOBJ=   -6522.62730035875     
 iteration         8990 MCMCOBJ=   -6489.07551819955     
 iteration         9000 MCMCOBJ=   -6490.12848847572     
 iteration         9010 MCMCOBJ=   -6443.92773012107     
 iteration         9020 MCMCOBJ=   -6469.41629166467     
 iteration         9030 MCMCOBJ=   -6553.16737804695     
 iteration         9040 MCMCOBJ=   -6476.74833597137     
 iteration         9050 MCMCOBJ=   -6485.06921572937     
 iteration         9060 MCMCOBJ=   -6498.03582824779     
 iteration         9070 MCMCOBJ=   -6500.15593663128     
 iteration         9080 MCMCOBJ=   -6542.48108271972     
 iteration         9090 MCMCOBJ=   -6564.36482457405     
 iteration         9100 MCMCOBJ=   -6523.53527082516     
 iteration         9110 MCMCOBJ=   -6448.52031390378     
 iteration         9120 MCMCOBJ=   -6534.10914807230     
 iteration         9130 MCMCOBJ=   -6469.93683860818     
 iteration         9140 MCMCOBJ=   -6509.60888248388     
 iteration         9150 MCMCOBJ=   -6463.42698754020     
 iteration         9160 MCMCOBJ=   -6463.93943993946     
 iteration         9170 MCMCOBJ=   -6481.52918852498     
 iteration         9180 MCMCOBJ=   -6474.89838340666     
 iteration         9190 MCMCOBJ=   -6493.21546884806     
 iteration         9200 MCMCOBJ=   -6515.29500869303     
 iteration         9210 MCMCOBJ=   -6392.19369466117     
 iteration         9220 MCMCOBJ=   -6489.10697384196     
 iteration         9230 MCMCOBJ=   -6449.10019924854     
 iteration         9240 MCMCOBJ=   -6458.51664606635     
 iteration         9250 MCMCOBJ=   -6471.50046761024     
 iteration         9260 MCMCOBJ=   -6505.31844522559     
 iteration         9270 MCMCOBJ=   -6482.87256840995     
 iteration         9280 MCMCOBJ=   -6488.71428586604     
 iteration         9290 MCMCOBJ=   -6549.14232679974     
 iteration         9300 MCMCOBJ=   -6450.21815846652     
 iteration         9310 MCMCOBJ=   -6500.06274170671     
 iteration         9320 MCMCOBJ=   -6559.39855874808     
 iteration         9330 MCMCOBJ=   -6449.37776144162     
 iteration         9340 MCMCOBJ=   -6468.21974965554     
 iteration         9350 MCMCOBJ=   -6404.60014351515     
 iteration         9360 MCMCOBJ=   -6497.59165316487     
 iteration         9370 MCMCOBJ=   -6494.84950702001     
 iteration         9380 MCMCOBJ=   -6469.05497555620     
 iteration         9390 MCMCOBJ=   -6455.29257515727     
 iteration         9400 MCMCOBJ=   -6460.37456592396     
 iteration         9410 MCMCOBJ=   -6470.20416633507     
 iteration         9420 MCMCOBJ=   -6526.62941751265     
 iteration         9430 MCMCOBJ=   -6528.18780450210     
 iteration         9440 MCMCOBJ=   -6523.97800199471     
 iteration         9450 MCMCOBJ=   -6475.52974442426     
 iteration         9460 MCMCOBJ=   -6511.35081935609     
 iteration         9470 MCMCOBJ=   -6549.56059805261     
 iteration         9480 MCMCOBJ=   -6538.97175010150     
 iteration         9490 MCMCOBJ=   -6465.97700124772     
 iteration         9500 MCMCOBJ=   -6446.30738918594     
 iteration         9510 MCMCOBJ=   -6507.02888570236     
 iteration         9520 MCMCOBJ=   -6530.07039069118     
 iteration         9530 MCMCOBJ=   -6479.84248866648     
 iteration         9540 MCMCOBJ=   -6464.20645253758     
 iteration         9550 MCMCOBJ=   -6494.79206693001     
 iteration         9560 MCMCOBJ=   -6444.85633269297     
 iteration         9570 MCMCOBJ=   -6479.14006980156     
 iteration         9580 MCMCOBJ=   -6472.08702590078     
 iteration         9590 MCMCOBJ=   -6442.64639129176     
 iteration         9600 MCMCOBJ=   -6444.62580821474     
 iteration         9610 MCMCOBJ=   -6501.74925232540     
 iteration         9620 MCMCOBJ=   -6487.34116884315     
 iteration         9630 MCMCOBJ=   -6467.17660115307     
 iteration         9640 MCMCOBJ=   -6483.38645344381     
 iteration         9650 MCMCOBJ=   -6479.13765895513     
 iteration         9660 MCMCOBJ=   -6453.03108288709     
 iteration         9670 MCMCOBJ=   -6435.01630728295     
 iteration         9680 MCMCOBJ=   -6534.40517337601     
 iteration         9690 MCMCOBJ=   -6489.64953712413     
 iteration         9700 MCMCOBJ=   -6424.11803417656     
 iteration         9710 MCMCOBJ=   -6459.48353154172     
 iteration         9720 MCMCOBJ=   -6438.12241399239     
 iteration         9730 MCMCOBJ=   -6463.56589800351     
 iteration         9740 MCMCOBJ=   -6441.33958877896     
 iteration         9750 MCMCOBJ=   -6477.89835123369     
 iteration         9760 MCMCOBJ=   -6469.04905435196     
 iteration         9770 MCMCOBJ=   -6556.81916581734     
 iteration         9780 MCMCOBJ=   -6471.10789568646     
 iteration         9790 MCMCOBJ=   -6467.20114893933     
 iteration         9800 MCMCOBJ=   -6458.73377303666     
 iteration         9810 MCMCOBJ=   -6507.74676255197     
 iteration         9820 MCMCOBJ=   -6554.89072442473     
 iteration         9830 MCMCOBJ=   -6492.43798064333     
 iteration         9840 MCMCOBJ=   -6482.45309947071     
 iteration         9850 MCMCOBJ=   -6491.53508324553     
 iteration         9860 MCMCOBJ=   -6503.38294041610     
 iteration         9870 MCMCOBJ=   -6450.73173349021     
 iteration         9880 MCMCOBJ=   -6486.62173632571     
 iteration         9890 MCMCOBJ=   -6493.93138144107     
 iteration         9900 MCMCOBJ=   -6545.81659402544     
 iteration         9910 MCMCOBJ=   -6508.54419005746     
 iteration         9920 MCMCOBJ=   -6505.63885903598     
 iteration         9930 MCMCOBJ=   -6453.00215605494     
 iteration         9940 MCMCOBJ=   -6404.21168490662     
 iteration         9950 MCMCOBJ=   -6513.44033156208     
 iteration         9960 MCMCOBJ=   -6444.74188491641     
 iteration         9970 MCMCOBJ=   -6515.39177285872     
 iteration         9980 MCMCOBJ=   -6438.20631476796     
 iteration         9990 MCMCOBJ=   -6477.97787948004     
 iteration        10000 MCMCOBJ=   -6455.00834175455     
 
 #TERM:
 BURN-IN WAS COMPLETED
 STATISTICAL PORTION WAS COMPLETED

 ETABAR IS THE ARITHMETIC MEAN OF THE ETA-ESTIMATES,
 AND THE P-VALUE IS GIVEN FOR THE NULL HYPOTHESIS THAT THE TRUE MEAN IS 0.
 
 ETABAR:        -2.6313E-05  5.4299E-04  3.2774E-04 -5.5926E-05 -2.7576E-05 -6.4502E-04 -8.9926E-04  4.7099E-04
 SE:             6.9333E-02  5.3426E-02  4.0930E-02  6.5331E-02  5.7152E-02  5.7516E-02  6.4341E-02  6.1879E-02
 N:                      50          50          50          50          50          50          50          50
 
 P VAL.:         9.9970E-01  9.9189E-01  9.9361E-01  9.9932E-01  9.9962E-01  9.9105E-01  9.8885E-01  9.9393E-01
 
 ETASHRINKSD(%)  7.6588E+00  1.8290E+01  2.2789E+01  9.6878E+00  1.0978E+01  1.6093E+01  8.4692E+00  1.0132E+01
 ETASHRINKVR(%)  1.4731E+01  3.3235E+01  4.0385E+01  1.8437E+01  2.0751E+01  2.9597E+01  1.6221E+01  1.9237E+01
 EBVSHRINKSD(%)  6.5870E-01  8.8401E+00  8.3176E+00  2.1241E+00  1.8283E+00  7.4078E+00  6.2554E-01  1.7916E+00
 EBVSHRINKVR(%)  1.3131E+00  1.6899E+01  1.5943E+01  4.2032E+00  3.6232E+00  1.4267E+01  1.2472E+00  3.5511E+00
 RELATIVEINF(%)  9.7785E+01  7.1500E+01  7.9150E+01  9.3368E+01  9.1037E+01  7.6894E+01  1.0000E+02  9.1442E+01
 EPSSHRINKSD(%)  1.6873E+01  8.1582E+00
 EPSSHRINKVR(%)  3.0900E+01  1.5651E+01
 
  
 TOTAL DATA POINTS NORMALLY DISTRIBUTED (N):         1568
 N*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    2881.79124012985     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -6490.94969271459     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -3609.15845258474     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 TOTAL EFFECTIVE ETAS (NIND*NETA):                           400
 NIND*NETA*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    735.150826563738     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -6490.94969271459     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -5755.79886615085     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 PRIOR CONSTANT TO OBJECTIVE FUNCTION:    55.1779157436876     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -6490.94969271459     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -6435.77177697090     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 #TERE:
 Elapsed estimation  time in seconds:  3097.66
 Elapsed covariance  time in seconds:     0.00
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 #OBJT:**************                       AVERAGE VALUE OF LIKELIHOOD FUNCTION                     ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 





 #OBJV:********************************************         -6490.950       *********************************************
 #OBJS:********************************************            38.373 (STD) *********************************************
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 ********************                             FINAL PARAMETER ESTIMATE                           ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8     
 
         3.91E+00 -2.22E+00  5.53E-01 -1.83E-01  2.27E+00  2.39E-01  3.71E+00 -7.04E-01
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4      ETA5      ETA6      ETA7      ETA8     
 
 ETA1
+        2.82E-01
 
 ETA2
+       -3.48E-02  2.14E-01
 
 ETA3
+        4.52E-02 -9.80E-03  1.41E-01
 
 ETA4
+        3.02E-02  5.48E-02 -1.43E-02  2.62E-01
 
 ETA5
+        2.72E-02  1.65E-02 -7.45E-04 -3.22E-02  2.06E-01
 
 ETA6
+       -2.68E-02  4.75E-03  1.58E-02  1.46E-02 -7.38E-02  2.35E-01
 
 ETA7
+        2.97E-02 -4.76E-02  3.19E-02 -7.39E-02  2.37E-02 -3.16E-04  2.47E-01
 
 ETA8
+        9.58E-02  7.36E-02  4.13E-02  4.61E-02  3.99E-03 -5.14E-02  5.74E-02  2.37E-01
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1      EPS2     
 
 EPS1
+        9.32E-03
 
 EPS2
+        0.00E+00  2.24E-02
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4      ETA5      ETA6      ETA7      ETA8     
 
 ETA1
+        5.28E-01
 
 ETA2
+       -1.39E-01  4.59E-01
 
 ETA3
+        2.27E-01 -5.67E-02  3.72E-01
 
 ETA4
+        1.10E-01  2.29E-01 -7.67E-02  5.09E-01
 
 ETA5
+        1.12E-01  7.96E-02 -5.94E-03 -1.38E-01  4.51E-01
 
 ETA6
+       -1.05E-01  2.30E-02  8.93E-02  5.82E-02 -3.35E-01  4.81E-01
 
 ETA7
+        1.11E-01 -2.01E-01  1.70E-01 -2.88E-01  1.03E-01 -1.66E-03  4.94E-01
 
 ETA8
+        3.68E-01  3.27E-01  2.23E-01  1.83E-01  1.67E-02 -2.16E-01  2.35E-01  4.84E-01
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1      EPS2     
 
 EPS1
+        9.65E-02
 
 EPS2
+        0.00E+00  1.50E-01
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 ********************                STANDARD ERROR OF ESTIMATE (From Sample Variance)               ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8     
 
         7.63E-02  7.50E-02  5.94E-02  7.38E-02  6.57E-02  7.48E-02  7.16E-02  7.02E-02
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4      ETA5      ETA6      ETA7      ETA8     
 
 ETA1
+        5.88E-02
 
 ETA2
+        4.03E-02  5.63E-02
 
 ETA3
+        3.19E-02  2.91E-02  3.48E-02
 
 ETA4
+        4.06E-02  3.96E-02  3.20E-02  5.79E-02
 
 ETA5
+        3.56E-02  3.30E-02  2.72E-02  3.51E-02  4.43E-02
 
 ETA6
+        4.07E-02  3.77E-02  2.99E-02  3.99E-02  3.56E-02  5.65E-02
 
 ETA7
+        3.91E-02  4.06E-02  2.96E-02  3.94E-02  3.39E-02  3.75E-02  5.14E-02
 
 ETA8
+        4.09E-02  3.74E-02  3.01E-02  3.82E-02  3.36E-02  3.81E-02  3.69E-02  5.14E-02
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1      EPS2     
 
 EPS1
+        6.44E-04
 
 EPS2
+        0.00E+00  1.19E-03
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4      ETA5      ETA6      ETA7      ETA8     
 
 ETA1
+        5.42E-02
 
 ETA2
+        1.52E-01  5.89E-02
 
 ETA3
+        1.46E-01  1.60E-01  4.52E-02
 
 ETA4
+        1.41E-01  1.47E-01  1.60E-01  5.53E-02
 
 ETA5
+        1.40E-01  1.51E-01  1.54E-01  1.42E-01  4.76E-02
 
 ETA6
+        1.51E-01  1.62E-01  1.58E-01  1.54E-01  1.38E-01  5.70E-02
 
 ETA7
+        1.40E-01  1.55E-01  1.47E-01  1.31E-01  1.41E-01  1.50E-01  5.06E-02
 
 ETA8
+        1.25E-01  1.41E-01  1.45E-01  1.40E-01  1.46E-01  1.45E-01  1.35E-01  5.15E-02
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1      EPS2     
 
 EPS1
+        3.33E-03
 
 EPS2
+        0.00E+00  3.97E-03
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 ********************               COVARIANCE MATRIX OF ESTIMATE (From Sample Variance)             ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      OM11      OM12      OM13      OM14  
             OM15      OM16      OM17      OM18      OM22      OM23      OM24      OM25      OM26      OM27      OM28      OM33  
            OM34      OM35      OM36      OM37      OM38      OM44      OM45      OM46      OM47      OM48      OM55      OM56  
             OM57      OM58      OM66      OM67      OM68      OM77      OM78      OM88      SG11      SG12      SG22  
 
 TH 1
+        5.82E-03
 
 TH 2
+       -7.99E-04  5.63E-03
 
 TH 3
+        6.50E-04  1.47E-05  3.53E-03
 
 TH 4
+        5.59E-04  9.58E-04  6.07E-05  5.45E-03
 
 TH 5
+        4.90E-04  2.86E-04  7.63E-05 -5.61E-04  4.31E-03
 
 TH 6
+       -3.75E-04 -1.09E-04  1.98E-04  3.02E-04 -1.15E-03  5.60E-03
 
 TH 7
+        5.92E-04 -1.15E-03  5.99E-04 -1.52E-03  5.33E-04  1.49E-04  5.13E-03
 
 TH 8
+        1.93E-03  1.18E-03  9.93E-04  9.05E-04  1.38E-04 -9.81E-04  1.20E-03  4.93E-03
 
 OM11
+        6.17E-05  9.13E-06 -6.62E-05  4.41E-05  1.57E-05  1.11E-05 -1.26E-05  3.43E-05  3.46E-03
 
 OM12
+       -8.75E-06  2.13E-04 -3.28E-05 -5.21E-05 -2.84E-05 -1.48E-05 -4.71E-05 -5.88E-05 -4.62E-04  1.63E-03
 
 OM13
+        1.87E-05  3.44E-05  5.99E-05  5.07E-05  7.85E-06  3.37E-05  1.29E-05  2.41E-05  4.39E-04 -6.82E-05  1.02E-03
 
 OM14
+        3.86E-05  4.68E-05 -1.06E-05  3.93E-05 -1.85E-05  3.10E-05  1.89E-05  3.03E-05  3.40E-04  2.97E-04 -8.71E-06  1.65E-03
 
 OM15
+       -6.67E-05  3.13E-05  2.13E-05 -1.49E-05  1.46E-05  4.61E-05 -2.40E-05 -3.19E-05  3.24E-04  6.02E-05  3.14E-05 -1.56E-04
          1.27E-03
 
 OM16
+        1.59E-05  2.13E-05  3.27E-05 -4.24E-06  2.36E-05 -2.20E-05  4.48E-05  6.52E-05 -2.43E-04 -2.00E-05  3.60E-05  8.91E-05
         -3.75E-04  1.65E-03
 
 OM17
+        1.83E-05 -1.98E-05  4.17E-05  1.01E-05 -1.80E-05  3.71E-05  2.68E-05  2.82E-05  3.94E-04 -3.84E-04  2.33E-04 -4.43E-04
          1.56E-04  1.29E-05  1.53E-03
 
 OM18
+        5.23E-05 -1.42E-05 -2.53E-05  3.38E-05 -1.60E-05  2.74E-05 -2.33E-05  3.10E-05  1.13E-03  3.15E-04  3.53E-04  3.34E-04
          8.27E-05 -3.37E-04  4.46E-04  1.68E-03
 
 OM22
+        1.95E-05 -7.59E-04  1.18E-04  8.70E-05 -2.83E-06  3.98E-05  5.53E-05  1.27E-04  1.04E-04 -6.54E-04  5.38E-05 -1.04E-04
         -1.47E-05  7.02E-06  1.52E-04 -9.33E-05  3.17E-03
 
 OM23
+       -5.25E-06  1.77E-04  9.18E-05  6.57E-05 -1.60E-05  8.97E-06 -4.03E-05 -3.24E-05 -8.02E-05  2.73E-04 -1.05E-04  7.30E-05
          5.02E-06 -1.43E-05 -7.73E-05  3.00E-05 -1.65E-04  8.49E-04
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      OM11      OM12      OM13      OM14  
             OM15      OM16      OM17      OM18      OM22      OM23      OM24      OM25      OM26      OM27      OM28      OM33  
            OM34      OM35      OM36      OM37      OM38      OM44      OM45      OM46      OM47      OM48      OM55      OM56  
             OM57      OM58      OM66      OM67      OM68      OM77      OM78      OM88      SG11      SG12      SG22  
 
 OM24
+        2.05E-05 -4.19E-04  9.53E-05  3.13E-05 -1.39E-05  5.41E-05  2.53E-05  3.02E-05 -5.10E-05 -3.80E-05  4.64E-05 -1.72E-04
          3.76E-05 -3.54E-05  5.69E-05  2.30E-05  8.57E-04 -6.48E-05  1.57E-03
 
 OM25
+        5.93E-05  2.70E-05 -4.07E-05 -2.84E-05  1.47E-05 -2.59E-05  2.87E-05  7.16E-05 -1.81E-05  1.37E-04 -1.32E-05  5.46E-05
         -1.45E-04  4.17E-05 -3.78E-05  3.87E-05  7.73E-05  1.78E-05 -1.78E-04  1.09E-03
 
 OM26
+        4.87E-07  5.71E-05  2.18E-05  1.60E-05  1.31E-05 -1.05E-05 -2.61E-05 -2.26E-05  1.74E-05 -1.09E-04  2.44E-06 -2.93E-05
          6.57E-05 -1.99E-04  4.16E-05  3.55E-05 -4.80E-05  7.21E-05  6.66E-05 -3.58E-04  1.42E-03
 
 OM27
+       -2.08E-05  5.70E-04 -6.71E-05 -5.27E-05  5.89E-06  1.19E-06 -4.05E-05 -5.50E-05 -4.64E-05  3.61E-04 -4.71E-05  1.08E-04
         -4.58E-06 -2.68E-05 -2.38E-04 -1.90E-06 -9.12E-04  2.35E-04 -6.53E-04  1.87E-04 -1.59E-05  1.65E-03
 
 OM28
+        2.87E-05  8.27E-05  3.62E-05  1.15E-05 -2.44E-05  3.18E-05 -1.80E-05  2.11E-05 -1.36E-04  4.44E-04 -1.35E-05  7.42E-05
          3.89E-05  3.03E-05 -1.58E-04 -6.95E-05  7.12E-04  2.39E-04  2.84E-04  7.80E-05 -3.39E-04  3.17E-04  1.40E-03
 
 OM33
+        6.03E-05 -8.79E-05 -1.20E-04 -9.52E-05 -2.19E-05  7.74E-05  1.70E-05 -3.96E-06  7.02E-05 -2.96E-05  3.10E-04  1.40E-05
         -6.46E-06  9.90E-06  5.26E-05  6.65E-05  8.56E-05 -3.11E-05  3.85E-05  6.47E-06 -2.32E-05 -5.67E-05  1.66E-05  1.21E-03
 
 OM34
+       -4.38E-05  1.10E-05  2.12E-04  1.20E-04  9.07E-06 -6.18E-06  1.05E-05  3.43E-05  1.81E-05  5.66E-05  1.38E-04  2.49E-04
         -1.90E-05  2.46E-05 -5.84E-05  4.08E-05  3.70E-05  2.05E-04  1.82E-05 -1.08E-05  3.39E-05  1.11E-05  6.60E-05 -7.04E-05
         1.02E-03
 
 OM35
+        1.41E-05 -7.06E-05 -6.47E-05 -4.84E-05  3.50E-05  1.24E-05  2.54E-05 -1.29E-05  1.58E-05 -1.69E-06  7.33E-05 -3.36E-05
          1.81E-04 -4.96E-05  3.82E-05  1.91E-05  3.28E-06  7.53E-05  2.72E-07  1.80E-07 -8.94E-06  9.77E-06  2.06E-05  4.38E-05
        -1.52E-04  7.39E-04
 
 OM36
+       -2.29E-05  5.23E-06  1.01E-05 -3.86E-05 -4.71E-06 -4.73E-05 -8.15E-06 -1.82E-05 -2.53E-05 -1.42E-05 -7.69E-05  1.73E-05
         -5.77E-05  2.43E-04 -5.92E-07 -6.05E-05  3.80E-05  1.62E-06  1.90E-05 -9.97E-06  8.85E-06 -2.46E-05  9.93E-06  1.25E-05
         7.49E-05 -2.50E-04  8.96E-04
 
 OM37
+        1.84E-05 -1.17E-06 -1.19E-04 -8.41E-05 -2.49E-05 -7.67E-06  5.55E-06  7.76E-06  6.83E-05 -7.56E-05  1.06E-04 -7.59E-05
          1.40E-05  8.25E-06  2.65E-04  8.29E-05  2.39E-05 -2.01E-04 -2.22E-05  5.86E-06 -2.34E-05 -6.91E-05 -8.33E-05  2.18E-04
        -2.94E-04  1.07E-04 -8.44E-06  8.75E-04
 
 OM38
+        1.10E-05  2.48E-05  6.52E-05  7.41E-05 -1.03E-05  6.11E-05 -1.18E-05  3.55E-06  1.27E-04  5.22E-05  4.05E-04  5.07E-05
         -2.63E-07 -1.60E-05  1.13E-04  2.94E-04  4.49E-05  2.55E-04  2.75E-05  1.67E-05 -1.14E-06  2.99E-06  7.04E-05  3.71E-04
         1.85E-04  3.44E-05 -1.97E-04  2.20E-04  9.07E-04
 
 OM44
+        3.03E-05  6.22E-06  3.22E-04  2.72E-04  2.64E-05  1.01E-04  1.97E-05  5.27E-05  7.77E-05  7.46E-05  8.33E-05  3.57E-04
         -7.70E-06  1.47E-05 -6.91E-05  1.18E-04  2.44E-04  2.39E-05  6.68E-04 -9.53E-05  5.55E-05 -2.43E-04  1.46E-04  6.85E-06
         6.24E-05 -4.38E-05  2.04E-06 -1.02E-05  6.21E-05  3.36E-03
 
 OM45
+        4.20E-05  4.46E-05  7.68E-06 -2.36E-05  5.62E-05 -1.06E-05 -2.24E-05  2.96E-05  5.02E-05  4.37E-05  6.47E-06  1.57E-04
          9.66E-05 -1.93E-06 -2.00E-05  8.49E-05 -3.05E-06  2.12E-05  3.65E-05  2.50E-04 -4.72E-05  4.78E-05  1.31E-05 -1.50E-05
        -5.66E-06 -2.40E-05 -2.24E-06 -8.71E-06  8.97E-06 -3.99E-04  1.23E-03
 
 OM46
+        2.40E-05  6.44E-05  1.07E-05  6.29E-05  3.04E-05  1.11E-04  1.04E-05  7.92E-05 -4.90E-05 -4.10E-05  2.51E-05 -1.49E-04
          8.53E-06  1.96E-04  6.72E-05 -6.71E-05 -3.91E-06  6.11E-06  1.99E-05 -5.57E-05  3.08E-04 -9.75E-06 -3.63E-05  2.36E-05
         4.67E-05 -1.66E-05 -1.63E-05 -2.04E-05  1.33E-05  2.33E-04 -3.60E-04  1.59E-03
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      OM11      OM12      OM13      OM14  
             OM15      OM16      OM17      OM18      OM22      OM23      OM24      OM25      OM26      OM27      OM28      OM33  
            OM34      OM35      OM36      OM37      OM38      OM44      OM45      OM46      OM47      OM48      OM55      OM56  
             OM57      OM58      OM66      OM67      OM68      OM77      OM78      OM88      SG11      SG12      SG22  
 
 OM47
+        2.96E-05  1.79E-05 -7.63E-05 -7.65E-05 -1.25E-05 -1.79E-05  3.92E-06  2.96E-05  1.40E-05  1.18E-05 -2.47E-05  1.46E-04
         -2.21E-05  1.92E-05  1.14E-04  6.33E-05 -1.93E-04  2.82E-05 -4.32E-04  9.80E-05 -5.76E-06  4.49E-04  1.91E-05 -1.82E-05
         1.52E-04 -3.22E-05  1.71E-05 -7.52E-05 -1.68E-05 -8.93E-04  2.33E-04 -3.99E-05  1.55E-03
 
 OM48
+        5.41E-05  3.41E-05  5.15E-05  3.26E-05  5.79E-06  6.70E-05  3.28E-05  1.78E-05  1.18E-04  1.62E-04  4.09E-05  6.26E-04
         -3.82E-05  1.27E-05 -1.22E-04  2.54E-04  1.22E-04  7.73E-05  4.13E-04 -2.81E-05 -1.57E-05  3.95E-06  3.50E-04  2.66E-05
         2.95E-04 -3.81E-05  1.54E-05 -1.02E-04  4.26E-05  5.82E-04  2.80E-05 -2.92E-04  2.50E-04  1.46E-03
 
 OM55
+        1.59E-05 -4.27E-05 -4.53E-06 -4.78E-05 -1.81E-05 -3.71E-05 -1.89E-05 -2.08E-05  4.70E-05  7.85E-06  2.51E-05 -4.43E-05
          2.56E-04 -8.06E-05  3.42E-05  2.93E-06  8.41E-05  1.23E-05 -2.09E-05  1.26E-04 -1.18E-05 -3.73E-05 -5.55E-06  3.94E-05
         1.59E-06  4.19E-05 -4.20E-05 -2.09E-05  1.67E-05  9.20E-05 -2.81E-04  7.11E-05 -6.36E-05 -3.32E-05  1.96E-03
 
 OM56
+        4.15E-05 -6.03E-05  1.04E-05 -6.79E-06  2.84E-05 -5.71E-05  2.46E-05  1.80E-05 -3.09E-05  1.66E-05 -8.27E-06  3.10E-05
         -1.53E-04  1.68E-04  4.59E-06 -8.24E-06  5.51E-06 -2.11E-08  2.89E-05 -2.47E-05  7.54E-05 -2.31E-06 -1.52E-05 -1.82E-05
        -2.73E-06  4.40E-05  1.49E-05  7.97E-06 -8.78E-06 -1.33E-05  1.13E-04 -2.15E-04  3.71E-06  3.95E-05 -5.67E-04  1.27E-03
 
 OM57
+       -4.03E-05 -1.69E-05  1.10E-06  2.29E-05 -2.12E-06  4.74E-06  9.43E-06 -6.57E-05  4.12E-05 -2.62E-05  1.10E-05 -7.76E-05
          1.57E-04 -1.51E-05  1.55E-04  3.12E-06 -6.01E-06 -1.98E-06  3.62E-05 -2.47E-04  7.93E-05 -1.43E-05 -2.17E-06  1.35E-05
        -2.54E-05  1.43E-04 -3.32E-05  2.55E-05 -3.56E-07  1.18E-04 -3.51E-04  9.29E-05 -2.02E-04 -5.03E-05  2.43E-04 -2.18E-05
          1.15E-03
 
 OM58
+       -6.35E-06 -2.40E-05  1.23E-05 -3.48E-05  2.07E-05  1.01E-05  3.63E-06  1.03E-06  1.16E-04  5.50E-05  2.46E-05 -1.17E-05
          4.32E-04 -1.30E-04  8.26E-05  1.58E-04  4.34E-05  3.21E-05 -2.72E-05  3.21E-04 -9.78E-05  4.42E-05  9.45E-05  2.47E-05
        -3.08E-05  2.08E-04 -5.81E-05  2.61E-05  3.11E-05 -5.72E-05  2.07E-04 -1.75E-05 -1.94E-06 -1.33E-04  7.42E-05 -2.46E-04
          2.94E-04  1.13E-03
 
 OM66
+       -2.20E-05 -1.13E-05 -4.52E-05  2.72E-05  1.76E-05  6.81E-05 -1.75E-05 -6.88E-05  4.04E-05 -7.27E-06  1.22E-05  1.58E-05
          4.18E-05 -2.04E-04 -3.04E-05  2.25E-05  5.47E-05  2.47E-06 -1.07E-05  2.62E-05 -4.90E-05 -4.34E-05  3.21E-05  5.86E-05
         1.37E-05 -4.59E-06  1.11E-04  4.49E-06  1.50E-05  4.73E-05 -2.55E-05  1.60E-04 -2.32E-05 -1.30E-05  1.27E-04 -7.54E-04
         -2.61E-05  9.68E-05  3.19E-03
 
 OM67
+        1.12E-05 -2.86E-05 -3.24E-05 -7.05E-06 -7.11E-06 -1.45E-04 -3.34E-05  1.54E-05 -6.84E-05  4.44E-05 -4.28E-05  5.51E-05
         -6.31E-05  1.76E-04 -1.40E-04 -8.52E-05 -1.99E-05 -1.30E-05 -1.06E-05  7.54E-05 -3.43E-04 -6.81E-06  4.66E-05 -6.92E-06
        -1.02E-05 -2.43E-05  1.62E-04  4.99E-05 -3.05E-05 -9.71E-05  9.79E-05 -4.32E-04  6.65E-05  7.54E-05 -6.10E-05  1.72E-04
         -3.25E-04 -1.06E-04  2.17E-05  1.41E-03
 
 OM68
+        5.37E-06  2.69E-05  1.04E-05  2.82E-05  3.45E-05 -2.25E-06  1.02E-05  4.08E-05 -1.13E-04 -3.30E-05 -1.15E-05  8.01E-06
         -1.18E-04  5.89E-04 -1.23E-06 -2.29E-04 -5.79E-05  4.70E-06  8.84E-06 -8.37E-05  4.33E-04 -2.18E-05 -1.07E-04 -5.78E-06
         3.96E-05 -7.48E-05  2.78E-04  1.15E-05 -9.75E-06  4.21E-06 -4.26E-05  2.84E-04  2.81E-05  3.79E-05 -5.69E-05  1.11E-04
         -7.22E-05 -3.39E-04 -6.17E-04  3.01E-04  1.45E-03
 
 OM77
+       -2.27E-05 -9.15E-05  5.49E-05  6.95E-05 -4.74E-05  1.84E-05 -2.50E-05 -8.00E-05  7.43E-05 -1.14E-04  6.85E-05 -1.40E-04
          2.82E-05  5.72E-07  3.36E-04  7.96E-05  2.59E-04 -6.54E-05  2.51E-04 -8.27E-05  7.67E-06 -6.72E-04 -1.58E-04  8.17E-05
        -9.34E-05  5.33E-05 -8.31E-06  3.35E-04  9.87E-05  3.26E-04 -1.10E-04 -7.15E-06 -7.92E-04 -1.75E-04  1.04E-04 -9.76E-06
          2.71E-04  6.96E-05  5.03E-05  2.34E-05  3.65E-06  2.65E-03
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      OM11      OM12      OM13      OM14  
             OM15      OM16      OM17      OM18      OM22      OM23      OM24      OM25      OM26      OM27      OM28      OM33  
            OM34      OM35      OM36      OM37      OM38      OM44      OM45      OM46      OM47      OM48      OM55      OM56  
             OM57      OM58      OM66      OM67      OM68      OM77      OM78      OM88      SG11      SG12      SG22  
 
 OM78
+        1.56E-05 -7.80E-05  4.20E-05  8.14E-06 -2.30E-05  3.27E-05  1.72E-05  2.62E-05  1.40E-04 -8.00E-05  1.01E-04 -1.48E-04
          5.17E-05 -1.84E-05  5.60E-04  2.92E-04 -9.76E-05 -1.13E-05 -1.39E-04  5.41E-05  8.15E-05  2.57E-04 -1.97E-04  6.41E-05
        -5.50E-05  4.31E-05 -5.13E-05  2.99E-04  2.24E-04 -1.41E-04  3.39E-05  7.53E-05  1.81E-04 -3.29E-04  2.48E-05 -1.82E-05
          4.21E-05  1.42E-04 -2.68E-05 -2.54E-04 -5.66E-05  6.06E-04  1.36E-03
 
 OM88
+        6.52E-05 -1.62E-05  6.07E-05  3.00E-05  3.94E-06  1.23E-04  1.28E-05  3.02E-05  3.95E-04  2.73E-04  2.14E-04  1.72E-04
          6.28E-05 -1.71E-04  2.75E-04  1.00E-03  2.58E-04  1.66E-04  1.65E-04  5.13E-05 -1.65E-04  1.86E-04  7.66E-04  1.36E-04
         7.83E-05  4.13E-05 -1.27E-04  1.25E-04  5.12E-04  2.05E-04  5.00E-05 -1.02E-04  1.01E-04  5.03E-04  5.92E-05 -2.78E-05
         -7.63E-06  1.27E-04  1.59E-04 -1.45E-04 -5.79E-04  1.69E-04  6.42E-04  2.64E-03
 
 SG11
+       -3.90E-07  1.73E-07 -5.27E-07 -9.12E-07 -1.01E-07  7.38E-07 -5.61E-07 -1.02E-06  3.80E-07  6.02E-07  3.49E-07 -4.67E-08
          8.85E-09 -6.84E-11  1.53E-07 -1.34E-09 -6.74E-07 -7.86E-07  7.82E-08 -1.10E-07 -2.74E-07 -6.70E-08 -4.71E-07  3.59E-07
        -5.22E-07  8.46E-09  9.65E-08  2.27E-07 -2.20E-08 -1.23E-06 -3.34E-07 -2.62E-07 -6.98E-08 -2.34E-07  1.18E-07  1.37E-07
          1.19E-07  1.94E-07  9.45E-07  6.33E-07 -1.76E-07  1.84E-07  2.57E-07 -1.33E-07  4.14E-07
 
 SG12
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
        ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 SG22
+        1.55E-07 -3.09E-07 -1.51E-06  3.99E-07  5.87E-08 -9.79E-07 -1.81E-08  7.69E-07  1.84E-06 -2.34E-07  4.04E-07  1.71E-07
         -1.54E-07  1.76E-07  5.19E-08  6.29E-07  1.48E-06  3.17E-07 -3.86E-07 -4.43E-07 -4.34E-07  4.95E-08 -5.90E-07  8.45E-07
         4.08E-07 -1.12E-06  2.97E-07 -6.01E-07 -3.67E-09  6.12E-07 -1.48E-07  3.64E-07  1.50E-07 -6.64E-08 -7.74E-07  3.44E-07
         -5.42E-07 -1.16E-07 -3.99E-06 -1.83E-07  9.77E-07 -3.43E-07 -3.45E-07 -8.53E-07 -2.55E-08  0.00E+00  1.41E-06
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 ********************              CORRELATION MATRIX OF ESTIMATE (From Sample Variance)             ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      OM11      OM12      OM13      OM14  
             OM15      OM16      OM17      OM18      OM22      OM23      OM24      OM25      OM26      OM27      OM28      OM33  
            OM34      OM35      OM36      OM37      OM38      OM44      OM45      OM46      OM47      OM48      OM55      OM56  
             OM57      OM58      OM66      OM67      OM68      OM77      OM78      OM88      SG11      SG12      SG22  
 
 TH 1
+        7.63E-02
 
 TH 2
+       -1.40E-01  7.50E-02
 
 TH 3
+        1.43E-01  3.31E-03  5.94E-02
 
 TH 4
+        9.92E-02  1.73E-01  1.38E-02  7.38E-02
 
 TH 5
+        9.78E-02  5.81E-02  1.96E-02 -1.16E-01  6.57E-02
 
 TH 6
+       -6.57E-02 -1.93E-02  4.45E-02  5.47E-02 -2.35E-01  7.48E-02
 
 TH 7
+        1.08E-01 -2.13E-01  1.41E-01 -2.87E-01  1.13E-01  2.78E-02  7.16E-02
 
 TH 8
+        3.60E-01  2.24E-01  2.38E-01  1.75E-01  3.00E-02 -1.87E-01  2.39E-01  7.02E-02
 
 OM11
+        1.38E-02  2.07E-03 -1.89E-02  1.01E-02  4.07E-03  2.52E-03 -2.98E-03  8.30E-03  5.88E-02
 
 OM12
+       -2.85E-03  7.04E-02 -1.37E-02 -1.75E-02 -1.07E-02 -4.90E-03 -1.63E-02 -2.07E-02 -1.95E-01  4.03E-02
 
 OM13
+        7.68E-03  1.44E-02  3.16E-02  2.15E-02  3.75E-03  1.41E-02  5.64E-03  1.08E-02  2.34E-01 -5.29E-02  3.19E-02
 
 OM14
+        1.25E-02  1.54E-02 -4.40E-03  1.31E-02 -6.92E-03  1.02E-02  6.49E-03  1.06E-02  1.42E-01  1.82E-01 -6.72E-03  4.06E-02
 
 OM15
+       -2.45E-02  1.17E-02  1.01E-02 -5.65E-03  6.23E-03  1.73E-02 -9.42E-03 -1.28E-02  1.54E-01  4.19E-02  2.76E-02 -1.08E-01
          3.56E-02
 
 OM16
+        5.12E-03  6.98E-03  1.35E-02 -1.41E-03  8.83E-03 -7.24E-03  1.54E-02  2.28E-02 -1.02E-01 -1.22E-02  2.77E-02  5.40E-02
         -2.59E-01  4.07E-02
 
 OM17
+        6.14E-03 -6.75E-03  1.79E-02  3.51E-03 -6.99E-03  1.27E-02  9.58E-03  1.02E-02  1.71E-01 -2.44E-01  1.86E-01 -2.79E-01
          1.12E-01  8.11E-03  3.91E-02
 
 OM18
+        1.67E-02 -4.63E-03 -1.04E-02  1.12E-02 -5.96E-03  8.94E-03 -7.94E-03  1.08E-02  4.68E-01  1.91E-01  2.70E-01  2.01E-01
          5.67E-02 -2.02E-01  2.79E-01  4.09E-02
 
 OM22
+        4.55E-03 -1.80E-01  3.54E-02  2.09E-02 -7.65E-04  9.46E-03  1.37E-02  3.22E-02  3.13E-02 -2.88E-01  3.00E-02 -4.54E-02
         -7.32E-03  3.07E-03  6.93E-02 -4.05E-02  5.63E-02
 
 OM23
+       -2.36E-03  8.08E-02  5.31E-02  3.05E-02 -8.38E-03  4.12E-03 -1.93E-02 -1.58E-02 -4.68E-02  2.32E-01 -1.13E-01  6.17E-02
          4.84E-03 -1.21E-02 -6.78E-02  2.52E-02 -1.01E-01  2.91E-02
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      OM11      OM12      OM13      OM14  
             OM15      OM16      OM17      OM18      OM22      OM23      OM24      OM25      OM26      OM27      OM28      OM33  
            OM34      OM35      OM36      OM37      OM38      OM44      OM45      OM46      OM47      OM48      OM55      OM56  
             OM57      OM58      OM66      OM67      OM68      OM77      OM78      OM88      SG11      SG12      SG22  
 
 OM24
+        6.78E-03 -1.41E-01  4.05E-02  1.07E-02 -5.33E-03  1.83E-02  8.93E-03  1.09E-02 -2.19E-02 -2.38E-02  3.67E-02 -1.07E-01
          2.67E-02 -2.20E-02  3.68E-02  1.42E-02  3.85E-01 -5.62E-02  3.96E-02
 
 OM25
+        2.36E-02  1.09E-02 -2.08E-02 -1.17E-02  6.81E-03 -1.05E-02  1.21E-02  3.09E-02 -9.35E-03  1.03E-01 -1.25E-02  4.08E-02
         -1.24E-01  3.11E-02 -2.93E-02  2.87E-02  4.17E-02  1.85E-02 -1.37E-01  3.30E-02
 
 OM26
+        1.69E-04  2.02E-02  9.72E-03  5.75E-03  5.29E-03 -3.73E-03 -9.67E-03 -8.52E-03  7.84E-03 -7.18E-02  2.02E-03 -1.91E-02
          4.89E-02 -1.30E-01  2.82E-02  2.30E-02 -2.26E-02  6.56E-02  4.46E-02 -2.88E-01  3.77E-02
 
 OM27
+       -6.72E-03  1.87E-01 -2.78E-02 -1.76E-02  2.21E-03  3.92E-04 -1.39E-02 -1.93E-02 -1.94E-02  2.20E-01 -3.63E-02  6.53E-02
         -3.16E-03 -1.62E-02 -1.50E-01 -1.14E-03 -3.99E-01  1.99E-01 -4.06E-01  1.40E-01 -1.04E-02  4.06E-02
 
 OM28
+        1.01E-02  2.95E-02  1.63E-02  4.17E-03 -9.94E-03  1.14E-02 -6.73E-03  8.05E-03 -6.19E-02  2.94E-01 -1.13E-02  4.90E-02
          2.92E-02  2.00E-02 -1.08E-01 -4.54E-02  3.39E-01  2.19E-01  1.92E-01  6.34E-02 -2.41E-01  2.09E-01  3.74E-02
 
 OM33
+        2.27E-02 -3.36E-02 -5.80E-02 -3.70E-02 -9.57E-03  2.97E-02  6.80E-03 -1.62E-03  3.42E-02 -2.11E-02  2.79E-01  9.92E-03
         -5.20E-03  6.99E-03  3.86E-02  4.66E-02  4.37E-02 -3.06E-02  2.79E-02  5.64E-03 -1.77E-02 -4.01E-02  1.28E-02  3.48E-02
 
 OM34
+       -1.80E-02  4.57E-03  1.12E-01  5.09E-02  4.32E-03 -2.58E-03  4.61E-03  1.53E-02  9.61E-03  4.39E-02  1.35E-01  1.92E-01
         -1.67E-02  1.90E-02 -4.67E-02  3.12E-02  2.06E-02  2.20E-01  1.44E-02 -1.02E-02  2.81E-02  8.54E-03  5.53E-02 -6.32E-02
         3.20E-02
 
 OM35
+        6.78E-03 -3.46E-02 -4.01E-02 -2.41E-02  1.96E-02  6.11E-03  1.30E-02 -6.73E-03  9.87E-03 -1.54E-03  8.44E-02 -3.05E-02
          1.86E-01 -4.49E-02  3.59E-02  1.71E-02  2.14E-03  9.50E-02  2.52E-04  2.01E-04 -8.72E-03  8.85E-03  2.03E-02  4.62E-02
        -1.75E-01  2.72E-02
 
 OM36
+       -1.00E-02  2.33E-03  5.70E-03 -1.75E-02 -2.39E-03 -2.11E-02 -3.80E-03 -8.66E-03 -1.44E-02 -1.18E-02 -8.05E-02  1.42E-02
         -5.41E-02  2.00E-01 -5.05E-04 -4.94E-02  2.26E-02  1.86E-03  1.60E-02 -1.01E-02  7.84E-03 -2.02E-02  8.88E-03  1.20E-02
         7.84E-02 -3.08E-01  2.99E-02
 
 OM37
+        8.17E-03 -5.27E-04 -6.76E-02 -3.85E-02 -1.28E-02 -3.47E-03  2.62E-03  3.74E-03  3.92E-02 -6.33E-02  1.12E-01 -6.32E-02
          1.33E-02  6.86E-03  2.29E-01  6.84E-02  1.44E-02 -2.33E-01 -1.90E-02  6.01E-03 -2.09E-02 -5.75E-02 -7.54E-02  2.12E-01
        -3.11E-01  1.33E-01 -9.53E-03  2.96E-02
 
 OM38
+        4.78E-03  1.10E-02  3.64E-02  3.33E-02 -5.22E-03  2.71E-02 -5.47E-03  1.68E-03  7.16E-02  4.29E-02  4.21E-01  4.14E-02
         -2.45E-04 -1.30E-02  9.55E-02  2.39E-01  2.65E-02  2.90E-01  2.30E-02  1.69E-02 -1.00E-03  2.44E-03  6.26E-02  3.54E-01
         1.93E-01  4.20E-02 -2.19E-01  2.47E-01  3.01E-02
 
 OM44
+        6.86E-03  1.43E-03  9.35E-02  6.35E-02  6.93E-03  2.32E-02  4.74E-03  1.30E-02  2.28E-02  3.19E-02  4.50E-02  1.52E-01
         -3.73E-03  6.24E-03 -3.05E-02  4.98E-02  7.49E-02  1.42E-02  2.91E-01 -4.99E-02  2.54E-02 -1.03E-01  6.75E-02  3.40E-03
         3.37E-02 -2.78E-02  1.17E-03 -5.94E-03  3.56E-02  5.79E-02
 
 OM45
+        1.57E-02  1.69E-02  3.69E-03 -9.11E-03  2.44E-02 -4.04E-03 -8.92E-03  1.20E-02  2.43E-02  3.09E-02  5.78E-03  1.10E-01
          7.73E-02 -1.35E-03 -1.46E-02  5.91E-02 -1.54E-03  2.07E-02  2.63E-02  2.17E-01 -3.57E-02  3.36E-02  1.00E-02 -1.22E-02
        -5.05E-03 -2.52E-02 -2.13E-03 -8.39E-03  8.49E-03 -1.97E-01  3.51E-02
 
 OM46
+        7.88E-03  2.15E-02  4.50E-03  2.14E-02  1.16E-02  3.73E-02  3.64E-03  2.83E-02 -2.09E-02 -2.55E-02  1.97E-02 -9.24E-02
          6.01E-03  1.21E-01  4.31E-02 -4.11E-02 -1.75E-03  5.27E-03  1.26E-02 -4.24E-02  2.05E-01 -6.02E-03 -2.44E-02  1.70E-02
         3.67E-02 -1.53E-02 -1.37E-02 -1.73E-02  1.11E-02  1.01E-01 -2.57E-01  3.99E-02
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      OM11      OM12      OM13      OM14  
             OM15      OM16      OM17      OM18      OM22      OM23      OM24      OM25      OM26      OM27      OM28      OM33  
            OM34      OM35      OM36      OM37      OM38      OM44      OM45      OM46      OM47      OM48      OM55      OM56  
             OM57      OM58      OM66      OM67      OM68      OM77      OM78      OM88      SG11      SG12      SG22  
 
 OM47
+        9.85E-03  6.04E-03 -3.26E-02 -2.63E-02 -4.82E-03 -6.08E-03  1.39E-03  1.07E-02  6.04E-03  7.45E-03 -1.96E-02  9.13E-02
         -1.58E-02  1.20E-02  7.40E-02  3.93E-02 -8.72E-02  2.45E-02 -2.77E-01  7.55E-02 -3.88E-03  2.81E-01  1.30E-02 -1.33E-02
         1.21E-01 -3.00E-02  1.45E-02 -6.46E-02 -1.41E-02 -3.92E-01  1.69E-01 -2.54E-02  3.94E-02
 
 OM48
+        1.86E-02  1.19E-02  2.27E-02  1.15E-02  2.31E-03  2.35E-02  1.20E-02  6.64E-03  5.24E-02  1.05E-01  3.36E-02  4.04E-01
         -2.81E-02  8.21E-03 -8.14E-02  1.62E-01  5.69E-02  6.94E-02  2.73E-01 -2.23E-02 -1.09E-02  2.55E-03  2.45E-01  2.00E-02
         2.42E-01 -3.67E-02  1.35E-02 -9.00E-02  3.70E-02  2.63E-01  2.09E-02 -1.92E-01  1.66E-01  3.82E-02
 
 OM55
+        4.72E-03 -1.29E-02 -1.72E-03 -1.46E-02 -6.23E-03 -1.12E-02 -5.97E-03 -6.67E-03  1.80E-02  4.39E-03  1.77E-02 -2.46E-02
          1.62E-01 -4.47E-02  1.97E-02  1.61E-03  3.38E-02  9.55E-03 -1.19E-02  8.60E-02 -7.07E-03 -2.08E-02 -3.35E-03  2.55E-02
         1.12E-03  3.48E-02 -3.17E-02 -1.60E-02  1.25E-02  3.58E-02 -1.81E-01  4.03E-02 -3.65E-02 -1.96E-02  4.43E-02
 
 OM56
+        1.53E-02 -2.25E-02  4.92E-03 -2.58E-03  1.21E-02 -2.14E-02  9.65E-03  7.20E-03 -1.47E-02  1.16E-02 -7.27E-03  2.15E-02
         -1.20E-01  1.16E-01  3.29E-03 -5.64E-03  2.75E-03 -2.03E-05  2.05E-02 -2.10E-02  5.60E-02 -1.60E-03 -1.14E-02 -1.46E-02
        -2.40E-03  4.54E-02  1.39E-02  7.56E-03 -8.18E-03 -6.46E-03  9.03E-02 -1.52E-01  2.64E-03  2.90E-02 -3.59E-01  3.56E-02
 
 OM57
+       -1.56E-02 -6.63E-03  5.47E-04  9.15E-03 -9.53E-04  1.87E-03  3.88E-03 -2.76E-02  2.06E-02 -1.92E-02  1.02E-02 -5.64E-02
          1.30E-01 -1.09E-02  1.17E-01  2.25E-03 -3.15E-03 -2.01E-03  2.70E-02 -2.21E-01  6.20E-02 -1.04E-02 -1.71E-03  1.14E-02
        -2.35E-02  1.55E-01 -3.27E-02  2.54E-02 -3.48E-04  6.00E-02 -2.95E-01  6.88E-02 -1.51E-01 -3.89E-02  1.62E-01 -1.80E-02
          3.39E-02
 
 OM58
+       -2.48E-03 -9.52E-03  6.17E-03 -1.40E-02  9.40E-03  4.02E-03  1.51E-03  4.36E-04  5.87E-02  4.06E-02  2.29E-02 -8.57E-03
          3.61E-01 -9.52E-02  6.28E-02  1.15E-01  2.30E-02  3.28E-02 -2.04E-02  2.90E-01 -7.71E-02  3.23E-02  7.52E-02  2.11E-02
        -2.86E-02  2.28E-01 -5.78E-02  2.62E-02  3.07E-02 -2.93E-02  1.75E-01 -1.31E-02 -1.46E-03 -1.04E-01  4.98E-02 -2.05E-01
          2.58E-01  3.36E-02
 
 OM66
+       -5.11E-03 -2.65E-03 -1.35E-02  6.52E-03  4.75E-03  1.61E-02 -4.34E-03 -1.73E-02  1.21E-02 -3.19E-03  6.77E-03  6.89E-03
          2.07E-02 -8.87E-02 -1.38E-02  9.71E-03  1.72E-02  1.50E-03 -4.77E-03  1.40E-02 -2.30E-02 -1.89E-02  1.52E-02  2.97E-02
         7.59E-03 -2.99E-03  6.57E-02  2.69E-03  8.79E-03  1.44E-02 -1.29E-02  7.11E-02 -1.04E-02 -6.01E-03  5.07E-02 -3.75E-01
         -1.36E-02  5.09E-02  5.65E-02
 
 OM67
+        3.90E-03 -1.02E-02 -1.46E-02 -2.54E-03 -2.88E-03 -5.17E-02 -1.24E-02  5.84E-03 -3.10E-02  2.93E-02 -3.57E-02  3.62E-02
         -4.72E-02  1.15E-01 -9.56E-02 -5.54E-02 -9.42E-03 -1.19E-02 -7.13E-03  6.10E-02 -2.42E-01 -4.47E-03  3.32E-02 -5.29E-03
        -8.52E-03 -2.38E-02  1.44E-01  4.49E-02 -2.70E-02 -4.46E-02  7.44E-02 -2.89E-01  4.50E-02  5.26E-02 -3.67E-02  1.28E-01
         -2.55E-01 -8.43E-02  1.03E-02  3.75E-02
 
 OM68
+        1.85E-03  9.41E-03  4.60E-03  1.00E-02  1.38E-02 -7.89E-04  3.75E-03  1.52E-02 -5.03E-02 -2.14E-02 -9.42E-03  5.17E-03
         -8.65E-02  3.80E-01 -8.25E-04 -1.47E-01 -2.70E-02  4.23E-03  5.86E-03 -6.66E-02  3.01E-01 -1.40E-02 -7.48E-02 -4.35E-03
         3.25E-02 -7.21E-02  2.44E-01  1.02E-02 -8.49E-03  1.91E-03 -3.19E-02  1.87E-01  1.87E-02  2.60E-02 -3.37E-02  8.16E-02
         -5.59E-02 -2.64E-01 -2.86E-01  2.11E-01  3.81E-02
 
 OM77
+       -5.78E-03 -2.37E-02  1.80E-02  1.83E-02 -1.40E-02  4.78E-03 -6.80E-03 -2.22E-02  2.46E-02 -5.48E-02  4.17E-02 -6.71E-02
          1.54E-02  2.73E-04  1.67E-01  3.78E-02  8.96E-02 -4.36E-02  1.23E-01 -4.88E-02  3.95E-03 -3.22E-01 -8.21E-02  4.56E-02
        -5.69E-02  3.81E-02 -5.40E-03  2.20E-01  6.37E-02  1.09E-01 -6.11E-02 -3.49E-03 -3.91E-01 -8.90E-02  4.55E-02 -5.33E-03
          1.55E-01  4.02E-02  1.73E-02  1.21E-02  1.86E-03  5.14E-02
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      OM11      OM12      OM13      OM14  
             OM15      OM16      OM17      OM18      OM22      OM23      OM24      OM25      OM26      OM27      OM28      OM33  
            OM34      OM35      OM36      OM37      OM38      OM44      OM45      OM46      OM47      OM48      OM55      OM56  
             OM57      OM58      OM66      OM67      OM68      OM77      OM78      OM88      SG11      SG12      SG22  
 
 OM78
+        5.55E-03 -2.81E-02  1.91E-02  2.99E-03 -9.47E-03  1.18E-02  6.52E-03  1.01E-02  6.44E-02 -5.37E-02  8.56E-02 -9.88E-02
          3.93E-02 -1.23E-02  3.88E-01  1.93E-01 -4.70E-02 -1.05E-02 -9.48E-02  4.45E-02  5.85E-02  1.72E-01 -1.43E-01  4.99E-02
        -4.66E-02  4.29E-02 -4.64E-02  2.74E-01  2.02E-01 -6.59E-02  2.62E-02  5.12E-02  1.25E-01 -2.33E-01  1.52E-02 -1.38E-02
          3.36E-02  1.15E-01 -1.28E-02 -1.83E-01 -4.02E-02  3.19E-01  3.69E-02
 
 OM88
+        1.66E-02 -4.21E-03  1.99E-02  7.92E-03  1.17E-03  3.20E-02  3.48E-03  8.38E-03  1.31E-01  1.32E-01  1.30E-01  8.24E-02
          3.43E-02 -8.17E-02  1.37E-01  4.76E-01  8.94E-02  1.11E-01  8.12E-02  3.03E-02 -8.53E-02  8.92E-02  3.99E-01  7.58E-02
         4.77E-02  2.96E-02 -8.28E-02  8.22E-02  3.31E-01  6.90E-02  2.78E-02 -4.98E-02  5.01E-02  2.56E-01  2.60E-02 -1.52E-02
         -4.38E-03  7.36E-02  5.47E-02 -7.54E-02 -2.96E-01  6.38E-02  3.39E-01  5.14E-02
 
 SG11
+       -7.94E-03  3.58E-03 -1.38E-02 -1.92E-02 -2.38E-03  1.53E-02 -1.22E-02 -2.25E-02  1.00E-02  2.32E-02  1.70E-02 -1.79E-03
          3.86E-04 -2.62E-06  6.08E-03 -5.10E-05 -1.86E-02 -4.19E-02  3.07E-03 -5.19E-03 -1.13E-02 -2.56E-03 -1.96E-02  1.60E-02
        -2.54E-02  4.83E-04  5.01E-03  1.19E-02 -1.14E-03 -3.30E-02 -1.48E-02 -1.02E-02 -2.75E-03 -9.54E-03  4.14E-03  5.98E-03
          5.46E-03  8.99E-03  2.60E-02  2.62E-02 -7.16E-03  5.54E-03  1.08E-02 -4.03E-03  6.44E-04
 
 SG12
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
        ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 SG22
+        1.71E-03 -3.46E-03 -2.14E-02  4.55E-03  7.53E-04 -1.10E-02 -2.13E-04  9.22E-03  2.63E-02 -4.89E-03  1.07E-02  3.54E-03
         -3.64E-03  3.65E-03  1.12E-03  1.29E-02  2.22E-02  9.17E-03 -8.22E-03 -1.13E-02 -9.68E-03  1.03E-03 -1.33E-02  2.04E-02
         1.08E-02 -3.46E-02  8.36E-03 -1.71E-02 -1.03E-04  8.89E-03 -3.56E-03  7.68E-03  3.20E-03 -1.46E-03 -1.47E-02  8.14E-03
         -1.35E-02 -2.90E-03 -5.94E-02 -4.11E-03  2.16E-02 -5.61E-03 -7.87E-03 -1.40E-02 -3.34E-02  0.00E+00  1.19E-03
 
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                              MCMC BAYESIAN ANALYSIS                            ********************
 ********************           INVERSE COVARIANCE MATRIX OF ESTIMATE (From Sample Variance)         ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      OM11      OM12      OM13      OM14  
             OM15      OM16      OM17      OM18      OM22      OM23      OM24      OM25      OM26      OM27      OM28      OM33  
            OM34      OM35      OM36      OM37      OM38      OM44      OM45      OM46      OM47      OM48      OM55      OM56  
             OM57      OM58      OM66      OM67      OM68      OM77      OM78      OM88      SG11      SG12      SG22  
 
 TH 1
+        2.18E+02
 
 TH 2
+        6.31E+01  2.45E+02
 
 TH 3
+       -1.60E+01  5.14E+00  3.19E+02
 
 TH 4
+       -1.76E+01 -1.65E+01  7.80E+00  2.24E+02
 
 TH 5
+       -3.12E+01 -3.56E+01 -5.06E+00  1.93E+01  2.60E+02
 
 TH 6
+       -6.60E+00 -1.76E+01 -2.37E+01 -2.11E+01  5.71E+01  2.06E+02
 
 TH 7
+        1.15E+01  6.84E+01 -1.67E+01  7.51E+01 -3.34E+01 -3.28E+01  2.61E+02
 
 TH 8
+       -9.80E+01 -1.03E+02 -6.08E+01 -5.43E+01  3.11E+01  6.31E+01 -1.01E+02  3.27E+02
 
 OM11
+       -4.24E+00 -2.86E+00  6.83E+00  1.10E+00 -2.18E+00  1.13E-01 -7.14E-01  2.46E+00  4.69E+02
 
 OM12
+       -7.79E-02 -1.78E+00  1.32E+00  7.88E+00 -9.85E-01 -5.65E-01  7.90E-01  8.71E+00  2.88E+02  1.25E+03
 
 OM13
+       -2.65E+00 -1.92E+01 -2.24E+01 -8.68E+00  5.53E-01  2.19E+00 -9.04E+00  1.24E+01 -1.03E+02  2.54E+01  1.54E+03
 
 OM14
+       -1.19E-01  2.80E+00  1.14E+01  6.35E-02  6.51E+00 -5.68E+00 -3.89E+00 -4.98E+00 -8.38E+01 -8.91E+01  9.20E+01  1.01E+03
 
 OM15
+        1.13E+01 -4.67E+00 -4.67E+00 -8.22E-01  3.03E-01 -4.88E+00  1.39E+00 -4.10E+00 -1.65E+02 -2.41E+02  1.29E+01  1.23E+02
          1.22E+03
 
 OM16
+        1.22E+00 -5.78E+00 -2.43E+00  3.27E+00  5.13E+00  6.99E+00 -1.30E+00 -4.63E+00 -3.52E+01 -1.29E+02 -1.21E+02 -8.33E+01
          3.13E+02  9.57E+02
 
 OM17
+       -7.68E+00 -2.92E+01 -6.62E+00  2.91E+00  4.51E+00 -4.66E+00 -1.17E+01  1.66E+01  5.42E+01  3.65E+02 -1.12E+02  3.34E+02
         -1.62E+02 -1.39E+02  1.19E+03
 
 OM18
+        9.38E-01  1.30E+01  1.32E+01 -4.56E-01  8.47E+00  7.86E+00  1.14E+01 -1.73E+01 -4.22E+02 -6.76E+02 -2.34E+02 -2.17E+02
          2.60E+02  3.43E+02 -5.21E+02  1.68E+03
 
 OM22
+        1.82E+01  4.35E+01 -7.23E+00 -7.33E+00 -1.31E+01 -9.47E+00  9.74E+00 -2.02E+01  5.46E+01  4.26E+02  4.73E+01 -3.41E+01
         -7.31E+01 -6.90E+01  1.21E+02 -2.24E+02  7.19E+02
 
 OM23
+       -7.96E+00 -2.56E+01 -2.38E+01 -6.98E+00  1.00E+01  3.72E+00 -3.93E+00  2.06E+01 -4.73E+01 -1.59E+02  5.39E+02  3.83E+01
          6.70E+01 -3.14E+01 -1.18E+02 -4.09E-01  1.09E+02  1.97E+03
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      OM11      OM12      OM13      OM14  
             OM15      OM16      OM17      OM18      OM22      OM23      OM24      OM25      OM26      OM27      OM28      OM33  
            OM34      OM35      OM36      OM37      OM38      OM44      OM45      OM46      OM47      OM48      OM55      OM56  
             OM57      OM58      OM66      OM67      OM68      OM77      OM78      OM88      SG11      SG12      SG22  
 
 OM24
+        9.99E+00  4.02E+01  5.47E+00  6.08E+00  1.36E+00 -7.00E+00  1.07E+01 -1.77E+01 -4.46E+00 -8.23E+01  1.92E+01  3.09E+02
          5.59E+01  1.37E+01  7.14E+01 -5.34E+01 -1.35E+02  9.55E+01  1.20E+03
 
 OM25
+       -5.78E+00 -2.04E+00  2.00E+01  1.78E+00  7.22E+00  7.39E+00 -4.62E+00 -1.36E+01 -6.63E+01 -2.83E+02 -3.32E+00  6.14E+01
          4.47E+02  1.78E+02 -1.26E+02  2.01E+02 -2.56E+02 -2.46E+01  1.45E+02  1.60E+03
 
 OM26
+       -1.12E+01 -2.22E+01  1.02E+00  5.12E+00  1.50E+01  2.18E+01 -2.08E-01  1.12E+01 -9.62E+00 -1.28E+02 -9.10E+01 -1.91E+01
          1.27E+02  3.61E+02 -5.47E+01  1.66E+02 -2.07E+02 -2.53E+02 -9.45E+01  5.22E+02  1.31E+03
 
 OM27
+       -1.04E+01 -6.36E+01  3.20E+00  8.92E+00  1.44E+00 -6.54E+00 -1.58E+01  3.58E+01  2.08E+01  2.08E+02 -2.54E+01  1.13E+02
         -7.64E+01 -6.03E+01  3.61E+02 -2.31E+02  4.58E+02 -8.67E+01  4.09E+02 -3.19E+02 -2.81E+02  1.50E+03
 
 OM28
+       -1.68E+01 -2.74E+01  3.69E+00  3.32E+00  2.42E+01  1.94E+01  1.19E+00  2.26E+00 -1.53E+02 -7.95E+02 -1.82E+02 -5.05E+01
          1.66E+02  2.37E+02 -3.37E+02  8.43E+02 -7.71E+02 -3.92E+02 -2.66E+02  4.15E+02  6.65E+02 -9.31E+02  2.42E+03
 
 OM33
+       -9.31E+00  1.48E+01  3.69E+01  2.19E+01  2.11E+00 -1.48E+01  7.21E+00 -1.59E+01 -5.67E+00 -1.89E+01 -1.80E+02 -1.94E+01
          2.49E+01  2.88E+01  4.52E+00  8.26E+01 -1.13E+01  1.04E+02  8.57E+00  1.40E+01  4.48E+00  3.29E+00 -4.02E-01  1.05E+03
 
 OM34
+        2.11E+01  1.46E+01 -4.63E+01 -1.51E+01 -5.28E+00  8.84E+00 -3.44E-01 -5.74E+00  4.46E+00 -2.60E+01 -1.90E+02 -1.29E+02
         -1.77E+01  3.82E+01 -3.93E+01  1.09E+02 -2.83E+01 -1.29E+02  5.12E+01  1.79E+01  4.72E-01  4.39E+01  3.15E+01  1.61E+02
         1.41E+03
 
 OM35
+        2.55E+00  2.52E+01  2.25E+01  7.36E+00 -1.58E+01 -4.49E+00  1.91E+00 -1.18E+01  4.35E+01  5.75E+01 -2.76E+02 -4.77E+01
         -1.67E+02 -1.23E+01  4.70E+01  1.47E+01 -2.60E+01 -4.61E+02 -4.56E+01  5.57E+01  9.64E+01 -1.03E+01  9.30E+01 -4.52E+01
         2.01E+02  1.85E+03
 
 OM36
+       -6.35E-01 -6.20E+00 -3.33E+00  9.24E+00  3.92E+00  1.02E+01  8.68E-01  1.15E+01  2.21E+01  4.72E+01 -9.09E+01 -1.26E+01
         -4.40E+01 -1.16E+02  2.61E+01 -6.80E+01 -2.98E+01 -3.25E+02 -6.60E+01  3.98E+01  1.00E+02 -7.27E+00  6.57E+01 -1.72E+02
        -1.77E+02  5.86E+02  1.59E+03
 
 OM37
+       -5.04E+00 -1.67E+01  2.83E+01  2.16E+01  1.21E+01  5.58E+00  6.19E-01 -7.66E+00 -2.52E+01 -6.95E+01  1.65E+02 -3.33E+01
          3.01E+01  1.65E+01 -2.14E+02  7.85E+01  1.58E+01  6.06E+02  1.06E+02 -1.38E+01 -7.88E+01  2.60E+01 -9.93E+01 -5.95E+01
         4.85E+02 -2.96E+02 -2.79E+02  1.83E+03
 
 OM38
+        1.67E+00 -7.34E+00 -1.79E+01 -1.74E+01 -1.38E+00  3.39E-01 -2.04E+00  1.57E+01  7.99E+01  6.63E+01 -7.54E+02 -3.42E+01
         -2.09E+01  3.41E+01  1.41E+02 -7.71E+01 -6.79E+01 -1.03E+03 -9.58E+01  1.91E+01  1.94E+02  3.57E+01  2.80E+02 -4.73E+02
        -4.23E+02  3.72E+02  6.82E+02 -7.91E+02  2.55E+03
 
 OM44
+        2.34E+00 -1.69E+00 -3.13E+01 -1.72E+01 -4.80E+00  3.47E-01 -2.20E+00  6.67E+00  1.45E+00 -6.63E+00 -2.74E+01 -8.82E+01
         -7.42E+00  6.37E+00 -3.16E+01  2.21E+01 -6.06E+00 -1.04E+01 -9.49E+01 -4.56E+00  6.23E+00 -3.59E+01  2.98E+01  1.52E+01
         3.90E+01  4.01E+01 -5.26E+00  8.73E+00 -6.93E+00  4.53E+02
 
 OM45
+       -8.17E+00 -1.55E+01 -1.08E+01 -2.20E+00 -1.53E+01 -2.81E+00  4.69E+00  4.10E+00  2.14E+01  3.52E+01 -4.46E+01 -1.75E+02
         -1.44E+02 -5.11E+01 -2.96E+01 -8.20E+00  2.12E+01 -6.64E+01 -2.27E+02 -1.62E+02 -2.03E+01 -5.85E+01  5.52E+01  3.75E+00
         3.15E+01  1.17E+02  7.29E+01 -2.30E+01  5.33E+01  1.08E+02  1.20E+03
 
 OM46
+       -5.24E+00 -7.44E+00  1.06E+01 -7.43E-01 -9.67E+00 -1.38E+01  2.55E+00 -1.38E+01  1.79E+01  3.66E+01 -1.53E+00 -2.89E+01
         -6.49E+01 -1.13E+02 -3.23E+00 -4.20E+01  3.30E+01 -2.01E+01 -1.28E+02 -7.07E+01 -8.19E+01 -2.73E+01 -3.02E+01 -3.95E+01
        -1.09E+02  3.83E+01  1.16E+02 -4.76E+01  8.84E+01 -8.94E+01  2.85E+02  9.37E+02
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      OM11      OM12      OM13      OM14  
             OM15      OM16      OM17      OM18      OM22      OM23      OM24      OM25      OM26      OM27      OM28      OM33  
            OM34      OM35      OM36      OM37      OM38      OM44      OM45      OM46      OM47      OM48      OM55      OM56  
             OM57      OM58      OM66      OM67      OM68      OM77      OM78      OM88      SG11      SG12      SG22  
 
 OM47
+        2.34E+00  1.95E+01  3.69E+00  1.57E+00  4.56E+00  3.22E-01  7.13E+00 -6.41E+00  1.50E+00 -2.05E+01  3.36E+01  2.25E+01
          4.65E+01  1.74E+01 -1.07E+02  1.82E+01 -4.30E+01  6.19E+01  3.00E+02  5.48E+01 -2.45E+01 -1.15E+01 -4.89E+01 -8.19E+00
        -5.32E+01 -1.25E+01 -2.91E+01  7.75E+01 -7.41E+00  2.94E+02 -1.41E+02 -1.63E+02  1.25E+03
 
 OM48
+       -1.36E+01 -2.02E+01  5.82E+00  3.08E+00 -3.33E+00 -6.40E+00 -8.23E+00  6.76E+00  4.13E+01  8.38E+01 -1.72E+01 -4.57E+02
         -9.60E+01 -1.49E+01 -1.07E+02 -2.72E+01  7.48E+01 -5.86E+01 -5.22E+02 -1.05E+02  6.14E+00 -1.71E+02  1.01E+01 -7.03E+01
        -2.83E+02  6.52E+00  1.18E+02 -1.38E+02  2.21E+02 -2.18E+02  1.52E+02  3.59E+02 -5.31E+02  1.56E+03
 
 OM55
+       -6.19E+00  3.73E+00 -5.80E-01  8.26E+00  2.80E-01  4.57E+00  5.32E+00  1.32E+00  1.06E+01  1.55E+01 -1.06E+01 -2.06E+01
         -1.97E+02 -6.64E+01  2.00E+01 -2.09E+01  1.11E+00 -1.07E+01 -4.23E+00 -2.62E+02 -1.10E+02  4.88E+01 -2.63E+01 -2.28E+01
        -2.77E+00 -3.27E+01  1.61E+00  3.77E+01  1.93E+00 -1.07E+01  1.15E+02  3.84E+01 -2.09E+01  3.93E+01  6.88E+02
 
 OM56
+       -3.46E+00  1.49E+01 -1.00E+00  5.43E-01 -1.09E+01 -1.87E+00  8.21E-01 -2.05E+00  3.78E+00  1.63E+01  4.56E+01  3.81E+00
         -8.82E+01 -1.92E+02  1.30E+01 -7.87E+01  3.56E+01  6.83E+01  1.70E+01 -2.75E+02 -3.10E+02  8.85E+01 -1.58E+02 -9.63E+00
        -2.46E+01 -2.10E+02 -1.06E+02  5.86E+01 -7.73E+01 -2.53E+01 -9.84E+01  7.30E+01  1.07E+01  2.24E+01  3.78E+02  1.31E+03
 
 OM57
+        1.68E+00  2.71E+00  2.28E+00 -1.34E+01  1.54E+00  1.14E+01 -5.60E+00  7.11E+00 -2.88E+01 -1.05E+02  2.90E+01 -3.58E+01
          1.20E+02  4.33E+01 -2.00E+02  1.36E+02 -7.97E+01  2.89E+01 -5.01E+01  5.01E+02  1.86E+02 -2.63E+02  2.21E+02  7.05E+00
        -3.75E+00 -7.59E+01 -5.00E+00  4.76E-02 -1.12E+01  3.40E+01  3.64E+02  8.19E+01  5.09E+01  8.90E+00 -2.03E+02 -2.73E+02
          1.42E+03
 
 OM58
+        2.45E+00  9.72E+00 -1.17E+01  6.26E+00 -1.13E+01 -9.01E+00  3.39E+00 -1.14E+00  8.12E+01  2.15E+02  7.71E+01 -2.93E+01
         -5.98E+02 -2.48E+02  1.56E+02 -3.56E+02  1.62E+02  7.28E+01  1.15E+01 -8.09E+02 -3.72E+02  2.60E+02 -5.11E+02 -2.58E+01
        -3.63E+01 -3.37E+02 -1.56E+02  6.82E+01 -1.15E+02 -3.28E+01 -2.69E+02 -3.83E+01 -1.82E+01  1.14E+02  2.34E+02  5.22E+02
         -6.77E+02  1.88E+03
 
 OM66
+        6.53E-01  5.53E+00  3.67E+00 -4.14E+00 -8.09E+00 -6.71E+00 -1.24E+00  1.95E+00 -5.97E+00  4.60E+00  2.16E+01  3.10E+00
         -2.32E+01 -5.99E+01  6.00E+00 -9.63E+00  1.77E+01  4.83E+01  3.48E+01 -8.76E+01 -1.58E+02  5.07E+01 -8.36E+01 -2.44E+00
         8.38E+00 -8.56E+01 -1.57E+02  3.88E+01 -9.05E+01 -2.99E+00 -4.68E+01 -8.67E+01  1.78E+01 -4.07E+01  7.42E+01  3.33E+02
         -6.97E+01  1.70E+02  4.58E+02
 
 OM67
+       -1.41E+00  4.88E+00  1.38E+00 -4.21E+00  9.25E+00  2.45E+01  6.16E+00 -4.96E+00  1.65E+00 -4.34E+01 -4.08E+00 -2.74E+01
          3.19E+01  7.49E+01 -3.05E+01  6.48E+01 -6.10E+01 -6.46E+01 -1.08E+02  1.80E+02  4.29E+02 -2.08E+02  2.61E+02  7.83E+00
        -3.45E+01  1.73E+01 -1.15E+01 -1.43E+02  7.00E+01 -1.41E+01  1.26E+02  3.20E+02 -1.50E+02  1.70E+02 -8.56E+01 -2.67E+02
          3.95E+02 -2.64E+02 -1.62E+02  1.17E+03
 
 OM68
+        7.96E+00  1.70E+01  6.77E-01 -6.95E+00 -2.34E+01 -2.44E+01 -5.70E-01 -8.85E+00  1.43E+01  1.06E+02  1.22E+02  6.02E+01
         -1.87E+02 -5.17E+02  7.27E+01 -2.29E+02  1.51E+02  2.04E+02  1.17E+02 -3.38E+02 -7.49E+02  2.38E+02 -5.60E+02  4.09E+01
         5.98E+01 -1.81E+02 -4.12E+02  1.33E+02 -4.13E+02  2.24E+01 -1.14E+02 -2.78E+02  8.87E+01 -2.64E+02  1.09E+02  3.71E+02
         -2.33E+02  6.57E+02  3.65E+02 -5.72E+02  1.75E+03
 
 OM77
+       -5.80E+00 -2.21E+01 -4.16E+00 -2.38E+00  6.07E+00  8.74E-01 -5.46E+00  2.33E+01 -2.44E+00  1.96E+01 -1.46E+01  3.42E+01
          9.25E+00  6.82E-01  5.52E+01 -2.92E+01  5.40E+01 -4.91E+01  1.13E+02 -4.48E+01 -4.92E+01  3.75E+02 -1.72E+02 -1.30E+01
        -1.86E+01  9.58E+00  1.40E+01 -9.47E+01  6.61E+01  3.47E+01 -5.15E+01 -3.88E+01  3.64E+02 -1.62E+02 -3.21E+00  2.57E+01
         -1.63E+02  6.39E+01  5.36E+00 -1.59E+02  6.68E+01  6.75E+02
 
1

            TH 1      TH 2      TH 3      TH 4      TH 5      TH 6      TH 7      TH 8      OM11      OM12      OM13      OM14  
             OM15      OM16      OM17      OM18      OM22      OM23      OM24      OM25      OM26      OM27      OM28      OM33  
            OM34      OM35      OM36      OM37      OM38      OM44      OM45      OM46      OM47      OM48      OM55      OM56  
             OM57      OM58      OM66      OM67      OM68      OM77      OM78      OM88      SG11      SG12      SG22  
 
 OM78
+        9.12E+00  4.74E+01 -1.20E+01 -8.57E+00  2.99E+00  7.39E+00  1.13E+01 -3.01E+01 -2.96E+01 -2.18E+02  5.06E+00 -1.75E+02
          7.47E+01  9.05E+01 -5.41E+02  3.43E+02 -2.35E+02 -4.79E+01 -3.01E+02  1.63E+02  2.32E+02 -8.11E+02  9.09E+02  1.93E+01
        -8.42E+01  3.74E+01  5.64E+01 -2.80E+02  1.41E+01 -6.34E+01  1.06E+02  1.45E+02 -4.51E+02  6.47E+02 -2.98E+01 -1.12E+02
          2.79E+02 -3.40E+02 -6.04E+01  4.43E+02 -4.37E+02 -5.40E+02  2.02E+03
 
 OM88
+        2.96E+00  4.21E+00 -2.51E+00  1.06E+00 -1.58E+01 -1.76E+01 -2.47E+00  1.99E-01  9.65E+01  2.75E+02  1.64E+02  1.36E+02
         -9.82E+01 -1.99E+02  2.18E+02 -7.08E+02  2.07E+02  1.73E+02  1.59E+02 -1.43E+02 -2.74E+02  3.29E+02 -9.79E+02  2.88E+01
         6.64E+01 -7.67E+01 -1.15E+02  1.25E+02 -4.49E+02  1.31E+01 -5.83E+01 -7.83E+01  1.20E+02 -3.88E+02  3.31E-01  9.40E+01
         -1.02E+02  2.90E+02  6.98E+01 -1.91E+02  5.86E+02  1.12E+02 -7.74E+02  1.18E+03
 
 SG11
+       -7.84E+01 -2.72E+02  7.35E+01  3.94E+02 -3.19E+00 -3.19E+02  2.72E+02  3.25E+02 -8.86E+02 -2.54E+03 -7.14E+02 -5.20E+02
          8.40E+02  5.45E+02 -1.02E+03  2.07E+03 -2.22E+02  2.51E+03 -8.47E+02  1.09E+03  6.83E+02 -5.12E+02  2.06E+03 -2.39E+02
         1.29E+03  2.95E+02 -3.43E+02  1.12E+03 -8.66E+02  1.39E+03  1.30E+03 -6.76E+01  8.15E+02 -6.09E+02 -4.31E+02 -1.27E+03
          4.62E+02 -1.81E+03 -9.35E+02 -1.01E+03 -3.76E+02  1.79E+02 -4.34E+02 -5.45E+02  2.44E+06
 
 SG12
+       ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
        ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
         ......... ......... ......... ......... ......... ......... ......... ......... ......... .........
 
 SG22
+        2.42E+00  6.90E+01  4.09E+02 -1.62E+01  6.14E+00  1.02E+02  4.97E+00 -1.68E+02 -4.52E+02 -7.79E+02 -3.93E+02  1.42E+02
          2.91E+02  3.18E+02 -2.79E+02  3.43E+02 -1.20E+03 -8.86E+02  3.43E+02  1.20E+03  1.14E+03 -9.65E+02  1.86E+03 -6.73E+02
        -1.93E+01  1.25E+03  1.43E+02  2.89E+02  5.85E+02 -2.07E+02  1.42E+02 -2.02E+02 -7.49E+01 -2.07E+01  2.11E+02  1.01E+02
          7.33E+02 -9.76E+02  8.54E+02  4.35E+02 -7.34E+02 -2.24E+02  7.71E+02 -3.73E+02  4.19E+04  0.00E+00  7.18E+05
 
 Elapsed postprocess time in seconds:     0.00
 Elapsed finaloutput time in seconds:     0.00
 #CPUT: Total CPU Time in Seconds,     3293.359
Stop Time: 
Tue 10/22/2024 
02:22 PM
