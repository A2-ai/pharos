# NONMEM Parametrization Test Results

## case01_diag_plain

Plain diagonal — two values, default parametrization, no splitting

**Input $OMEGA:**

```
$OMEGA
0.04
0.09
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 0.4000E-01
 0.0000E+00   0.9000E-01
 0.0000E+00   0.0000E+00   0.1000E+00
 0.0000E+00   0.0000E+00   0.0000E+00   0.1000E+00
 0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.1000E+00
 0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.1000E+00
```

## case02_diag_sd

Inline SD — splits into 3 blocks: [0.04], [0.01 as SD], [0.09]

**Input $OMEGA:**

```
$OMEGA
0.04
0.01 SD
0.09
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.4000E-01
        2                                                                                   NO
                  0.1000E-03
        3                                                                                   NO
                  0.9000E-01
        4                                                                                   NO
                  0.1000E+00
        5                                                                                   NO
                  0.1000E+00
        6                                                                                   NO
                  0.1000E+00
```

## case03_diag_fix

Inline FIX — splits into 3 blocks: [0.25 fixed], [0.25 unfixed], [0.49 fixed]

**Input $OMEGA:**

```
$OMEGA
0.25 FIXED
0.25
(0.49 FIXED)
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                  YES
                  0.2500E+00
        2                                                                                   NO
                  0.2500E+00
        3                                                                                  YES
                  0.4900E+00
        4                                                                                   NO
                  0.1000E+00
        5                                                                                   NO
                  0.1000E+00
        6                                                                                   NO
                  0.1000E+00
```

## case04_diag_sd_fix

SD + FIX together — splits into 3 blocks: [0.04], [0.01 as SD fixed], [0.09]

**Input $OMEGA:**

```
$OMEGA
0.04
0.01 SD FIX
0.09
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.4000E-01
        2                                                                                  YES
                  0.1000E-03
        3                                                                                   NO
                  0.9000E-01
        4                                                                                   NO
                  0.1000E+00
        5                                                                                   NO
                  0.1000E+00
        6                                                                                   NO
                  0.1000E+00
```

## case05_diag_repeat_sd

Repeat with SD flag — (0.1 SD)x3 expands to 3 individual blocks

**Input $OMEGA:**

```
$OMEGA
(0.1 SD)x3
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.1000E-01
        2                                                                                   NO
                  0.1000E-01
        3                                                                                   NO
                  0.1000E-01
        4                                                                                   NO
                  0.1000E+00
        5                                                                                   NO
                  0.1000E+00
        6                                                                                   NO
                  0.1000E+00
```

## case06_diag_var

VAR explicit — stored but does not trigger split (default interpretation)

**Input $OMEGA:**

```
$OMEGA
0.04
0.05 VAR
0.03
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 0.4000E-01
 0.0000E+00   0.5000E-01
 0.0000E+00   0.0000E+00   0.3000E-01
 0.0000E+00   0.0000E+00   0.0000E+00   0.1000E+00
 0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.1000E+00
 0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.1000E+00
```

## case07_diag_named_mixed

Named diagonal with mixed flags — 4 blocks, one per param

**Input $OMEGA:**

```
$OMEGA
ECL=0.04 FIX
EV=0.09
EKA=0.16 SD
EF=1
$OMEGA 0.1
$OMEGA 0.1
```

**ETA labels:**

```
0LABELS FOR ETAS
 ETA(1)=ETA(ECL)
 ETA(2)=ETA(EV)
 ETA(3)=ETA(EKA)
 ETA(4)=ETA(EF)
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                  YES
                  0.4000E-01
        2                                                                                   NO
                  0.9000E-01
        3                                                                                   NO
                  0.2560E-01
        4                                                                                   NO
                  0.1000E+01
        5                                                                                   NO
                  0.1000E+00
        6                                                                                   NO
                  0.1000E+00
```

## case09_diag_named_var

Named param with VAR — stored but does not split from unnamed default

**Input $OMEGA:**

```
$OMEGA
0.04
EV=0.05 VAR
0.03
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**ETA labels:**

```
0LABELS FOR ETAS
 ETA(2)=ETA(EV)
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 0.4000E-01
 0.0000E+00   0.5000E-01
 0.0000E+00   0.0000E+00   0.3000E-01
 0.0000E+00   0.0000E+00   0.0000E+00   0.1000E+00
 0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.1000E+00
 0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.1000E+00
```

## case10_diag_variance

Long-form VARIANCE — same as VAR, stored but does not split

**Input $OMEGA:**

```
$OMEGA
0.04
0.05 VARIANCE
0.03
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 0.4000E-01
 0.0000E+00   0.5000E-01
 0.0000E+00   0.0000E+00   0.3000E-01
 0.0000E+00   0.0000E+00   0.0000E+00   0.1000E+00
 0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.1000E+00
 0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.1000E+00
```

## case11_diag_standard

Long-form STANDARD — same as SD, triggers split

**Input $OMEGA:**

```
$OMEGA
0.04
0.05 STANDARD
0.03
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.4000E-01
        2                                                                                   NO
                  0.2500E-02
        3                                                                                   NO
                  0.3000E-01
        4                                                                                   NO
                  0.1000E+00
        5                                                                                   NO
                  0.1000E+00
        6                                                                                   NO
                  0.1000E+00
```

## case12_diag_cholesky

CHOLESKY on diagonal value — stored but does not split (no-op on scalar)

**Input $OMEGA:**

```
$OMEGA
0.04
0.05 CHOLESKY
0.03
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 0.4000E-01
 0.0000E+00   0.5000E-01
 0.0000E+00   0.0000E+00   0.3000E-01
 0.0000E+00   0.0000E+00   0.0000E+00   0.1000E+00
 0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.1000E+00
 0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.1000E+00
```

## case13_block_plain

Plain BLOCK(3), no flags — default parametrization

**Input $OMEGA:**

```
$OMEGA BLOCK(3)
0.1
0.01 0.1
0.01 0.01 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.1000E+00
                  0.1000E-01   0.1000E+00
                  0.1000E-01   0.1000E-01   0.1000E+00
        2                                                                                   NO
                  0.1000E+00
        3                                                                                   NO
                  0.1000E+00
        4                                                                                   NO
                  0.1000E+00
```

## case14_block_cholesky

BLOCK(3) with record-level CHOLESKY

**Input $OMEGA:**

```
$OMEGA BLOCK(3) CHOLESKY
0.1
0.01 0.1
0.01 0.01 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.1000E-01
                  0.1000E-02   0.1010E-01
                  0.1000E-02   0.1100E-02   0.1020E-01
        2                                                                                   NO
                  0.1000E+00
        3                                                                                   NO
                  0.1000E+00
        4                                                                                   NO
                  0.1000E+00
```

## case15_block_sd_corr

BLOCK(3) with SD CORR — both axes specified

**Input $OMEGA:**

```
$OMEGA BLOCK(3) SD CORR
0.1
0.01 0.1
0.01 0.01 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.1000E-01
                  0.1000E-03   0.1000E-01
                  0.1000E-03   0.1000E-03   0.1000E-01
        2                                                                                   NO
                  0.1000E+00
        3                                                                                   NO
                  0.1000E+00
        4                                                                                   NO
                  0.1000E+00
```

## case16_block_corr_sd

BLOCK(3) with CORR SD — reversed order, same result as case 15

**Input $OMEGA:**

```
$OMEGA BLOCK(3) CORR SD
0.1
0.01 0.1
0.01 0.01 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.1000E-01
                  0.1000E-03   0.1000E-01
                  0.1000E-03   0.1000E-03   0.1000E-01
        2                                                                                   NO
                  0.1000E+00
        3                                                                                   NO
                  0.1000E+00
        4                                                                                   NO
                  0.1000E+00
```

## case17_block_inline_flags

BLOCK(3) with inline flags on values — flags accumulate into block parametrization

**Input $OMEGA:**

```
$OMEGA BLOCK(3)
0.1
0.01
0.1
0.01
0.01 SD
0.1 CORR
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.1000E-01
                  0.1000E-03   0.1000E-01
                  0.1000E-03   0.1000E-03   0.1000E-01
        2                                                                                   NO
                  0.1000E+00
        3                                                                                   NO
                  0.1000E+00
        4                                                                                   NO
                  0.1000E+00
```

## case18_block_mixed

BLOCK(3) with mixed record-level SD + inline CORR

**Input $OMEGA:**

```
$OMEGA BLOCK(3) SD
0.1
0.01 0.1
0.01 0.01 CORR 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.1000E-01
                  0.1000E-03   0.1000E-01
                  0.1000E-03   0.1000E-03   0.1000E-01
        2                                                                                   NO
                  0.1000E+00
        3                                                                                   NO
                  0.1000E+00
        4                                                                                   NO
                  0.1000E+00
```

## case19_block_fix_inline

BLOCK(2) with FIX on one value — fixes entire block

**Input $OMEGA:**

```
$OMEGA BLOCK(2)
0.3
0.01 FIX 0.35
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                  YES
                  0.3000E+00
                  0.1000E-01   0.3500E+00
        2                                                                                   NO
                  0.1000E+00
        3                                                                                   NO
                  0.1000E+00
        4                                                                                   NO
                  0.1000E+00
        5                                                                                   NO
                  0.1000E+00
```

## case20_block_fix_record

BLOCK(2) with record-level FIX

**Input $OMEGA:**

```
$OMEGA BLOCK(2) FIX
0.1
0.01 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                  YES
                  0.1000E+00
                  0.1000E-01   0.1000E+00
        2                                                                                   NO
                  0.1000E+00
        3                                                                                   NO
                  0.1000E+00
        4                                                                                   NO
                  0.1000E+00
        5                                                                                   NO
                  0.1000E+00
```

## case21_block_values

BLOCK(4) with NAMES and VALUES syntax

**Input $OMEGA:**

```
$OMEGA BLOCK(4) NAMES(ECL,EV,EQ,EVP) VALUES(0.03,0.01)
$OMEGA 0.1
$OMEGA 0.1
```

**ETA labels:**

```
0LABELS FOR ETAS
 ETA(1)=ETA(ECL)
 ETA(2)=ETA(EV)
 ETA(3)=ETA(EQ)
 ETA(4)=ETA(EVP)
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.3000E-01
                  0.1000E-01   0.3000E-01
                  0.1000E-01   0.1000E-01   0.3000E-01
                  0.1000E-01   0.1000E-01   0.1000E-01   0.3000E-01
        2                                                                                   NO
                  0.1000E+00
        3                                                                                   NO
                  0.1000E+00
```

## case22_block_cov

BLOCK(2) with COV explicit — same interpretation as default

**Input $OMEGA:**

```
$OMEGA BLOCK(2) COV
0.1
0.01 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.1000E+00
                  0.1000E-01   0.1000E+00
        2                                                                                   NO
                  0.1000E+00
        3                                                                                   NO
                  0.1000E+00
        4                                                                                   NO
                  0.1000E+00
        5                                                                                   NO
                  0.1000E+00
```

## case23_block_var

BLOCK(2) with VAR explicit — same interpretation as default

**Input $OMEGA:**

```
$OMEGA BLOCK(2) VAR
0.1
0.01 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.1000E+00
                  0.1000E-01   0.1000E+00
        2                                                                                   NO
                  0.1000E+00
        3                                                                                   NO
                  0.1000E+00
        4                                                                                   NO
                  0.1000E+00
        5                                                                                   NO
                  0.1000E+00
```

## case24_block_covariance

BLOCK(2) with COVARIANCE long form

**Input $OMEGA:**

```
$OMEGA BLOCK(2) COVARIANCE
0.1
0.01 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.1000E+00
                  0.1000E-01   0.1000E+00
        2                                                                                   NO
                  0.1000E+00
        3                                                                                   NO
                  0.1000E+00
        4                                                                                   NO
                  0.1000E+00
        5                                                                                   NO
                  0.1000E+00
```

## case25_block_correlation

BLOCK(3) with CORRELATION long form

**Input $OMEGA:**

```
$OMEGA BLOCK(3) CORRELATION
0.1
0.01 0.1
0.01 0.01 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.1000E+00
                  0.1000E-02   0.1000E+00
                  0.1000E-02   0.1000E-02   0.1000E+00
        2                                                                                   NO
                  0.1000E+00
        3                                                                                   NO
                  0.1000E+00
        4                                                                                   NO
                  0.1000E+00
```

## case26_block1_cholesky

BLOCK(1) with CHOLESKY — valid

**Input $OMEGA:**

```
$OMEGA BLOCK(1) CHOLESKY
0.04
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 0.1600E-02
 0.0000E+00   0.1000E-01
 0.0000E+00   0.0000E+00   0.1000E-01
 0.0000E+00   0.0000E+00   0.0000E+00   0.1000E-01
 0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.1000E-01
 0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.1000E-01
```

## case27_block1_sd

BLOCK(1) with SD — valid

**Input $OMEGA:**

```
$OMEGA BLOCK(1) SD
0.2
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.4000E-01
        2                                                                                   NO
                  0.1000E+00
        3                                                                                   NO
                  0.1000E+00
        4                                                                                   NO
                  0.1000E+00
        5                                                                                   NO
                  0.1000E+00
        6                                                                                   NO
                  0.1000E+00
```

## case28_same_basic

Basic SAME — references prior BLOCK(2) CORR

**Input $OMEGA:**

```
$OMEGA BLOCK(2) CORR
0.2
0.3 0.15
$OMEGA BLOCK(2) SAME
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.2000E+00
                  0.5196E-01   0.1500E+00
        2                                                                                   NO
                  0.1000E+00
        3                                                                                   NO
                  0.1000E+00
```

## case29_same_intervening

SAME with intervening diagonal — NONMEM rejects this (BLOCK size mismatch)

**Input $OMEGA:**

```
$OMEGA BLOCK(2) SD CORR
0.2
0.3 0.15
$OMEGA 0.04
$OMEGA BLOCK(2) SAME
$OMEGA 0.1
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.
 
AN ERROR WAS FOUND ON LINE 61 AT THE APPROXIMATE POSITION NOTED:
 $OMEGA BLOCK(2) SAME
                 X   
 THE CHARACTERS IN ERROR ARE: SAME
  315  BLOCK HAS DIFFERENT SIZE THAN PRECEDING BLOCK.
```

## case30_same_repeats

SAME(3) repeats — BLOCK(2) repeated 3 times

**Input $OMEGA:**

```
$OMEGA BLOCK(2)
0.1
0.01 0.1
$OMEGA BLOCK(2) SAME(3)
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.1000E+00
                  0.1000E-01   0.1000E+00
```

## case31_sigma_diag_sd

$SIGMA diagonal with SD — splits into 3 blocks

**Input $SIGMA:**

```
$SIGMA
0.1
2
0.04 SD
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF SIGMA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.1000E+00
        2                                                                                   NO
                  0.2000E+01
        3                                                                                   NO
                  0.1600E-02
```

## case32_sigma_block_corr

$SIGMA BLOCK(2) with CORR

**Input $SIGMA:**

```
$SIGMA BLOCK(2) CORR
0.1
0.3 0.2
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF SIGMA:
 BLOCK SET NO.   BLOCK                                                                    FIXED
        1                                                                                   NO
                  0.1000E+00
                  0.4243E-01   0.2000E+00
```

## case_diag_var_uniform

All values have VAR — uniform flag stored on block

**Input $OMEGA:**

```
$OMEGA
0.04 VAR
0.05 VAR
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 0.4000E-01
 0.0000E+00   0.5000E-01
 0.0000E+00   0.0000E+00   0.1000E+00
 0.0000E+00   0.0000E+00   0.0000E+00   0.1000E+00
 0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.1000E+00
 0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.0000E+00   0.1000E+00
```

## reject08_names_diagonal

NAMES on diagonal record — expected rejection (requires BLOCK)

**Input $OMEGA:**

```
$OMEGA NAMES(CL,V)
0.04
0.09
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.
 
AN ERROR WAS FOUND ON LINE 57 AT THE APPROXIMATE POSITION NOTED:
 $OMEGA NAMES(CL,V)
        X    
 THE CHARACTERS IN ERROR ARE: NAMES
   20  UNKNOWN OPTION.
```

## reject33_record_fix_diagonal

Record-level FIX on diagonal — expected rejection

**Input $OMEGA:**

```
$OMEGA FIX
0.04
0.01
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.
 
AN ERROR WAS FOUND ON LINE 57 AT THE APPROXIMATE POSITION NOTED:
 $OMEGA FIX
        X  
 THE CHARACTERS IN ERROR ARE: FIX
  519  OPTION MUST FOLLOW INITIAL ESTIMATE UNLESS WITHIN PARENTHESES.
```

## reject34_record_sd_diagonal

Record-level SD on diagonal — expected rejection

**Input $OMEGA:**

```
$OMEGA SD
0.04
0.01
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.
 
AN ERROR WAS FOUND ON LINE 58 AT THE APPROXIMATE POSITION NOTED:
 0.04
    X
  519  OPTION MUST FOLLOW INITIAL ESTIMATE UNLESS WITHIN PARENTHESES.
```

## reject35_cholesky_sd_conflict

CHOLESKY + SD on block — expected rejection (mutually exclusive)

**Input $OMEGA:**

```
$OMEGA BLOCK(3) CHOLESKY SD
0.1
0.01 0.1
0.01 0.01 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.
 
AN ERROR WAS FOUND ON LINE 57 AT THE APPROXIMATE POSITION NOTED:
 $OMEGA BLOCK(3) CHOLESKY SD
                          X 
 THE CHARACTERS IN ERROR ARE: SD
   52  THIS OPTION HAS ALREADY BEEN SPECIFIED.
```

## reject36_sd_var_duplicate

SD + VAR on block — expected rejection (duplicate diagonal axis)

**Input $OMEGA:**

```
$OMEGA BLOCK(3) SD VAR
0.1
0.01 0.1
0.01 0.01 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.
 
AN ERROR WAS FOUND ON LINE 57 AT THE APPROXIMATE POSITION NOTED:
 $OMEGA BLOCK(3) SD VAR
                    X  
 THE CHARACTERS IN ERROR ARE: VAR
   52  THIS OPTION HAS ALREADY BEEN SPECIFIED.
```

## reject37_corr_cov_duplicate

CORR + COV on block — expected rejection (duplicate off-diagonal axis)

**Input $OMEGA:**

```
$OMEGA BLOCK(3) CORR COV
0.1
0.01 0.1
0.01 0.01 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.
 
AN ERROR WAS FOUND ON LINE 57 AT THE APPROXIMATE POSITION NOTED:
 $OMEGA BLOCK(3) CORR COV
                      X  
 THE CHARACTERS IN ERROR ARE: COV
   52  THIS OPTION HAS ALREADY BEEN SPECIFIED.
```

## reject38_flag_in_parens_block

Parametrization flag inside parens in BLOCK — expected rejection

**Input $OMEGA:**

```
$OMEGA BLOCK(3)
0.01
(0.02 SD)x2
(0.03)x3
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.
 
AN ERROR WAS FOUND ON LINE 59 AT THE APPROXIMATE POSITION NOTED:
 (0.02 SD)x2
       X 
 THE CHARACTERS IN ERROR ARE: SD
   59  THIS OPTION IS INVALID WITHIN PARENTHESES.
```

## reject39_same_with_param_flag

SAME with parametrization flag — expected rejection

**Input $OMEGA:**

```
$OMEGA BLOCK(2)
0.1
0.01 0.1
$OMEGA BLOCK(2) SAME SD
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.
 
AN ERROR WAS FOUND ON LINE 60 AT THE APPROXIMATE POSITION NOTED:
 $OMEGA BLOCK(2) SAME SD
                      X 
 THE CHARACTERS IN ERROR ARE: SD
   20  UNKNOWN OPTION.
```

## reject40_same_with_fix

SAME with FIX — expected rejection

**Input $OMEGA:**

```
$OMEGA BLOCK(2)
0.1
0.01 0.1
$OMEGA BLOCK(2) SAME FIX
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.
 
AN ERROR WAS FOUND ON LINE 60 AT THE APPROXIMATE POSITION NOTED:
 $OMEGA BLOCK(2) SAME FIX
                      X  
 THE CHARACTERS IN ERROR ARE: FIX
   20  UNKNOWN OPTION.
```

## reject41_same_without_block

SAME without BLOCK — expected rejection

**Input $OMEGA:**

```
$OMEGA SAME
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.
 
AN ERROR WAS FOUND ON LINE 57 AT THE APPROXIMATE POSITION NOTED:
 $OMEGA SAME
        X   
 THE CHARACTERS IN ERROR ARE: SAME
   20  UNKNOWN OPTION.
```

## reject42_same_m_without_block

SAME(3) without BLOCK — expected rejection

**Input $OMEGA:**

```
$OMEGA SAME(3)
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.
 
AN ERROR WAS FOUND ON LINE 57 AT THE APPROXIMATE POSITION NOTED:
 $OMEGA SAME(3)
        X   
 THE CHARACTERS IN ERROR ARE: SAME
   20  UNKNOWN OPTION.
```

## reject43_named_missing_value

Named param missing value after = — expected rejection

**Input $OMEGA:**

```
$OMEGA
ECL=
EV=0.09
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.
 
  223  $OMEGA MUST INCLUDE VALUES FOR ALL ETA VARIABLES THAT ARE USED.
```

## reject44_corr_on_diagonal

CORR on a diagonal value — expected rejection

**Input $OMEGA:**

```
$OMEGA
0.04 CORR
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.
 
AN ERROR WAS FOUND ON LINE 58 AT THE APPROXIMATE POSITION NOTED:
 0.04 CORR
      X   
 THE CHARACTERS IN ERROR ARE: CORR
  544  "COVAR", "CORR" MAY ONLY BE USED WHEN THERE ARE OFF-DIAGONAL ELEMENTS.
```

## reject45_cov_on_diagonal

COV on a diagonal value — expected rejection

**Input $OMEGA:**

```
$OMEGA
0.04 COV
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.
 
AN ERROR WAS FOUND ON LINE 58 AT THE APPROXIMATE POSITION NOTED:
 0.04 COV
      X  
 THE CHARACTERS IN ERROR ARE: COV
  542  THIS OPTION MAY ONLY BE USED WITH AN INITIAL SUB-BLOCK RECORD.
```

## reject46_corr_on_block1

CORR on BLOCK(1) — expected rejection (no off-diagonal elements)

**Input $OMEGA:**

```
$OMEGA BLOCK(1) CORR
0.04
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.
 
AN ERROR WAS FOUND ON LINE 57 AT THE APPROXIMATE POSITION NOTED:
 $OMEGA BLOCK(1) CORR
                 X   
 THE CHARACTERS IN ERROR ARE: CORR
  544  "COVAR", "CORR" MAY ONLY BE USED WHEN THERE ARE OFF-DIAGONAL ELEMENTS.
```

## reject47_cov_on_block1

COV on BLOCK(1) — expected rejection (no off-diagonal elements)

**Input $OMEGA:**

```
$OMEGA BLOCK(1) COV
0.04
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
$OMEGA 0.1
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.
 
AN ERROR WAS FOUND ON LINE 57 AT THE APPROXIMATE POSITION NOTED:
 $OMEGA BLOCK(1) COV
                 X  
 THE CHARACTERS IN ERROR ARE: COV
  544  "COVAR", "CORR" MAY ONLY BE USED WHEN THERE ARE OFF-DIAGONAL ELEMENTS.
```

## reject_same_with_values

SAME block with parameter values — expected rejection

**Input $OMEGA:**

```
$OMEGA BLOCK(2)
0.1
0.01 0.1
$OMEGA BLOCK(2) SAME
0.1 SD
$OMEGA 0.1
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.
 
AN ERROR WAS FOUND ON LINE 61 AT THE APPROXIMATE POSITION NOTED:
 0.1 SD
 X  
 THE CHARACTERS IN ERROR ARE: 0.1
   20  UNKNOWN OPTION.
```

## case50_diag_cholesky

CHOLESKY on diagonal values — NONMEM accepts it. Each value is treated as a
Cholesky factor; the implied variance is the square of the given value (0.1² = 0.01).
This is valid on both diagonal and BLOCK records.

**Input $OMEGA:**

```
$OMEGA
0.1 CHOLESKY
0.1 CHOLESKY
0.1 CHOLESKY
0.1 CHOLESKY
```

**NONMEM output:**

```
0INITIAL ESTIMATE OF OMEGA:
 0.1000E-01
 0.0000E+00   0.1000E-01
 0.0000E+00   0.0000E+00   0.1000E-01
 0.0000E+00   0.0000E+00   0.0000E+00   0.1000E-01
```

CHOLESKY on diagonal is **accepted**. The parser should represent this as
`Diagonal Cholesky` — not an error.
