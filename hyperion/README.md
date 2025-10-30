
<!-- README.md is generated from README.Rmd. Please edit that file -->

# hyperion

<!-- badges: start -->

[![R-CMD-check](https://github.com/A2-ai/hyperion/actions/workflows/R-CMD-check.yaml/badge.svg)](https://github.com/A2-ai/hyperion/actions/workflows/R-CMD-check.yaml)
<!-- badges: end -->

Hyperion is a companion R packge to the cli tool
[pharos](https://github.com/a2-ai/pharos) for managing and running
pharamceutical models directly from R. NONMEM is the first supported
modelling software.

## Installation

You can install the development version of hyperion from
[GitHub](https://github.com/) with:

``` r
# install.packages("pak")
pak::pak("A2-ai/hyperion")
```

## Getting started

To initialize hyperion/pharos use the `hyperion::init()` cfunction to
create a `pharos.toml` configuration file

``` r
library(hyperion)

if (!file.exists("pharos.toml")) {
  hyperion::init(".")
}
#> NULL
```

The `pharos.toml` file contains several configuration options for NONMEM
and pharos. You can see more detailed explanations [from
pharos](https://github.com/A2-ai/pharos?tab=readme-ov-file#nonmem)

## Checking a model

You can check a model for correct compilation before submitting to catch
any data path issues, or syntax errors within the control stream with:

``` r
check_model("vignettes/test_data/models/onecmt/run002a.mod") |> 
  cat()
#>   
#>  WARNINGS AND ERRORS (IF ANY) FOR PROBLEM    1
#>              
#>  (WARNING  2) NM-TRAN INFERS THAT THE DATA ARE POPULATION.
#>   
#> Note: Analytical 2nd Derivatives are constructed in FSUBS but are never used.
#>       You may insert $ABBR DERIV2=NO after the first $PROB to save FSUBS construction and compilation time
#> 
```

``` r
check_model("vignettes/test_data/models/onecmt/run004.mod") |> 
  cat()
#>  
#>  AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.
#>  
#> AN ERROR WAS FOUND ON LINE 11 AT THE APPROXIMATE POSITION NOTED:
#>  TVCL = THETA1
#>         X     
#>  THE CHARACTERS IN ERROR ARE: THETA1
#>   208  UNDEFINED VARIABLE.
#> 
#> nmtran failed with exit code 4
```

## Viewing a model object

Hyperion can read .mod files to give an overview of the mod file with:

``` r
read_model("vignettes/test_data/models/onecmt/run002.mod")
```

# NONMEM Model: run002

**Problem:** Base one-compartment oral absorption model

**Dataset:** ../../data/derived/onecmpt-oral-30ind.csv

**Ignore:** @

**Included Columns:** ID, TIME, EVID, AMT, CMT, DV, MDV, WT, SEX

## Theta Parameters

<table class="table table-striped">

<thead>

<tr>

<th style="text-align:left;">

Parameter
</th>

<th style="text-align:right;">

Initial
</th>

<th style="text-align:right;">

Lower
</th>

<th style="text-align:right;">

Upper
</th>

<th style="text-align:right;">

Fixed
</th>

<th style="text-align:left;">

Comment
</th>

</tr>

</thead>

<tbody>

<tr>

<td style="text-align:left;">

THETA1
</td>

<td style="text-align:right;">

1.24
</td>

<td style="text-align:right;">

0
</td>

<td style="text-align:right;">

NA
</td>

<td style="text-align:right;">

no
</td>

<td style="text-align:left;">

TVCL (L/hr)
</td>

</tr>

<tr>

<td style="text-align:left;">

THETA2
</td>

<td style="text-align:right;">

40.86
</td>

<td style="text-align:right;">

0
</td>

<td style="text-align:right;">

NA
</td>

<td style="text-align:right;">

no
</td>

<td style="text-align:left;">

TVV (L)
</td>

</tr>

<tr>

<td style="text-align:left;">

THETA3
</td>

<td style="text-align:right;">

1.24
</td>

<td style="text-align:right;">

0
</td>

<td style="text-align:right;">

NA
</td>

<td style="text-align:right;">

no
</td>

<td style="text-align:left;">

TVKA (1/hr)
</td>

</tr>

</tbody>

</table>

## Omega Parameters

<table class="table table-striped">

<thead>

<tr>

<th style="text-align:left;">

Parameter
</th>

<th style="text-align:right;">

Initial
</th>

<th style="text-align:right;">

Lower
</th>

<th style="text-align:right;">

Upper
</th>

<th style="text-align:right;">

Fixed
</th>

<th style="text-align:left;">

Parametrization
</th>

<th style="text-align:left;">

Comment
</th>

</tr>

</thead>

<tbody>

<tr>

<td style="text-align:left;">

OMEGA(1,1)
</td>

<td style="text-align:right;">

0.131
</td>

<td style="text-align:right;">

NA
</td>

<td style="text-align:right;">

NA
</td>

<td style="text-align:right;">

no
</td>

<td style="text-align:left;">

</td>

<td style="text-align:left;">

OM1 TVCL :EXP
</td>

</tr>

<tr>

<td style="text-align:left;">

OMEGA(2,2)
</td>

<td style="text-align:right;">

0.136
</td>

<td style="text-align:right;">

NA
</td>

<td style="text-align:right;">

NA
</td>

<td style="text-align:right;">

no
</td>

<td style="text-align:left;">

</td>

<td style="text-align:left;">

OM2 TVV :EXP
</td>

</tr>

<tr>

<td style="text-align:left;">

OMEGA(3,3)
</td>

<td style="text-align:right;">

0.100
</td>

<td style="text-align:right;">

NA
</td>

<td style="text-align:right;">

NA
</td>

<td style="text-align:right;">

no
</td>

<td style="text-align:left;">

</td>

<td style="text-align:left;">

OM3 TVKA :EXP
</td>

</tr>

</tbody>

</table>

## Sigma Parameters

<table class="table table-striped">

<thead>

<tr>

<th style="text-align:left;">

Parameter
</th>

<th style="text-align:right;">

Initial
</th>

<th style="text-align:right;">

Lower
</th>

<th style="text-align:right;">

Upper
</th>

<th style="text-align:right;">

Fixed
</th>

<th style="text-align:left;">

Parametrization
</th>

<th style="text-align:left;">

Comment
</th>

</tr>

</thead>

<tbody>

<tr>

<td style="text-align:left;">

SIGMA(1,1)
</td>

<td style="text-align:right;">

0.0364
</td>

<td style="text-align:right;">

NA
</td>

<td style="text-align:right;">

NA
</td>

<td style="text-align:right;">

no
</td>

<td style="text-align:left;">

</td>

<td style="text-align:left;">

SIG1 Proportional error (variance, 20% CV)
</td>

</tr>

<tr>

<td style="text-align:left;">

SIGMA(2,2)
</td>

<td style="text-align:right;">

0.0100
</td>

<td style="text-align:right;">

NA
</td>

<td style="text-align:right;">

NA
</td>

<td style="text-align:right;">

no
</td>

<td style="text-align:left;">

</td>

<td style="text-align:left;">

SIG2 Additive error (variance, 0.01 mg/L SD)
</td>

</tr>

</tbody>

</table>

## Running a model

There is no current support from hyperion to run a model, but SLURM job
submission will be coming soon.

## Model summary

After running a model you can view run details and final estimates with:

``` r
get_model_summary("vignettes/test_data/models/onecmt/run002")
```

# Model Summary: run002

**Problem:** Base one-compartment oral absorption model

**Records:** 240 \| **Observations:** 210 \| **Subjects:** 30

**Final OFV:** -103.468

## Estimation Methods

- **First Order Conditional Estimation with Interaction**
  - Condition Number: 29.6

## Heuristic Checks

\[<span style="color:green">OK</span>\] Minimization Successful

\[<span style="color:green">OK</span>\] Covariance Step Successful

\[<span style="color:green">OK</span>\] No Eigenvalue Issues

\[<span style="color:green">OK</span>\] No Parameters Near Boundary

\[<span style="color:green">OK</span>\] No Hessian Resets

## Theta Parameters

<table class="table table-striped">

<thead>

<tr>

<th style="text-align:left;">

Parameter
</th>

<th style="text-align:right;">

Estimate
</th>

<th style="text-align:right;">

SE
</th>

<th style="text-align:right;">

RSE (%)
</th>

</tr>

</thead>

<tbody>

<tr>

<td style="text-align:left;">

THETA1
</td>

<td style="text-align:right;">

1.2468
</td>

<td style="text-align:right;">

0.1288
</td>

<td style="text-align:right;">

10.333
</td>

</tr>

<tr>

<td style="text-align:left;">

THETA2
</td>

<td style="text-align:right;">

40.8482
</td>

<td style="text-align:right;">

3.0272
</td>

<td style="text-align:right;">

7.411
</td>

</tr>

<tr>

<td style="text-align:left;">

THETA3
</td>

<td style="text-align:right;">

1.2439
</td>

<td style="text-align:right;">

0.1134
</td>

<td style="text-align:right;">

9.117
</td>

</tr>

</tbody>

</table>

## Omega Parameters

<table class="table table-striped">

<thead>

<tr>

<th style="text-align:left;">

Parameter
</th>

<th style="text-align:right;">

Random Effect
</th>

<th style="text-align:right;">

Estimate
</th>

<th style="text-align:right;">

SE
</th>

<th style="text-align:right;">

RSE (%)
</th>

<th style="text-align:right;">

Shrinkage (%)
</th>

</tr>

</thead>

<tbody>

<tr>

<td style="text-align:left;">

OMEGA(1,1)
</td>

<td style="text-align:right;">

ETA1
</td>

<td style="text-align:right;">

0.1304
</td>

<td style="text-align:right;">

0.0602
</td>

<td style="text-align:right;">

46.149
</td>

<td style="text-align:right;">

18.06
</td>

</tr>

<tr>

<td style="text-align:left;">

OMEGA(2,2)
</td>

<td style="text-align:right;">

ETA2
</td>

<td style="text-align:right;">

0.1363
</td>

<td style="text-align:right;">

0.0397
</td>

<td style="text-align:right;">

29.128
</td>

<td style="text-align:right;">

4.99
</td>

</tr>

<tr>

<td style="text-align:left;">

OMEGA(3,3)
</td>

<td style="text-align:right;">

ETA3
</td>

<td style="text-align:right;">

0.1144
</td>

<td style="text-align:right;">

0.0614
</td>

<td style="text-align:right;">

53.705
</td>

<td style="text-align:right;">

27.19
</td>

</tr>

</tbody>

</table>

## Sigma Parameters

<table class="table table-striped">

<thead>

<tr>

<th style="text-align:left;">

Parameter
</th>

<th style="text-align:right;">

Random Effect
</th>

<th style="text-align:right;">

Estimate
</th>

<th style="text-align:right;">

SE
</th>

<th style="text-align:right;">

RSE (%)
</th>

<th style="text-align:right;">

Shrinkage (%)
</th>

</tr>

</thead>

<tbody>

<tr>

<td style="text-align:left;">

SIGMA(1,1)
</td>

<td style="text-align:right;">

EPS1
</td>

<td style="text-align:right;">

0.0372
</td>

<td style="text-align:right;">

0.0116
</td>

<td style="text-align:right;">

31.155
</td>

<td style="text-align:right;">

15.44
</td>

</tr>

<tr>

<td style="text-align:left;">

SIGMA(2,2)
</td>

<td style="text-align:right;">

EPS2
</td>

<td style="text-align:right;">

0.0066
</td>

<td style="text-align:right;">

0.0279
</td>

<td style="text-align:right;">

422.579
</td>

<td style="text-align:right;">

15.44
</td>

</tr>

</tbody>

</table>

## Copying a model

You can copy a model to a new control stream and alter the initial
estimates of the new model. This will create a new mod file and a
`*_metadata.json` file that contains the description and which model it
is based on.

``` r
copy_model(
  from = "vignettes/test_data/models/onecmt/run002.mod", 
  to = "vignettes/test_data/models/onecmt/run002a.mod",
  update = "all", #sets initial estimates of `to` with final estimates of `from` 
  jitter = 0.1, #jitters run002a initial estimates by 10%
  description = "Some description about what makes run002a different",
  overwrite = TRUE,
  seed = 804
)
#> NULL
```

## Model lineage

If you use hyperion to copy models you can extract the model lineage
with

``` r
get_model_lineage("vignettes/test_data/models/onecmt")
```

# Hyperion Model Tree

ℹ️ **Models:** 6

- <strong style="color:blue">run001</strong>
  - <span style="color:green">run004</span> <span style="color:gray">-
    Updating run001 to run004 with jittered params …</span>
  - <span style="color:orange">run002</span> <span style="color:gray">-
    Adding COV step, unfixing eps(2)</span>
    - <span style="color:green">run002a</span>
      <span style="color:gray">- Some description about what makes
      run002a diffe…</span>
    - <span style="color:green">run002b001</span>
      <span style="color:gray">- Jittering initial sigma estimates,
      using theta/…</span>
    - <span style="color:orange">run003</span>
      <span style="color:gray">- Jittering initial estimates</span>
      - <span style="color:green">run003b1</span>
        <span style="color:gray">- Updating run003 to 003b1 with
        jittered params</span>
