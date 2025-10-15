Tue 10/22/2024 
02:31 PM
;Model Desc: Two compartment Model, Using ADVAN3, TRANS4
;Project Name: nm7examples
;Project ID: NO PROJECT DESCRIPTION

$PROB RUN# Example 9 (from samp5l)
$INPUT C SET ID JID TIME  DV=CONC AMT=DOSE RATE EVID MDV CMT CLX V1X QX V2X SDIX SDSX
$DATA example9.csv IGNORE=C

$SUBROUTINES ADVAN3 TRANS4 OTHER=aneal.f90

$PK
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
0.05   ;[P]
0.01  ;[F]
0.05   ;[P]
0.01  ;[F]
0.01  ;[F]
0.05   ;[P]
0.01  ;[F]
0.01  ;[F]
0.01  ;[F]
0.05   ;[P]
;Initial value of SIGMA
$SIGMA 
(0.6 )   ;[P]

$EST METHOD=SAEM INTERACTION FILE=example9.ext NBURN=5000 NITER=500 PRINT=10 NOABORT SIGL=6 
    CTYPE=3 CINTERVAL=100 CITER=10 CALPHA=0.05
  
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
 RUN# Example 9 (from samp5l)
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
0LENGTH OF THETA:   4
0DEFAULT THETA BOUNDARY TEST OMITTED:    NO
0OMEGA HAS BLOCK FORM:
  1
  1  1
  1  1  1
  1  1  1  1
0DEFAULT OMEGA BOUNDARY TEST OMITTED:    NO
0SIGMA HAS SIMPLE DIAGONAL FORM WITH DIMENSION:   1
0DEFAULT SIGMA BOUNDARY TEST OMITTED:    NO
0INITIAL ESTIMATE OF THETA:
 LOWER BOUND    INITIAL EST    UPPER BOUND
  0.1000E-02     0.2000E+01     0.1000E+07
  0.1000E-02     0.2000E+01     0.1000E+07
  0.1000E-02     0.2000E+01     0.1000E+07
  0.1000E-02     0.2000E+01     0.1000E+07
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.5000E-01
                  0.1000E-01   0.5000E-01
                  0.1000E-01   0.1000E-01   0.5000E-01
                  0.1000E-01   0.1000E-01   0.1000E-01   0.5000E-01
0INITIAL ESTIMATE OF SIGMA:
 0.6000E+00
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
 #METH: Stochastic Approximation Expectation-Maximization
 
 ESTIMATION STEP OMITTED:                 NO
 SHRINK INFO WITH EVALUATION (EVALSHRINK) NO
 ANALYSIS TYPE:                           POPULATION
 NUMBER OF SADDLE POINT RESET ITERATIONS:      0
 GRADIENT METHOD USED:               NOSLOW
 CONDITIONAL ESTIMATES USED:              YES
 CENTERED ETA:                            NO
 EPS-ETA INTERACTION:                     YES
 LAPLACIAN OBJ. FUNC.:                    NO
 NO. OF FUNCT. EVALS. ALLOWED:            528
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
 SIGDIGITS FOR MAP ESTIMATION (SIGLO):      6
 GRADIENT SIGDIGITS OF
       FIXED EFFECTS PARAMETERS (SIGL):     6
 NOPRIOR SETTING (NOPRIOR):                 0
 NOCOV SETTING (NOCOV):                     OFF
 DERCONT SETTING (DERCONT):                 OFF
 FINAL ETA RE-EVALUATION (FNLETA):          1
 EXCLUDE NON-INFLUENTIAL (NON-INFL.) ETAS
       IN SHRINKAGE (ETASTYPE):             NO
 NON-INFL. ETA CORRECTION (NONINFETA):      0
 RAW OUTPUT FILE (FILE): example9.ext
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
 BURN-IN ITERATIONS (NBURN):                5000
 FIRST ITERATION FOR MAP (MAPITERS):          NO
 ITERATIONS (NITER):                        500
 ANNEAL SETTING (CONSTRAIN):                 1
 STARTING SEED FOR MC METHODS (SEED):       11456
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
 iteration        -5000  SAEMOBJ=   395.685629670216
 iteration        -4990  SAEMOBJ=  -1444.31513699827
 iteration        -4980  SAEMOBJ=  -2202.41268434728
 iteration        -4970  SAEMOBJ=  -2317.22097386931
 iteration        -4960  SAEMOBJ=  -2344.06130728694
 iteration        -4950  SAEMOBJ=  -2386.83259128787
 iteration        -4940  SAEMOBJ=  -2388.58780999046
 iteration        -4930  SAEMOBJ=  -2390.59033403475
 iteration        -4920  SAEMOBJ=  -2475.78934739638
 iteration        -4910  SAEMOBJ=  -2457.47488792994
 iteration        -4900  SAEMOBJ=  -2412.70098241476
 iteration        -4890  SAEMOBJ=  -2461.50611512411
 iteration        -4880  SAEMOBJ=  -2447.90154160012
 iteration        -4870  SAEMOBJ=  -2429.37169787715
 iteration        -4860  SAEMOBJ=  -2463.56283976574
 iteration        -4850  SAEMOBJ=  -2403.95680295129
 iteration        -4840  SAEMOBJ=  -2478.53340051741
 iteration        -4830  SAEMOBJ=  -2462.56850270694
 iteration        -4820  SAEMOBJ=  -2417.11268997500
 iteration        -4810  SAEMOBJ=  -2413.64567537428
 iteration        -4800  SAEMOBJ=  -2423.29714623540
 iteration        -4790  SAEMOBJ=  -2435.86868699640
 iteration        -4780  SAEMOBJ=  -2447.28628701980
 iteration        -4770  SAEMOBJ=  -2427.52546874268
 iteration        -4760  SAEMOBJ=  -2438.99078055955
 iteration        -4750  SAEMOBJ=  -2464.71922919328
 iteration        -4740  SAEMOBJ=  -2423.41456523142
 iteration        -4730  SAEMOBJ=  -2463.98904438887
 iteration        -4720  SAEMOBJ=  -2457.90058054585
 iteration        -4710  SAEMOBJ=  -2389.37446150331
 iteration        -4700  SAEMOBJ=  -2445.64176247360
 iteration        -4690  SAEMOBJ=  -2503.85270010776
 iteration        -4680  SAEMOBJ=  -2372.03507073638
 iteration        -4670  SAEMOBJ=  -2434.26635796019
 iteration        -4660  SAEMOBJ=  -2436.16099721987
 iteration        -4650  SAEMOBJ=  -2422.50735970052
 iteration        -4640  SAEMOBJ=  -2465.75187040928
 iteration        -4630  SAEMOBJ=  -2432.55630550845
 iteration        -4620  SAEMOBJ=  -2428.16546492284
 iteration        -4610  SAEMOBJ=  -2434.66505757245
 iteration        -4600  SAEMOBJ=  -2464.95856963440
 iteration        -4590  SAEMOBJ=  -2436.07522563557
 iteration        -4580  SAEMOBJ=  -2458.33408142922
 iteration        -4570  SAEMOBJ=  -2446.34566145147
 iteration        -4560  SAEMOBJ=  -2430.40930609636
 iteration        -4550  SAEMOBJ=  -2427.80550891340
 iteration        -4540  SAEMOBJ=  -2445.59854963970
 iteration        -4530  SAEMOBJ=  -2413.97449236602
 iteration        -4520  SAEMOBJ=  -2477.17680543363
 iteration        -4510  SAEMOBJ=  -2414.02735975557
 iteration        -4500  SAEMOBJ=  -2429.45075860918
 iteration        -4490  SAEMOBJ=  -2385.59490170205
 iteration        -4480  SAEMOBJ=  -2457.41017526704
 iteration        -4470  SAEMOBJ=  -2454.45117356483
 iteration        -4460  SAEMOBJ=  -2447.35991861653
 iteration        -4450  SAEMOBJ=  -2454.15486128298
 iteration        -4440  SAEMOBJ=  -2445.93250247212
 iteration        -4430  SAEMOBJ=  -2439.25154972604
 iteration        -4420  SAEMOBJ=  -2433.14247882930
 iteration        -4410  SAEMOBJ=  -2444.05764820746
 iteration        -4400  SAEMOBJ=  -2415.15037804278
 iteration        -4390  SAEMOBJ=  -2485.62325213446
 iteration        -4380  SAEMOBJ=  -2482.73379420407
 iteration        -4370  SAEMOBJ=  -2410.44302339298
 iteration        -4360  SAEMOBJ=  -2428.83374096700
 iteration        -4350  SAEMOBJ=  -2410.58019306797
 iteration        -4340  SAEMOBJ=  -2447.02461148363
 iteration        -4330  SAEMOBJ=  -2461.75518927830
 iteration        -4320  SAEMOBJ=  -2466.94710294825
 iteration        -4310  SAEMOBJ=  -2499.91143024293
 iteration        -4300  SAEMOBJ=  -2504.30976234030
 iteration        -4290  SAEMOBJ=  -2433.70242482332
 iteration        -4280  SAEMOBJ=  -2496.67723959039
 iteration        -4270  SAEMOBJ=  -2457.75479115455
 iteration        -4260  SAEMOBJ=  -2457.07682901604
 iteration        -4250  SAEMOBJ=  -2478.98257203407
 iteration        -4240  SAEMOBJ=  -2401.42626720276
 iteration        -4230  SAEMOBJ=  -2448.38484082720
 iteration        -4220  SAEMOBJ=  -2463.09262729285
 iteration        -4210  SAEMOBJ=  -2447.01402644273
 iteration        -4200  SAEMOBJ=  -2454.61508783718
 iteration        -4190  SAEMOBJ=  -2415.34810091285
 iteration        -4180  SAEMOBJ=  -2427.22905434547
 iteration        -4170  SAEMOBJ=  -2467.78010569744
 iteration        -4160  SAEMOBJ=  -2408.71039118767
 iteration        -4150  SAEMOBJ=  -2464.13564369389
 iteration        -4140  SAEMOBJ=  -2466.03713002039
 iteration        -4130  SAEMOBJ=  -2370.50626190948
 iteration        -4120  SAEMOBJ=  -2443.07852250951
 iteration        -4110  SAEMOBJ=  -2428.36280511935
 iteration        -4100  SAEMOBJ=  -2443.92534807378
 iteration        -4090  SAEMOBJ=  -2409.52392146612
 iteration        -4080  SAEMOBJ=  -2404.58232203762
 iteration        -4070  SAEMOBJ=  -2432.62933052483
 iteration        -4060  SAEMOBJ=  -2428.83170910368
 iteration        -4050  SAEMOBJ=  -2426.66788586882
 iteration        -4040  SAEMOBJ=  -2459.91729412059
 iteration        -4030  SAEMOBJ=  -2462.68372598521
 iteration        -4020  SAEMOBJ=  -2439.84984464290
 iteration        -4010  SAEMOBJ=  -2463.38312027487
 iteration        -4000  SAEMOBJ=  -2425.31259353914
 iteration        -3990  SAEMOBJ=  -2473.66358559346
 iteration        -3980  SAEMOBJ=  -2457.12901213115
 iteration        -3970  SAEMOBJ=  -2432.73283021896
 iteration        -3960  SAEMOBJ=  -2464.82150160222
 iteration        -3950  SAEMOBJ=  -2495.80422722935
 iteration        -3940  SAEMOBJ=  -2463.59375200069
 iteration        -3930  SAEMOBJ=  -2479.37976404726
 iteration        -3920  SAEMOBJ=  -2483.71045837199
 iteration        -3910  SAEMOBJ=  -2430.10441116161
 iteration        -3900  SAEMOBJ=  -2452.38063726771
 Convergence achieved
 Elapsed burn-in time in seconds:    49.75
 Reduced Stochastic/Accumulation Mode
 iteration            0  SAEMOBJ=  -2462.43220609098
 iteration           10  SAEMOBJ=  -2499.89471657059
 iteration           20  SAEMOBJ=  -2501.98438329162
 iteration           30  SAEMOBJ=  -2501.51513782663
 iteration           40  SAEMOBJ=  -2499.05285117232
 iteration           50  SAEMOBJ=  -2497.52610079037
 iteration           60  SAEMOBJ=  -2495.45034359540
 iteration           70  SAEMOBJ=  -2494.57299168801
 iteration           80  SAEMOBJ=  -2494.70592569078
 iteration           90  SAEMOBJ=  -2494.78296575824
 iteration          100  SAEMOBJ=  -2494.72417002896
 iteration          110  SAEMOBJ=  -2494.08868723786
 iteration          120  SAEMOBJ=  -2493.47095712204
 iteration          130  SAEMOBJ=  -2492.94781788230
 iteration          140  SAEMOBJ=  -2492.42945764329
 iteration          150  SAEMOBJ=  -2492.68229131325
 iteration          160  SAEMOBJ=  -2493.14790424331
 iteration          170  SAEMOBJ=  -2493.11100146497
 iteration          180  SAEMOBJ=  -2492.79825332195
 iteration          190  SAEMOBJ=  -2492.43584914117
 iteration          200  SAEMOBJ=  -2492.32560574692
 iteration          210  SAEMOBJ=  -2492.26245524892
 iteration          220  SAEMOBJ=  -2492.08962626713
 iteration          230  SAEMOBJ=  -2492.64433539438
 iteration          240  SAEMOBJ=  -2492.38568169220
 iteration          250  SAEMOBJ=  -2492.54114579896
 iteration          260  SAEMOBJ=  -2492.59115916168
 iteration          270  SAEMOBJ=  -2492.54887006186
 iteration          280  SAEMOBJ=  -2492.69289941052
 iteration          290  SAEMOBJ=  -2492.89891472407
 iteration          300  SAEMOBJ=  -2492.64370144508
 iteration          310  SAEMOBJ=  -2492.59202480337
 iteration          320  SAEMOBJ=  -2492.61922700621
 iteration          330  SAEMOBJ=  -2492.44819008268
 iteration          340  SAEMOBJ=  -2492.16441237175
 iteration          350  SAEMOBJ=  -2492.14484144460
 iteration          360  SAEMOBJ=  -2492.09207605158
 iteration          370  SAEMOBJ=  -2491.89154866942
 iteration          380  SAEMOBJ=  -2492.09259679271
 iteration          390  SAEMOBJ=  -2492.28252456764
 iteration          400  SAEMOBJ=  -2492.41534315671
 iteration          410  SAEMOBJ=  -2492.51600393747
 iteration          420  SAEMOBJ=  -2492.46009100665
 iteration          430  SAEMOBJ=  -2492.45213636231
 iteration          440  SAEMOBJ=  -2492.63910892723
 iteration          450  SAEMOBJ=  -2492.60797567565
 iteration          460  SAEMOBJ=  -2492.65541710504
 iteration          470  SAEMOBJ=  -2492.73008854801
 iteration          480  SAEMOBJ=  -2492.81979311407
 iteration          490  SAEMOBJ=  -2492.86008495992
 iteration          500  SAEMOBJ=  -2493.02547326576
 
 #TERM:
 STOCHASTIC PORTION WAS COMPLETED
 REDUCED STOCHASTIC PORTION WAS COMPLETED

 ETABAR IS THE ARITHMETIC MEAN OF THE ETA-ESTIMATES,
 AND THE P-VALUE IS GIVEN FOR THE NULL HYPOTHESIS THAT THE TRUE MEAN IS 0.
 
 ETABAR:        -8.6391E-06 -4.7532E-05 -4.7020E-06  1.8191E-05
 SE:             3.8766E-02  2.9018E-02  2.9368E-02  3.2191E-02
 N:                     100         100         100         100
 
 P VAL.:         9.9982E-01  9.9869E-01  9.9987E-01  9.9955E-01
 
 ETASHRINKSD(%)  3.4702E+00  2.2678E+01  2.8837E+01  1.6145E+01
 ETASHRINKVR(%)  6.8200E+00  4.0214E+01  4.9358E+01  2.9684E+01
 EBVSHRINKSD(%)  3.4743E+00  2.2673E+01  2.8827E+01  1.6143E+01
 EBVSHRINKVR(%)  6.8279E+00  4.0205E+01  4.9345E+01  2.9680E+01
 RELATIVEINF(%)  8.8133E+01  5.8405E+01  4.6648E+01  6.2319E+01
 EPSSHRINKSD(%)  2.9439E+01
 EPSSHRINKVR(%)  5.0211E+01
 
  
 TOTAL DATA POINTS NORMALLY DISTRIBUTED (N):          500
 N*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    918.938533204673     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -2493.02547326576     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -1574.08694006109     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 TOTAL EFFECTIVE ETAS (NIND*NETA):                           400
 NIND*NETA*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:    735.150826563738     
 OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:   -2493.02547326576     
 OBJECTIVE FUNCTION VALUE WITH CONSTANT:      -1757.87464670203     
 REPORTED OBJECTIVE FUNCTION DOES NOT CONTAIN CONSTANT
  
 #TERE:
 Elapsed estimation  time in seconds:    72.93
1
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                STOCHASTIC APPROXIMATION EXPECTATION-MAXIMIZATION               ********************
 #OBJT:**************                        FINAL VALUE OF LIKELIHOOD FUNCTION                      ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 





 #OBJV:********************************************         -2493.025       *********************************************
1
 ************************************************************************************************************************
 ********************                                                                                ********************
 ********************                STOCHASTIC APPROXIMATION EXPECTATION-MAXIMIZATION               ********************
 ********************                             FINAL PARAMETER ESTIMATE                           ********************
 ********************                                                                                ********************
 ************************************************************************************************************************
 


 THETA - VECTOR OF FIXED EFFECTS PARAMETERS   *********


         TH 1      TH 2      TH 3      TH 4     
 
         1.63E+00  1.55E+00  7.36E-01  2.34E+00
 


 OMEGA - COV MATRIX FOR RANDOM EFFECTS - ETAS  ********


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        1.61E-01
 
 ETA2
+       -7.31E-03  1.41E-01
 
 ETA3
+        7.29E-03 -1.47E-02  1.70E-01
 
 ETA4
+       -2.21E-02  8.68E-03  1.91E-02  1.47E-01
 


 SIGMA - COV MATRIX FOR RANDOM EFFECTS - EPSILONS  ****


         EPS1     
 
 EPS1
+        5.73E-02
 
1


 OMEGA - CORR MATRIX FOR RANDOM EFFECTS - ETAS  *******


         ETA1      ETA2      ETA3      ETA4     
 
 ETA1
+        4.02E-01
 
 ETA2
+       -4.85E-02  3.75E-01
 
 ETA3
+        4.40E-02 -9.46E-02  4.13E-01
 
 ETA4
+       -1.43E-01  6.02E-02  1.21E-01  3.84E-01
 


 SIGMA - CORR MATRIX FOR RANDOM EFFECTS - EPSILONS  ***


         EPS1     
 
 EPS1
+        2.39E-01
 
 Elapsed postprocess time in seconds:     0.00
 Elapsed finaloutput time in seconds:     0.00
 #CPUT: Total CPU Time in Seconds,       72.391
Stop Time: 
Tue 10/22/2024 
02:32 PM
