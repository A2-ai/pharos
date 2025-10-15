;Model Desc: Two compartment Model, Using ADVAN3, TRANS4
;Project Name: nm7examples
;Project ID: NO PROJECT DESCRIPTION

$PROB RUN# Example 1 (from samp5l)
$INPUT C SET ID JID TIME  DV=CONC AMT=DOSE RATE EVID MDV CMT CLX V1X QX V2X SDIX SDSX
$DATA example1b.csv IGNORE=C

$SUBROUTINES ADVAN3 TRANS4

;NTHETA=number of Thetas to be estimated
;NETA=number of Etas to be estimated (and to be described by NETAxBETA OMEGA matrix)
;NTHP=number of thetas which have a prior
;NETP=number of Omegas with prior
;Prior information is important for MCMC Bayesian analysis, not necessary for maximization
; methods
$PRIOR NWPRI NTHETA=4, NETA=4, NTHP=4, NETP=4

$PK
; The thetas are MU modeled.  Best that there is a linear relationship between THETAs and Mus
;  The linear MU modeling of THETAS allows them to be efficiently Gibbs sampled.
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

; Prior information of THETAS
$THETA (2.0 FIX) (2.0 FIX) (2.0 FIX) (2.0 FIX)

; Variance to prior information of THETAS.  Because variances are very large, this
; means that the prior information to the THETAS is highly uninformative.
$OMEGA BLOCK(4)
10000 FIX 
0.00 10000
0.00  0.00 10000
0.00  0.00 0.0 10000

; Prior information to the OMEGAS.
$OMEGA BLOCK(4)
0.2 FIX 
0.0  0.2 
0.0  0.0 0.2
0.0  0.0 0.0 0.2
;Degrees of freedom to prior OMEGA matrix.  Because degrees of freedom is very low, equal to the
; the dimension of the prior OMEGA, this means that the prior information to the OMEGAS is
; highly uninformative
$THETA (4 FIX)

; The first analysis is iterative two-stage, maximum of 500 iterations (NITER), iteration results
; are printed every 5 iterations, gradient precision (SIGL) is 4. Termination is tested on all of
; the population parameters (CITER=3), and for less then 2 significant digits change (NSIG).
; Prior information is not necessary for ITS, so NOPRIOR=1.  The intermediate and final results
; of the ITS method will be recoded in row/column format in example1B.ext
$EST METHOD=ITS INTERACTION FILE=EXAMPLE1B.ext NITER=500 PRINT=5 NOABORT SIGL=4 CTYPE=3 CITER=5   
     CALPHA=0.05 NOPRIOR=1 NSIG=2
; The results of ITS are used as the initial values for the SAEM method.  A maximum of 3000
; stochastic iterations (NBURN) is requested, but may end early if statistical test determines
; that variations in all parameters is stationary (note that any settings from the previous $EST
; carries over to the next $EST statement, within a $PROB).  The SAEM is a Monte Carlo process,
; so setting the SEED assures repeatability of results.  Each iteration obtains only 2 Monte
; Carlo samples ISAMPLE), so they are very fast.  But many iterations are needed, so PRINT only
; every 100th iteration.  After the stochastic phase, 500 accumulation iterations will be
; Performed (NITER), to obtain good parameters estimates with little stochastic noise.
; As a new FILE has not been given, the SAEM results will append to EXAMPLE1B.ext.
$EST METHOD=SAEM INTERACTION NBURN=3000 NITER=500 PRINT=100 SEED=1556678 ISAMPLE=2
; After the SAEM method, obtain good estimates of the marginal density (objective function),
; along with good estimates of the standard errors.  This is best done with importance sampling
; (IMP), performing the expectation step only (EONLY=1), so that final population parameters
; remain at the final SAEM result.  Five iterations (NITER) should allow the importance sampling
; proposal density to become stationary.  This is observed by the objective function settling 
; to a particular value (with some stochastic noise).  By using 3000 Monte Carlo samples
; (ISAMPLE), this assures a precise assessment of standard errors.
$EST METHOD=IMP  INTERACTION EONLY=1 NITER=5 ISAMPLE=3000 PRINT=1 SIGL=8 NOPRIOR=1
; The Bayesian analysis is performed.  While 10000 burn-in
; iterations are requested as a maximum, because the termination test is on (CTYPE<>0, set at the
; first $EST statement), and because the initial parameters are at the SAEM result, which is the
; maximum likelihood position, the analysis should settle down to a stationary distribution in
; several hundred iterations.  Prior information is also used to facilitate Bayesian analysis.
; The individual Bayesian iteration results are important, and may be need for post-processing
; analysis. So specify a separate FILE for the Bayesian analysis. 
$EST METHOD=BAYES INTERACTION FILE=EXAMPLE1B.txt NBURN=10000 NITER=10000 PRINT=100 NOPRIOR=0
; Just for old-times sake, let?s see what the traditional FOCE method will give us.  
; And, remember to introduce a new FILE, so its results won?t append to our Bayesian FILE. 
; Appending to EXAMPLE1B.ext with the EM methods is fine.
$EST METHOD=COND INTERACTION MAXEVAL=9999 NSIG=3 SIGL=10 PRINT=5 NOABORT NOPRIOR=1
     FILE=EXAMPLE1B.ext
; Time for the standard error results.  You may request a more precise gradient precision (SIGL)
; that differed from that used during estimation.
$COV MATRIX=R PRINT=E UNCONDITIONAL SIGL=12
; Print out results in tables. Include some of the new weighted residual types
$TABLE ID TIME PRED RES WRES CPRED CWRES EPRED ERES EWRES NOAPPEND ONEHEADER 
 FILE=EXAMPLE1B.TAB NOPRINT
$TABLE ID CL V1 Q V2 FIRSTONLY NOAPPEND NOPRINT FILE=EXAMPLE1B.PAR
$TABLE ID ETA1 ETA2 ETA3 ETA4 FIRSTONLY NOAPPEND NOPRINT FILE=EXAMPLE1B.ETA

