
<!-- README.md is generated from README.Rmd. Please edit that file -->

# hyperion

<!-- badges: start -->

[![R-CMD-check](https://github.com/A2-ai/hyperion/actions/workflows/R-CMD-check.yaml/badge.svg)](https://github.com/A2-ai/hyperion/actions/workflows/R-CMD-check.yaml)
[![Pharos
Dependency](https://github.com/A2-ai/hyperion/actions/workflows/pharos-dependency-check.yaml/badge.svg)](https://github.com/A2-ai/hyperion/actions/workflows/pharos-dependency-check.yaml)
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
#> 
#> 
#> ── pharos configuration ────────────────────────────────────────────────────────
#> ✔ pharos.toml found: /data/user-homes/matthews/Packages/hyperion/pharos.toml
#> ── hyperion options ────────────────────────────────────────────────────────────
#> ✔ hyperion.significant_number_display : 4
#> ── hyperion nonmem object options ──────────────────────────────────────────────
#> ✔ hyperion.nonmem_model.show_included_columns : FALSE
#> ✔ hyperion.nonmem_summary.rse_threshold : 50
#> ✔ hyperion.nonmem_summary.shrinkage_threshold : 30
test_data_dir <- system.file("extdata", package = "hyperion")

if (!file.exists("pharos.toml")) {
  hyperion::init(".")
}
```

The `pharos.toml` file contains several configuration options for NONMEM
and pharos. You can see more detailed explanations [from
pharos](https://github.com/A2-ai/pharos?tab=readme-ov-file#nonmem)

## Checking a model

You can check a model for correct compilation before submitting to catch
any data path issues, or syntax errors within the control stream with:

``` r
check_model(file.path(test_data_dir, "models", "onecmt", "run002a.mod"))
#> WARNINGS AND ERRORS (IF ANY) FOR PROBLEM    1
#>              
#>  (WARNING  2) NM-TRAN INFERS THAT THE DATA ARE POPULATION.
#>   
#> Note: Analytical 2nd Derivatives are constructed in FSUBS but are never used.
#>       You may insert $ABBR DERIV2=NO after the first $PROB to save FSUBS construction and compilation time
#> [1] 0
```

``` r
check_model(file.path(test_data_dir, "models", "onecmt", "run004.mod"))
#> AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.
#>  
#> AN ERROR WAS FOUND ON LINE 11 AT THE APPROXIMATE POSITION NOTED:
#>  TVCL = THETA1
#>         X     
#>  THE CHARACTERS IN ERROR ARE: THETA1
#>   208  UNDEFINED VARIABLE.
#> [1] 4
```

## Viewing a model object

Hyperion can read .mod files to give an overview of the mod file with:

``` r
run002 <- read_model(file.path(test_data_dir, "models", "onecmt", "run002.mod"))
run002
```

<strong>NONMEM Model: run002</strong>

<strong>Problem:</strong> Base one-compartment oral absorption model

<strong>Run Status:</strong> Run

<strong>Dataset:</strong> ../../data/derived/onecmpt-oral-30ind.csv

<strong>Ignore:</strong> @

<strong>Theta Parameters</strong>

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

<th style="text-align:left;">

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

<td style="text-align:left;">

No
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

<td style="text-align:left;">

No
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

<td style="text-align:left;">

No
</td>

<td style="text-align:left;">

TVKA (1/hr)
</td>

</tr>

</tbody>

</table>

<strong>Omega Parameters</strong>

<table class="table table-striped">

<thead>

<tr>

<th style="text-align:left;">

Parameter
</th>

<th style="text-align:right;">

Initial
</th>

<th style="text-align:left;">

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

OMEGA(1,1)
</td>

<td style="text-align:right;">

0.131
</td>

<td style="text-align:left;">

No
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

<td style="text-align:left;">

No
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

0.1
</td>

<td style="text-align:left;">

No
</td>

<td style="text-align:left;">

OM3 TVKA :EXP
</td>

</tr>

</tbody>

</table>

<strong>Sigma Parameters</strong>

<table class="table table-striped">

<thead>

<tr>

<th style="text-align:left;">

Parameter
</th>

<th style="text-align:right;">

Initial
</th>

<th style="text-align:left;">

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

SIGMA(1,1)
</td>

<td style="text-align:right;">

0.0364
</td>

<td style="text-align:left;">

No
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

0.01
</td>

<td style="text-align:left;">

No
</td>

<td style="text-align:left;">

SIG2 Additive error (variance, 0.01 mg/L SD)
</td>

</tr>

</tbody>

</table>

## Running a model

You can submit a model to a `slurm` or `sge` grid using the following
commands:

``` r
submit_model_to_slurm(run002, ncpu = 1)
submit_model_to_sge(run002, ncpu = 1)
```

## Model summary

After running a model you can view run details and final estimates with:

``` r
summary(run002)
```

<strong>Model Summary: run002</strong>

<strong>Problem:</strong> Base one-compartment oral absorption model

<strong>Records: 240 \| Observations: 210 \| Subjects: 30</strong>

<strong>Final OFV:</strong> -103.5

<strong>Estimation Methods</strong>

- <strong>First Order Conditional Estimation with Interaction</strong>
  - Condition Number: 29.63

<strong>Heuristic Checks</strong>

\[<span style="color:green">OK</span>\] Minimization Successful

\[<span style="color:green">OK</span>\] Covariance Step Successful

\[<span style="color:green">OK</span>\] No Eigenvalue Issues

\[<span style="color:green">OK</span>\] No Parameters Near Boundary

\[<span style="color:green">OK</span>\] No Hessian Resets

<strong>Theta Parameters</strong>

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

<th style="text-align:left;">

Fixed
</th>

</tr>

</thead>

<tbody>

<tr>

<td style="text-align:left;">

TVCL
</td>

<td style="text-align:right;">

1.247
</td>

<td style="text-align:right;">

0.1288
</td>

<td style="text-align:right;">

10.33
</td>

<td style="text-align:left;">

No
</td>

</tr>

<tr>

<td style="text-align:left;">

TVV
</td>

<td style="text-align:right;">

40.85
</td>

<td style="text-align:right;">

3.027
</td>

<td style="text-align:right;">

7.411
</td>

<td style="text-align:left;">

No
</td>

</tr>

<tr>

<td style="text-align:left;">

TVKA
</td>

<td style="text-align:right;">

1.244
</td>

<td style="text-align:right;">

0.1134
</td>

<td style="text-align:right;">

9.117
</td>

<td style="text-align:left;">

No
</td>

</tr>

</tbody>

</table>

<strong>Omega Parameters</strong>

<table class="table table-striped">

<thead>

<tr>

<th style="text-align:left;">

Parameter
</th>

<th style="text-align:left;">

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

<th style="text-align:left;">

Fixed
</th>

</tr>

</thead>

<tbody>

<tr>

<td style="text-align:left;">

OM1 (TVCL)
</td>

<td style="text-align:left;">

ETA1
</td>

<td style="text-align:right;">

0.1304
</td>

<td style="text-align:right;">

0.06019
</td>

<td style="text-align:right;">

46.15
</td>

<td style="text-align:right;">

18.06
</td>

<td style="text-align:left;">

No
</td>

</tr>

<tr>

<td style="text-align:left;">

OM2 (TVV)
</td>

<td style="text-align:left;">

ETA2
</td>

<td style="text-align:right;">

0.1363
</td>

<td style="text-align:right;">

0.03971
</td>

<td style="text-align:right;">

29.13
</td>

<td style="text-align:right;">

4.986
</td>

<td style="text-align:left;">

No
</td>

</tr>

<tr>

<td style="text-align:left;">

OM3 (TVKA)
</td>

<td style="text-align:left;">

ETA3
</td>

<td style="text-align:right;">

0.1144
</td>

<td style="text-align:right;">

0.06144
</td>

<td style="text-align:right;">

<span style="color: #DD0000;">53.71</span>
</td>

<td style="text-align:right;">

27.19
</td>

<td style="text-align:left;">

No
</td>

</tr>

</tbody>

</table>

<strong>Sigma Parameters</strong>

<table class="table table-striped">

<thead>

<tr>

<th style="text-align:left;">

Parameter
</th>

<th style="text-align:left;">

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

<th style="text-align:left;">

Fixed
</th>

</tr>

</thead>

<tbody>

<tr>

<td style="text-align:left;">

SIGMA(1,1)
</td>

<td style="text-align:left;">

EPS1
</td>

<td style="text-align:right;">

0.03723
</td>

<td style="text-align:right;">

0.0116
</td>

<td style="text-align:right;">

31.16
</td>

<td style="text-align:right;">

15.44
</td>

<td style="text-align:left;">

No
</td>

</tr>

<tr>

<td style="text-align:left;">

SIGMA(2,2)
</td>

<td style="text-align:left;">

EPS2
</td>

<td style="text-align:right;">

0.006607
</td>

<td style="text-align:right;">

0.02792
</td>

<td style="text-align:right;">

<span style="color: #DD0000;">422.6</span>
</td>

<td style="text-align:right;">

15.44
</td>

<td style="text-align:left;">

No
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
  from = file.path(test_data_dir, "models", "onecmt", "run002.mod"),
  to = file.path(test_data_dir, "models", "onecmt", "run002a.mod"),
  update = "all", # sets initial estimates of `to` with final estimates of `from`
  jitter = 0.1, # jitters run002a initial estimates by 10%
  description = "Some description about what makes run002a different",
  overwrite = TRUE,
  seed = 804831
)
#> NULL
```

## Model lineage

If you use hyperion to copy models you can extract the model lineage
with

``` r
get_model_lineage(file.path(test_data_dir, "models", "onecmt"))
```

<strong>Hyperion Model Tree</strong>

ℹ️ <strong>Models:</strong> 9

- <strong style="color:blue">run001</strong> <span style="color:gray">-
  Base model</span>
  - <span style="color:green">run004</span> <span style="color:gray">-
    Updating run001 to run004 with jittered params …</span>
  - <span style="color:green">run005</span> <span style="color:gray">-
    Updating run001 to run004 with jittered params …</span>
  - <span style="color:orange">run002</span> <span style="color:gray">-
    Adding COV step, unfixing eps(2)</span>
    - <span style="color:green">run002b001</span>
      <span style="color:gray">- Jittering initial sigma estimates,
      using theta/…</span>
    - <span style="color:orange">run003</span>
      <span style="color:gray">- Jittering initial estimates</span>
      - <span style="color:green">run003b2</span>
        <span style="color:gray">- Updating run003 with mod
        object</span>
      - <span style="color:green">run003b1</span>
        <span style="color:gray">- Updating run003 to 003b1 with
        jittered params. …</span>
    - <span style="color:green">run002a</span>
      <span style="color:gray">- Some description about what makes
      run002a diffe…</span>
