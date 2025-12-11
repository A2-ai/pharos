
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

0.100
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

0.0100
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

**Final OFV:** -103.5

## Estimation Methods

- **First Order Conditional Estimation with Interaction**
  - Condition Number: 29.63

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

10.330
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

40.850
</td>

<td style="text-align:right;">

3.0270
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

## Omega Parameters

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

<th style="text-align:left;">

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

<td style="text-align:left;">

46.15
</td>

<td style="text-align:right;">

18.060
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

<td style="text-align:left;">

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

<td style="text-align:left;">

<span style="color: #DD0000;">53.71</span>
</td>

<td style="text-align:right;">

27.190
</td>

<td style="text-align:left;">

No
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

<th style="text-align:left;">

Random Effect
</th>

<th style="text-align:right;">

Estimate
</th>

<th style="text-align:right;">

SE
</th>

<th style="text-align:left;">

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

0.037230
</td>

<td style="text-align:right;">

0.01160
</td>

<td style="text-align:left;">

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

<td style="text-align:left;">

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

### Parameter Table

``` r
spec <- TableSpec(
  display_transforms = list(omega = c("cv")),
  sections = section_rules(
    kind == "THETA" ~ "Structural model parameters",
    kind == "OMEGA" & diagonal ~ "Interindividual variance parameters",
    kind == "OMEGA" & !diagonal ~ "Interindividual covariance parameters",
    kind == "SIGMA" ~ "Residual variance",
    TRUE ~ "Other"
  )
)

mod_sum <- get_model_summary("vignettes/test_data/models/onecmt/run003")
info <- get_model_parameter_info("vignettes/test_data/models/onecmt/run003")

get_parameters("vignettes/test_data/models/onecmt/run003") |>
  apply_table_spec(info, spec) |>
  add_summary_rows(mod_sum) |>
  make_parameter_table()
```

<div id="asiijfungo" style="padding-left:0px;padding-right:0px;padding-top:10px;padding-bottom:10px;overflow-x:auto;overflow-y:auto;width:auto;height:auto;">
<style>#asiijfungo table {
  font-family: system-ui, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji';
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}
&#10;#asiijfungo thead, #asiijfungo tbody, #asiijfungo tfoot, #asiijfungo tr, #asiijfungo td, #asiijfungo th {
  border-style: none;
}
&#10;#asiijfungo p {
  margin: 0;
  padding: 0;
}
&#10;#asiijfungo .gt_table {
  display: table;
  border-collapse: collapse;
  line-height: normal;
  margin-left: auto;
  margin-right: auto;
  color: #333333;
  font-size: 16px;
  font-weight: normal;
  font-style: normal;
  background-color: #FFFFFF;
  width: auto;
  border-top-style: solid;
  border-top-width: 2px;
  border-top-color: #A8A8A8;
  border-right-style: none;
  border-right-width: 2px;
  border-right-color: #D3D3D3;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #A8A8A8;
  border-left-style: none;
  border-left-width: 2px;
  border-left-color: #D3D3D3;
}
&#10;#asiijfungo .gt_caption {
  padding-top: 4px;
  padding-bottom: 4px;
}
&#10;#asiijfungo .gt_title {
  color: #333333;
  font-size: 125%;
  font-weight: initial;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-color: #FFFFFF;
  border-bottom-width: 0;
}
&#10;#asiijfungo .gt_subtitle {
  color: #333333;
  font-size: 85%;
  font-weight: initial;
  padding-top: 3px;
  padding-bottom: 5px;
  padding-left: 5px;
  padding-right: 5px;
  border-top-color: #FFFFFF;
  border-top-width: 0;
}
&#10;#asiijfungo .gt_heading {
  background-color: #FFFFFF;
  text-align: center;
  border-bottom-color: #FFFFFF;
  border-left-style: none;
  border-left-width: 1px;
  border-left-color: #D3D3D3;
  border-right-style: none;
  border-right-width: 1px;
  border-right-color: #D3D3D3;
}
&#10;#asiijfungo .gt_bottom_border {
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}
&#10;#asiijfungo .gt_col_headings {
  border-top-style: solid;
  border-top-width: 2px;
  border-top-color: #D3D3D3;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
  border-left-style: none;
  border-left-width: 1px;
  border-left-color: #D3D3D3;
  border-right-style: none;
  border-right-width: 1px;
  border-right-color: #D3D3D3;
}
&#10;#asiijfungo .gt_col_heading {
  color: #333333;
  background-color: #FFFFFF;
  font-size: 100%;
  font-weight: normal;
  text-transform: inherit;
  border-left-style: none;
  border-left-width: 1px;
  border-left-color: #D3D3D3;
  border-right-style: none;
  border-right-width: 1px;
  border-right-color: #D3D3D3;
  vertical-align: bottom;
  padding-top: 5px;
  padding-bottom: 6px;
  padding-left: 5px;
  padding-right: 5px;
  overflow-x: hidden;
}
&#10;#asiijfungo .gt_column_spanner_outer {
  color: #333333;
  background-color: #FFFFFF;
  font-size: 100%;
  font-weight: normal;
  text-transform: inherit;
  padding-top: 0;
  padding-bottom: 0;
  padding-left: 4px;
  padding-right: 4px;
}
&#10;#asiijfungo .gt_column_spanner_outer:first-child {
  padding-left: 0;
}
&#10;#asiijfungo .gt_column_spanner_outer:last-child {
  padding-right: 0;
}
&#10;#asiijfungo .gt_column_spanner {
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
  vertical-align: bottom;
  padding-top: 5px;
  padding-bottom: 5px;
  overflow-x: hidden;
  display: inline-block;
  width: 100%;
}
&#10;#asiijfungo .gt_spanner_row {
  border-bottom-style: hidden;
}
&#10;#asiijfungo .gt_group_heading {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  color: #333333;
  background-color: #FFFFFF;
  font-size: 100%;
  font-weight: initial;
  text-transform: inherit;
  border-top-style: solid;
  border-top-width: 2px;
  border-top-color: #D3D3D3;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
  border-left-style: none;
  border-left-width: 1px;
  border-left-color: #D3D3D3;
  border-right-style: none;
  border-right-width: 1px;
  border-right-color: #D3D3D3;
  vertical-align: middle;
  text-align: left;
}
&#10;#asiijfungo .gt_empty_group_heading {
  padding: 0.5px;
  color: #333333;
  background-color: #FFFFFF;
  font-size: 100%;
  font-weight: initial;
  border-top-style: solid;
  border-top-width: 2px;
  border-top-color: #D3D3D3;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
  vertical-align: middle;
}
&#10;#asiijfungo .gt_from_md > :first-child {
  margin-top: 0;
}
&#10;#asiijfungo .gt_from_md > :last-child {
  margin-bottom: 0;
}
&#10;#asiijfungo .gt_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  margin: 10px;
  border-top-style: solid;
  border-top-width: 1px;
  border-top-color: #D3D3D3;
  border-left-style: none;
  border-left-width: 1px;
  border-left-color: #D3D3D3;
  border-right-style: none;
  border-right-width: 1px;
  border-right-color: #D3D3D3;
  vertical-align: middle;
  overflow-x: hidden;
}
&#10;#asiijfungo .gt_stub {
  color: #333333;
  background-color: #FFFFFF;
  font-size: 100%;
  font-weight: initial;
  text-transform: inherit;
  border-right-style: solid;
  border-right-width: 2px;
  border-right-color: #D3D3D3;
  padding-left: 5px;
  padding-right: 5px;
}
&#10;#asiijfungo .gt_stub_row_group {
  color: #333333;
  background-color: #FFFFFF;
  font-size: 100%;
  font-weight: initial;
  text-transform: inherit;
  border-right-style: solid;
  border-right-width: 2px;
  border-right-color: #D3D3D3;
  padding-left: 5px;
  padding-right: 5px;
  vertical-align: top;
}
&#10;#asiijfungo .gt_row_group_first td {
  border-top-width: 2px;
}
&#10;#asiijfungo .gt_row_group_first th {
  border-top-width: 2px;
}
&#10;#asiijfungo .gt_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}
&#10;#asiijfungo .gt_first_summary_row {
  border-top-style: solid;
  border-top-color: #D3D3D3;
}
&#10;#asiijfungo .gt_first_summary_row.thick {
  border-top-width: 2px;
}
&#10;#asiijfungo .gt_last_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}
&#10;#asiijfungo .gt_grand_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}
&#10;#asiijfungo .gt_first_grand_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-top-style: double;
  border-top-width: 6px;
  border-top-color: #D3D3D3;
}
&#10;#asiijfungo .gt_last_grand_summary_row_top {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: double;
  border-bottom-width: 6px;
  border-bottom-color: #D3D3D3;
}
&#10;#asiijfungo .gt_striped {
  background-color: rgba(128, 128, 128, 0.05);
}
&#10;#asiijfungo .gt_table_body {
  border-top-style: solid;
  border-top-width: 2px;
  border-top-color: #D3D3D3;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}
&#10;#asiijfungo .gt_footnotes {
  color: #333333;
  background-color: #FFFFFF;
  border-bottom-style: none;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
  border-left-style: none;
  border-left-width: 2px;
  border-left-color: #D3D3D3;
  border-right-style: none;
  border-right-width: 2px;
  border-right-color: #D3D3D3;
}
&#10;#asiijfungo .gt_footnote {
  margin: 0px;
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}
&#10;#asiijfungo .gt_sourcenotes {
  color: #333333;
  background-color: #FFFFFF;
  border-bottom-style: none;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
  border-left-style: none;
  border-left-width: 2px;
  border-left-color: #D3D3D3;
  border-right-style: none;
  border-right-width: 2px;
  border-right-color: #D3D3D3;
}
&#10;#asiijfungo .gt_sourcenote {
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}
&#10;#asiijfungo .gt_left {
  text-align: left;
}
&#10;#asiijfungo .gt_center {
  text-align: center;
}
&#10;#asiijfungo .gt_right {
  text-align: right;
  font-variant-numeric: tabular-nums;
}
&#10;#asiijfungo .gt_font_normal {
  font-weight: normal;
}
&#10;#asiijfungo .gt_font_bold {
  font-weight: bold;
}
&#10;#asiijfungo .gt_font_italic {
  font-style: italic;
}
&#10;#asiijfungo .gt_super {
  font-size: 65%;
}
&#10;#asiijfungo .gt_footnote_marks {
  font-size: 75%;
  vertical-align: 0.4em;
  position: initial;
}
&#10;#asiijfungo .gt_asterisk {
  font-size: 100%;
  vertical-align: 0;
}
&#10;#asiijfungo .gt_indent_1 {
  text-indent: 5px;
}
&#10;#asiijfungo .gt_indent_2 {
  text-indent: 10px;
}
&#10;#asiijfungo .gt_indent_3 {
  text-indent: 15px;
}
&#10;#asiijfungo .gt_indent_4 {
  text-indent: 20px;
}
&#10;#asiijfungo .gt_indent_5 {
  text-indent: 25px;
}
&#10;#asiijfungo .katex-display {
  display: inline-flex !important;
  margin-bottom: 0.75em !important;
}
&#10;#asiijfungo div.Reactable > div.rt-table > div.rt-thead > div.rt-tr.rt-tr-group-header > div.rt-th-group:after {
  height: 0px !important;
}
&#10;td, th {
  white-space: nowrap;
}
</style>
<table class="gt_table" data-quarto-disable-processing="false" data-quarto-bootstrap="false">
  <thead>
    <tr class="gt_heading">
      <td colspan="8" class="gt_heading gt_title gt_font_normal gt_bottom_border" style="font-weight: bold;">Model Parameters</td>
    </tr>
    &#10;    <tr class="gt_col_headings">
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="name">Parameter</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="symbol"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="unit"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="estimate">Estimate</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="ci_low">95% CI</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="cv"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="rse">RSE (%)</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="shrinkage">Shrinkage (%)</th>
    </tr>
  </thead>
  <tbody class="gt_table_body">
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Structural model parameters">Structural model parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Structural model parameters  name" class="gt_row gt_left"><span class='gt_from_md'>TVCL</span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span class='gt_from_md'>θ<sub>1</sub></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span class='gt_from_md'>L/hr</span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">1.32</td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[1.05, 1.59]</td>
<td headers="Structural model parameters  cv" class="gt_row gt_right"><br /></td>
<td headers="Structural model parameters  rse" class="gt_row gt_right">10.5</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr><td headers="Structural model parameters  name" class="gt_row gt_left"><span class='gt_from_md'>TVV</span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span class='gt_from_md'>θ<sub>2</sub></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span class='gt_from_md'>L</span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">40.2</td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[34.2, 46.2]</td>
<td headers="Structural model parameters  cv" class="gt_row gt_right"><br /></td>
<td headers="Structural model parameters  rse" class="gt_row gt_right">7.65</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr><td headers="Structural model parameters  name" class="gt_row gt_left"><span class='gt_from_md'>TVKA</span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span class='gt_from_md'>θ<sub>3</sub></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span class='gt_from_md'>1/hr</span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">1.21</td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[0.941, 1.48]</td>
<td headers="Structural model parameters  cv" class="gt_row gt_right"><br /></td>
<td headers="Structural model parameters  rse" class="gt_row gt_right">11.4</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Interindividual variance parameters">Interindividual variance parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span class='gt_from_md'>OM1 (TVCL)</span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span class='gt_from_md'>Ω<sub>(1,1)</sub></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.119</td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.00314, 0.235]</td>
<td headers="Interindividual variance parameters  cv" class="gt_row gt_right">[CV = 35.5%]</td>
<td headers="Interindividual variance parameters  rse" class="gt_row gt_right">49.7</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">14.0</td></tr>
    <tr><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span class='gt_from_md'>OM2 (TVV)</span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span class='gt_from_md'>Ω<sub>(2,2)</sub></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.125</td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0376, 0.213]</td>
<td headers="Interindividual variance parameters  cv" class="gt_row gt_right">[CV = 36.5%]</td>
<td headers="Interindividual variance parameters  rse" class="gt_row gt_right">35.7</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">4.58</td></tr>
    <tr><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span class='gt_from_md'>OM3 (TVKA)</span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span class='gt_from_md'>Ω<sub>(3,3)</sub></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.124</td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[−0.0149, 0.262]</td>
<td headers="Interindividual variance parameters  cv" class="gt_row gt_right">[CV = 36.3%]</td>
<td headers="Interindividual variance parameters  rse" class="gt_row gt_right">57.2</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">24.7</td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Interindividual covariance parameters">Interindividual covariance parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Interindividual covariance parameters  name" class="gt_row gt_left"><span class='gt_from_md'>OM(1,2)</span></td>
<td headers="Interindividual covariance parameters  symbol" class="gt_row gt_left"><span class='gt_from_md'>Ω<sub>(2,1)</sub></span></td>
<td headers="Interindividual covariance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual covariance parameters  estimate" class="gt_row gt_right">0.0746</td>
<td headers="Interindividual covariance parameters  ci_low" class="gt_row gt_right">[−0.00433, 0.153]</td>
<td headers="Interindividual covariance parameters  cv" class="gt_row gt_right">[Corr = 0.611]</td>
<td headers="Interindividual covariance parameters  rse" class="gt_row gt_right">54.0</td>
<td headers="Interindividual covariance parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Residual variance">Residual variance</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Residual variance  name" class="gt_row gt_left"><span class='gt_from_md'>SIGMA(1,1)</span></td>
<td headers="Residual variance  symbol" class="gt_row gt_left"><span class='gt_from_md'>Σ<sub>(1,1)</sub></span></td>
<td headers="Residual variance  unit" class="gt_row gt_left"><br /></td>
<td headers="Residual variance  estimate" class="gt_row gt_right">0.0359</td>
<td headers="Residual variance  ci_low" class="gt_row gt_right">[0.0189, 0.0529]</td>
<td headers="Residual variance  cv" class="gt_row gt_right">[SD = 0.189]</td>
<td headers="Residual variance  rse" class="gt_row gt_right">24.2</td>
<td headers="Residual variance  shrinkage" class="gt_row gt_right">14.5</td></tr>
    <tr><td headers="Residual variance  name" class="gt_row gt_left"><span class='gt_from_md'>SIGMA(2,2)</span></td>
<td headers="Residual variance  symbol" class="gt_row gt_left"><span class='gt_from_md'>Σ<sub>(2,2)</sub></span></td>
<td headers="Residual variance  unit" class="gt_row gt_left"><br /></td>
<td headers="Residual variance  estimate" class="gt_row gt_right">0.0100</td>
<td headers="Residual variance  ci_low" class="gt_row gt_right">Fixed</td>
<td headers="Residual variance  cv" class="gt_row gt_right">Fixed</td>
<td headers="Residual variance  rse" class="gt_row gt_right"><br /></td>
<td headers="Residual variance  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Other">Other</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Other  name" class="gt_row gt_left"><span class='gt_from_md'>OFV</span></td>
<td headers="Other  symbol" class="gt_row gt_left"><br /></td>
<td headers="Other  unit" class="gt_row gt_left"><br /></td>
<td headers="Other  estimate" class="gt_row gt_right">−110</td>
<td headers="Other  ci_low" class="gt_row gt_right"><br /></td>
<td headers="Other  cv" class="gt_row gt_right"><br /></td>
<td headers="Other  rse" class="gt_row gt_right"><br /></td>
<td headers="Other  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr><td headers="Other  name" class="gt_row gt_left"><span class='gt_from_md'>Condition Number</span></td>
<td headers="Other  symbol" class="gt_row gt_left"><br /></td>
<td headers="Other  unit" class="gt_row gt_left"><br /></td>
<td headers="Other  estimate" class="gt_row gt_right">15.3</td>
<td headers="Other  ci_low" class="gt_row gt_right"><br /></td>
<td headers="Other  cv" class="gt_row gt_right"><br /></td>
<td headers="Other  rse" class="gt_row gt_right"><br /></td>
<td headers="Other  shrinkage" class="gt_row gt_right"><br /></td></tr>
  </tbody>
  <tfoot>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> Abbreviations:</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> CI = confidence intervals; RSE = relative standard error; CV = coefficient of variation; SD = standard deviation</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> <span class='gt_from_md'>95% CI: <link rel="stylesheet" type="text/css" href="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.css" data-external="1">
<span class="katex"><span class="katex-mathml"><math xmlns="http://www.w3.org/1998/Math/MathML"><semantics><mrow><mtext>Estimate</mtext><mo>±</mo><msub><mi>z</mi><mn>0.025</mn></msub><mtext> </mtext><mrow><mi mathvariant="normal">S</mi><mi mathvariant="normal">E</mi></mrow></mrow><annotation encoding="application/x-tex">\text{Estimate} \pm z_{0.025}\,\mathrm{SE}</annotation></semantics></math></span><span class="katex-html" aria-hidden="true"><span class="base"><span class="strut" style="height:0.7667em;vertical-align:-0.0833em;"></span><span class="mord text"><span class="mord">Estimate</span></span><span class="mspace" style="margin-right:0.2222em;"></span><span class="mbin">±</span><span class="mspace" style="margin-right:0.2222em;"></span></span><span class="base"><span class="strut" style="height:0.8333em;vertical-align:-0.15em;"></span><span class="mord"><span class="mord mathnormal" style="margin-right:0.04398em;">z</span><span class="msupsub"><span class="vlist-t vlist-t2"><span class="vlist-r"><span class="vlist" style="height:0.3011em;"><span style="top:-2.55em;margin-left:-0.044em;margin-right:0.05em;"><span class="pstrut" style="height:2.7em;"></span><span class="sizing reset-size6 size3 mtight"><span class="mord mtight"><span class="mord mtight">0.025</span></span></span></span></span><span class="vlist-s">​</span></span><span class="vlist-r"><span class="vlist" style="height:0.15em;"><span></span></span></span></span></span></span><span class="mspace" style="margin-right:0.1667em;"></span><span class="mord"><span class="mord mathrm">SE</span></span></span></span></span></span></td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> <span class='gt_from_md'>CV% for log-normal OMEGA diagonals: <link rel="stylesheet" type="text/css" href="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.css" data-external="1">
<span class="katex"><span class="katex-mathml"><math xmlns="http://www.w3.org/1998/Math/MathML"><semantics><mrow><mtext>CV%</mtext><mo>=</mo><msqrt><mrow><msup><mi>e</mi><mtext>Estimate</mtext></msup><mo>−</mo><mn>1</mn></mrow></msqrt><mo>×</mo><mn>100</mn></mrow><annotation encoding="application/x-tex">\text{CV\%} = \sqrt{e^{\text{Estimate}} - 1} \times 100</annotation></semantics></math></span><span class="katex-html" aria-hidden="true"><span class="base"><span class="strut" style="height:0.8056em;vertical-align:-0.0556em;"></span><span class="mord text"><span class="mord">CV%</span></span><span class="mspace" style="margin-right:0.2778em;"></span><span class="mrel">=</span><span class="mspace" style="margin-right:0.2778em;"></span></span><span class="base"><span class="strut" style="height:1.04em;vertical-align:-0.113em;"></span><span class="mord sqrt"><span class="vlist-t vlist-t2"><span class="vlist-r"><span class="vlist" style="height:0.927em;"><span class="svg-align" style="top:-3em;"><span class="pstrut" style="height:3em;"></span><span class="mord" style="padding-left:0.833em;"><span class="mord"><span class="mord mathnormal">e</span><span class="msupsub"><span class="vlist-t"><span class="vlist-r"><span class="vlist" style="height:0.7673em;"><span style="top:-2.989em;margin-right:0.05em;"><span class="pstrut" style="height:2.7em;"></span><span class="sizing reset-size6 size3 mtight"><span class="mord mtight"><span class="mord text mtight"><span class="mord mtight">Estimate</span></span></span></span></span></span></span></span></span></span><span class="mspace" style="margin-right:0.2222em;"></span><span class="mbin">−</span><span class="mspace" style="margin-right:0.2222em;"></span><span class="mord">1</span></span></span><span style="top:-2.887em;"><span class="pstrut" style="height:3em;"></span><span class="hide-tail" style="min-width:0.853em;height:1.08em;"><svg xmlns="http://www.w3.org/2000/svg" width='400em' height='1.08em' viewBox='0 0 400000 1080' preserveAspectRatio='xMinYMin slice'><path d='M95,702
c-2.7,0,-7.17,-2.7,-13.5,-8c-5.8,-5.3,-9.5,-10,-9.5,-14
c0,-2,0.3,-3.3,1,-4c1.3,-2.7,23.83,-20.7,67.5,-54
c44.2,-33.3,65.8,-50.3,66.5,-51c1.3,-1.3,3,-2,5,-2c4.7,0,8.7,3.3,12,10
s173,378,173,378c0.7,0,35.3,-71,104,-213c68.7,-142,137.5,-285,206.5,-429
c69,-144,104.5,-217.7,106.5,-221
l0 -0
c5.3,-9.3,12,-14,20,-14
H400000v40H845.2724
s-225.272,467,-225.272,467s-235,486,-235,486c-2.7,4.7,-9,7,-19,7
c-6,0,-10,-1,-12,-3s-194,-422,-194,-422s-65,47,-65,47z
M834 80h400000v40h-400000z'/></svg></span></span></span><span class="vlist-s">​</span></span><span class="vlist-r"><span class="vlist" style="height:0.113em;"><span></span></span></span></span></span><span class="mspace" style="margin-right:0.2222em;"></span><span class="mbin">×</span><span class="mspace" style="margin-right:0.2222em;"></span></span><span class="base"><span class="strut" style="height:0.6444em;"></span><span class="mord">100</span></span></span></span></span></td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> <span class='gt_from_md'>CV% of proportional error: <link rel="stylesheet" type="text/css" href="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.css" data-external="1">
<span class="katex"><span class="katex-mathml"><math xmlns="http://www.w3.org/1998/Math/MathML"><semantics><mrow><mtext>CV%</mtext><mo>=</mo><msqrt><mtext>Estimate</mtext></msqrt><mo>×</mo><mn>100</mn></mrow><annotation encoding="application/x-tex">\text{CV\%} = \sqrt{\text{Estimate}} \times 100</annotation></semantics></math></span><span class="katex-html" aria-hidden="true"><span class="base"><span class="strut" style="height:0.8056em;vertical-align:-0.0556em;"></span><span class="mord text"><span class="mord">CV%</span></span><span class="mspace" style="margin-right:0.2778em;"></span><span class="mrel">=</span><span class="mspace" style="margin-right:0.2778em;"></span></span><span class="base"><span class="strut" style="height:1.04em;vertical-align:-0.1133em;"></span><span class="mord sqrt"><span class="vlist-t vlist-t2"><span class="vlist-r"><span class="vlist" style="height:0.9267em;"><span class="svg-align" style="top:-3em;"><span class="pstrut" style="height:3em;"></span><span class="mord" style="padding-left:0.833em;"><span class="mord text"><span class="mord">Estimate</span></span></span></span><span style="top:-2.8867em;"><span class="pstrut" style="height:3em;"></span><span class="hide-tail" style="min-width:0.853em;height:1.08em;"><svg xmlns="http://www.w3.org/2000/svg" width='400em' height='1.08em' viewBox='0 0 400000 1080' preserveAspectRatio='xMinYMin slice'><path d='M95,702
c-2.7,0,-7.17,-2.7,-13.5,-8c-5.8,-5.3,-9.5,-10,-9.5,-14
c0,-2,0.3,-3.3,1,-4c1.3,-2.7,23.83,-20.7,67.5,-54
c44.2,-33.3,65.8,-50.3,66.5,-51c1.3,-1.3,3,-2,5,-2c4.7,0,8.7,3.3,12,10
s173,378,173,378c0.7,0,35.3,-71,104,-213c68.7,-142,137.5,-285,206.5,-429
c69,-144,104.5,-217.7,106.5,-221
l0 -0
c5.3,-9.3,12,-14,20,-14
H400000v40H845.2724
s-225.272,467,-225.272,467s-235,486,-235,486c-2.7,4.7,-9,7,-19,7
c-6,0,-10,-1,-12,-3s-194,-422,-194,-422s-65,47,-65,47z
M834 80h400000v40h-400000z'/></svg></span></span></span><span class="vlist-s">​</span></span><span class="vlist-r"><span class="vlist" style="height:0.1133em;"><span></span></span></span></span></span><span class="mspace" style="margin-right:0.2222em;"></span><span class="mbin">×</span><span class="mspace" style="margin-right:0.2222em;"></span></span><span class="base"><span class="strut" style="height:0.6444em;"></span><span class="mord">100</span></span></span></span></span></td>
    </tr>
  </tfoot>
</table>
</div>

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

ℹ️ **Models:** 7

- <strong style="color:blue">run001</strong>
  - <span style="color:green">run004</span> <span style="color:gray">-
    Updating run001 to run004 with jittered params …</span>
  - <span style="color:orange">run002</span> <span style="color:gray">-
    Adding COV step, unfixing eps(2)</span>
    - <span style="color:green">run002b001</span>
      <span style="color:gray">- Jittering initial sigma estimates,
      using theta/…</span>
    - <span style="color:green">run002a</span>
      <span style="color:gray">- Some description about what makes
      run002a diffe…</span>
    - <span style="color:orange">run003</span>
      <span style="color:gray">- Jittering initial estimates</span>
      - <span style="color:green">run003b1</span>
        <span style="color:gray">- Updating run003 to 003b1 with
        jittered params</span>
