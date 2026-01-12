---
title: "parameter-tables"
output: rmarkdown::html_vignette
vignette: >
  %\VignetteIndexEntry{parameter-tables}
  %\VignetteEngine{knitr::rmarkdown}
  %\VignetteEncoding{UTF-8}
---



::: {.cell}

```{.r .cell-code}
#library(hyperion)
devtools::load_all()
#> ℹ Loading hyperion
#> 
#> 
#> ── pharos configuration ────────────────────────────────────────────────────────
#> ✔ pharos.toml found: /Users/mattsmith/Documents/hyperion/vignettes/pharos.toml
#> ── hyperion options ────────────────────────────────────────────────────────────
#> ✔ hyperion.significant_number_display : 4
#> ── hyperion nonmem object options ──────────────────────────────────────────────
#> ✔ hyperion.nonmem_model.show_included_columns : FALSE
#> ✔ hyperion.nonmem_summary.rse_threshold : 50
#> ✔ hyperion.nonmem_summary.shrinkage_threshold : 30
library(gt)

model_dir <- "test_data/models/onecmt"
model_run <- "run003"
```
:::

::: {.cell}

```{.r .cell-code}
spec <- TableSpec(
  display_transforms = list(omega = c("cv")),
  sections = section_rules(
    kind == "THETA" ~ "Structural model parameters",
    kind == "OMEGA" & diagonal ~ "Interindividual variance parameters",
    kind == "OMEGA" & !diagonal ~ "Interindividual covariance parameters",
    kind == "SIGMA" ~ "Residual error",
    TRUE ~ "Other"
  ),
  name_source = "display",
  drop_columns = "rse",
  title = paste(model_run, "Parameters")
)

info <- get_model_parameter_info(
  file.path(model_dir, model_run),
  normalizePath("../inst/lookup.yaml")
)
info@sigma$`SIGMA(1,1)`@parameterization <- "Proportional"

mod_sum <- get_model_summary(file.path(model_dir, model_run))

get_parameters(file.path(model_dir, model_run)) |>
  apply_table_spec(info, spec) |>
  add_summary_info(mod_sum) |>
	make_parameter_table()
```

::: {.cell-output-display}

```{=html}
<div id="mikqrclhgf" style="padding-left:0px;padding-right:0px;padding-top:10px;padding-bottom:10px;overflow-x:auto;overflow-y:auto;width:auto;height:auto;">
<style>#mikqrclhgf table {
  font-family: system-ui, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji';
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

#mikqrclhgf thead, #mikqrclhgf tbody, #mikqrclhgf tfoot, #mikqrclhgf tr, #mikqrclhgf td, #mikqrclhgf th {
  border-style: none;
}

#mikqrclhgf p {
  margin: 0;
  padding: 0;
}

#mikqrclhgf .gt_table {
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

#mikqrclhgf .gt_caption {
  padding-top: 4px;
  padding-bottom: 4px;
}

#mikqrclhgf .gt_title {
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

#mikqrclhgf .gt_subtitle {
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

#mikqrclhgf .gt_heading {
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

#mikqrclhgf .gt_bottom_border {
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#mikqrclhgf .gt_col_headings {
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

#mikqrclhgf .gt_col_heading {
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

#mikqrclhgf .gt_column_spanner_outer {
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

#mikqrclhgf .gt_column_spanner_outer:first-child {
  padding-left: 0;
}

#mikqrclhgf .gt_column_spanner_outer:last-child {
  padding-right: 0;
}

#mikqrclhgf .gt_column_spanner {
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

#mikqrclhgf .gt_spanner_row {
  border-bottom-style: hidden;
}

#mikqrclhgf .gt_group_heading {
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

#mikqrclhgf .gt_empty_group_heading {
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

#mikqrclhgf .gt_from_md > :first-child {
  margin-top: 0;
}

#mikqrclhgf .gt_from_md > :last-child {
  margin-bottom: 0;
}

#mikqrclhgf .gt_row {
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

#mikqrclhgf .gt_stub {
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

#mikqrclhgf .gt_stub_row_group {
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

#mikqrclhgf .gt_row_group_first td {
  border-top-width: 2px;
}

#mikqrclhgf .gt_row_group_first th {
  border-top-width: 2px;
}

#mikqrclhgf .gt_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#mikqrclhgf .gt_first_summary_row {
  border-top-style: solid;
  border-top-color: #D3D3D3;
}

#mikqrclhgf .gt_first_summary_row.thick {
  border-top-width: 2px;
}

#mikqrclhgf .gt_last_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#mikqrclhgf .gt_grand_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#mikqrclhgf .gt_first_grand_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-top-style: double;
  border-top-width: 6px;
  border-top-color: #D3D3D3;
}

#mikqrclhgf .gt_last_grand_summary_row_top {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: double;
  border-bottom-width: 6px;
  border-bottom-color: #D3D3D3;
}

#mikqrclhgf .gt_striped {
  background-color: rgba(128, 128, 128, 0.05);
}

#mikqrclhgf .gt_table_body {
  border-top-style: solid;
  border-top-width: 2px;
  border-top-color: #D3D3D3;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#mikqrclhgf .gt_footnotes {
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

#mikqrclhgf .gt_footnote {
  margin: 0px;
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#mikqrclhgf .gt_sourcenotes {
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

#mikqrclhgf .gt_sourcenote {
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#mikqrclhgf .gt_left {
  text-align: left;
}

#mikqrclhgf .gt_center {
  text-align: center;
}

#mikqrclhgf .gt_right {
  text-align: right;
  font-variant-numeric: tabular-nums;
}

#mikqrclhgf .gt_font_normal {
  font-weight: normal;
}

#mikqrclhgf .gt_font_bold {
  font-weight: bold;
}

#mikqrclhgf .gt_font_italic {
  font-style: italic;
}

#mikqrclhgf .gt_super {
  font-size: 65%;
}

#mikqrclhgf .gt_footnote_marks {
  font-size: 75%;
  vertical-align: 0.4em;
  position: initial;
}

#mikqrclhgf .gt_asterisk {
  font-size: 100%;
  vertical-align: 0;
}

#mikqrclhgf .gt_indent_1 {
  text-indent: 5px;
}

#mikqrclhgf .gt_indent_2 {
  text-indent: 10px;
}

#mikqrclhgf .gt_indent_3 {
  text-indent: 15px;
}

#mikqrclhgf .gt_indent_4 {
  text-indent: 20px;
}

#mikqrclhgf .gt_indent_5 {
  text-indent: 25px;
}

#mikqrclhgf .katex-display {
  display: inline-flex !important;
  margin-bottom: 0.75em !important;
}

#mikqrclhgf div.Reactable > div.rt-table > div.rt-thead > div.rt-tr.rt-tr-group-header > div.rt-th-group:after {
  height: 0px !important;
}

td, th {
  white-space: nowrap;
}
</style>
<table class="gt_table" data-quarto-disable-processing="false" data-quarto-bootstrap="false">
  <thead>
    <tr class="gt_heading">
      <td colspan="7" class="gt_heading gt_title gt_font_normal gt_bottom_border" style="font-weight: bold;">run003 Parameters</td>
    </tr>
    
    <tr class="gt_col_headings">
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="name">Parameter</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="symbol"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="unit"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="estimate">Estimate</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="variability"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="ci_low">95% CI</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="shrinkage">Shrinkage (%)</th>
    </tr>
  </thead>
  <tbody class="gt_table_body">
    <tr class="gt_group_heading_row">
      <th colspan="7" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Structural model parameters">Structural model parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="Q0w="><span class='gt_from_md'>CL</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97MX0k"><span class='gt_from_md'>\(\theta_{1}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="TC9ocg=="><span class='gt_from_md'>L/hr</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">1.33</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[1.11, 1.54]</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="VmM="><span class='gt_from_md'>Vc</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97Mn0k"><span class='gt_from_md'>\(\theta_{2}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="TA=="><span class='gt_from_md'>L</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">40.2</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[34.6, 45.7]</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="S0E="><span class='gt_from_md'>KA</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97M30k"><span class='gt_from_md'>\(\theta_{3}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="MS9ocg=="><span class='gt_from_md'>1/hr</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">1.21</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[0.997, 1.43]</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="7" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Interindividual variance parameters">Interindividual variance parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="SUlWLUNMIChUVkNMKQ=="><span class='gt_from_md'>IIV-CL (TVCL)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDEsMSl9JA=="><span class='gt_from_md'>\(\Omega_{(1,1)}\)</span></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.122</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMSUp"><span class='gt_from_md'>(CV = 36.1%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0236, 0.221]</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">13.1</td></tr>
    <tr><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="SUlWLVZjIChUVlYp"><span class='gt_from_md'>IIV-Vc (TVV)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDIsMil9JA=="><span class='gt_from_md'>\(\Omega_{(2,2)}\)</span></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.124</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMyUp"><span class='gt_from_md'>(CV = 36.3%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0519, 0.196]</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">4.63</td></tr>
    <tr><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="SUlWLUtBIChUVktBKQ=="><span class='gt_from_md'>IIV-KA (TVKA)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDMsMyl9JA=="><span class='gt_from_md'>\(\Omega_{(3,3)}\)</span></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.122</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMSUp"><span class='gt_from_md'>(CV = 36.1%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0121, 0.233]</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">24.3</td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="7" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Interindividual covariance parameters">Interindividual covariance parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Interindividual covariance parameters  name" class="gt_row gt_left"><span data-qmd-base64="SUlWLUNMLVZjIChUVkNMLVRWVik="><span class='gt_from_md'>IIV-CL-Vc (TVCL-TVV)</span></span></td>
<td headers="Interindividual covariance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDIsMSl9JA=="><span class='gt_from_md'>\(\Omega_{(2,1)}\)</span></span></td>
<td headers="Interindividual covariance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual covariance parameters  estimate" class="gt_row gt_right">0.0745</td>
<td headers="Interindividual covariance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENvcnIgPSAwLjYwNik="><span class='gt_from_md'>(Corr = 0.606)</span></span></td>
<td headers="Interindividual covariance parameters  ci_low" class="gt_row gt_right">[0.0131, 0.136]</td>
<td headers="Interindividual covariance parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="7" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Residual error">Residual error</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Residual error  name" class="gt_row gt_left"><span data-qmd-base64="UHJvcEVycg=="><span class='gt_from_md'>PropErr</span></span></td>
<td headers="Residual error  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxTaWdtYV97KDEsMSl9JA=="><span class='gt_from_md'>\(\Sigma_{(1,1)}\)</span></span></td>
<td headers="Residual error  unit" class="gt_row gt_left"><br /></td>
<td headers="Residual error  estimate" class="gt_row gt_right">0.0375</td>
<td headers="Residual error  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMTkuNCUp"><span class='gt_from_md'>(CV = 19.4%)</span></span></td>
<td headers="Residual error  ci_low" class="gt_row gt_right">[0.0257, 0.0494]</td>
<td headers="Residual error  shrinkage" class="gt_row gt_right">14.4</td></tr>
    <tr><td headers="Residual error  name" class="gt_row gt_left"><span data-qmd-base64="QWRkRXJy"><span class='gt_from_md'>AddErr</span></span></td>
<td headers="Residual error  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxTaWdtYV97KDIsMil9JA=="><span class='gt_from_md'>\(\Sigma_{(2,2)}\)</span></span></td>
<td headers="Residual error  unit" class="gt_row gt_left"><br /></td>
<td headers="Residual error  estimate" class="gt_row gt_right">0.00527</td>
<td headers="Residual error  variability" class="gt_row gt_left"><span data-qmd-base64="KFNEID0gMC4wNzI2KQ=="><span class='gt_from_md'>(SD = 0.0726)</span></span></td>
<td headers="Residual error  ci_low" class="gt_row gt_right">[−0.0128, 0.0233]</td>
<td headers="Residual error  shrinkage" class="gt_row gt_right">14.4</td></tr>
  </tbody>
  <tfoot>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> First Order Conditional Estimation with Interaction | Objective function value: -110 | Condition Number: 6.17</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> Abbreviations:</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> CI = confidence intervals; CV = coefficient of variation; SD = standard deviation; Corr = correlation</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> <span data-qmd-base64="OTUlIENJOiAkXG1hdGhybXtFc3RpbWF0ZX0gXHBtIHpfezAuMDI1fSBcY2RvdCBcbWF0aHJte1NFfSQ="><span class='gt_from_md'>95% CI: \(\mathrm{Estimate} \pm z_{0.025} \cdot \mathrm{SE}\)</span></span></td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> <span data-qmd-base64="Q1YlIGZvciBsb2ctbm9ybWFsICRcT21lZ2EkOiAkXHNxcnR7XGV4cChcbWF0aHJte0VzdGltYXRlfSkgLSAxfSBcdGltZXMgMTAwJA=="><span class='gt_from_md'>CV% for log-normal \(\Omega\): \(\sqrt{\exp(\mathrm{Estimate}) - 1} \times 100\)</span></span></td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> <span data-qmd-base64="Q1YlIGZvciBwcm9wb3J0aW9uYWwgJFxTaWdtYSQ6ICRcc3FydHtcbWF0aHJte0VzdGltYXRlfX0gXHRpbWVzIDEwMCQ="><span class='gt_from_md'>CV% for proportional \(\Sigma\): \(\sqrt{\mathrm{Estimate}} \times 100\)</span></span></td>
    </tr>
  </tfoot>
</table>
</div>
```

:::
:::


# Changing display names with model info `@name_source`



::: {.cell}

```{.r .cell-code}
spec@name_source <- "display"

info@sigma$`SIGMA(1,1)`@display <- "Additive Error"
info@sigma$`SIGMA(2,2)`@display <- "Proportional Error"

mod_sum <- get_model_summary(file.path(model_dir, model_run))

get_parameters(file.path(model_dir, model_run)) |>
  apply_table_spec(info, spec) |>
  add_summary_info(mod_sum) |>
  make_parameter_table()
```

::: {.cell-output-display}

```{=html}
<div id="ntnlsvwbjs" style="padding-left:0px;padding-right:0px;padding-top:10px;padding-bottom:10px;overflow-x:auto;overflow-y:auto;width:auto;height:auto;">
<style>#ntnlsvwbjs table {
  font-family: system-ui, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji';
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

#ntnlsvwbjs thead, #ntnlsvwbjs tbody, #ntnlsvwbjs tfoot, #ntnlsvwbjs tr, #ntnlsvwbjs td, #ntnlsvwbjs th {
  border-style: none;
}

#ntnlsvwbjs p {
  margin: 0;
  padding: 0;
}

#ntnlsvwbjs .gt_table {
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

#ntnlsvwbjs .gt_caption {
  padding-top: 4px;
  padding-bottom: 4px;
}

#ntnlsvwbjs .gt_title {
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

#ntnlsvwbjs .gt_subtitle {
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

#ntnlsvwbjs .gt_heading {
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

#ntnlsvwbjs .gt_bottom_border {
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#ntnlsvwbjs .gt_col_headings {
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

#ntnlsvwbjs .gt_col_heading {
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

#ntnlsvwbjs .gt_column_spanner_outer {
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

#ntnlsvwbjs .gt_column_spanner_outer:first-child {
  padding-left: 0;
}

#ntnlsvwbjs .gt_column_spanner_outer:last-child {
  padding-right: 0;
}

#ntnlsvwbjs .gt_column_spanner {
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

#ntnlsvwbjs .gt_spanner_row {
  border-bottom-style: hidden;
}

#ntnlsvwbjs .gt_group_heading {
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

#ntnlsvwbjs .gt_empty_group_heading {
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

#ntnlsvwbjs .gt_from_md > :first-child {
  margin-top: 0;
}

#ntnlsvwbjs .gt_from_md > :last-child {
  margin-bottom: 0;
}

#ntnlsvwbjs .gt_row {
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

#ntnlsvwbjs .gt_stub {
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

#ntnlsvwbjs .gt_stub_row_group {
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

#ntnlsvwbjs .gt_row_group_first td {
  border-top-width: 2px;
}

#ntnlsvwbjs .gt_row_group_first th {
  border-top-width: 2px;
}

#ntnlsvwbjs .gt_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#ntnlsvwbjs .gt_first_summary_row {
  border-top-style: solid;
  border-top-color: #D3D3D3;
}

#ntnlsvwbjs .gt_first_summary_row.thick {
  border-top-width: 2px;
}

#ntnlsvwbjs .gt_last_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#ntnlsvwbjs .gt_grand_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#ntnlsvwbjs .gt_first_grand_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-top-style: double;
  border-top-width: 6px;
  border-top-color: #D3D3D3;
}

#ntnlsvwbjs .gt_last_grand_summary_row_top {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: double;
  border-bottom-width: 6px;
  border-bottom-color: #D3D3D3;
}

#ntnlsvwbjs .gt_striped {
  background-color: rgba(128, 128, 128, 0.05);
}

#ntnlsvwbjs .gt_table_body {
  border-top-style: solid;
  border-top-width: 2px;
  border-top-color: #D3D3D3;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#ntnlsvwbjs .gt_footnotes {
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

#ntnlsvwbjs .gt_footnote {
  margin: 0px;
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#ntnlsvwbjs .gt_sourcenotes {
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

#ntnlsvwbjs .gt_sourcenote {
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#ntnlsvwbjs .gt_left {
  text-align: left;
}

#ntnlsvwbjs .gt_center {
  text-align: center;
}

#ntnlsvwbjs .gt_right {
  text-align: right;
  font-variant-numeric: tabular-nums;
}

#ntnlsvwbjs .gt_font_normal {
  font-weight: normal;
}

#ntnlsvwbjs .gt_font_bold {
  font-weight: bold;
}

#ntnlsvwbjs .gt_font_italic {
  font-style: italic;
}

#ntnlsvwbjs .gt_super {
  font-size: 65%;
}

#ntnlsvwbjs .gt_footnote_marks {
  font-size: 75%;
  vertical-align: 0.4em;
  position: initial;
}

#ntnlsvwbjs .gt_asterisk {
  font-size: 100%;
  vertical-align: 0;
}

#ntnlsvwbjs .gt_indent_1 {
  text-indent: 5px;
}

#ntnlsvwbjs .gt_indent_2 {
  text-indent: 10px;
}

#ntnlsvwbjs .gt_indent_3 {
  text-indent: 15px;
}

#ntnlsvwbjs .gt_indent_4 {
  text-indent: 20px;
}

#ntnlsvwbjs .gt_indent_5 {
  text-indent: 25px;
}

#ntnlsvwbjs .katex-display {
  display: inline-flex !important;
  margin-bottom: 0.75em !important;
}

#ntnlsvwbjs div.Reactable > div.rt-table > div.rt-thead > div.rt-tr.rt-tr-group-header > div.rt-th-group:after {
  height: 0px !important;
}

td, th {
  white-space: nowrap;
}
</style>
<table class="gt_table" data-quarto-disable-processing="false" data-quarto-bootstrap="false">
  <thead>
    <tr class="gt_heading">
      <td colspan="7" class="gt_heading gt_title gt_font_normal gt_bottom_border" style="font-weight: bold;">run003 Parameters</td>
    </tr>
    
    <tr class="gt_col_headings">
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="name">Parameter</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="symbol"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="unit"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="estimate">Estimate</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="variability"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="ci_low">95% CI</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="shrinkage">Shrinkage (%)</th>
    </tr>
  </thead>
  <tbody class="gt_table_body">
    <tr class="gt_group_heading_row">
      <th colspan="7" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Structural model parameters">Structural model parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="Q0w="><span class='gt_from_md'>CL</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97MX0k"><span class='gt_from_md'>\(\theta_{1}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="TC9ocg=="><span class='gt_from_md'>L/hr</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">1.33</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[1.11, 1.54]</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="VmM="><span class='gt_from_md'>Vc</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97Mn0k"><span class='gt_from_md'>\(\theta_{2}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="TA=="><span class='gt_from_md'>L</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">40.2</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[34.6, 45.7]</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="S0E="><span class='gt_from_md'>KA</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97M30k"><span class='gt_from_md'>\(\theta_{3}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="MS9ocg=="><span class='gt_from_md'>1/hr</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">1.21</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[0.997, 1.43]</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="7" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Interindividual variance parameters">Interindividual variance parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="SUlWLUNMIChUVkNMKQ=="><span class='gt_from_md'>IIV-CL (TVCL)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDEsMSl9JA=="><span class='gt_from_md'>\(\Omega_{(1,1)}\)</span></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.122</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMSUp"><span class='gt_from_md'>(CV = 36.1%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0236, 0.221]</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">13.1</td></tr>
    <tr><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="SUlWLVZjIChUVlYp"><span class='gt_from_md'>IIV-Vc (TVV)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDIsMil9JA=="><span class='gt_from_md'>\(\Omega_{(2,2)}\)</span></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.124</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMyUp"><span class='gt_from_md'>(CV = 36.3%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0519, 0.196]</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">4.63</td></tr>
    <tr><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="SUlWLUtBIChUVktBKQ=="><span class='gt_from_md'>IIV-KA (TVKA)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDMsMyl9JA=="><span class='gt_from_md'>\(\Omega_{(3,3)}\)</span></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.122</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMSUp"><span class='gt_from_md'>(CV = 36.1%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0121, 0.233]</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">24.3</td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="7" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Interindividual covariance parameters">Interindividual covariance parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Interindividual covariance parameters  name" class="gt_row gt_left"><span data-qmd-base64="SUlWLUNMLVZjIChUVkNMLVRWVik="><span class='gt_from_md'>IIV-CL-Vc (TVCL-TVV)</span></span></td>
<td headers="Interindividual covariance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDIsMSl9JA=="><span class='gt_from_md'>\(\Omega_{(2,1)}\)</span></span></td>
<td headers="Interindividual covariance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual covariance parameters  estimate" class="gt_row gt_right">0.0745</td>
<td headers="Interindividual covariance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENvcnIgPSAwLjYwNik="><span class='gt_from_md'>(Corr = 0.606)</span></span></td>
<td headers="Interindividual covariance parameters  ci_low" class="gt_row gt_right">[0.0131, 0.136]</td>
<td headers="Interindividual covariance parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="7" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Residual error">Residual error</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Residual error  name" class="gt_row gt_left"><span data-qmd-base64="QWRkaXRpdmUgRXJyb3I="><span class='gt_from_md'>Additive Error</span></span></td>
<td headers="Residual error  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxTaWdtYV97KDEsMSl9JA=="><span class='gt_from_md'>\(\Sigma_{(1,1)}\)</span></span></td>
<td headers="Residual error  unit" class="gt_row gt_left"><br /></td>
<td headers="Residual error  estimate" class="gt_row gt_right">0.0375</td>
<td headers="Residual error  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMTkuNCUp"><span class='gt_from_md'>(CV = 19.4%)</span></span></td>
<td headers="Residual error  ci_low" class="gt_row gt_right">[0.0257, 0.0494]</td>
<td headers="Residual error  shrinkage" class="gt_row gt_right">14.4</td></tr>
    <tr><td headers="Residual error  name" class="gt_row gt_left"><span data-qmd-base64="UHJvcG9ydGlvbmFsIEVycm9y"><span class='gt_from_md'>Proportional Error</span></span></td>
<td headers="Residual error  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxTaWdtYV97KDIsMil9JA=="><span class='gt_from_md'>\(\Sigma_{(2,2)}\)</span></span></td>
<td headers="Residual error  unit" class="gt_row gt_left"><br /></td>
<td headers="Residual error  estimate" class="gt_row gt_right">0.00527</td>
<td headers="Residual error  variability" class="gt_row gt_left"><span data-qmd-base64="KFNEID0gMC4wNzI2KQ=="><span class='gt_from_md'>(SD = 0.0726)</span></span></td>
<td headers="Residual error  ci_low" class="gt_row gt_right">[−0.0128, 0.0233]</td>
<td headers="Residual error  shrinkage" class="gt_row gt_right">14.4</td></tr>
  </tbody>
  <tfoot>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> First Order Conditional Estimation with Interaction | Objective function value: -110 | Condition Number: 6.17</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> Abbreviations:</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> CI = confidence intervals; CV = coefficient of variation; SD = standard deviation; Corr = correlation</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> <span data-qmd-base64="OTUlIENJOiAkXG1hdGhybXtFc3RpbWF0ZX0gXHBtIHpfezAuMDI1fSBcY2RvdCBcbWF0aHJte1NFfSQ="><span class='gt_from_md'>95% CI: \(\mathrm{Estimate} \pm z_{0.025} \cdot \mathrm{SE}\)</span></span></td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> <span data-qmd-base64="Q1YlIGZvciBsb2ctbm9ybWFsICRcT21lZ2EkOiAkXHNxcnR7XGV4cChcbWF0aHJte0VzdGltYXRlfSkgLSAxfSBcdGltZXMgMTAwJA=="><span class='gt_from_md'>CV% for log-normal \(\Omega\): \(\sqrt{\exp(\mathrm{Estimate}) - 1} \times 100\)</span></span></td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> <span data-qmd-base64="Q1YlIGZvciBwcm9wb3J0aW9uYWwgJFxTaWdtYSQ6ICRcc3FydHtcbWF0aHJte0VzdGltYXRlfX0gXHRpbWVzIDEwMCQ="><span class='gt_from_md'>CV% for proportional \(\Sigma\): \(\sqrt{\mathrm{Estimate}} \times 100\)</span></span></td>
    </tr>
  </tfoot>
</table>
</div>
```

:::
:::

::: {.cell}

```{.r .cell-code}
spec@name_source <- "nonmem_name"

get_parameters(file.path(model_dir, model_run)) |>
  apply_table_spec(info, spec) |>
  add_summary_info(mod_sum) |>
  make_parameter_table()
```

::: {.cell-output-display}

```{=html}
<div id="cadhissaxn" style="padding-left:0px;padding-right:0px;padding-top:10px;padding-bottom:10px;overflow-x:auto;overflow-y:auto;width:auto;height:auto;">
<style>#cadhissaxn table {
  font-family: system-ui, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji';
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

#cadhissaxn thead, #cadhissaxn tbody, #cadhissaxn tfoot, #cadhissaxn tr, #cadhissaxn td, #cadhissaxn th {
  border-style: none;
}

#cadhissaxn p {
  margin: 0;
  padding: 0;
}

#cadhissaxn .gt_table {
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

#cadhissaxn .gt_caption {
  padding-top: 4px;
  padding-bottom: 4px;
}

#cadhissaxn .gt_title {
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

#cadhissaxn .gt_subtitle {
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

#cadhissaxn .gt_heading {
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

#cadhissaxn .gt_bottom_border {
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#cadhissaxn .gt_col_headings {
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

#cadhissaxn .gt_col_heading {
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

#cadhissaxn .gt_column_spanner_outer {
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

#cadhissaxn .gt_column_spanner_outer:first-child {
  padding-left: 0;
}

#cadhissaxn .gt_column_spanner_outer:last-child {
  padding-right: 0;
}

#cadhissaxn .gt_column_spanner {
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

#cadhissaxn .gt_spanner_row {
  border-bottom-style: hidden;
}

#cadhissaxn .gt_group_heading {
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

#cadhissaxn .gt_empty_group_heading {
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

#cadhissaxn .gt_from_md > :first-child {
  margin-top: 0;
}

#cadhissaxn .gt_from_md > :last-child {
  margin-bottom: 0;
}

#cadhissaxn .gt_row {
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

#cadhissaxn .gt_stub {
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

#cadhissaxn .gt_stub_row_group {
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

#cadhissaxn .gt_row_group_first td {
  border-top-width: 2px;
}

#cadhissaxn .gt_row_group_first th {
  border-top-width: 2px;
}

#cadhissaxn .gt_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#cadhissaxn .gt_first_summary_row {
  border-top-style: solid;
  border-top-color: #D3D3D3;
}

#cadhissaxn .gt_first_summary_row.thick {
  border-top-width: 2px;
}

#cadhissaxn .gt_last_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#cadhissaxn .gt_grand_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#cadhissaxn .gt_first_grand_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-top-style: double;
  border-top-width: 6px;
  border-top-color: #D3D3D3;
}

#cadhissaxn .gt_last_grand_summary_row_top {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: double;
  border-bottom-width: 6px;
  border-bottom-color: #D3D3D3;
}

#cadhissaxn .gt_striped {
  background-color: rgba(128, 128, 128, 0.05);
}

#cadhissaxn .gt_table_body {
  border-top-style: solid;
  border-top-width: 2px;
  border-top-color: #D3D3D3;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#cadhissaxn .gt_footnotes {
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

#cadhissaxn .gt_footnote {
  margin: 0px;
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#cadhissaxn .gt_sourcenotes {
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

#cadhissaxn .gt_sourcenote {
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#cadhissaxn .gt_left {
  text-align: left;
}

#cadhissaxn .gt_center {
  text-align: center;
}

#cadhissaxn .gt_right {
  text-align: right;
  font-variant-numeric: tabular-nums;
}

#cadhissaxn .gt_font_normal {
  font-weight: normal;
}

#cadhissaxn .gt_font_bold {
  font-weight: bold;
}

#cadhissaxn .gt_font_italic {
  font-style: italic;
}

#cadhissaxn .gt_super {
  font-size: 65%;
}

#cadhissaxn .gt_footnote_marks {
  font-size: 75%;
  vertical-align: 0.4em;
  position: initial;
}

#cadhissaxn .gt_asterisk {
  font-size: 100%;
  vertical-align: 0;
}

#cadhissaxn .gt_indent_1 {
  text-indent: 5px;
}

#cadhissaxn .gt_indent_2 {
  text-indent: 10px;
}

#cadhissaxn .gt_indent_3 {
  text-indent: 15px;
}

#cadhissaxn .gt_indent_4 {
  text-indent: 20px;
}

#cadhissaxn .gt_indent_5 {
  text-indent: 25px;
}

#cadhissaxn .katex-display {
  display: inline-flex !important;
  margin-bottom: 0.75em !important;
}

#cadhissaxn div.Reactable > div.rt-table > div.rt-thead > div.rt-tr.rt-tr-group-header > div.rt-th-group:after {
  height: 0px !important;
}

td, th {
  white-space: nowrap;
}
</style>
<table class="gt_table" data-quarto-disable-processing="false" data-quarto-bootstrap="false">
  <thead>
    <tr class="gt_heading">
      <td colspan="7" class="gt_heading gt_title gt_font_normal gt_bottom_border" style="font-weight: bold;">run003 Parameters</td>
    </tr>
    
    <tr class="gt_col_headings">
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="name">Parameter</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="symbol"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="unit"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="estimate">Estimate</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="variability"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="ci_low">95% CI</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="shrinkage">Shrinkage (%)</th>
    </tr>
  </thead>
  <tbody class="gt_table_body">
    <tr class="gt_group_heading_row">
      <th colspan="7" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Structural model parameters">Structural model parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="VEhFVEEx"><span class='gt_from_md'>THETA1</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97MX0k"><span class='gt_from_md'>\(\theta_{1}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="TC9ocg=="><span class='gt_from_md'>L/hr</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">1.33</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[1.11, 1.54]</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="VEhFVEEy"><span class='gt_from_md'>THETA2</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97Mn0k"><span class='gt_from_md'>\(\theta_{2}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="TA=="><span class='gt_from_md'>L</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">40.2</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[34.6, 45.7]</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="VEhFVEEz"><span class='gt_from_md'>THETA3</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97M30k"><span class='gt_from_md'>\(\theta_{3}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="MS9ocg=="><span class='gt_from_md'>1/hr</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">1.21</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[0.997, 1.43]</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="7" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Interindividual variance parameters">Interindividual variance parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="T01FR0EoMSwxKSAoVFZDTCk="><span class='gt_from_md'>OMEGA(1,1) (TVCL)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDEsMSl9JA=="><span class='gt_from_md'>\(\Omega_{(1,1)}\)</span></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.122</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMSUp"><span class='gt_from_md'>(CV = 36.1%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0236, 0.221]</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">13.1</td></tr>
    <tr><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="T01FR0EoMiwyKSAoVFZWKQ=="><span class='gt_from_md'>OMEGA(2,2) (TVV)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDIsMil9JA=="><span class='gt_from_md'>\(\Omega_{(2,2)}\)</span></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.124</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMyUp"><span class='gt_from_md'>(CV = 36.3%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0519, 0.196]</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">4.63</td></tr>
    <tr><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="T01FR0EoMywzKSAoVFZLQSk="><span class='gt_from_md'>OMEGA(3,3) (TVKA)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDMsMyl9JA=="><span class='gt_from_md'>\(\Omega_{(3,3)}\)</span></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.122</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMSUp"><span class='gt_from_md'>(CV = 36.1%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0121, 0.233]</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">24.3</td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="7" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Interindividual covariance parameters">Interindividual covariance parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Interindividual covariance parameters  name" class="gt_row gt_left"><span data-qmd-base64="T01FR0EoMiwxKSAoVFZDTC1UVlYp"><span class='gt_from_md'>OMEGA(2,1) (TVCL-TVV)</span></span></td>
<td headers="Interindividual covariance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDIsMSl9JA=="><span class='gt_from_md'>\(\Omega_{(2,1)}\)</span></span></td>
<td headers="Interindividual covariance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual covariance parameters  estimate" class="gt_row gt_right">0.0745</td>
<td headers="Interindividual covariance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENvcnIgPSAwLjYwNik="><span class='gt_from_md'>(Corr = 0.606)</span></span></td>
<td headers="Interindividual covariance parameters  ci_low" class="gt_row gt_right">[0.0131, 0.136]</td>
<td headers="Interindividual covariance parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="7" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Residual error">Residual error</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Residual error  name" class="gt_row gt_left"><span data-qmd-base64="U0lHTUEoMSwxKQ=="><span class='gt_from_md'>SIGMA(1,1)</span></span></td>
<td headers="Residual error  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxTaWdtYV97KDEsMSl9JA=="><span class='gt_from_md'>\(\Sigma_{(1,1)}\)</span></span></td>
<td headers="Residual error  unit" class="gt_row gt_left"><br /></td>
<td headers="Residual error  estimate" class="gt_row gt_right">0.0375</td>
<td headers="Residual error  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMTkuNCUp"><span class='gt_from_md'>(CV = 19.4%)</span></span></td>
<td headers="Residual error  ci_low" class="gt_row gt_right">[0.0257, 0.0494]</td>
<td headers="Residual error  shrinkage" class="gt_row gt_right">14.4</td></tr>
    <tr><td headers="Residual error  name" class="gt_row gt_left"><span data-qmd-base64="U0lHTUEoMiwyKQ=="><span class='gt_from_md'>SIGMA(2,2)</span></span></td>
<td headers="Residual error  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxTaWdtYV97KDIsMil9JA=="><span class='gt_from_md'>\(\Sigma_{(2,2)}\)</span></span></td>
<td headers="Residual error  unit" class="gt_row gt_left"><br /></td>
<td headers="Residual error  estimate" class="gt_row gt_right">0.00527</td>
<td headers="Residual error  variability" class="gt_row gt_left"><span data-qmd-base64="KFNEID0gMC4wNzI2KQ=="><span class='gt_from_md'>(SD = 0.0726)</span></span></td>
<td headers="Residual error  ci_low" class="gt_row gt_right">[−0.0128, 0.0233]</td>
<td headers="Residual error  shrinkage" class="gt_row gt_right">14.4</td></tr>
  </tbody>
  <tfoot>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> First Order Conditional Estimation with Interaction | Objective function value: -110 | Condition Number: 6.17</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> Abbreviations:</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> CI = confidence intervals; CV = coefficient of variation; SD = standard deviation; Corr = correlation</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> <span data-qmd-base64="OTUlIENJOiAkXG1hdGhybXtFc3RpbWF0ZX0gXHBtIHpfezAuMDI1fSBcY2RvdCBcbWF0aHJte1NFfSQ="><span class='gt_from_md'>95% CI: \(\mathrm{Estimate} \pm z_{0.025} \cdot \mathrm{SE}\)</span></span></td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> <span data-qmd-base64="Q1YlIGZvciBsb2ctbm9ybWFsICRcT21lZ2EkOiAkXHNxcnR7XGV4cChcbWF0aHJte0VzdGltYXRlfSkgLSAxfSBcdGltZXMgMTAwJA=="><span class='gt_from_md'>CV% for log-normal \(\Omega\): \(\sqrt{\exp(\mathrm{Estimate}) - 1} \times 100\)</span></span></td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> <span data-qmd-base64="Q1YlIGZvciBwcm9wb3J0aW9uYWwgJFxTaWdtYSQ6ICRcc3FydHtcbWF0aHJte0VzdGltYXRlfX0gXHRpbWVzIDEwMCQ="><span class='gt_from_md'>CV% for proportional \(\Sigma\): \(\sqrt{\mathrm{Estimate}} \times 100\)</span></span></td>
    </tr>
  </tfoot>
</table>
</div>
```

:::
:::


## Adding descriptions with `@show_description`


::: {.cell}

```{.r .cell-code}
spec@name_source <- "display"
spec@show_description <- TRUE

get_parameters(file.path(model_dir, model_run)) |>
  apply_table_spec(info, spec) |>
  add_summary_info(mod_sum) |>
  make_parameter_table()
```

::: {.cell-output-display}

```{=html}
<div id="xnlqgdylii" style="padding-left:0px;padding-right:0px;padding-top:10px;padding-bottom:10px;overflow-x:auto;overflow-y:auto;width:auto;height:auto;">
<style>#xnlqgdylii table {
  font-family: system-ui, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji';
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

#xnlqgdylii thead, #xnlqgdylii tbody, #xnlqgdylii tfoot, #xnlqgdylii tr, #xnlqgdylii td, #xnlqgdylii th {
  border-style: none;
}

#xnlqgdylii p {
  margin: 0;
  padding: 0;
}

#xnlqgdylii .gt_table {
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

#xnlqgdylii .gt_caption {
  padding-top: 4px;
  padding-bottom: 4px;
}

#xnlqgdylii .gt_title {
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

#xnlqgdylii .gt_subtitle {
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

#xnlqgdylii .gt_heading {
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

#xnlqgdylii .gt_bottom_border {
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#xnlqgdylii .gt_col_headings {
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

#xnlqgdylii .gt_col_heading {
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

#xnlqgdylii .gt_column_spanner_outer {
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

#xnlqgdylii .gt_column_spanner_outer:first-child {
  padding-left: 0;
}

#xnlqgdylii .gt_column_spanner_outer:last-child {
  padding-right: 0;
}

#xnlqgdylii .gt_column_spanner {
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

#xnlqgdylii .gt_spanner_row {
  border-bottom-style: hidden;
}

#xnlqgdylii .gt_group_heading {
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

#xnlqgdylii .gt_empty_group_heading {
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

#xnlqgdylii .gt_from_md > :first-child {
  margin-top: 0;
}

#xnlqgdylii .gt_from_md > :last-child {
  margin-bottom: 0;
}

#xnlqgdylii .gt_row {
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

#xnlqgdylii .gt_stub {
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

#xnlqgdylii .gt_stub_row_group {
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

#xnlqgdylii .gt_row_group_first td {
  border-top-width: 2px;
}

#xnlqgdylii .gt_row_group_first th {
  border-top-width: 2px;
}

#xnlqgdylii .gt_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#xnlqgdylii .gt_first_summary_row {
  border-top-style: solid;
  border-top-color: #D3D3D3;
}

#xnlqgdylii .gt_first_summary_row.thick {
  border-top-width: 2px;
}

#xnlqgdylii .gt_last_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#xnlqgdylii .gt_grand_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#xnlqgdylii .gt_first_grand_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-top-style: double;
  border-top-width: 6px;
  border-top-color: #D3D3D3;
}

#xnlqgdylii .gt_last_grand_summary_row_top {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: double;
  border-bottom-width: 6px;
  border-bottom-color: #D3D3D3;
}

#xnlqgdylii .gt_striped {
  background-color: rgba(128, 128, 128, 0.05);
}

#xnlqgdylii .gt_table_body {
  border-top-style: solid;
  border-top-width: 2px;
  border-top-color: #D3D3D3;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#xnlqgdylii .gt_footnotes {
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

#xnlqgdylii .gt_footnote {
  margin: 0px;
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#xnlqgdylii .gt_sourcenotes {
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

#xnlqgdylii .gt_sourcenote {
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#xnlqgdylii .gt_left {
  text-align: left;
}

#xnlqgdylii .gt_center {
  text-align: center;
}

#xnlqgdylii .gt_right {
  text-align: right;
  font-variant-numeric: tabular-nums;
}

#xnlqgdylii .gt_font_normal {
  font-weight: normal;
}

#xnlqgdylii .gt_font_bold {
  font-weight: bold;
}

#xnlqgdylii .gt_font_italic {
  font-style: italic;
}

#xnlqgdylii .gt_super {
  font-size: 65%;
}

#xnlqgdylii .gt_footnote_marks {
  font-size: 75%;
  vertical-align: 0.4em;
  position: initial;
}

#xnlqgdylii .gt_asterisk {
  font-size: 100%;
  vertical-align: 0;
}

#xnlqgdylii .gt_indent_1 {
  text-indent: 5px;
}

#xnlqgdylii .gt_indent_2 {
  text-indent: 10px;
}

#xnlqgdylii .gt_indent_3 {
  text-indent: 15px;
}

#xnlqgdylii .gt_indent_4 {
  text-indent: 20px;
}

#xnlqgdylii .gt_indent_5 {
  text-indent: 25px;
}

#xnlqgdylii .katex-display {
  display: inline-flex !important;
  margin-bottom: 0.75em !important;
}

#xnlqgdylii div.Reactable > div.rt-table > div.rt-thead > div.rt-tr.rt-tr-group-header > div.rt-th-group:after {
  height: 0px !important;
}

td, th {
  white-space: nowrap;
}
</style>
<table class="gt_table" data-quarto-disable-processing="false" data-quarto-bootstrap="false">
  <thead>
    <tr class="gt_heading">
      <td colspan="8" class="gt_heading gt_title gt_font_normal gt_bottom_border" style="font-weight: bold;">run003 Parameters</td>
    </tr>
    
    <tr class="gt_col_headings">
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="name">Parameter</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="description"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="symbol"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="unit"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="estimate">Estimate</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="variability"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="ci_low">95% CI</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="shrinkage">Shrinkage (%)</th>
    </tr>
  </thead>
  <tbody class="gt_table_body">
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Structural model parameters">Structural model parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="Q0w="><span class='gt_from_md'>CL</span></span></td>
<td headers="Structural model parameters  description" class="gt_row gt_left"><span data-qmd-base64="Q2xlYXJhbmNl"><span class='gt_from_md'>Clearance</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97MX0k"><span class='gt_from_md'>\(\theta_{1}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="TC9ocg=="><span class='gt_from_md'>L/hr</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">1.33</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[1.11, 1.54]</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="VmM="><span class='gt_from_md'>Vc</span></span></td>
<td headers="Structural model parameters  description" class="gt_row gt_left"><span data-qmd-base64="Q2VudHJhbCBWb2x1bWU="><span class='gt_from_md'>Central Volume</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97Mn0k"><span class='gt_from_md'>\(\theta_{2}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="TA=="><span class='gt_from_md'>L</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">40.2</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[34.6, 45.7]</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="S0E="><span class='gt_from_md'>KA</span></span></td>
<td headers="Structural model parameters  description" class="gt_row gt_left"><span data-qmd-base64="QWJzb3JwdGlvbiBSYXRlIENvbnN0YW50"><span class='gt_from_md'>Absorption Rate Constant</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97M30k"><span class='gt_from_md'>\(\theta_{3}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="MS9ocg=="><span class='gt_from_md'>1/hr</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">1.21</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[0.997, 1.43]</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Interindividual variance parameters">Interindividual variance parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="SUlWLUNMIChUVkNMKQ=="><span class='gt_from_md'>IIV-CL (TVCL)</span></span></td>
<td headers="Interindividual variance parameters  description" class="gt_row gt_left"><span data-qmd-base64="SW50ZXJpbmRpdmlkdWFsIHZhcmlhYmlsaXR5IG9uIENM"><span class='gt_from_md'>Interindividual variability on CL</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDEsMSl9JA=="><span class='gt_from_md'>\(\Omega_{(1,1)}\)</span></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.122</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMSUp"><span class='gt_from_md'>(CV = 36.1%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0236, 0.221]</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">13.1</td></tr>
    <tr><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="SUlWLVZjIChUVlYp"><span class='gt_from_md'>IIV-Vc (TVV)</span></span></td>
<td headers="Interindividual variance parameters  description" class="gt_row gt_left"><span data-qmd-base64="SW50ZXJpbmRpdmlkdWFsIHZhcmlhYmlsaXR5IG9uIFZj"><span class='gt_from_md'>Interindividual variability on Vc</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDIsMil9JA=="><span class='gt_from_md'>\(\Omega_{(2,2)}\)</span></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.124</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMyUp"><span class='gt_from_md'>(CV = 36.3%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0519, 0.196]</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">4.63</td></tr>
    <tr><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="SUlWLUtBIChUVktBKQ=="><span class='gt_from_md'>IIV-KA (TVKA)</span></span></td>
<td headers="Interindividual variance parameters  description" class="gt_row gt_left"><span data-qmd-base64="SW50ZXJpbmRpdmlkdWFsIHZhcmlhYmlsaXR5IG9uIEtB"><span class='gt_from_md'>Interindividual variability on KA</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDMsMyl9JA=="><span class='gt_from_md'>\(\Omega_{(3,3)}\)</span></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.122</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMSUp"><span class='gt_from_md'>(CV = 36.1%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0121, 0.233]</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">24.3</td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Interindividual covariance parameters">Interindividual covariance parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Interindividual covariance parameters  name" class="gt_row gt_left"><span data-qmd-base64="SUlWLUNMLVZjIChUVkNMLVRWVik="><span class='gt_from_md'>IIV-CL-Vc (TVCL-TVV)</span></span></td>
<td headers="Interindividual covariance parameters  description" class="gt_row gt_left"><span data-qmd-base64="SW50ZXJpbmRpdmlkdWFsIGNvdmFyaWFuY2UgZm9yIENMLVZj"><span class='gt_from_md'>Interindividual covariance for CL-Vc</span></span></td>
<td headers="Interindividual covariance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDIsMSl9JA=="><span class='gt_from_md'>\(\Omega_{(2,1)}\)</span></span></td>
<td headers="Interindividual covariance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual covariance parameters  estimate" class="gt_row gt_right">0.0745</td>
<td headers="Interindividual covariance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENvcnIgPSAwLjYwNik="><span class='gt_from_md'>(Corr = 0.606)</span></span></td>
<td headers="Interindividual covariance parameters  ci_low" class="gt_row gt_right">[0.0131, 0.136]</td>
<td headers="Interindividual covariance parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Residual error">Residual error</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Residual error  name" class="gt_row gt_left"><span data-qmd-base64="QWRkaXRpdmUgRXJyb3I="><span class='gt_from_md'>Additive Error</span></span></td>
<td headers="Residual error  description" class="gt_row gt_left"><span data-qmd-base64="UHJvcG9ydGlvbmFsIEVycm9y"><span class='gt_from_md'>Proportional Error</span></span></td>
<td headers="Residual error  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxTaWdtYV97KDEsMSl9JA=="><span class='gt_from_md'>\(\Sigma_{(1,1)}\)</span></span></td>
<td headers="Residual error  unit" class="gt_row gt_left"><br /></td>
<td headers="Residual error  estimate" class="gt_row gt_right">0.0375</td>
<td headers="Residual error  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMTkuNCUp"><span class='gt_from_md'>(CV = 19.4%)</span></span></td>
<td headers="Residual error  ci_low" class="gt_row gt_right">[0.0257, 0.0494]</td>
<td headers="Residual error  shrinkage" class="gt_row gt_right">14.4</td></tr>
    <tr><td headers="Residual error  name" class="gt_row gt_left"><span data-qmd-base64="UHJvcG9ydGlvbmFsIEVycm9y"><span class='gt_from_md'>Proportional Error</span></span></td>
<td headers="Residual error  description" class="gt_row gt_left"><span data-qmd-base64="QWRkaXRpdmUgRXJyb3I="><span class='gt_from_md'>Additive Error</span></span></td>
<td headers="Residual error  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxTaWdtYV97KDIsMil9JA=="><span class='gt_from_md'>\(\Sigma_{(2,2)}\)</span></span></td>
<td headers="Residual error  unit" class="gt_row gt_left"><br /></td>
<td headers="Residual error  estimate" class="gt_row gt_right">0.00527</td>
<td headers="Residual error  variability" class="gt_row gt_left"><span data-qmd-base64="KFNEID0gMC4wNzI2KQ=="><span class='gt_from_md'>(SD = 0.0726)</span></span></td>
<td headers="Residual error  ci_low" class="gt_row gt_right">[−0.0128, 0.0233]</td>
<td headers="Residual error  shrinkage" class="gt_row gt_right">14.4</td></tr>
  </tbody>
  <tfoot>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> First Order Conditional Estimation with Interaction | Objective function value: -110 | Condition Number: 6.17</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> Abbreviations:</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> CI = confidence intervals; CV = coefficient of variation; SD = standard deviation; Corr = correlation</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> <span data-qmd-base64="OTUlIENJOiAkXG1hdGhybXtFc3RpbWF0ZX0gXHBtIHpfezAuMDI1fSBcY2RvdCBcbWF0aHJte1NFfSQ="><span class='gt_from_md'>95% CI: \(\mathrm{Estimate} \pm z_{0.025} \cdot \mathrm{SE}\)</span></span></td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> <span data-qmd-base64="Q1YlIGZvciBsb2ctbm9ybWFsICRcT21lZ2EkOiAkXHNxcnR7XGV4cChcbWF0aHJte0VzdGltYXRlfSkgLSAxfSBcdGltZXMgMTAwJA=="><span class='gt_from_md'>CV% for log-normal \(\Omega\): \(\sqrt{\exp(\mathrm{Estimate}) - 1} \times 100\)</span></span></td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> <span data-qmd-base64="Q1YlIGZvciBwcm9wb3J0aW9uYWwgJFxTaWdtYSQ6ICRcc3FydHtcbWF0aHJte0VzdGltYXRlfX0gXHRpbWVzIDEwMCQ="><span class='gt_from_md'>CV% for proportional \(\Sigma\): \(\sqrt{\mathrm{Estimate}} \times 100\)</span></span></td>
    </tr>
  </tfoot>
</table>
</div>
```

:::
:::



## Removing columns with `@drop_columns` property


::: {.cell}

```{.r .cell-code}
spec@show_description <- FALSE
spec@name_source <- "display"
spec@drop_columns <- "unit"

get_parameters(file.path(model_dir, model_run)) |>
  apply_table_spec(info, spec) |>
  add_summary_info(mod_sum) |>
  make_parameter_table()
```

::: {.cell-output-display}

```{=html}
<div id="rgzwinrqib" style="padding-left:0px;padding-right:0px;padding-top:10px;padding-bottom:10px;overflow-x:auto;overflow-y:auto;width:auto;height:auto;">
<style>#rgzwinrqib table {
  font-family: system-ui, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji';
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

#rgzwinrqib thead, #rgzwinrqib tbody, #rgzwinrqib tfoot, #rgzwinrqib tr, #rgzwinrqib td, #rgzwinrqib th {
  border-style: none;
}

#rgzwinrqib p {
  margin: 0;
  padding: 0;
}

#rgzwinrqib .gt_table {
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

#rgzwinrqib .gt_caption {
  padding-top: 4px;
  padding-bottom: 4px;
}

#rgzwinrqib .gt_title {
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

#rgzwinrqib .gt_subtitle {
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

#rgzwinrqib .gt_heading {
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

#rgzwinrqib .gt_bottom_border {
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#rgzwinrqib .gt_col_headings {
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

#rgzwinrqib .gt_col_heading {
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

#rgzwinrqib .gt_column_spanner_outer {
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

#rgzwinrqib .gt_column_spanner_outer:first-child {
  padding-left: 0;
}

#rgzwinrqib .gt_column_spanner_outer:last-child {
  padding-right: 0;
}

#rgzwinrqib .gt_column_spanner {
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

#rgzwinrqib .gt_spanner_row {
  border-bottom-style: hidden;
}

#rgzwinrqib .gt_group_heading {
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

#rgzwinrqib .gt_empty_group_heading {
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

#rgzwinrqib .gt_from_md > :first-child {
  margin-top: 0;
}

#rgzwinrqib .gt_from_md > :last-child {
  margin-bottom: 0;
}

#rgzwinrqib .gt_row {
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

#rgzwinrqib .gt_stub {
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

#rgzwinrqib .gt_stub_row_group {
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

#rgzwinrqib .gt_row_group_first td {
  border-top-width: 2px;
}

#rgzwinrqib .gt_row_group_first th {
  border-top-width: 2px;
}

#rgzwinrqib .gt_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#rgzwinrqib .gt_first_summary_row {
  border-top-style: solid;
  border-top-color: #D3D3D3;
}

#rgzwinrqib .gt_first_summary_row.thick {
  border-top-width: 2px;
}

#rgzwinrqib .gt_last_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#rgzwinrqib .gt_grand_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#rgzwinrqib .gt_first_grand_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-top-style: double;
  border-top-width: 6px;
  border-top-color: #D3D3D3;
}

#rgzwinrqib .gt_last_grand_summary_row_top {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: double;
  border-bottom-width: 6px;
  border-bottom-color: #D3D3D3;
}

#rgzwinrqib .gt_striped {
  background-color: rgba(128, 128, 128, 0.05);
}

#rgzwinrqib .gt_table_body {
  border-top-style: solid;
  border-top-width: 2px;
  border-top-color: #D3D3D3;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#rgzwinrqib .gt_footnotes {
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

#rgzwinrqib .gt_footnote {
  margin: 0px;
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#rgzwinrqib .gt_sourcenotes {
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

#rgzwinrqib .gt_sourcenote {
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#rgzwinrqib .gt_left {
  text-align: left;
}

#rgzwinrqib .gt_center {
  text-align: center;
}

#rgzwinrqib .gt_right {
  text-align: right;
  font-variant-numeric: tabular-nums;
}

#rgzwinrqib .gt_font_normal {
  font-weight: normal;
}

#rgzwinrqib .gt_font_bold {
  font-weight: bold;
}

#rgzwinrqib .gt_font_italic {
  font-style: italic;
}

#rgzwinrqib .gt_super {
  font-size: 65%;
}

#rgzwinrqib .gt_footnote_marks {
  font-size: 75%;
  vertical-align: 0.4em;
  position: initial;
}

#rgzwinrqib .gt_asterisk {
  font-size: 100%;
  vertical-align: 0;
}

#rgzwinrqib .gt_indent_1 {
  text-indent: 5px;
}

#rgzwinrqib .gt_indent_2 {
  text-indent: 10px;
}

#rgzwinrqib .gt_indent_3 {
  text-indent: 15px;
}

#rgzwinrqib .gt_indent_4 {
  text-indent: 20px;
}

#rgzwinrqib .gt_indent_5 {
  text-indent: 25px;
}

#rgzwinrqib .katex-display {
  display: inline-flex !important;
  margin-bottom: 0.75em !important;
}

#rgzwinrqib div.Reactable > div.rt-table > div.rt-thead > div.rt-tr.rt-tr-group-header > div.rt-th-group:after {
  height: 0px !important;
}

td, th {
  white-space: nowrap;
}
</style>
<table class="gt_table" data-quarto-disable-processing="false" data-quarto-bootstrap="false">
  <thead>
    <tr class="gt_heading">
      <td colspan="7" class="gt_heading gt_title gt_font_normal gt_bottom_border" style="font-weight: bold;">run003 Parameters</td>
    </tr>
    
    <tr class="gt_col_headings">
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="name">Parameter</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="symbol"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="estimate">Estimate</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="variability"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="ci_low">95% CI</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="rse">RSE (%)</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="shrinkage">Shrinkage (%)</th>
    </tr>
  </thead>
  <tbody class="gt_table_body">
    <tr class="gt_group_heading_row">
      <th colspan="7" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Structural model parameters">Structural model parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="Q0w="><span class='gt_from_md'>CL</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97MX0k"><span class='gt_from_md'>\(\theta_{1}\)</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">1.33</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[1.11, 1.54]</td>
<td headers="Structural model parameters  rse" class="gt_row gt_right">8.41</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="VmM="><span class='gt_from_md'>Vc</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97Mn0k"><span class='gt_from_md'>\(\theta_{2}\)</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">40.2</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[34.6, 45.7]</td>
<td headers="Structural model parameters  rse" class="gt_row gt_right">7.07</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="S0E="><span class='gt_from_md'>KA</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97M30k"><span class='gt_from_md'>\(\theta_{3}\)</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">1.21</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[0.997, 1.43]</td>
<td headers="Structural model parameters  rse" class="gt_row gt_right">9.06</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="7" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Interindividual variance parameters">Interindividual variance parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="SUlWLUNMIChUVkNMKQ=="><span class='gt_from_md'>IIV-CL (TVCL)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDEsMSl9JA=="><span class='gt_from_md'>\(\Omega_{(1,1)}\)</span></span></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.122</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMSUp"><span class='gt_from_md'>(CV = 36.1%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0236, 0.221]</td>
<td headers="Interindividual variance parameters  rse" class="gt_row gt_right">41.2</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">13.1</td></tr>
    <tr><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="SUlWLVZjIChUVlYp"><span class='gt_from_md'>IIV-Vc (TVV)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDIsMil9JA=="><span class='gt_from_md'>\(\Omega_{(2,2)}\)</span></span></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.124</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMyUp"><span class='gt_from_md'>(CV = 36.3%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0519, 0.196]</td>
<td headers="Interindividual variance parameters  rse" class="gt_row gt_right">29.7</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">4.63</td></tr>
    <tr><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="SUlWLUtBIChUVktBKQ=="><span class='gt_from_md'>IIV-KA (TVKA)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDMsMyl9JA=="><span class='gt_from_md'>\(\Omega_{(3,3)}\)</span></span></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.122</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMSUp"><span class='gt_from_md'>(CV = 36.1%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0121, 0.233]</td>
<td headers="Interindividual variance parameters  rse" class="gt_row gt_right">46.0</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">24.3</td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="7" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Interindividual covariance parameters">Interindividual covariance parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Interindividual covariance parameters  name" class="gt_row gt_left"><span data-qmd-base64="SUlWLUNMLVZjIChUVkNMLVRWVik="><span class='gt_from_md'>IIV-CL-Vc (TVCL-TVV)</span></span></td>
<td headers="Interindividual covariance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDIsMSl9JA=="><span class='gt_from_md'>\(\Omega_{(2,1)}\)</span></span></td>
<td headers="Interindividual covariance parameters  estimate" class="gt_row gt_right">0.0745</td>
<td headers="Interindividual covariance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENvcnIgPSAwLjYwNik="><span class='gt_from_md'>(Corr = 0.606)</span></span></td>
<td headers="Interindividual covariance parameters  ci_low" class="gt_row gt_right">[0.0131, 0.136]</td>
<td headers="Interindividual covariance parameters  rse" class="gt_row gt_right">42.0</td>
<td headers="Interindividual covariance parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="7" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Residual error">Residual error</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Residual error  name" class="gt_row gt_left"><span data-qmd-base64="QWRkaXRpdmUgRXJyb3I="><span class='gt_from_md'>Additive Error</span></span></td>
<td headers="Residual error  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxTaWdtYV97KDEsMSl9JA=="><span class='gt_from_md'>\(\Sigma_{(1,1)}\)</span></span></td>
<td headers="Residual error  estimate" class="gt_row gt_right">0.0375</td>
<td headers="Residual error  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMTkuNCUp"><span class='gt_from_md'>(CV = 19.4%)</span></span></td>
<td headers="Residual error  ci_low" class="gt_row gt_right">[0.0257, 0.0494]</td>
<td headers="Residual error  rse" class="gt_row gt_right">16.1</td>
<td headers="Residual error  shrinkage" class="gt_row gt_right">14.4</td></tr>
    <tr><td headers="Residual error  name" class="gt_row gt_left"><span data-qmd-base64="UHJvcG9ydGlvbmFsIEVycm9y"><span class='gt_from_md'>Proportional Error</span></span></td>
<td headers="Residual error  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxTaWdtYV97KDIsMil9JA=="><span class='gt_from_md'>\(\Sigma_{(2,2)}\)</span></span></td>
<td headers="Residual error  estimate" class="gt_row gt_right">0.00527</td>
<td headers="Residual error  variability" class="gt_row gt_left"><span data-qmd-base64="KFNEID0gMC4wNzI2KQ=="><span class='gt_from_md'>(SD = 0.0726)</span></span></td>
<td headers="Residual error  ci_low" class="gt_row gt_right">[−0.0128, 0.0233]</td>
<td headers="Residual error  rse" class="gt_row gt_right">175</td>
<td headers="Residual error  shrinkage" class="gt_row gt_right">14.4</td></tr>
  </tbody>
  <tfoot>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> First Order Conditional Estimation with Interaction | Objective function value: -110 | Condition Number: 6.17</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> Abbreviations:</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> CI = confidence intervals; RSE = relative standard error; CV = coefficient of variation; SD = standard deviation; Corr = correlation</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> <span data-qmd-base64="OTUlIENJOiAkXG1hdGhybXtFc3RpbWF0ZX0gXHBtIHpfezAuMDI1fSBcY2RvdCBcbWF0aHJte1NFfSQ="><span class='gt_from_md'>95% CI: \(\mathrm{Estimate} \pm z_{0.025} \cdot \mathrm{SE}\)</span></span></td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> <span data-qmd-base64="Q1YlIGZvciBsb2ctbm9ybWFsICRcT21lZ2EkOiAkXHNxcnR7XGV4cChcbWF0aHJte0VzdGltYXRlfSkgLSAxfSBcdGltZXMgMTAwJA=="><span class='gt_from_md'>CV% for log-normal \(\Omega\): \(\sqrt{\exp(\mathrm{Estimate}) - 1} \times 100\)</span></span></td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> <span data-qmd-base64="Q1YlIGZvciBwcm9wb3J0aW9uYWwgJFxTaWdtYSQ6ICRcc3FydHtcbWF0aHJte0VzdGltYXRlfX0gXHRpbWVzIDEwMCQ="><span class='gt_from_md'>CV% for proportional \(\Sigma\): \(\sqrt{\mathrm{Estimate}} \times 100\)</span></span></td>
    </tr>
  </tfoot>
</table>
</div>
```

:::
:::

::: {.cell}

```{.r .cell-code}
spec@drop_columns <- c("unit", "shrinkage")

get_parameters(file.path(model_dir, model_run)) |>
  apply_table_spec(info, spec) |>
  add_summary_info(mod_sum) |>
  make_parameter_table()
```

::: {.cell-output-display}

```{=html}
<div id="jdckbhvfxe" style="padding-left:0px;padding-right:0px;padding-top:10px;padding-bottom:10px;overflow-x:auto;overflow-y:auto;width:auto;height:auto;">
<style>#jdckbhvfxe table {
  font-family: system-ui, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji';
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

#jdckbhvfxe thead, #jdckbhvfxe tbody, #jdckbhvfxe tfoot, #jdckbhvfxe tr, #jdckbhvfxe td, #jdckbhvfxe th {
  border-style: none;
}

#jdckbhvfxe p {
  margin: 0;
  padding: 0;
}

#jdckbhvfxe .gt_table {
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

#jdckbhvfxe .gt_caption {
  padding-top: 4px;
  padding-bottom: 4px;
}

#jdckbhvfxe .gt_title {
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

#jdckbhvfxe .gt_subtitle {
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

#jdckbhvfxe .gt_heading {
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

#jdckbhvfxe .gt_bottom_border {
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#jdckbhvfxe .gt_col_headings {
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

#jdckbhvfxe .gt_col_heading {
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

#jdckbhvfxe .gt_column_spanner_outer {
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

#jdckbhvfxe .gt_column_spanner_outer:first-child {
  padding-left: 0;
}

#jdckbhvfxe .gt_column_spanner_outer:last-child {
  padding-right: 0;
}

#jdckbhvfxe .gt_column_spanner {
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

#jdckbhvfxe .gt_spanner_row {
  border-bottom-style: hidden;
}

#jdckbhvfxe .gt_group_heading {
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

#jdckbhvfxe .gt_empty_group_heading {
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

#jdckbhvfxe .gt_from_md > :first-child {
  margin-top: 0;
}

#jdckbhvfxe .gt_from_md > :last-child {
  margin-bottom: 0;
}

#jdckbhvfxe .gt_row {
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

#jdckbhvfxe .gt_stub {
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

#jdckbhvfxe .gt_stub_row_group {
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

#jdckbhvfxe .gt_row_group_first td {
  border-top-width: 2px;
}

#jdckbhvfxe .gt_row_group_first th {
  border-top-width: 2px;
}

#jdckbhvfxe .gt_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#jdckbhvfxe .gt_first_summary_row {
  border-top-style: solid;
  border-top-color: #D3D3D3;
}

#jdckbhvfxe .gt_first_summary_row.thick {
  border-top-width: 2px;
}

#jdckbhvfxe .gt_last_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#jdckbhvfxe .gt_grand_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#jdckbhvfxe .gt_first_grand_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-top-style: double;
  border-top-width: 6px;
  border-top-color: #D3D3D3;
}

#jdckbhvfxe .gt_last_grand_summary_row_top {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: double;
  border-bottom-width: 6px;
  border-bottom-color: #D3D3D3;
}

#jdckbhvfxe .gt_striped {
  background-color: rgba(128, 128, 128, 0.05);
}

#jdckbhvfxe .gt_table_body {
  border-top-style: solid;
  border-top-width: 2px;
  border-top-color: #D3D3D3;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#jdckbhvfxe .gt_footnotes {
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

#jdckbhvfxe .gt_footnote {
  margin: 0px;
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#jdckbhvfxe .gt_sourcenotes {
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

#jdckbhvfxe .gt_sourcenote {
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#jdckbhvfxe .gt_left {
  text-align: left;
}

#jdckbhvfxe .gt_center {
  text-align: center;
}

#jdckbhvfxe .gt_right {
  text-align: right;
  font-variant-numeric: tabular-nums;
}

#jdckbhvfxe .gt_font_normal {
  font-weight: normal;
}

#jdckbhvfxe .gt_font_bold {
  font-weight: bold;
}

#jdckbhvfxe .gt_font_italic {
  font-style: italic;
}

#jdckbhvfxe .gt_super {
  font-size: 65%;
}

#jdckbhvfxe .gt_footnote_marks {
  font-size: 75%;
  vertical-align: 0.4em;
  position: initial;
}

#jdckbhvfxe .gt_asterisk {
  font-size: 100%;
  vertical-align: 0;
}

#jdckbhvfxe .gt_indent_1 {
  text-indent: 5px;
}

#jdckbhvfxe .gt_indent_2 {
  text-indent: 10px;
}

#jdckbhvfxe .gt_indent_3 {
  text-indent: 15px;
}

#jdckbhvfxe .gt_indent_4 {
  text-indent: 20px;
}

#jdckbhvfxe .gt_indent_5 {
  text-indent: 25px;
}

#jdckbhvfxe .katex-display {
  display: inline-flex !important;
  margin-bottom: 0.75em !important;
}

#jdckbhvfxe div.Reactable > div.rt-table > div.rt-thead > div.rt-tr.rt-tr-group-header > div.rt-th-group:after {
  height: 0px !important;
}

td, th {
  white-space: nowrap;
}
</style>
<table class="gt_table" data-quarto-disable-processing="false" data-quarto-bootstrap="false">
  <thead>
    <tr class="gt_heading">
      <td colspan="6" class="gt_heading gt_title gt_font_normal gt_bottom_border" style="font-weight: bold;">run003 Parameters</td>
    </tr>
    
    <tr class="gt_col_headings">
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="name">Parameter</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="symbol"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="estimate">Estimate</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="variability"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="ci_low">95% CI</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="rse">RSE (%)</th>
    </tr>
  </thead>
  <tbody class="gt_table_body">
    <tr class="gt_group_heading_row">
      <th colspan="6" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Structural model parameters">Structural model parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="Q0w="><span class='gt_from_md'>CL</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97MX0k"><span class='gt_from_md'>\(\theta_{1}\)</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">1.33</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[1.11, 1.54]</td>
<td headers="Structural model parameters  rse" class="gt_row gt_right">8.41</td></tr>
    <tr><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="VmM="><span class='gt_from_md'>Vc</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97Mn0k"><span class='gt_from_md'>\(\theta_{2}\)</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">40.2</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[34.6, 45.7]</td>
<td headers="Structural model parameters  rse" class="gt_row gt_right">7.07</td></tr>
    <tr><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="S0E="><span class='gt_from_md'>KA</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97M30k"><span class='gt_from_md'>\(\theta_{3}\)</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">1.21</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[0.997, 1.43]</td>
<td headers="Structural model parameters  rse" class="gt_row gt_right">9.06</td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="6" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Interindividual variance parameters">Interindividual variance parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="SUlWLUNMIChUVkNMKQ=="><span class='gt_from_md'>IIV-CL (TVCL)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDEsMSl9JA=="><span class='gt_from_md'>\(\Omega_{(1,1)}\)</span></span></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.122</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMSUp"><span class='gt_from_md'>(CV = 36.1%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0236, 0.221]</td>
<td headers="Interindividual variance parameters  rse" class="gt_row gt_right">41.2</td></tr>
    <tr><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="SUlWLVZjIChUVlYp"><span class='gt_from_md'>IIV-Vc (TVV)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDIsMil9JA=="><span class='gt_from_md'>\(\Omega_{(2,2)}\)</span></span></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.124</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMyUp"><span class='gt_from_md'>(CV = 36.3%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0519, 0.196]</td>
<td headers="Interindividual variance parameters  rse" class="gt_row gt_right">29.7</td></tr>
    <tr><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="SUlWLUtBIChUVktBKQ=="><span class='gt_from_md'>IIV-KA (TVKA)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDMsMyl9JA=="><span class='gt_from_md'>\(\Omega_{(3,3)}\)</span></span></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.122</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMSUp"><span class='gt_from_md'>(CV = 36.1%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0121, 0.233]</td>
<td headers="Interindividual variance parameters  rse" class="gt_row gt_right">46.0</td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="6" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Interindividual covariance parameters">Interindividual covariance parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Interindividual covariance parameters  name" class="gt_row gt_left"><span data-qmd-base64="SUlWLUNMLVZjIChUVkNMLVRWVik="><span class='gt_from_md'>IIV-CL-Vc (TVCL-TVV)</span></span></td>
<td headers="Interindividual covariance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDIsMSl9JA=="><span class='gt_from_md'>\(\Omega_{(2,1)}\)</span></span></td>
<td headers="Interindividual covariance parameters  estimate" class="gt_row gt_right">0.0745</td>
<td headers="Interindividual covariance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENvcnIgPSAwLjYwNik="><span class='gt_from_md'>(Corr = 0.606)</span></span></td>
<td headers="Interindividual covariance parameters  ci_low" class="gt_row gt_right">[0.0131, 0.136]</td>
<td headers="Interindividual covariance parameters  rse" class="gt_row gt_right">42.0</td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="6" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Residual error">Residual error</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Residual error  name" class="gt_row gt_left"><span data-qmd-base64="QWRkaXRpdmUgRXJyb3I="><span class='gt_from_md'>Additive Error</span></span></td>
<td headers="Residual error  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxTaWdtYV97KDEsMSl9JA=="><span class='gt_from_md'>\(\Sigma_{(1,1)}\)</span></span></td>
<td headers="Residual error  estimate" class="gt_row gt_right">0.0375</td>
<td headers="Residual error  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMTkuNCUp"><span class='gt_from_md'>(CV = 19.4%)</span></span></td>
<td headers="Residual error  ci_low" class="gt_row gt_right">[0.0257, 0.0494]</td>
<td headers="Residual error  rse" class="gt_row gt_right">16.1</td></tr>
    <tr><td headers="Residual error  name" class="gt_row gt_left"><span data-qmd-base64="UHJvcG9ydGlvbmFsIEVycm9y"><span class='gt_from_md'>Proportional Error</span></span></td>
<td headers="Residual error  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxTaWdtYV97KDIsMil9JA=="><span class='gt_from_md'>\(\Sigma_{(2,2)}\)</span></span></td>
<td headers="Residual error  estimate" class="gt_row gt_right">0.00527</td>
<td headers="Residual error  variability" class="gt_row gt_left"><span data-qmd-base64="KFNEID0gMC4wNzI2KQ=="><span class='gt_from_md'>(SD = 0.0726)</span></span></td>
<td headers="Residual error  ci_low" class="gt_row gt_right">[−0.0128, 0.0233]</td>
<td headers="Residual error  rse" class="gt_row gt_right">175</td></tr>
  </tbody>
  <tfoot>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="6"> First Order Conditional Estimation with Interaction | Objective function value: -110 | Condition Number: 6.17</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="6"> Abbreviations:</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="6"> CI = confidence intervals; RSE = relative standard error; CV = coefficient of variation; SD = standard deviation; Corr = correlation</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="6"> <span data-qmd-base64="OTUlIENJOiAkXG1hdGhybXtFc3RpbWF0ZX0gXHBtIHpfezAuMDI1fSBcY2RvdCBcbWF0aHJte1NFfSQ="><span class='gt_from_md'>95% CI: \(\mathrm{Estimate} \pm z_{0.025} \cdot \mathrm{SE}\)</span></span></td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="6"> <span data-qmd-base64="Q1YlIGZvciBsb2ctbm9ybWFsICRcT21lZ2EkOiAkXHNxcnR7XGV4cChcbWF0aHJte0VzdGltYXRlfSkgLSAxfSBcdGltZXMgMTAwJA=="><span class='gt_from_md'>CV% for log-normal \(\Omega\): \(\sqrt{\exp(\mathrm{Estimate}) - 1} \times 100\)</span></span></td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="6"> <span data-qmd-base64="Q1YlIGZvciBwcm9wb3J0aW9uYWwgJFxTaWdtYSQ6ICRcc3FydHtcbWF0aHJte0VzdGltYXRlfX0gXHRpbWVzIDEwMCQ="><span class='gt_from_md'>CV% for proportional \(\Sigma\): \(\sqrt{\mathrm{Estimate}} \times 100\)</span></span></td>
    </tr>
  </tfoot>
</table>
</div>
```

:::
:::



# Structural parameters only



::: {.cell}

```{.r .cell-code}
sp_spec <- TableSpec(
  sections = section_rules(
    kind == "THETA" ~ "Structural model parameters",
    TRUE ~ "Other"
  ),
  row_filter = filter_rules(
    kind == "THETA"
  ),
  drop_columns = "shrinkage"
)

get_parameters(file.path(model_dir, model_run)) |>
  apply_table_spec(info, sp_spec) |>
  make_parameter_table()
```

::: {.cell-output-display}

```{=html}
<div id="nzmgaxdwmm" style="padding-left:0px;padding-right:0px;padding-top:10px;padding-bottom:10px;overflow-x:auto;overflow-y:auto;width:auto;height:auto;">
<style>#nzmgaxdwmm table {
  font-family: system-ui, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji';
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

#nzmgaxdwmm thead, #nzmgaxdwmm tbody, #nzmgaxdwmm tfoot, #nzmgaxdwmm tr, #nzmgaxdwmm td, #nzmgaxdwmm th {
  border-style: none;
}

#nzmgaxdwmm p {
  margin: 0;
  padding: 0;
}

#nzmgaxdwmm .gt_table {
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

#nzmgaxdwmm .gt_caption {
  padding-top: 4px;
  padding-bottom: 4px;
}

#nzmgaxdwmm .gt_title {
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

#nzmgaxdwmm .gt_subtitle {
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

#nzmgaxdwmm .gt_heading {
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

#nzmgaxdwmm .gt_bottom_border {
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#nzmgaxdwmm .gt_col_headings {
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

#nzmgaxdwmm .gt_col_heading {
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

#nzmgaxdwmm .gt_column_spanner_outer {
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

#nzmgaxdwmm .gt_column_spanner_outer:first-child {
  padding-left: 0;
}

#nzmgaxdwmm .gt_column_spanner_outer:last-child {
  padding-right: 0;
}

#nzmgaxdwmm .gt_column_spanner {
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

#nzmgaxdwmm .gt_spanner_row {
  border-bottom-style: hidden;
}

#nzmgaxdwmm .gt_group_heading {
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

#nzmgaxdwmm .gt_empty_group_heading {
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

#nzmgaxdwmm .gt_from_md > :first-child {
  margin-top: 0;
}

#nzmgaxdwmm .gt_from_md > :last-child {
  margin-bottom: 0;
}

#nzmgaxdwmm .gt_row {
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

#nzmgaxdwmm .gt_stub {
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

#nzmgaxdwmm .gt_stub_row_group {
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

#nzmgaxdwmm .gt_row_group_first td {
  border-top-width: 2px;
}

#nzmgaxdwmm .gt_row_group_first th {
  border-top-width: 2px;
}

#nzmgaxdwmm .gt_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#nzmgaxdwmm .gt_first_summary_row {
  border-top-style: solid;
  border-top-color: #D3D3D3;
}

#nzmgaxdwmm .gt_first_summary_row.thick {
  border-top-width: 2px;
}

#nzmgaxdwmm .gt_last_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#nzmgaxdwmm .gt_grand_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#nzmgaxdwmm .gt_first_grand_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-top-style: double;
  border-top-width: 6px;
  border-top-color: #D3D3D3;
}

#nzmgaxdwmm .gt_last_grand_summary_row_top {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: double;
  border-bottom-width: 6px;
  border-bottom-color: #D3D3D3;
}

#nzmgaxdwmm .gt_striped {
  background-color: rgba(128, 128, 128, 0.05);
}

#nzmgaxdwmm .gt_table_body {
  border-top-style: solid;
  border-top-width: 2px;
  border-top-color: #D3D3D3;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#nzmgaxdwmm .gt_footnotes {
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

#nzmgaxdwmm .gt_footnote {
  margin: 0px;
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#nzmgaxdwmm .gt_sourcenotes {
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

#nzmgaxdwmm .gt_sourcenote {
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#nzmgaxdwmm .gt_left {
  text-align: left;
}

#nzmgaxdwmm .gt_center {
  text-align: center;
}

#nzmgaxdwmm .gt_right {
  text-align: right;
  font-variant-numeric: tabular-nums;
}

#nzmgaxdwmm .gt_font_normal {
  font-weight: normal;
}

#nzmgaxdwmm .gt_font_bold {
  font-weight: bold;
}

#nzmgaxdwmm .gt_font_italic {
  font-style: italic;
}

#nzmgaxdwmm .gt_super {
  font-size: 65%;
}

#nzmgaxdwmm .gt_footnote_marks {
  font-size: 75%;
  vertical-align: 0.4em;
  position: initial;
}

#nzmgaxdwmm .gt_asterisk {
  font-size: 100%;
  vertical-align: 0;
}

#nzmgaxdwmm .gt_indent_1 {
  text-indent: 5px;
}

#nzmgaxdwmm .gt_indent_2 {
  text-indent: 10px;
}

#nzmgaxdwmm .gt_indent_3 {
  text-indent: 15px;
}

#nzmgaxdwmm .gt_indent_4 {
  text-indent: 20px;
}

#nzmgaxdwmm .gt_indent_5 {
  text-indent: 25px;
}

#nzmgaxdwmm .katex-display {
  display: inline-flex !important;
  margin-bottom: 0.75em !important;
}

#nzmgaxdwmm div.Reactable > div.rt-table > div.rt-thead > div.rt-tr.rt-tr-group-header > div.rt-th-group:after {
  height: 0px !important;
}

td, th {
  white-space: nowrap;
}
</style>
<table class="gt_table" data-quarto-disable-processing="false" data-quarto-bootstrap="false">
  <thead>
    <tr class="gt_heading">
      <td colspan="6" class="gt_heading gt_title gt_font_normal gt_bottom_border" style="font-weight: bold;">Model Parameters</td>
    </tr>
    
    <tr class="gt_col_headings">
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="name">Parameter</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="symbol"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="unit"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="estimate">Estimate</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="ci_low">95% CI</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="rse">RSE (%)</th>
    </tr>
  </thead>
  <tbody class="gt_table_body">
    <tr class="gt_group_heading_row">
      <th colspan="6" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Structural model parameters">Structural model parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="VFZDTA=="><span class='gt_from_md'>TVCL</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97MX0k"><span class='gt_from_md'>\(\theta_{1}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="TC9ocg=="><span class='gt_from_md'>L/hr</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">1.33</td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[1.11, 1.54]</td>
<td headers="Structural model parameters  rse" class="gt_row gt_right">8.41</td></tr>
    <tr><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="VFZW"><span class='gt_from_md'>TVV</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97Mn0k"><span class='gt_from_md'>\(\theta_{2}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="TA=="><span class='gt_from_md'>L</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">40.2</td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[34.6, 45.7]</td>
<td headers="Structural model parameters  rse" class="gt_row gt_right">7.07</td></tr>
    <tr><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="VFZLQQ=="><span class='gt_from_md'>TVKA</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97M30k"><span class='gt_from_md'>\(\theta_{3}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="MS9ocg=="><span class='gt_from_md'>1/hr</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">1.21</td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[0.997, 1.43]</td>
<td headers="Structural model parameters  rse" class="gt_row gt_right">9.06</td></tr>
  </tbody>
  <tfoot>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="6"> Abbreviations:</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="6"> CI = confidence intervals; RSE = relative standard error</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="6"> <span data-qmd-base64="OTUlIENJOiAkXG1hdGhybXtFc3RpbWF0ZX0gXHBtIHpfezAuMDI1fSBcY2RvdCBcbWF0aHJte1NFfSQ="><span class='gt_from_md'>95% CI: \(\mathrm{Estimate} \pm z_{0.025} \cdot \mathrm{SE}\)</span></span></td>
    </tr>
  </tfoot>
</table>
</div>
```

:::
:::



# Random Effect Parameters only


::: {.cell}

```{.r .cell-code}
re_spec <- TableSpec(
  sections = section_rules(
    kind == "OMEGA" ~ "Random Effect Parameters",
    kind == "SIGMA" ~ "Residual Error",
    TRUE ~ "Other"
  ),
  row_filter = filter_rules(
    kind != "THETA"
  ),
  drop_columns = "unit",
)

get_parameters(file.path(model_dir, model_run)) |>
  apply_table_spec(info, re_spec) |>
  make_parameter_table()
```

::: {.cell-output-display}

```{=html}
<div id="ozkjkwmxcn" style="padding-left:0px;padding-right:0px;padding-top:10px;padding-bottom:10px;overflow-x:auto;overflow-y:auto;width:auto;height:auto;">
<style>#ozkjkwmxcn table {
  font-family: system-ui, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji';
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

#ozkjkwmxcn thead, #ozkjkwmxcn tbody, #ozkjkwmxcn tfoot, #ozkjkwmxcn tr, #ozkjkwmxcn td, #ozkjkwmxcn th {
  border-style: none;
}

#ozkjkwmxcn p {
  margin: 0;
  padding: 0;
}

#ozkjkwmxcn .gt_table {
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

#ozkjkwmxcn .gt_caption {
  padding-top: 4px;
  padding-bottom: 4px;
}

#ozkjkwmxcn .gt_title {
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

#ozkjkwmxcn .gt_subtitle {
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

#ozkjkwmxcn .gt_heading {
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

#ozkjkwmxcn .gt_bottom_border {
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#ozkjkwmxcn .gt_col_headings {
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

#ozkjkwmxcn .gt_col_heading {
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

#ozkjkwmxcn .gt_column_spanner_outer {
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

#ozkjkwmxcn .gt_column_spanner_outer:first-child {
  padding-left: 0;
}

#ozkjkwmxcn .gt_column_spanner_outer:last-child {
  padding-right: 0;
}

#ozkjkwmxcn .gt_column_spanner {
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

#ozkjkwmxcn .gt_spanner_row {
  border-bottom-style: hidden;
}

#ozkjkwmxcn .gt_group_heading {
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

#ozkjkwmxcn .gt_empty_group_heading {
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

#ozkjkwmxcn .gt_from_md > :first-child {
  margin-top: 0;
}

#ozkjkwmxcn .gt_from_md > :last-child {
  margin-bottom: 0;
}

#ozkjkwmxcn .gt_row {
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

#ozkjkwmxcn .gt_stub {
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

#ozkjkwmxcn .gt_stub_row_group {
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

#ozkjkwmxcn .gt_row_group_first td {
  border-top-width: 2px;
}

#ozkjkwmxcn .gt_row_group_first th {
  border-top-width: 2px;
}

#ozkjkwmxcn .gt_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#ozkjkwmxcn .gt_first_summary_row {
  border-top-style: solid;
  border-top-color: #D3D3D3;
}

#ozkjkwmxcn .gt_first_summary_row.thick {
  border-top-width: 2px;
}

#ozkjkwmxcn .gt_last_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#ozkjkwmxcn .gt_grand_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#ozkjkwmxcn .gt_first_grand_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-top-style: double;
  border-top-width: 6px;
  border-top-color: #D3D3D3;
}

#ozkjkwmxcn .gt_last_grand_summary_row_top {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: double;
  border-bottom-width: 6px;
  border-bottom-color: #D3D3D3;
}

#ozkjkwmxcn .gt_striped {
  background-color: rgba(128, 128, 128, 0.05);
}

#ozkjkwmxcn .gt_table_body {
  border-top-style: solid;
  border-top-width: 2px;
  border-top-color: #D3D3D3;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#ozkjkwmxcn .gt_footnotes {
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

#ozkjkwmxcn .gt_footnote {
  margin: 0px;
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#ozkjkwmxcn .gt_sourcenotes {
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

#ozkjkwmxcn .gt_sourcenote {
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#ozkjkwmxcn .gt_left {
  text-align: left;
}

#ozkjkwmxcn .gt_center {
  text-align: center;
}

#ozkjkwmxcn .gt_right {
  text-align: right;
  font-variant-numeric: tabular-nums;
}

#ozkjkwmxcn .gt_font_normal {
  font-weight: normal;
}

#ozkjkwmxcn .gt_font_bold {
  font-weight: bold;
}

#ozkjkwmxcn .gt_font_italic {
  font-style: italic;
}

#ozkjkwmxcn .gt_super {
  font-size: 65%;
}

#ozkjkwmxcn .gt_footnote_marks {
  font-size: 75%;
  vertical-align: 0.4em;
  position: initial;
}

#ozkjkwmxcn .gt_asterisk {
  font-size: 100%;
  vertical-align: 0;
}

#ozkjkwmxcn .gt_indent_1 {
  text-indent: 5px;
}

#ozkjkwmxcn .gt_indent_2 {
  text-indent: 10px;
}

#ozkjkwmxcn .gt_indent_3 {
  text-indent: 15px;
}

#ozkjkwmxcn .gt_indent_4 {
  text-indent: 20px;
}

#ozkjkwmxcn .gt_indent_5 {
  text-indent: 25px;
}

#ozkjkwmxcn .katex-display {
  display: inline-flex !important;
  margin-bottom: 0.75em !important;
}

#ozkjkwmxcn div.Reactable > div.rt-table > div.rt-thead > div.rt-tr.rt-tr-group-header > div.rt-th-group:after {
  height: 0px !important;
}

td, th {
  white-space: nowrap;
}
</style>
<table class="gt_table" data-quarto-disable-processing="false" data-quarto-bootstrap="false">
  <thead>
    <tr class="gt_heading">
      <td colspan="7" class="gt_heading gt_title gt_font_normal gt_bottom_border" style="font-weight: bold;">Model Parameters</td>
    </tr>
    
    <tr class="gt_col_headings">
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="name">Parameter</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="symbol"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="estimate">Estimate</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="variability"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="ci_low">95% CI</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="rse">RSE (%)</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="shrinkage">Shrinkage (%)</th>
    </tr>
  </thead>
  <tbody class="gt_table_body">
    <tr class="gt_group_heading_row">
      <th colspan="7" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Random Effect Parameters">Random Effect Parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Random Effect Parameters  name" class="gt_row gt_left"><span data-qmd-base64="T00xIChUVkNMKQ=="><span class='gt_from_md'>OM1 (TVCL)</span></span></td>
<td headers="Random Effect Parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxleHAoXE9tZWdhX3soMSwxKX0pJA=="><span class='gt_from_md'>\(\exp(\Omega_{(1,1)})\)</span></span></td>
<td headers="Random Effect Parameters  estimate" class="gt_row gt_right">1.13</td>
<td headers="Random Effect Parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMSUp"><span class='gt_from_md'>(CV = 36.1%)</span></span></td>
<td headers="Random Effect Parameters  ci_low" class="gt_row gt_right">[1.02, 1.25]</td>
<td headers="Random Effect Parameters  rse" class="gt_row gt_right">41.2</td>
<td headers="Random Effect Parameters  shrinkage" class="gt_row gt_right">13.1</td></tr>
    <tr><td headers="Random Effect Parameters  name" class="gt_row gt_left"><span data-qmd-base64="T00xLDIgKFRWQ0wtVFZWKQ=="><span class='gt_from_md'>OM1,2 (TVCL-TVV)</span></span></td>
<td headers="Random Effect Parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxleHAoXE9tZWdhX3soMiwxKX0pJA=="><span class='gt_from_md'>\(\exp(\Omega_{(2,1)})\)</span></span></td>
<td headers="Random Effect Parameters  estimate" class="gt_row gt_right">1.08</td>
<td headers="Random Effect Parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENvcnIgPSAwLjYwNik="><span class='gt_from_md'>(Corr = 0.606)</span></span></td>
<td headers="Random Effect Parameters  ci_low" class="gt_row gt_right">[1.01, 1.15]</td>
<td headers="Random Effect Parameters  rse" class="gt_row gt_right">42.0</td>
<td headers="Random Effect Parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr><td headers="Random Effect Parameters  name" class="gt_row gt_left"><span data-qmd-base64="T00yIChUVlYp"><span class='gt_from_md'>OM2 (TVV)</span></span></td>
<td headers="Random Effect Parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxleHAoXE9tZWdhX3soMiwyKX0pJA=="><span class='gt_from_md'>\(\exp(\Omega_{(2,2)})\)</span></span></td>
<td headers="Random Effect Parameters  estimate" class="gt_row gt_right">1.13</td>
<td headers="Random Effect Parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMyUp"><span class='gt_from_md'>(CV = 36.3%)</span></span></td>
<td headers="Random Effect Parameters  ci_low" class="gt_row gt_right">[1.05, 1.22]</td>
<td headers="Random Effect Parameters  rse" class="gt_row gt_right">29.7</td>
<td headers="Random Effect Parameters  shrinkage" class="gt_row gt_right">4.63</td></tr>
    <tr><td headers="Random Effect Parameters  name" class="gt_row gt_left"><span data-qmd-base64="T00zIChUVktBKQ=="><span class='gt_from_md'>OM3 (TVKA)</span></span></td>
<td headers="Random Effect Parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxleHAoXE9tZWdhX3soMywzKX0pJA=="><span class='gt_from_md'>\(\exp(\Omega_{(3,3)})\)</span></span></td>
<td headers="Random Effect Parameters  estimate" class="gt_row gt_right">1.13</td>
<td headers="Random Effect Parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMSUp"><span class='gt_from_md'>(CV = 36.1%)</span></span></td>
<td headers="Random Effect Parameters  ci_low" class="gt_row gt_right">[1.01, 1.26]</td>
<td headers="Random Effect Parameters  rse" class="gt_row gt_right">46.0</td>
<td headers="Random Effect Parameters  shrinkage" class="gt_row gt_right">24.3</td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="7" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Residual Error">Residual Error</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Residual Error  name" class="gt_row gt_left"><span data-qmd-base64="U0lHMQ=="><span class='gt_from_md'>SIG1</span></span></td>
<td headers="Residual Error  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxTaWdtYV97KDEsMSl9JA=="><span class='gt_from_md'>\(\Sigma_{(1,1)}\)</span></span></td>
<td headers="Residual Error  estimate" class="gt_row gt_right">0.0375</td>
<td headers="Residual Error  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMTkuNCUp"><span class='gt_from_md'>(CV = 19.4%)</span></span></td>
<td headers="Residual Error  ci_low" class="gt_row gt_right">[0.0257, 0.0494]</td>
<td headers="Residual Error  rse" class="gt_row gt_right">16.1</td>
<td headers="Residual Error  shrinkage" class="gt_row gt_right">14.4</td></tr>
    <tr><td headers="Residual Error  name" class="gt_row gt_left"><span data-qmd-base64="U0lHMg=="><span class='gt_from_md'>SIG2</span></span></td>
<td headers="Residual Error  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxTaWdtYV97KDIsMil9JA=="><span class='gt_from_md'>\(\Sigma_{(2,2)}\)</span></span></td>
<td headers="Residual Error  estimate" class="gt_row gt_right">0.00527</td>
<td headers="Residual Error  variability" class="gt_row gt_left"><span data-qmd-base64="KFNEID0gMC4wNzI2KQ=="><span class='gt_from_md'>(SD = 0.0726)</span></span></td>
<td headers="Residual Error  ci_low" class="gt_row gt_right">[−0.0128, 0.0233]</td>
<td headers="Residual Error  rse" class="gt_row gt_right">175</td>
<td headers="Residual Error  shrinkage" class="gt_row gt_right">14.4</td></tr>
  </tbody>
  <tfoot>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> Abbreviations:</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> CI = confidence intervals; RSE = relative standard error; CV = coefficient of variation; SD = standard deviation; Corr = correlation</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> <span data-qmd-base64="OTUlIENJOiAkXG1hdGhybXtFc3RpbWF0ZX0gXHBtIHpfezAuMDI1fSBcY2RvdCBcbWF0aHJte1NFfSQ="><span class='gt_from_md'>95% CI: \(\mathrm{Estimate} \pm z_{0.025} \cdot \mathrm{SE}\)</span></span></td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> <span data-qmd-base64="Q1YlIGZvciBsb2ctbm9ybWFsICRcT21lZ2EkOiAkXHNxcnR7XGV4cChcbWF0aHJte0VzdGltYXRlfSkgLSAxfSBcdGltZXMgMTAwJA=="><span class='gt_from_md'>CV% for log-normal \(\Omega\): \(\sqrt{\exp(\mathrm{Estimate}) - 1} \times 100\)</span></span></td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="7"> <span data-qmd-base64="Q1YlIGZvciBwcm9wb3J0aW9uYWwgJFxTaWdtYSQ6ICRcc3FydHtcbWF0aHJte0VzdGltYXRlfX0gXHRpbWVzIDEwMCQ="><span class='gt_from_md'>CV% for proportional \(\Sigma\): \(\sqrt{\mathrm{Estimate}} \times 100\)</span></span></td>
    </tr>
  </tfoot>
</table>
</div>
```

:::
:::



# Confidence Interval level


::: {.cell}

```{.r .cell-code}
spec <- TableSpec(
  display_transforms = list(omega = c("cv")),
  sections = section_rules(
    kind == "THETA" ~ "Structural model parameters",
    kind == "OMEGA" & diagonal ~ "Interindividual variance parameters",
    kind == "OMEGA" & !diagonal ~ "Interindividual covariance parameters",
    kind == "SIGMA" ~ "Residual variance",
    TRUE ~ "Other"
  ),
  ci_level = 0.7,
  n_sigfig = 3
)

mod_sum <- get_model_summary(file.path(model_dir, model_run))
info <- get_model_parameter_info(file.path(model_dir, model_run))

get_parameters(file.path(model_dir, model_run)) |>
  apply_table_spec(info, spec) |>
  add_summary_info(mod_sum) |>
  make_parameter_table()
```

::: {.cell-output-display}

```{=html}
<div id="azxbyhkmty" style="padding-left:0px;padding-right:0px;padding-top:10px;padding-bottom:10px;overflow-x:auto;overflow-y:auto;width:auto;height:auto;">
<style>#azxbyhkmty table {
  font-family: system-ui, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji';
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

#azxbyhkmty thead, #azxbyhkmty tbody, #azxbyhkmty tfoot, #azxbyhkmty tr, #azxbyhkmty td, #azxbyhkmty th {
  border-style: none;
}

#azxbyhkmty p {
  margin: 0;
  padding: 0;
}

#azxbyhkmty .gt_table {
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

#azxbyhkmty .gt_caption {
  padding-top: 4px;
  padding-bottom: 4px;
}

#azxbyhkmty .gt_title {
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

#azxbyhkmty .gt_subtitle {
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

#azxbyhkmty .gt_heading {
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

#azxbyhkmty .gt_bottom_border {
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#azxbyhkmty .gt_col_headings {
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

#azxbyhkmty .gt_col_heading {
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

#azxbyhkmty .gt_column_spanner_outer {
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

#azxbyhkmty .gt_column_spanner_outer:first-child {
  padding-left: 0;
}

#azxbyhkmty .gt_column_spanner_outer:last-child {
  padding-right: 0;
}

#azxbyhkmty .gt_column_spanner {
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

#azxbyhkmty .gt_spanner_row {
  border-bottom-style: hidden;
}

#azxbyhkmty .gt_group_heading {
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

#azxbyhkmty .gt_empty_group_heading {
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

#azxbyhkmty .gt_from_md > :first-child {
  margin-top: 0;
}

#azxbyhkmty .gt_from_md > :last-child {
  margin-bottom: 0;
}

#azxbyhkmty .gt_row {
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

#azxbyhkmty .gt_stub {
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

#azxbyhkmty .gt_stub_row_group {
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

#azxbyhkmty .gt_row_group_first td {
  border-top-width: 2px;
}

#azxbyhkmty .gt_row_group_first th {
  border-top-width: 2px;
}

#azxbyhkmty .gt_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#azxbyhkmty .gt_first_summary_row {
  border-top-style: solid;
  border-top-color: #D3D3D3;
}

#azxbyhkmty .gt_first_summary_row.thick {
  border-top-width: 2px;
}

#azxbyhkmty .gt_last_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#azxbyhkmty .gt_grand_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#azxbyhkmty .gt_first_grand_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-top-style: double;
  border-top-width: 6px;
  border-top-color: #D3D3D3;
}

#azxbyhkmty .gt_last_grand_summary_row_top {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: double;
  border-bottom-width: 6px;
  border-bottom-color: #D3D3D3;
}

#azxbyhkmty .gt_striped {
  background-color: rgba(128, 128, 128, 0.05);
}

#azxbyhkmty .gt_table_body {
  border-top-style: solid;
  border-top-width: 2px;
  border-top-color: #D3D3D3;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#azxbyhkmty .gt_footnotes {
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

#azxbyhkmty .gt_footnote {
  margin: 0px;
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#azxbyhkmty .gt_sourcenotes {
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

#azxbyhkmty .gt_sourcenote {
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#azxbyhkmty .gt_left {
  text-align: left;
}

#azxbyhkmty .gt_center {
  text-align: center;
}

#azxbyhkmty .gt_right {
  text-align: right;
  font-variant-numeric: tabular-nums;
}

#azxbyhkmty .gt_font_normal {
  font-weight: normal;
}

#azxbyhkmty .gt_font_bold {
  font-weight: bold;
}

#azxbyhkmty .gt_font_italic {
  font-style: italic;
}

#azxbyhkmty .gt_super {
  font-size: 65%;
}

#azxbyhkmty .gt_footnote_marks {
  font-size: 75%;
  vertical-align: 0.4em;
  position: initial;
}

#azxbyhkmty .gt_asterisk {
  font-size: 100%;
  vertical-align: 0;
}

#azxbyhkmty .gt_indent_1 {
  text-indent: 5px;
}

#azxbyhkmty .gt_indent_2 {
  text-indent: 10px;
}

#azxbyhkmty .gt_indent_3 {
  text-indent: 15px;
}

#azxbyhkmty .gt_indent_4 {
  text-indent: 20px;
}

#azxbyhkmty .gt_indent_5 {
  text-indent: 25px;
}

#azxbyhkmty .katex-display {
  display: inline-flex !important;
  margin-bottom: 0.75em !important;
}

#azxbyhkmty div.Reactable > div.rt-table > div.rt-thead > div.rt-tr.rt-tr-group-header > div.rt-th-group:after {
  height: 0px !important;
}

td, th {
  white-space: nowrap;
}
</style>
<table class="gt_table" data-quarto-disable-processing="false" data-quarto-bootstrap="false">
  <thead>
    <tr class="gt_heading">
      <td colspan="8" class="gt_heading gt_title gt_font_normal gt_bottom_border" style="font-weight: bold;">Model Parameters</td>
    </tr>
    
    <tr class="gt_col_headings">
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="name">Parameter</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="symbol"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="unit"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="estimate">Estimate</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="variability"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="ci_low">70% CI</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="rse">RSE (%)</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="shrinkage">Shrinkage (%)</th>
    </tr>
  </thead>
  <tbody class="gt_table_body">
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Structural model parameters">Structural model parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="VFZDTA=="><span class='gt_from_md'>TVCL</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97MX0k"><span class='gt_from_md'>\(\theta_{1}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="TC9ocg=="><span class='gt_from_md'>L/hr</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">1.33</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[1.21, 1.44]</td>
<td headers="Structural model parameters  rse" class="gt_row gt_right">8.41</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="VFZW"><span class='gt_from_md'>TVV</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97Mn0k"><span class='gt_from_md'>\(\theta_{2}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="TA=="><span class='gt_from_md'>L</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">40.2</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[37.2, 43.1]</td>
<td headers="Structural model parameters  rse" class="gt_row gt_right">7.07</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="VFZLQQ=="><span class='gt_from_md'>TVKA</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97M30k"><span class='gt_from_md'>\(\theta_{3}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="MS9ocg=="><span class='gt_from_md'>1/hr</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">1.21</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[1.10, 1.33]</td>
<td headers="Structural model parameters  rse" class="gt_row gt_right">9.06</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Interindividual variance parameters">Interindividual variance parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="T00xIChUVkNMKQ=="><span class='gt_from_md'>OM1 (TVCL)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDEsMSl9JA=="><span class='gt_from_md'>\(\Omega_{(1,1)}\)</span></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.122</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMSUp"><span class='gt_from_md'>(CV = 36.1%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0702, 0.175]</td>
<td headers="Interindividual variance parameters  rse" class="gt_row gt_right">41.2</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">13.1</td></tr>
    <tr><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="T00yIChUVlYp"><span class='gt_from_md'>OM2 (TVV)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDIsMil9JA=="><span class='gt_from_md'>\(\Omega_{(2,2)}\)</span></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.124</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMyUp"><span class='gt_from_md'>(CV = 36.3%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0858, 0.162]</td>
<td headers="Interindividual variance parameters  rse" class="gt_row gt_right">29.7</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">4.63</td></tr>
    <tr><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="T00zIChUVktBKQ=="><span class='gt_from_md'>OM3 (TVKA)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDMsMyl9JA=="><span class='gt_from_md'>\(\Omega_{(3,3)}\)</span></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.122</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMSUp"><span class='gt_from_md'>(CV = 36.1%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0641, 0.181]</td>
<td headers="Interindividual variance parameters  rse" class="gt_row gt_right">46.0</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">24.3</td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Interindividual covariance parameters">Interindividual covariance parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Interindividual covariance parameters  name" class="gt_row gt_left"><span data-qmd-base64="T00xLDIgKFRWQ0wtVFZWKQ=="><span class='gt_from_md'>OM1,2 (TVCL-TVV)</span></span></td>
<td headers="Interindividual covariance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDIsMSl9JA=="><span class='gt_from_md'>\(\Omega_{(2,1)}\)</span></span></td>
<td headers="Interindividual covariance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual covariance parameters  estimate" class="gt_row gt_right">0.0745</td>
<td headers="Interindividual covariance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENvcnIgPSAwLjYwNik="><span class='gt_from_md'>(Corr = 0.606)</span></span></td>
<td headers="Interindividual covariance parameters  ci_low" class="gt_row gt_right">[0.0421, 0.107]</td>
<td headers="Interindividual covariance parameters  rse" class="gt_row gt_right">42.0</td>
<td headers="Interindividual covariance parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Residual variance">Residual variance</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Residual variance  name" class="gt_row gt_left"><span data-qmd-base64="U0lHMQ=="><span class='gt_from_md'>SIG1</span></span></td>
<td headers="Residual variance  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxTaWdtYV97KDEsMSl9JA=="><span class='gt_from_md'>\(\Sigma_{(1,1)}\)</span></span></td>
<td headers="Residual variance  unit" class="gt_row gt_left"><br /></td>
<td headers="Residual variance  estimate" class="gt_row gt_right">0.0375</td>
<td headers="Residual variance  variability" class="gt_row gt_left"><span data-qmd-base64="KFNEID0gMC4xOTQp"><span class='gt_from_md'>(SD = 0.194)</span></span></td>
<td headers="Residual variance  ci_low" class="gt_row gt_right">[0.0313, 0.0438]</td>
<td headers="Residual variance  rse" class="gt_row gt_right">16.1</td>
<td headers="Residual variance  shrinkage" class="gt_row gt_right">14.4</td></tr>
    <tr><td headers="Residual variance  name" class="gt_row gt_left"><span data-qmd-base64="U0lHMg=="><span class='gt_from_md'>SIG2</span></span></td>
<td headers="Residual variance  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxTaWdtYV97KDIsMil9JA=="><span class='gt_from_md'>\(\Sigma_{(2,2)}\)</span></span></td>
<td headers="Residual variance  unit" class="gt_row gt_left"><br /></td>
<td headers="Residual variance  estimate" class="gt_row gt_right">0.00527</td>
<td headers="Residual variance  variability" class="gt_row gt_left"><span data-qmd-base64="KFNEID0gMC4wNzI2KQ=="><span class='gt_from_md'>(SD = 0.0726)</span></span></td>
<td headers="Residual variance  ci_low" class="gt_row gt_right">[−0.00427, 0.0148]</td>
<td headers="Residual variance  rse" class="gt_row gt_right">175</td>
<td headers="Residual variance  shrinkage" class="gt_row gt_right">14.4</td></tr>
  </tbody>
  <tfoot>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> First Order Conditional Estimation with Interaction | Objective function value: -110 | Condition Number: 6.17</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> Abbreviations:</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> CI = confidence intervals; RSE = relative standard error; CV = coefficient of variation; SD = standard deviation; Corr = correlation</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> <span data-qmd-base64="NzAlIENJOiAkXG1hdGhybXtFc3RpbWF0ZX0gXHBtIHpfezAuMTV9IFxjZG90IFxtYXRocm17U0V9JA=="><span class='gt_from_md'>70% CI: \(\mathrm{Estimate} \pm z_{0.15} \cdot \mathrm{SE}\)</span></span></td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> <span data-qmd-base64="Q1YlIGZvciBsb2ctbm9ybWFsICRcT21lZ2EkOiAkXHNxcnR7XGV4cChcbWF0aHJte0VzdGltYXRlfSkgLSAxfSBcdGltZXMgMTAwJA=="><span class='gt_from_md'>CV% for log-normal \(\Omega\): \(\sqrt{\exp(\mathrm{Estimate}) - 1} \times 100\)</span></span></td>
    </tr>
  </tfoot>
</table>
</div>
```

:::
:::


# Changing summary info shown



::: {.cell}

```{.r .cell-code}
spec <- TableSpec(
  display_transforms = list(omega = c("cv")),
  sections = section_rules(
    kind == "THETA" ~ "Structural model parameters",
    kind == "OMEGA" & diagonal ~ "Interindividual variance parameters",
    kind == "OMEGA" & !diagonal ~ "Interindividual covariance parameters",
    kind == "SIGMA" ~ "Residual variance",
    TRUE ~ "Other"
  ),
  ci_level = 0.7,
  n_sigfig = 3
)

mod_sum <- get_model_summary(file.path(model_dir, model_run))
info <- get_model_parameter_info(file.path(model_dir, model_run))

get_parameters(file.path(model_dir, model_run)) |>
  apply_table_spec(info, spec) |>
  add_summary_info(mod_sum, show_cond_num = FALSE) |>
  make_parameter_table()
```

::: {.cell-output-display}

```{=html}
<div id="lkozfneewi" style="padding-left:0px;padding-right:0px;padding-top:10px;padding-bottom:10px;overflow-x:auto;overflow-y:auto;width:auto;height:auto;">
<style>#lkozfneewi table {
  font-family: system-ui, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji';
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

#lkozfneewi thead, #lkozfneewi tbody, #lkozfneewi tfoot, #lkozfneewi tr, #lkozfneewi td, #lkozfneewi th {
  border-style: none;
}

#lkozfneewi p {
  margin: 0;
  padding: 0;
}

#lkozfneewi .gt_table {
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

#lkozfneewi .gt_caption {
  padding-top: 4px;
  padding-bottom: 4px;
}

#lkozfneewi .gt_title {
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

#lkozfneewi .gt_subtitle {
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

#lkozfneewi .gt_heading {
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

#lkozfneewi .gt_bottom_border {
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#lkozfneewi .gt_col_headings {
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

#lkozfneewi .gt_col_heading {
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

#lkozfneewi .gt_column_spanner_outer {
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

#lkozfneewi .gt_column_spanner_outer:first-child {
  padding-left: 0;
}

#lkozfneewi .gt_column_spanner_outer:last-child {
  padding-right: 0;
}

#lkozfneewi .gt_column_spanner {
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

#lkozfneewi .gt_spanner_row {
  border-bottom-style: hidden;
}

#lkozfneewi .gt_group_heading {
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

#lkozfneewi .gt_empty_group_heading {
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

#lkozfneewi .gt_from_md > :first-child {
  margin-top: 0;
}

#lkozfneewi .gt_from_md > :last-child {
  margin-bottom: 0;
}

#lkozfneewi .gt_row {
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

#lkozfneewi .gt_stub {
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

#lkozfneewi .gt_stub_row_group {
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

#lkozfneewi .gt_row_group_first td {
  border-top-width: 2px;
}

#lkozfneewi .gt_row_group_first th {
  border-top-width: 2px;
}

#lkozfneewi .gt_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#lkozfneewi .gt_first_summary_row {
  border-top-style: solid;
  border-top-color: #D3D3D3;
}

#lkozfneewi .gt_first_summary_row.thick {
  border-top-width: 2px;
}

#lkozfneewi .gt_last_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#lkozfneewi .gt_grand_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#lkozfneewi .gt_first_grand_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-top-style: double;
  border-top-width: 6px;
  border-top-color: #D3D3D3;
}

#lkozfneewi .gt_last_grand_summary_row_top {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: double;
  border-bottom-width: 6px;
  border-bottom-color: #D3D3D3;
}

#lkozfneewi .gt_striped {
  background-color: rgba(128, 128, 128, 0.05);
}

#lkozfneewi .gt_table_body {
  border-top-style: solid;
  border-top-width: 2px;
  border-top-color: #D3D3D3;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#lkozfneewi .gt_footnotes {
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

#lkozfneewi .gt_footnote {
  margin: 0px;
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#lkozfneewi .gt_sourcenotes {
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

#lkozfneewi .gt_sourcenote {
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#lkozfneewi .gt_left {
  text-align: left;
}

#lkozfneewi .gt_center {
  text-align: center;
}

#lkozfneewi .gt_right {
  text-align: right;
  font-variant-numeric: tabular-nums;
}

#lkozfneewi .gt_font_normal {
  font-weight: normal;
}

#lkozfneewi .gt_font_bold {
  font-weight: bold;
}

#lkozfneewi .gt_font_italic {
  font-style: italic;
}

#lkozfneewi .gt_super {
  font-size: 65%;
}

#lkozfneewi .gt_footnote_marks {
  font-size: 75%;
  vertical-align: 0.4em;
  position: initial;
}

#lkozfneewi .gt_asterisk {
  font-size: 100%;
  vertical-align: 0;
}

#lkozfneewi .gt_indent_1 {
  text-indent: 5px;
}

#lkozfneewi .gt_indent_2 {
  text-indent: 10px;
}

#lkozfneewi .gt_indent_3 {
  text-indent: 15px;
}

#lkozfneewi .gt_indent_4 {
  text-indent: 20px;
}

#lkozfneewi .gt_indent_5 {
  text-indent: 25px;
}

#lkozfneewi .katex-display {
  display: inline-flex !important;
  margin-bottom: 0.75em !important;
}

#lkozfneewi div.Reactable > div.rt-table > div.rt-thead > div.rt-tr.rt-tr-group-header > div.rt-th-group:after {
  height: 0px !important;
}

td, th {
  white-space: nowrap;
}
</style>
<table class="gt_table" data-quarto-disable-processing="false" data-quarto-bootstrap="false">
  <thead>
    <tr class="gt_heading">
      <td colspan="8" class="gt_heading gt_title gt_font_normal gt_bottom_border" style="font-weight: bold;">Model Parameters</td>
    </tr>
    
    <tr class="gt_col_headings">
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="name">Parameter</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="symbol"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="unit"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="estimate">Estimate</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="variability"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="ci_low">70% CI</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="rse">RSE (%)</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="shrinkage">Shrinkage (%)</th>
    </tr>
  </thead>
  <tbody class="gt_table_body">
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Structural model parameters">Structural model parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="VFZDTA=="><span class='gt_from_md'>TVCL</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97MX0k"><span class='gt_from_md'>\(\theta_{1}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="TC9ocg=="><span class='gt_from_md'>L/hr</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">1.33</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[1.21, 1.44]</td>
<td headers="Structural model parameters  rse" class="gt_row gt_right">8.41</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="VFZW"><span class='gt_from_md'>TVV</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97Mn0k"><span class='gt_from_md'>\(\theta_{2}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="TA=="><span class='gt_from_md'>L</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">40.2</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[37.2, 43.1]</td>
<td headers="Structural model parameters  rse" class="gt_row gt_right">7.07</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="VFZLQQ=="><span class='gt_from_md'>TVKA</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97M30k"><span class='gt_from_md'>\(\theta_{3}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="MS9ocg=="><span class='gt_from_md'>1/hr</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">1.21</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[1.10, 1.33]</td>
<td headers="Structural model parameters  rse" class="gt_row gt_right">9.06</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Interindividual variance parameters">Interindividual variance parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="T00xIChUVkNMKQ=="><span class='gt_from_md'>OM1 (TVCL)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDEsMSl9JA=="><span class='gt_from_md'>\(\Omega_{(1,1)}\)</span></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.122</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMSUp"><span class='gt_from_md'>(CV = 36.1%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0702, 0.175]</td>
<td headers="Interindividual variance parameters  rse" class="gt_row gt_right">41.2</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">13.1</td></tr>
    <tr><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="T00yIChUVlYp"><span class='gt_from_md'>OM2 (TVV)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDIsMil9JA=="><span class='gt_from_md'>\(\Omega_{(2,2)}\)</span></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.124</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMyUp"><span class='gt_from_md'>(CV = 36.3%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0858, 0.162]</td>
<td headers="Interindividual variance parameters  rse" class="gt_row gt_right">29.7</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">4.63</td></tr>
    <tr><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="T00zIChUVktBKQ=="><span class='gt_from_md'>OM3 (TVKA)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDMsMyl9JA=="><span class='gt_from_md'>\(\Omega_{(3,3)}\)</span></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.122</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMSUp"><span class='gt_from_md'>(CV = 36.1%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0641, 0.181]</td>
<td headers="Interindividual variance parameters  rse" class="gt_row gt_right">46.0</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">24.3</td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Interindividual covariance parameters">Interindividual covariance parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Interindividual covariance parameters  name" class="gt_row gt_left"><span data-qmd-base64="T00xLDIgKFRWQ0wtVFZWKQ=="><span class='gt_from_md'>OM1,2 (TVCL-TVV)</span></span></td>
<td headers="Interindividual covariance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDIsMSl9JA=="><span class='gt_from_md'>\(\Omega_{(2,1)}\)</span></span></td>
<td headers="Interindividual covariance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual covariance parameters  estimate" class="gt_row gt_right">0.0745</td>
<td headers="Interindividual covariance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENvcnIgPSAwLjYwNik="><span class='gt_from_md'>(Corr = 0.606)</span></span></td>
<td headers="Interindividual covariance parameters  ci_low" class="gt_row gt_right">[0.0421, 0.107]</td>
<td headers="Interindividual covariance parameters  rse" class="gt_row gt_right">42.0</td>
<td headers="Interindividual covariance parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Residual variance">Residual variance</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Residual variance  name" class="gt_row gt_left"><span data-qmd-base64="U0lHMQ=="><span class='gt_from_md'>SIG1</span></span></td>
<td headers="Residual variance  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxTaWdtYV97KDEsMSl9JA=="><span class='gt_from_md'>\(\Sigma_{(1,1)}\)</span></span></td>
<td headers="Residual variance  unit" class="gt_row gt_left"><br /></td>
<td headers="Residual variance  estimate" class="gt_row gt_right">0.0375</td>
<td headers="Residual variance  variability" class="gt_row gt_left"><span data-qmd-base64="KFNEID0gMC4xOTQp"><span class='gt_from_md'>(SD = 0.194)</span></span></td>
<td headers="Residual variance  ci_low" class="gt_row gt_right">[0.0313, 0.0438]</td>
<td headers="Residual variance  rse" class="gt_row gt_right">16.1</td>
<td headers="Residual variance  shrinkage" class="gt_row gt_right">14.4</td></tr>
    <tr><td headers="Residual variance  name" class="gt_row gt_left"><span data-qmd-base64="U0lHMg=="><span class='gt_from_md'>SIG2</span></span></td>
<td headers="Residual variance  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxTaWdtYV97KDIsMil9JA=="><span class='gt_from_md'>\(\Sigma_{(2,2)}\)</span></span></td>
<td headers="Residual variance  unit" class="gt_row gt_left"><br /></td>
<td headers="Residual variance  estimate" class="gt_row gt_right">0.00527</td>
<td headers="Residual variance  variability" class="gt_row gt_left"><span data-qmd-base64="KFNEID0gMC4wNzI2KQ=="><span class='gt_from_md'>(SD = 0.0726)</span></span></td>
<td headers="Residual variance  ci_low" class="gt_row gt_right">[−0.00427, 0.0148]</td>
<td headers="Residual variance  rse" class="gt_row gt_right">175</td>
<td headers="Residual variance  shrinkage" class="gt_row gt_right">14.4</td></tr>
  </tbody>
  <tfoot>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> First Order Conditional Estimation with Interaction | Objective function value: -110</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> Abbreviations:</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> CI = confidence intervals; RSE = relative standard error; CV = coefficient of variation; SD = standard deviation; Corr = correlation</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> <span data-qmd-base64="NzAlIENJOiAkXG1hdGhybXtFc3RpbWF0ZX0gXHBtIHpfezAuMTV9IFxjZG90IFxtYXRocm17U0V9JA=="><span class='gt_from_md'>70% CI: \(\mathrm{Estimate} \pm z_{0.15} \cdot \mathrm{SE}\)</span></span></td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> <span data-qmd-base64="Q1YlIGZvciBsb2ctbm9ybWFsICRcT21lZ2EkOiAkXHNxcnR7XGV4cChcbWF0aHJte0VzdGltYXRlfSkgLSAxfSBcdGltZXMgMTAwJA=="><span class='gt_from_md'>CV% for log-normal \(\Omega\): \(\sqrt{\exp(\mathrm{Estimate}) - 1} \times 100\)</span></span></td>
    </tr>
  </tfoot>
</table>
</div>
```

:::
:::

::: {.cell}

```{.r .cell-code}
get_parameters(file.path(model_dir, model_run)) |>
  apply_table_spec(info, spec) |>
  add_summary_info(mod_sum, show_cond_num = FALSE, show_ofv = FALSE) |>
  make_parameter_table()
```

::: {.cell-output-display}

```{=html}
<div id="uaoctwblta" style="padding-left:0px;padding-right:0px;padding-top:10px;padding-bottom:10px;overflow-x:auto;overflow-y:auto;width:auto;height:auto;">
<style>#uaoctwblta table {
  font-family: system-ui, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji';
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

#uaoctwblta thead, #uaoctwblta tbody, #uaoctwblta tfoot, #uaoctwblta tr, #uaoctwblta td, #uaoctwblta th {
  border-style: none;
}

#uaoctwblta p {
  margin: 0;
  padding: 0;
}

#uaoctwblta .gt_table {
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

#uaoctwblta .gt_caption {
  padding-top: 4px;
  padding-bottom: 4px;
}

#uaoctwblta .gt_title {
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

#uaoctwblta .gt_subtitle {
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

#uaoctwblta .gt_heading {
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

#uaoctwblta .gt_bottom_border {
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#uaoctwblta .gt_col_headings {
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

#uaoctwblta .gt_col_heading {
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

#uaoctwblta .gt_column_spanner_outer {
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

#uaoctwblta .gt_column_spanner_outer:first-child {
  padding-left: 0;
}

#uaoctwblta .gt_column_spanner_outer:last-child {
  padding-right: 0;
}

#uaoctwblta .gt_column_spanner {
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

#uaoctwblta .gt_spanner_row {
  border-bottom-style: hidden;
}

#uaoctwblta .gt_group_heading {
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

#uaoctwblta .gt_empty_group_heading {
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

#uaoctwblta .gt_from_md > :first-child {
  margin-top: 0;
}

#uaoctwblta .gt_from_md > :last-child {
  margin-bottom: 0;
}

#uaoctwblta .gt_row {
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

#uaoctwblta .gt_stub {
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

#uaoctwblta .gt_stub_row_group {
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

#uaoctwblta .gt_row_group_first td {
  border-top-width: 2px;
}

#uaoctwblta .gt_row_group_first th {
  border-top-width: 2px;
}

#uaoctwblta .gt_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#uaoctwblta .gt_first_summary_row {
  border-top-style: solid;
  border-top-color: #D3D3D3;
}

#uaoctwblta .gt_first_summary_row.thick {
  border-top-width: 2px;
}

#uaoctwblta .gt_last_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#uaoctwblta .gt_grand_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#uaoctwblta .gt_first_grand_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-top-style: double;
  border-top-width: 6px;
  border-top-color: #D3D3D3;
}

#uaoctwblta .gt_last_grand_summary_row_top {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: double;
  border-bottom-width: 6px;
  border-bottom-color: #D3D3D3;
}

#uaoctwblta .gt_striped {
  background-color: rgba(128, 128, 128, 0.05);
}

#uaoctwblta .gt_table_body {
  border-top-style: solid;
  border-top-width: 2px;
  border-top-color: #D3D3D3;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#uaoctwblta .gt_footnotes {
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

#uaoctwblta .gt_footnote {
  margin: 0px;
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#uaoctwblta .gt_sourcenotes {
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

#uaoctwblta .gt_sourcenote {
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#uaoctwblta .gt_left {
  text-align: left;
}

#uaoctwblta .gt_center {
  text-align: center;
}

#uaoctwblta .gt_right {
  text-align: right;
  font-variant-numeric: tabular-nums;
}

#uaoctwblta .gt_font_normal {
  font-weight: normal;
}

#uaoctwblta .gt_font_bold {
  font-weight: bold;
}

#uaoctwblta .gt_font_italic {
  font-style: italic;
}

#uaoctwblta .gt_super {
  font-size: 65%;
}

#uaoctwblta .gt_footnote_marks {
  font-size: 75%;
  vertical-align: 0.4em;
  position: initial;
}

#uaoctwblta .gt_asterisk {
  font-size: 100%;
  vertical-align: 0;
}

#uaoctwblta .gt_indent_1 {
  text-indent: 5px;
}

#uaoctwblta .gt_indent_2 {
  text-indent: 10px;
}

#uaoctwblta .gt_indent_3 {
  text-indent: 15px;
}

#uaoctwblta .gt_indent_4 {
  text-indent: 20px;
}

#uaoctwblta .gt_indent_5 {
  text-indent: 25px;
}

#uaoctwblta .katex-display {
  display: inline-flex !important;
  margin-bottom: 0.75em !important;
}

#uaoctwblta div.Reactable > div.rt-table > div.rt-thead > div.rt-tr.rt-tr-group-header > div.rt-th-group:after {
  height: 0px !important;
}

td, th {
  white-space: nowrap;
}
</style>
<table class="gt_table" data-quarto-disable-processing="false" data-quarto-bootstrap="false">
  <thead>
    <tr class="gt_heading">
      <td colspan="8" class="gt_heading gt_title gt_font_normal gt_bottom_border" style="font-weight: bold;">Model Parameters</td>
    </tr>
    
    <tr class="gt_col_headings">
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="name">Parameter</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="symbol"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="unit"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="estimate">Estimate</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="variability"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="ci_low">70% CI</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="rse">RSE (%)</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="shrinkage">Shrinkage (%)</th>
    </tr>
  </thead>
  <tbody class="gt_table_body">
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Structural model parameters">Structural model parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="VFZDTA=="><span class='gt_from_md'>TVCL</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97MX0k"><span class='gt_from_md'>\(\theta_{1}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="TC9ocg=="><span class='gt_from_md'>L/hr</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">1.33</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[1.21, 1.44]</td>
<td headers="Structural model parameters  rse" class="gt_row gt_right">8.41</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="VFZW"><span class='gt_from_md'>TVV</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97Mn0k"><span class='gt_from_md'>\(\theta_{2}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="TA=="><span class='gt_from_md'>L</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">40.2</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[37.2, 43.1]</td>
<td headers="Structural model parameters  rse" class="gt_row gt_right">7.07</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="VFZLQQ=="><span class='gt_from_md'>TVKA</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97M30k"><span class='gt_from_md'>\(\theta_{3}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="MS9ocg=="><span class='gt_from_md'>1/hr</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">1.21</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[1.10, 1.33]</td>
<td headers="Structural model parameters  rse" class="gt_row gt_right">9.06</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Interindividual variance parameters">Interindividual variance parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="T00xIChUVkNMKQ=="><span class='gt_from_md'>OM1 (TVCL)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDEsMSl9JA=="><span class='gt_from_md'>\(\Omega_{(1,1)}\)</span></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.122</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMSUp"><span class='gt_from_md'>(CV = 36.1%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0702, 0.175]</td>
<td headers="Interindividual variance parameters  rse" class="gt_row gt_right">41.2</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">13.1</td></tr>
    <tr><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="T00yIChUVlYp"><span class='gt_from_md'>OM2 (TVV)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDIsMil9JA=="><span class='gt_from_md'>\(\Omega_{(2,2)}\)</span></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.124</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMyUp"><span class='gt_from_md'>(CV = 36.3%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0858, 0.162]</td>
<td headers="Interindividual variance parameters  rse" class="gt_row gt_right">29.7</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">4.63</td></tr>
    <tr><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="T00zIChUVktBKQ=="><span class='gt_from_md'>OM3 (TVKA)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDMsMyl9JA=="><span class='gt_from_md'>\(\Omega_{(3,3)}\)</span></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.122</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMSUp"><span class='gt_from_md'>(CV = 36.1%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0641, 0.181]</td>
<td headers="Interindividual variance parameters  rse" class="gt_row gt_right">46.0</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">24.3</td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Interindividual covariance parameters">Interindividual covariance parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Interindividual covariance parameters  name" class="gt_row gt_left"><span data-qmd-base64="T00xLDIgKFRWQ0wtVFZWKQ=="><span class='gt_from_md'>OM1,2 (TVCL-TVV)</span></span></td>
<td headers="Interindividual covariance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDIsMSl9JA=="><span class='gt_from_md'>\(\Omega_{(2,1)}\)</span></span></td>
<td headers="Interindividual covariance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual covariance parameters  estimate" class="gt_row gt_right">0.0745</td>
<td headers="Interindividual covariance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENvcnIgPSAwLjYwNik="><span class='gt_from_md'>(Corr = 0.606)</span></span></td>
<td headers="Interindividual covariance parameters  ci_low" class="gt_row gt_right">[0.0421, 0.107]</td>
<td headers="Interindividual covariance parameters  rse" class="gt_row gt_right">42.0</td>
<td headers="Interindividual covariance parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Residual variance">Residual variance</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Residual variance  name" class="gt_row gt_left"><span data-qmd-base64="U0lHMQ=="><span class='gt_from_md'>SIG1</span></span></td>
<td headers="Residual variance  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxTaWdtYV97KDEsMSl9JA=="><span class='gt_from_md'>\(\Sigma_{(1,1)}\)</span></span></td>
<td headers="Residual variance  unit" class="gt_row gt_left"><br /></td>
<td headers="Residual variance  estimate" class="gt_row gt_right">0.0375</td>
<td headers="Residual variance  variability" class="gt_row gt_left"><span data-qmd-base64="KFNEID0gMC4xOTQp"><span class='gt_from_md'>(SD = 0.194)</span></span></td>
<td headers="Residual variance  ci_low" class="gt_row gt_right">[0.0313, 0.0438]</td>
<td headers="Residual variance  rse" class="gt_row gt_right">16.1</td>
<td headers="Residual variance  shrinkage" class="gt_row gt_right">14.4</td></tr>
    <tr><td headers="Residual variance  name" class="gt_row gt_left"><span data-qmd-base64="U0lHMg=="><span class='gt_from_md'>SIG2</span></span></td>
<td headers="Residual variance  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxTaWdtYV97KDIsMil9JA=="><span class='gt_from_md'>\(\Sigma_{(2,2)}\)</span></span></td>
<td headers="Residual variance  unit" class="gt_row gt_left"><br /></td>
<td headers="Residual variance  estimate" class="gt_row gt_right">0.00527</td>
<td headers="Residual variance  variability" class="gt_row gt_left"><span data-qmd-base64="KFNEID0gMC4wNzI2KQ=="><span class='gt_from_md'>(SD = 0.0726)</span></span></td>
<td headers="Residual variance  ci_low" class="gt_row gt_right">[−0.00427, 0.0148]</td>
<td headers="Residual variance  rse" class="gt_row gt_right">175</td>
<td headers="Residual variance  shrinkage" class="gt_row gt_right">14.4</td></tr>
  </tbody>
  <tfoot>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> First Order Conditional Estimation with Interaction</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> Abbreviations:</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> CI = confidence intervals; RSE = relative standard error; CV = coefficient of variation; SD = standard deviation; Corr = correlation</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> <span data-qmd-base64="NzAlIENJOiAkXG1hdGhybXtFc3RpbWF0ZX0gXHBtIHpfezAuMTV9IFxjZG90IFxtYXRocm17U0V9JA=="><span class='gt_from_md'>70% CI: \(\mathrm{Estimate} \pm z_{0.15} \cdot \mathrm{SE}\)</span></span></td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> <span data-qmd-base64="Q1YlIGZvciBsb2ctbm9ybWFsICRcT21lZ2EkOiAkXHNxcnR7XGV4cChcbWF0aHJte0VzdGltYXRlfSkgLSAxfSBcdGltZXMgMTAwJA=="><span class='gt_from_md'>CV% for log-normal \(\Omega\): \(\sqrt{\exp(\mathrm{Estimate}) - 1} \times 100\)</span></span></td>
    </tr>
  </tfoot>
</table>
</div>
```

:::
:::

::: {.cell}

```{.r .cell-code}
get_parameters(file.path(model_dir, model_run)) |>
  apply_table_spec(info, spec) |>
  add_summary_info(mod_sum, show_method = FALSE) |>
  make_parameter_table()
```

::: {.cell-output-display}

```{=html}
<div id="zzpegzwteb" style="padding-left:0px;padding-right:0px;padding-top:10px;padding-bottom:10px;overflow-x:auto;overflow-y:auto;width:auto;height:auto;">
<style>#zzpegzwteb table {
  font-family: system-ui, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji';
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

#zzpegzwteb thead, #zzpegzwteb tbody, #zzpegzwteb tfoot, #zzpegzwteb tr, #zzpegzwteb td, #zzpegzwteb th {
  border-style: none;
}

#zzpegzwteb p {
  margin: 0;
  padding: 0;
}

#zzpegzwteb .gt_table {
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

#zzpegzwteb .gt_caption {
  padding-top: 4px;
  padding-bottom: 4px;
}

#zzpegzwteb .gt_title {
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

#zzpegzwteb .gt_subtitle {
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

#zzpegzwteb .gt_heading {
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

#zzpegzwteb .gt_bottom_border {
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#zzpegzwteb .gt_col_headings {
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

#zzpegzwteb .gt_col_heading {
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

#zzpegzwteb .gt_column_spanner_outer {
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

#zzpegzwteb .gt_column_spanner_outer:first-child {
  padding-left: 0;
}

#zzpegzwteb .gt_column_spanner_outer:last-child {
  padding-right: 0;
}

#zzpegzwteb .gt_column_spanner {
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

#zzpegzwteb .gt_spanner_row {
  border-bottom-style: hidden;
}

#zzpegzwteb .gt_group_heading {
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

#zzpegzwteb .gt_empty_group_heading {
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

#zzpegzwteb .gt_from_md > :first-child {
  margin-top: 0;
}

#zzpegzwteb .gt_from_md > :last-child {
  margin-bottom: 0;
}

#zzpegzwteb .gt_row {
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

#zzpegzwteb .gt_stub {
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

#zzpegzwteb .gt_stub_row_group {
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

#zzpegzwteb .gt_row_group_first td {
  border-top-width: 2px;
}

#zzpegzwteb .gt_row_group_first th {
  border-top-width: 2px;
}

#zzpegzwteb .gt_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#zzpegzwteb .gt_first_summary_row {
  border-top-style: solid;
  border-top-color: #D3D3D3;
}

#zzpegzwteb .gt_first_summary_row.thick {
  border-top-width: 2px;
}

#zzpegzwteb .gt_last_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#zzpegzwteb .gt_grand_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#zzpegzwteb .gt_first_grand_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-top-style: double;
  border-top-width: 6px;
  border-top-color: #D3D3D3;
}

#zzpegzwteb .gt_last_grand_summary_row_top {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: double;
  border-bottom-width: 6px;
  border-bottom-color: #D3D3D3;
}

#zzpegzwteb .gt_striped {
  background-color: rgba(128, 128, 128, 0.05);
}

#zzpegzwteb .gt_table_body {
  border-top-style: solid;
  border-top-width: 2px;
  border-top-color: #D3D3D3;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#zzpegzwteb .gt_footnotes {
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

#zzpegzwteb .gt_footnote {
  margin: 0px;
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#zzpegzwteb .gt_sourcenotes {
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

#zzpegzwteb .gt_sourcenote {
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#zzpegzwteb .gt_left {
  text-align: left;
}

#zzpegzwteb .gt_center {
  text-align: center;
}

#zzpegzwteb .gt_right {
  text-align: right;
  font-variant-numeric: tabular-nums;
}

#zzpegzwteb .gt_font_normal {
  font-weight: normal;
}

#zzpegzwteb .gt_font_bold {
  font-weight: bold;
}

#zzpegzwteb .gt_font_italic {
  font-style: italic;
}

#zzpegzwteb .gt_super {
  font-size: 65%;
}

#zzpegzwteb .gt_footnote_marks {
  font-size: 75%;
  vertical-align: 0.4em;
  position: initial;
}

#zzpegzwteb .gt_asterisk {
  font-size: 100%;
  vertical-align: 0;
}

#zzpegzwteb .gt_indent_1 {
  text-indent: 5px;
}

#zzpegzwteb .gt_indent_2 {
  text-indent: 10px;
}

#zzpegzwteb .gt_indent_3 {
  text-indent: 15px;
}

#zzpegzwteb .gt_indent_4 {
  text-indent: 20px;
}

#zzpegzwteb .gt_indent_5 {
  text-indent: 25px;
}

#zzpegzwteb .katex-display {
  display: inline-flex !important;
  margin-bottom: 0.75em !important;
}

#zzpegzwteb div.Reactable > div.rt-table > div.rt-thead > div.rt-tr.rt-tr-group-header > div.rt-th-group:after {
  height: 0px !important;
}

td, th {
  white-space: nowrap;
}
</style>
<table class="gt_table" data-quarto-disable-processing="false" data-quarto-bootstrap="false">
  <thead>
    <tr class="gt_heading">
      <td colspan="8" class="gt_heading gt_title gt_font_normal gt_bottom_border" style="font-weight: bold;">Model Parameters</td>
    </tr>
    
    <tr class="gt_col_headings">
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="name">Parameter</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="symbol"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="unit"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="estimate">Estimate</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="variability"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="ci_low">70% CI</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="rse">RSE (%)</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_right" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="shrinkage">Shrinkage (%)</th>
    </tr>
  </thead>
  <tbody class="gt_table_body">
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Structural model parameters">Structural model parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="VFZDTA=="><span class='gt_from_md'>TVCL</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97MX0k"><span class='gt_from_md'>\(\theta_{1}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="TC9ocg=="><span class='gt_from_md'>L/hr</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">1.33</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[1.21, 1.44]</td>
<td headers="Structural model parameters  rse" class="gt_row gt_right">8.41</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="VFZW"><span class='gt_from_md'>TVV</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97Mn0k"><span class='gt_from_md'>\(\theta_{2}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="TA=="><span class='gt_from_md'>L</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">40.2</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[37.2, 43.1]</td>
<td headers="Structural model parameters  rse" class="gt_row gt_right">7.07</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr><td headers="Structural model parameters  name" class="gt_row gt_left"><span data-qmd-base64="VFZLQQ=="><span class='gt_from_md'>TVKA</span></span></td>
<td headers="Structural model parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFx0aGV0YV97M30k"><span class='gt_from_md'>\(\theta_{3}\)</span></span></td>
<td headers="Structural model parameters  unit" class="gt_row gt_left"><span data-qmd-base64="MS9ocg=="><span class='gt_from_md'>1/hr</span></span></td>
<td headers="Structural model parameters  estimate" class="gt_row gt_right">1.21</td>
<td headers="Structural model parameters  variability" class="gt_row gt_left"><br /></td>
<td headers="Structural model parameters  ci_low" class="gt_row gt_right">[1.10, 1.33]</td>
<td headers="Structural model parameters  rse" class="gt_row gt_right">9.06</td>
<td headers="Structural model parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Interindividual variance parameters">Interindividual variance parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="T00xIChUVkNMKQ=="><span class='gt_from_md'>OM1 (TVCL)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDEsMSl9JA=="><span class='gt_from_md'>\(\Omega_{(1,1)}\)</span></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.122</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMSUp"><span class='gt_from_md'>(CV = 36.1%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0702, 0.175]</td>
<td headers="Interindividual variance parameters  rse" class="gt_row gt_right">41.2</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">13.1</td></tr>
    <tr><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="T00yIChUVlYp"><span class='gt_from_md'>OM2 (TVV)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDIsMil9JA=="><span class='gt_from_md'>\(\Omega_{(2,2)}\)</span></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.124</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMyUp"><span class='gt_from_md'>(CV = 36.3%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0858, 0.162]</td>
<td headers="Interindividual variance parameters  rse" class="gt_row gt_right">29.7</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">4.63</td></tr>
    <tr><td headers="Interindividual variance parameters  name" class="gt_row gt_left"><span data-qmd-base64="T00zIChUVktBKQ=="><span class='gt_from_md'>OM3 (TVKA)</span></span></td>
<td headers="Interindividual variance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDMsMyl9JA=="><span class='gt_from_md'>\(\Omega_{(3,3)}\)</span></span></td>
<td headers="Interindividual variance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual variance parameters  estimate" class="gt_row gt_right">0.122</td>
<td headers="Interindividual variance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENWID0gMzYuMSUp"><span class='gt_from_md'>(CV = 36.1%)</span></span></td>
<td headers="Interindividual variance parameters  ci_low" class="gt_row gt_right">[0.0641, 0.181]</td>
<td headers="Interindividual variance parameters  rse" class="gt_row gt_right">46.0</td>
<td headers="Interindividual variance parameters  shrinkage" class="gt_row gt_right">24.3</td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Interindividual covariance parameters">Interindividual covariance parameters</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Interindividual covariance parameters  name" class="gt_row gt_left"><span data-qmd-base64="T00xLDIgKFRWQ0wtVFZWKQ=="><span class='gt_from_md'>OM1,2 (TVCL-TVV)</span></span></td>
<td headers="Interindividual covariance parameters  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxPbWVnYV97KDIsMSl9JA=="><span class='gt_from_md'>\(\Omega_{(2,1)}\)</span></span></td>
<td headers="Interindividual covariance parameters  unit" class="gt_row gt_left"><br /></td>
<td headers="Interindividual covariance parameters  estimate" class="gt_row gt_right">0.0745</td>
<td headers="Interindividual covariance parameters  variability" class="gt_row gt_left"><span data-qmd-base64="KENvcnIgPSAwLjYwNik="><span class='gt_from_md'>(Corr = 0.606)</span></span></td>
<td headers="Interindividual covariance parameters  ci_low" class="gt_row gt_right">[0.0421, 0.107]</td>
<td headers="Interindividual covariance parameters  rse" class="gt_row gt_right">42.0</td>
<td headers="Interindividual covariance parameters  shrinkage" class="gt_row gt_right"><br /></td></tr>
    <tr class="gt_group_heading_row">
      <th colspan="8" class="gt_group_heading" style="font-weight: bold;" scope="colgroup" id="Residual variance">Residual variance</th>
    </tr>
    <tr class="gt_row_group_first"><td headers="Residual variance  name" class="gt_row gt_left"><span data-qmd-base64="U0lHMQ=="><span class='gt_from_md'>SIG1</span></span></td>
<td headers="Residual variance  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxTaWdtYV97KDEsMSl9JA=="><span class='gt_from_md'>\(\Sigma_{(1,1)}\)</span></span></td>
<td headers="Residual variance  unit" class="gt_row gt_left"><br /></td>
<td headers="Residual variance  estimate" class="gt_row gt_right">0.0375</td>
<td headers="Residual variance  variability" class="gt_row gt_left"><span data-qmd-base64="KFNEID0gMC4xOTQp"><span class='gt_from_md'>(SD = 0.194)</span></span></td>
<td headers="Residual variance  ci_low" class="gt_row gt_right">[0.0313, 0.0438]</td>
<td headers="Residual variance  rse" class="gt_row gt_right">16.1</td>
<td headers="Residual variance  shrinkage" class="gt_row gt_right">14.4</td></tr>
    <tr><td headers="Residual variance  name" class="gt_row gt_left"><span data-qmd-base64="U0lHMg=="><span class='gt_from_md'>SIG2</span></span></td>
<td headers="Residual variance  symbol" class="gt_row gt_left"><span data-qmd-base64="JFxTaWdtYV97KDIsMil9JA=="><span class='gt_from_md'>\(\Sigma_{(2,2)}\)</span></span></td>
<td headers="Residual variance  unit" class="gt_row gt_left"><br /></td>
<td headers="Residual variance  estimate" class="gt_row gt_right">0.00527</td>
<td headers="Residual variance  variability" class="gt_row gt_left"><span data-qmd-base64="KFNEID0gMC4wNzI2KQ=="><span class='gt_from_md'>(SD = 0.0726)</span></span></td>
<td headers="Residual variance  ci_low" class="gt_row gt_right">[−0.00427, 0.0148]</td>
<td headers="Residual variance  rse" class="gt_row gt_right">175</td>
<td headers="Residual variance  shrinkage" class="gt_row gt_right">14.4</td></tr>
  </tbody>
  <tfoot>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> Objective function value: -110 | Condition Number: 6.17</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> Abbreviations:</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> CI = confidence intervals; RSE = relative standard error; CV = coefficient of variation; SD = standard deviation; Corr = correlation</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> <span data-qmd-base64="NzAlIENJOiAkXG1hdGhybXtFc3RpbWF0ZX0gXHBtIHpfezAuMTV9IFxjZG90IFxtYXRocm17U0V9JA=="><span class='gt_from_md'>70% CI: \(\mathrm{Estimate} \pm z_{0.15} \cdot \mathrm{SE}\)</span></span></td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="8"> <span data-qmd-base64="Q1YlIGZvciBsb2ctbm9ybWFsICRcT21lZ2EkOiAkXHNxcnR7XGV4cChcbWF0aHJte0VzdGltYXRlfSkgLSAxfSBcdGltZXMgMTAwJA=="><span class='gt_from_md'>CV% for log-normal \(\Omega\): \(\sqrt{\exp(\mathrm{Estimate}) - 1} \times 100\)</span></span></td>
    </tr>
  </tfoot>
</table>
</div>
```

:::
:::



# Parameter Info Audit trail

Using just model comments the source is either model file, default, or hard-coded



::: {.cell}

```{.r .cell-code}
info <- get_model_parameter_info(file.path(model_dir, model_run))
audit_parameter_info(info)
```

::: {.cell-output-display}
# Parameter Info Audit

## Theta Sources

<table class="table table-striped">
 <thead>
  <tr>
   <th style="text-align:left;"> parameter </th>
   <th style="text-align:left;"> name </th>
   <th style="text-align:left;"> display </th>
   <th style="text-align:left;"> description </th>
   <th style="text-align:left;"> unit </th>
   <th style="text-align:left;"> parameterization </th>
  </tr>
 </thead>
<tbody>
  <tr>
   <td style="text-align:left;"> THETA1 </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
  </tr>
  <tr>
   <td style="text-align:left;"> THETA2 </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
  </tr>
  <tr>
   <td style="text-align:left;"> THETA3 </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
  </tr>
</tbody>
</table>

## Omega Sources

<table class="table table-striped">
 <thead>
  <tr>
   <th style="text-align:left;"> parameter </th>
   <th style="text-align:left;"> name </th>
   <th style="text-align:left;"> display </th>
   <th style="text-align:left;"> description </th>
   <th style="text-align:left;"> parameterization </th>
   <th style="text-align:left;"> associated_theta </th>
  </tr>
 </thead>
<tbody>
  <tr>
   <td style="text-align:left;"> OMEGA(1,1) </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
  </tr>
  <tr>
   <td style="text-align:left;"> OMEGA(2,1) </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
  </tr>
  <tr>
   <td style="text-align:left;"> OMEGA(2,2) </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
  </tr>
  <tr>
   <td style="text-align:left;"> OMEGA(3,3) </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
  </tr>
</tbody>
</table>

## Sigma Sources

<table class="table table-striped">
 <thead>
  <tr>
   <th style="text-align:left;"> parameter </th>
   <th style="text-align:left;"> name </th>
   <th style="text-align:left;"> display </th>
   <th style="text-align:left;"> description </th>
   <th style="text-align:left;"> parameterization </th>
  </tr>
 </thead>
<tbody>
  <tr>
   <td style="text-align:left;"> SIGMA(1,1) </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> default </td>
  </tr>
  <tr>
   <td style="text-align:left;"> SIGMA(2,2) </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> default </td>
  </tr>
</tbody>
</table>

:::
:::


Editing a property by hand:


::: {.cell}

```{.r .cell-code}
info@sigma$`SIGMA(1,1)`@display <- "Additive Error"
info@sigma$`SIGMA(1,1)`@parameterization <- "AddErr"

info@sigma$`SIGMA(2,2)`@display <- "Proportional Error"
info@sigma$`SIGMA(2,2)`@parameterization <- "Proportional"

audit_parameter_info(info)
```

::: {.cell-output-display}
# Parameter Info Audit

## Theta Sources

<table class="table table-striped">
 <thead>
  <tr>
   <th style="text-align:left;"> parameter </th>
   <th style="text-align:left;"> name </th>
   <th style="text-align:left;"> display </th>
   <th style="text-align:left;"> description </th>
   <th style="text-align:left;"> unit </th>
   <th style="text-align:left;"> parameterization </th>
  </tr>
 </thead>
<tbody>
  <tr>
   <td style="text-align:left;"> THETA1 </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
  </tr>
  <tr>
   <td style="text-align:left;"> THETA2 </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
  </tr>
  <tr>
   <td style="text-align:left;"> THETA3 </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
  </tr>
</tbody>
</table>

## Omega Sources

<table class="table table-striped">
 <thead>
  <tr>
   <th style="text-align:left;"> parameter </th>
   <th style="text-align:left;"> name </th>
   <th style="text-align:left;"> display </th>
   <th style="text-align:left;"> description </th>
   <th style="text-align:left;"> parameterization </th>
   <th style="text-align:left;"> associated_theta </th>
  </tr>
 </thead>
<tbody>
  <tr>
   <td style="text-align:left;"> OMEGA(1,1) </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
  </tr>
  <tr>
   <td style="text-align:left;"> OMEGA(2,1) </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
  </tr>
  <tr>
   <td style="text-align:left;"> OMEGA(2,2) </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
  </tr>
  <tr>
   <td style="text-align:left;"> OMEGA(3,3) </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
  </tr>
</tbody>
</table>

## Sigma Sources

<table class="table table-striped">
 <thead>
  <tr>
   <th style="text-align:left;"> parameter </th>
   <th style="text-align:left;"> name </th>
   <th style="text-align:left;"> display </th>
   <th style="text-align:left;"> description </th>
   <th style="text-align:left;"> parameterization </th>
  </tr>
 </thead>
<tbody>
  <tr>
   <td style="text-align:left;"> SIGMA(1,1) </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> hard-coded </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> hard-coded </td>
  </tr>
  <tr>
   <td style="text-align:left;"> SIGMA(2,2) </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> hard-coded </td>
   <td style="text-align:left;"> default </td>
   <td style="text-align:left;"> hard-coded </td>
  </tr>
</tbody>
</table>

:::
:::



Adding lookup values updates sources as well



::: {.cell}

```{.r .cell-code}
info <- apply_lookup(info, normalizePath("../inst/lookup.yaml"))

audit_parameter_info(info)
```

::: {.cell-output-display}
# Parameter Info Audit

## Theta Sources

<table class="table table-striped">
 <thead>
  <tr>
   <th style="text-align:left;"> parameter </th>
   <th style="text-align:left;"> name </th>
   <th style="text-align:left;"> display </th>
   <th style="text-align:left;"> description </th>
   <th style="text-align:left;"> unit </th>
   <th style="text-align:left;"> parameterization </th>
  </tr>
 </thead>
<tbody>
  <tr>
   <td style="text-align:left;"> THETA1 </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> ../inst/lookup.yaml </td>
   <td style="text-align:left;"> ../inst/lookup.yaml </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
  </tr>
  <tr>
   <td style="text-align:left;"> THETA2 </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> ../inst/lookup.yaml </td>
   <td style="text-align:left;"> ../inst/lookup.yaml </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
  </tr>
  <tr>
   <td style="text-align:left;"> THETA3 </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> ../inst/lookup.yaml </td>
   <td style="text-align:left;"> ../inst/lookup.yaml </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> default </td>
  </tr>
</tbody>
</table>

## Omega Sources

<table class="table table-striped">
 <thead>
  <tr>
   <th style="text-align:left;"> parameter </th>
   <th style="text-align:left;"> name </th>
   <th style="text-align:left;"> display </th>
   <th style="text-align:left;"> description </th>
   <th style="text-align:left;"> parameterization </th>
   <th style="text-align:left;"> associated_theta </th>
  </tr>
 </thead>
<tbody>
  <tr>
   <td style="text-align:left;"> OMEGA(1,1) </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> ../inst/lookup.yaml </td>
   <td style="text-align:left;"> ../inst/lookup.yaml </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
  </tr>
  <tr>
   <td style="text-align:left;"> OMEGA(2,1) </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> ../inst/lookup.yaml </td>
   <td style="text-align:left;"> ../inst/lookup.yaml </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
  </tr>
  <tr>
   <td style="text-align:left;"> OMEGA(2,2) </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> ../inst/lookup.yaml </td>
   <td style="text-align:left;"> ../inst/lookup.yaml </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
  </tr>
  <tr>
   <td style="text-align:left;"> OMEGA(3,3) </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> ../inst/lookup.yaml </td>
   <td style="text-align:left;"> ../inst/lookup.yaml </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
  </tr>
</tbody>
</table>

## Sigma Sources

<table class="table table-striped">
 <thead>
  <tr>
   <th style="text-align:left;"> parameter </th>
   <th style="text-align:left;"> name </th>
   <th style="text-align:left;"> display </th>
   <th style="text-align:left;"> description </th>
   <th style="text-align:left;"> parameterization </th>
  </tr>
 </thead>
<tbody>
  <tr>
   <td style="text-align:left;"> SIGMA(1,1) </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> hard-coded </td>
   <td style="text-align:left;"> ../inst/lookup.yaml </td>
   <td style="text-align:left;"> hard-coded </td>
  </tr>
  <tr>
   <td style="text-align:left;"> SIGMA(2,2) </td>
   <td style="text-align:left;"> test_data/models/onecmt/run003 </td>
   <td style="text-align:left;"> hard-coded </td>
   <td style="text-align:left;"> ../inst/lookup.yaml </td>
   <td style="text-align:left;"> hard-coded </td>
  </tr>
</tbody>
</table>

:::
:::

::: {.cell}

```{.r .cell-code}
info
```

::: {.cell-output-display}
# Model Parameter Info

## Theta Parameters

<table class="table table-striped">
 <thead>
  <tr>
   <th style="text-align:left;"> parameter </th>
   <th style="text-align:left;"> name </th>
   <th style="text-align:left;"> display </th>
   <th style="text-align:left;"> description </th>
   <th style="text-align:left;"> unit </th>
   <th style="text-align:left;"> parameterization </th>
  </tr>
 </thead>
<tbody>
  <tr>
   <td style="text-align:left;"> THETA1 </td>
   <td style="text-align:left;"> TVCL </td>
   <td style="text-align:left;"> CL </td>
   <td style="text-align:left;"> Clearance </td>
   <td style="text-align:left;"> L/hr </td>
   <td style="text-align:left;"> NA </td>
  </tr>
  <tr>
   <td style="text-align:left;"> THETA2 </td>
   <td style="text-align:left;"> TVV </td>
   <td style="text-align:left;"> Vc </td>
   <td style="text-align:left;"> Central Volume </td>
   <td style="text-align:left;"> L </td>
   <td style="text-align:left;"> NA </td>
  </tr>
  <tr>
   <td style="text-align:left;"> THETA3 </td>
   <td style="text-align:left;"> TVKA </td>
   <td style="text-align:left;"> KA </td>
   <td style="text-align:left;"> Absorption Rate Constant </td>
   <td style="text-align:left;"> 1/hr </td>
   <td style="text-align:left;"> NA </td>
  </tr>
</tbody>
</table>

## Omega Parameters

<table class="table table-striped">
 <thead>
  <tr>
   <th style="text-align:left;"> parameter </th>
   <th style="text-align:left;"> name </th>
   <th style="text-align:left;"> display </th>
   <th style="text-align:left;"> description </th>
   <th style="text-align:left;"> parameterization </th>
   <th style="text-align:left;"> associated_theta </th>
  </tr>
 </thead>
<tbody>
  <tr>
   <td style="text-align:left;"> OMEGA(1,1) </td>
   <td style="text-align:left;"> OM1 </td>
   <td style="text-align:left;"> IIV-CL </td>
   <td style="text-align:left;"> Interindividual variability on CL </td>
   <td style="text-align:left;"> LogNormal </td>
   <td style="text-align:left;"> TVCL </td>
  </tr>
  <tr>
   <td style="text-align:left;"> OMEGA(2,1) </td>
   <td style="text-align:left;"> OM1,2 </td>
   <td style="text-align:left;"> IIV-CL-Vc </td>
   <td style="text-align:left;"> Interindividual covariance for CL-Vc </td>
   <td style="text-align:left;"> LogNormal </td>
   <td style="text-align:left;"> TVCL, TVV </td>
  </tr>
  <tr>
   <td style="text-align:left;"> OMEGA(2,2) </td>
   <td style="text-align:left;"> OM2 </td>
   <td style="text-align:left;"> IIV-Vc </td>
   <td style="text-align:left;"> Interindividual variability on Vc </td>
   <td style="text-align:left;"> LogNormal </td>
   <td style="text-align:left;"> TVV </td>
  </tr>
  <tr>
   <td style="text-align:left;"> OMEGA(3,3) </td>
   <td style="text-align:left;"> OM3 </td>
   <td style="text-align:left;"> IIV-KA </td>
   <td style="text-align:left;"> Interindividual variability on KA </td>
   <td style="text-align:left;"> LogNormal </td>
   <td style="text-align:left;"> TVKA </td>
  </tr>
</tbody>
</table>

## Sigma Parameters

<table class="table table-striped">
 <thead>
  <tr>
   <th style="text-align:left;"> parameter </th>
   <th style="text-align:left;"> name </th>
   <th style="text-align:left;"> display </th>
   <th style="text-align:left;"> description </th>
   <th style="text-align:left;"> parameterization </th>
  </tr>
 </thead>
<tbody>
  <tr>
   <td style="text-align:left;"> SIGMA(1,1) </td>
   <td style="text-align:left;"> SIG1 </td>
   <td style="text-align:left;"> Additive Error </td>
   <td style="text-align:left;"> Proportional Error </td>
   <td style="text-align:left;"> AddErr </td>
  </tr>
  <tr>
   <td style="text-align:left;"> SIGMA(2,2) </td>
   <td style="text-align:left;"> SIG2 </td>
   <td style="text-align:left;"> Proportional Error </td>
   <td style="text-align:left;"> Additive Error </td>
   <td style="text-align:left;"> Proportional </td>
  </tr>
</tbody>
</table>

:::
:::



# Transformation Reference

The following table shows how CV, RSE, and CI are computed for each transform and parameter type combination.



::: {.cell}
::: {.cell-output-display}

```{=html}
<div id="reozaldnbk" style="padding-left:0px;padding-right:0px;padding-top:10px;padding-bottom:10px;overflow-x:auto;overflow-y:auto;width:auto;height:auto;">
<style>#reozaldnbk table {
  font-family: system-ui, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji';
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

#reozaldnbk thead, #reozaldnbk tbody, #reozaldnbk tfoot, #reozaldnbk tr, #reozaldnbk td, #reozaldnbk th {
  border-style: none;
}

#reozaldnbk p {
  margin: 0;
  padding: 0;
}

#reozaldnbk .gt_table {
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

#reozaldnbk .gt_caption {
  padding-top: 4px;
  padding-bottom: 4px;
}

#reozaldnbk .gt_title {
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

#reozaldnbk .gt_subtitle {
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

#reozaldnbk .gt_heading {
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

#reozaldnbk .gt_bottom_border {
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#reozaldnbk .gt_col_headings {
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

#reozaldnbk .gt_col_heading {
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

#reozaldnbk .gt_column_spanner_outer {
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

#reozaldnbk .gt_column_spanner_outer:first-child {
  padding-left: 0;
}

#reozaldnbk .gt_column_spanner_outer:last-child {
  padding-right: 0;
}

#reozaldnbk .gt_column_spanner {
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

#reozaldnbk .gt_spanner_row {
  border-bottom-style: hidden;
}

#reozaldnbk .gt_group_heading {
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

#reozaldnbk .gt_empty_group_heading {
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

#reozaldnbk .gt_from_md > :first-child {
  margin-top: 0;
}

#reozaldnbk .gt_from_md > :last-child {
  margin-bottom: 0;
}

#reozaldnbk .gt_row {
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

#reozaldnbk .gt_stub {
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

#reozaldnbk .gt_stub_row_group {
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

#reozaldnbk .gt_row_group_first td {
  border-top-width: 2px;
}

#reozaldnbk .gt_row_group_first th {
  border-top-width: 2px;
}

#reozaldnbk .gt_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#reozaldnbk .gt_first_summary_row {
  border-top-style: solid;
  border-top-color: #D3D3D3;
}

#reozaldnbk .gt_first_summary_row.thick {
  border-top-width: 2px;
}

#reozaldnbk .gt_last_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#reozaldnbk .gt_grand_summary_row {
  color: #333333;
  background-color: #FFFFFF;
  text-transform: inherit;
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
}

#reozaldnbk .gt_first_grand_summary_row {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-top-style: double;
  border-top-width: 6px;
  border-top-color: #D3D3D3;
}

#reozaldnbk .gt_last_grand_summary_row_top {
  padding-top: 8px;
  padding-bottom: 8px;
  padding-left: 5px;
  padding-right: 5px;
  border-bottom-style: double;
  border-bottom-width: 6px;
  border-bottom-color: #D3D3D3;
}

#reozaldnbk .gt_striped {
  background-color: rgba(128, 128, 128, 0.05);
}

#reozaldnbk .gt_table_body {
  border-top-style: solid;
  border-top-width: 2px;
  border-top-color: #D3D3D3;
  border-bottom-style: solid;
  border-bottom-width: 2px;
  border-bottom-color: #D3D3D3;
}

#reozaldnbk .gt_footnotes {
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

#reozaldnbk .gt_footnote {
  margin: 0px;
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#reozaldnbk .gt_sourcenotes {
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

#reozaldnbk .gt_sourcenote {
  font-size: 90%;
  padding-top: 4px;
  padding-bottom: 4px;
  padding-left: 5px;
  padding-right: 5px;
}

#reozaldnbk .gt_left {
  text-align: left;
}

#reozaldnbk .gt_center {
  text-align: center;
}

#reozaldnbk .gt_right {
  text-align: right;
  font-variant-numeric: tabular-nums;
}

#reozaldnbk .gt_font_normal {
  font-weight: normal;
}

#reozaldnbk .gt_font_bold {
  font-weight: bold;
}

#reozaldnbk .gt_font_italic {
  font-style: italic;
}

#reozaldnbk .gt_super {
  font-size: 65%;
}

#reozaldnbk .gt_footnote_marks {
  font-size: 75%;
  vertical-align: 0.4em;
  position: initial;
}

#reozaldnbk .gt_asterisk {
  font-size: 100%;
  vertical-align: 0;
}

#reozaldnbk .gt_indent_1 {
  text-indent: 5px;
}

#reozaldnbk .gt_indent_2 {
  text-indent: 10px;
}

#reozaldnbk .gt_indent_3 {
  text-indent: 15px;
}

#reozaldnbk .gt_indent_4 {
  text-indent: 20px;
}

#reozaldnbk .gt_indent_5 {
  text-indent: 25px;
}

#reozaldnbk .katex-display {
  display: inline-flex !important;
  margin-bottom: 0.75em !important;
}

#reozaldnbk div.Reactable > div.rt-table > div.rt-thead > div.rt-tr.rt-tr-group-header > div.rt-th-group:after {
  height: 0px !important;
}
</style>
<table class="gt_table" data-quarto-disable-processing="false" data-quarto-bootstrap="false">
  <thead>
    <tr class="gt_heading">
      <td colspan="5" class="gt_heading gt_title gt_font_normal gt_bottom_border" style="font-weight: bold;">Transformation Formulas</td>
    </tr>
    
    <tr class="gt_col_headings">
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" scope="col" id="a::stub"></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="parameter">Parameter</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="cv">CV</th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="rse">RSE<span class="gt_footnote_marks" style="white-space:nowrap;font-style:italic;font-weight:normal;line-height:0;"><sup>1</sup></span></th>
      <th class="gt_col_heading gt_columns_bottom_border gt_left" rowspan="1" colspan="1" style="font-weight: bold;" scope="col" id="ci">CI<span class="gt_footnote_marks" style="white-space:nowrap;font-style:italic;font-weight:normal;line-height:0;"><sup>2</sup></span></th>
    </tr>
  </thead>
  <tbody class="gt_table_body">
    <tr class="gt_row_group_first"><td headers="Identity stub_2_1 stub_1" rowspan="3" class="gt_row gt_left gt_stub_row_group" style="font-weight: bold;">Identity</td>
<td headers="Identity stub_2_1 parameter" class="gt_row gt_left"><span data-qmd-base64="VGhldGE="><span class='gt_from_md'>Theta</span></span></td>
<td headers="Identity stub_2_1 cv" class="gt_row gt_left"><span data-qmd-base64="Ti9B"><span class='gt_from_md'>N/A</span></span></td>
<td headers="Identity stub_2_1 rse" class="gt_row gt_left"></td>
<td headers="Identity stub_2_1 ci" class="gt_row gt_left"></td></tr>
    <tr><td headers="Identity stub_2_2 parameter" class="gt_row gt_left"><span data-qmd-base64="T21lZ2E="><span class='gt_from_md'>Omega</span></span></td>
<td headers="Identity stub_2_2 cv" class="gt_row gt_left"><span data-qmd-base64="Ti9B"><span class='gt_from_md'>N/A</span></span></td>
<td headers="Identity stub_2_2 rse" class="gt_row gt_left"></td>
<td headers="Identity stub_2_2 ci" class="gt_row gt_left"></td></tr>
    <tr><td headers="Identity stub_2_3 parameter" class="gt_row gt_left"><span data-qmd-base64="U2lnbWE="><span class='gt_from_md'>Sigma</span></span></td>
<td headers="Identity stub_2_3 cv" class="gt_row gt_left"><span data-qmd-base64="Ti9B"><span class='gt_from_md'>N/A</span></span></td>
<td headers="Identity stub_2_3 rse" class="gt_row gt_left"></td>
<td headers="Identity stub_2_3 ci" class="gt_row gt_left"></td></tr>
    <tr class="gt_row_group_first"><td headers="LogNormal stub_2_4 stub_1" rowspan="3" class="gt_row gt_left gt_stub_row_group" style="font-weight: bold;">LogNormal</td>
<td headers="LogNormal stub_2_4 parameter" class="gt_row gt_left"><span data-qmd-base64="VGhldGE="><span class='gt_from_md'>Theta</span></span></td>
<td headers="LogNormal stub_2_4 cv" class="gt_row gt_left"><span data-qmd-base64="Ti9B"><span class='gt_from_md'>N/A</span></span></td>
<td headers="LogNormal stub_2_4 rse" class="gt_row gt_left"><span data-qmd-base64="JFxzcXJ0e1xleHAoXHRleHR7U0V9XjIpIC0gMX0k"><span class='gt_from_md'>\(\sqrt{\exp(\text{SE}^2) - 1}\)</span></span></td>
<td headers="LogNormal stub_2_4 ci" class="gt_row gt_left"><span data-qmd-base64="JFxleHAoXHRleHR7RXN0fSBccG0geiBcY2RvdCBcdGV4dHtTRX0pJA=="><span class='gt_from_md'>\(\exp(\text{Est} \pm z \cdot \text{SE})\)</span></span></td></tr>
    <tr><td headers="LogNormal stub_2_5 parameter" class="gt_row gt_left"><span data-qmd-base64="T21lZ2E="><span class='gt_from_md'>Omega</span></span></td>
<td headers="LogNormal stub_2_5 cv" class="gt_row gt_left"><span data-qmd-base64="JFxzcXJ0e1xleHAoXHRleHR7RXN0fSkgLSAxfSQ="><span class='gt_from_md'>\(\sqrt{\exp(\text{Est}) - 1}\)</span></span></td>
<td headers="LogNormal stub_2_5 rse" class="gt_row gt_left"></td>
<td headers="LogNormal stub_2_5 ci" class="gt_row gt_left"><span data-qmd-base64="JFxleHAoXHRleHR7RXN0fSBccG0geiBcY2RvdCBcdGV4dHtTRX0pJA=="><span class='gt_from_md'>\(\exp(\text{Est} \pm z \cdot \text{SE})\)</span></span></td></tr>
    <tr><td headers="LogNormal stub_2_6 parameter" class="gt_row gt_left"><span data-qmd-base64="U2lnbWE="><span class='gt_from_md'>Sigma</span></span></td>
<td headers="LogNormal stub_2_6 cv" class="gt_row gt_left"><span data-qmd-base64="JFxzcXJ0e1xleHAoXHRleHR7RXN0fSkgLSAxfSQ="><span class='gt_from_md'>\(\sqrt{\exp(\text{Est}) - 1}\)</span></span></td>
<td headers="LogNormal stub_2_6 rse" class="gt_row gt_left"></td>
<td headers="LogNormal stub_2_6 ci" class="gt_row gt_left"><span data-qmd-base64="JFxleHAoXHRleHR7RXN0fSBccG0geiBcY2RvdCBcdGV4dHtTRX0pJA=="><span class='gt_from_md'>\(\exp(\text{Est} \pm z \cdot \text{SE})\)</span></span></td></tr>
    <tr class="gt_row_group_first"><td headers="Logit stub_2_7 stub_1" rowspan="3" class="gt_row gt_left gt_stub_row_group" style="font-weight: bold;">Logit</td>
<td headers="Logit stub_2_7 parameter" class="gt_row gt_left"><span data-qmd-base64="VGhldGE="><span class='gt_from_md'>Theta</span></span></td>
<td headers="Logit stub_2_7 cv" class="gt_row gt_left"><span data-qmd-base64="Ti9B"><span class='gt_from_md'>N/A</span></span></td>
<td headers="Logit stub_2_7 rse" class="gt_row gt_left"><span data-qmd-base64="JCgxIC0gXHRleHR7QlR9KSBcY2RvdCBcdGV4dHtTRX0k"><span class='gt_from_md'>\((1 - \text{BT}) \cdot \text{SE}\)</span></span></td>
<td headers="Logit stub_2_7 ci" class="gt_row gt_left"><span data-qmd-base64="JFxmcmFjezF9ezEgKyBcZXhwKC0oXHRleHR7RXN0fSBccG0geiBcY2RvdCBcdGV4dHtTRX0pKX0k"><span class='gt_from_md'>\(\frac{1}{1 + \exp(-(\text{Est} \pm z \cdot \text{SE}))}\)</span></span></td></tr>
    <tr><td headers="Logit stub_2_8 parameter" class="gt_row gt_left"><span data-qmd-base64="T21lZ2E="><span class='gt_from_md'>Omega</span></span></td>
<td headers="Logit stub_2_8 cv" class="gt_row gt_left"><span data-qmd-base64="Ti9B"><span class='gt_from_md'>N/A</span></span></td>
<td headers="Logit stub_2_8 rse" class="gt_row gt_left"></td>
<td headers="Logit stub_2_8 ci" class="gt_row gt_left"><span data-qmd-base64="JFxmcmFjezF9ezEgKyBcZXhwKC0oXHRleHR7RXN0fSBccG0geiBcY2RvdCBcdGV4dHtTRX0pKX0k"><span class='gt_from_md'>\(\frac{1}{1 + \exp(-(\text{Est} \pm z \cdot \text{SE}))}\)</span></span></td></tr>
    <tr><td headers="Logit stub_2_9 parameter" class="gt_row gt_left"><span data-qmd-base64="U2lnbWE="><span class='gt_from_md'>Sigma</span></span></td>
<td headers="Logit stub_2_9 cv" class="gt_row gt_left"><span data-qmd-base64="Ti9B"><span class='gt_from_md'>N/A</span></span></td>
<td headers="Logit stub_2_9 rse" class="gt_row gt_left"></td>
<td headers="Logit stub_2_9 ci" class="gt_row gt_left"><span data-qmd-base64="JFxmcmFjezF9ezEgKyBcZXhwKC0oXHRleHR7RXN0fSBccG0geiBcY2RvdCBcdGV4dHtTRX0pKX0k"><span class='gt_from_md'>\(\frac{1}{1 + \exp(-(\text{Est} \pm z \cdot \text{SE}))}\)</span></span></td></tr>
    <tr class="gt_row_group_first"><td headers="Proportional stub_2_10 stub_1" rowspan="3" class="gt_row gt_left gt_stub_row_group" style="font-weight: bold;">Proportional</td>
<td headers="Proportional stub_2_10 parameter" class="gt_row gt_left"><span data-qmd-base64="VGhldGE="><span class='gt_from_md'>Theta</span></span></td>
<td headers="Proportional stub_2_10 cv" class="gt_row gt_left"><span data-qmd-base64="Ti9B"><span class='gt_from_md'>N/A</span></span></td>
<td headers="Proportional stub_2_10 rse" class="gt_row gt_left"></td>
<td headers="Proportional stub_2_10 ci" class="gt_row gt_left"></td></tr>
    <tr><td headers="Proportional stub_2_11 parameter" class="gt_row gt_left"><span data-qmd-base64="T21lZ2E="><span class='gt_from_md'>Omega</span></span></td>
<td headers="Proportional stub_2_11 cv" class="gt_row gt_left"><span data-qmd-base64="JFxzcXJ0e1x0ZXh0e0VzdH19JA=="><span class='gt_from_md'>\(\sqrt{\text{Est}}\)</span></span></td>
<td headers="Proportional stub_2_11 rse" class="gt_row gt_left"></td>
<td headers="Proportional stub_2_11 ci" class="gt_row gt_left"></td></tr>
    <tr><td headers="Proportional stub_2_12 parameter" class="gt_row gt_left"><span data-qmd-base64="U2lnbWE="><span class='gt_from_md'>Sigma</span></span></td>
<td headers="Proportional stub_2_12 cv" class="gt_row gt_left"><span data-qmd-base64="JFxzcXJ0e1x0ZXh0e0VzdH19JA=="><span class='gt_from_md'>\(\sqrt{\text{Est}}\)</span></span></td>
<td headers="Proportional stub_2_12 rse" class="gt_row gt_left"></td>
<td headers="Proportional stub_2_12 ci" class="gt_row gt_left"></td></tr>
    <tr class="gt_row_group_first"><td headers="AddErr stub_2_13 stub_1" rowspan="3" class="gt_row gt_left gt_stub_row_group" style="font-weight: bold;">AddErr</td>
<td headers="AddErr stub_2_13 parameter" class="gt_row gt_left"><span data-qmd-base64="VGhldGE="><span class='gt_from_md'>Theta</span></span></td>
<td headers="AddErr stub_2_13 cv" class="gt_row gt_left"><span data-qmd-base64="Ti9B"><span class='gt_from_md'>N/A</span></span></td>
<td headers="AddErr stub_2_13 rse" class="gt_row gt_left"></td>
<td headers="AddErr stub_2_13 ci" class="gt_row gt_left"></td></tr>
    <tr><td headers="AddErr stub_2_14 parameter" class="gt_row gt_left"><span data-qmd-base64="T21lZ2E="><span class='gt_from_md'>Omega</span></span></td>
<td headers="AddErr stub_2_14 cv" class="gt_row gt_left"><span data-qmd-base64="Ti9B"><span class='gt_from_md'>N/A</span></span></td>
<td headers="AddErr stub_2_14 rse" class="gt_row gt_left"></td>
<td headers="AddErr stub_2_14 ci" class="gt_row gt_left"></td></tr>
    <tr><td headers="AddErr stub_2_15 parameter" class="gt_row gt_left"><span data-qmd-base64="U2lnbWE="><span class='gt_from_md'>Sigma</span></span></td>
<td headers="AddErr stub_2_15 cv" class="gt_row gt_left"><span data-qmd-base64="Ti9B"><span class='gt_from_md'>N/A</span></span></td>
<td headers="AddErr stub_2_15 rse" class="gt_row gt_left"></td>
<td headers="AddErr stub_2_15 ci" class="gt_row gt_left"></td></tr>
    <tr class="gt_row_group_first"><td headers="LogAddErr stub_2_16 stub_1" rowspan="3" class="gt_row gt_left gt_stub_row_group" style="font-weight: bold;">LogAddErr</td>
<td headers="LogAddErr stub_2_16 parameter" class="gt_row gt_left"><span data-qmd-base64="VGhldGE="><span class='gt_from_md'>Theta</span></span></td>
<td headers="LogAddErr stub_2_16 cv" class="gt_row gt_left"><span data-qmd-base64="JFxzcXJ0e1xleHAoXHRleHR7RXN0fV4yKSAtIDF9JA=="><span class='gt_from_md'>\(\sqrt{\exp(\text{Est}^2) - 1}\)</span></span></td>
<td headers="LogAddErr stub_2_16 rse" class="gt_row gt_left"></td>
<td headers="LogAddErr stub_2_16 ci" class="gt_row gt_left"></td></tr>
    <tr><td headers="LogAddErr stub_2_17 parameter" class="gt_row gt_left"><span data-qmd-base64="T21lZ2E="><span class='gt_from_md'>Omega</span></span></td>
<td headers="LogAddErr stub_2_17 cv" class="gt_row gt_left"><span data-qmd-base64="Ti9B"><span class='gt_from_md'>N/A</span></span></td>
<td headers="LogAddErr stub_2_17 rse" class="gt_row gt_left"></td>
<td headers="LogAddErr stub_2_17 ci" class="gt_row gt_left"></td></tr>
    <tr><td headers="LogAddErr stub_2_18 parameter" class="gt_row gt_left"><span data-qmd-base64="U2lnbWE="><span class='gt_from_md'>Sigma</span></span></td>
<td headers="LogAddErr stub_2_18 cv" class="gt_row gt_left"><span data-qmd-base64="JFxzcXJ0e1xleHAoXHRleHR7RXN0fSkgLSAxfSQ="><span class='gt_from_md'>\(\sqrt{\exp(\text{Est}) - 1}\)</span></span></td>
<td headers="LogAddErr stub_2_18 rse" class="gt_row gt_left"></td>
<td headers="LogAddErr stub_2_18 ci" class="gt_row gt_left"></td></tr>
  </tbody>
  <tfoot>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="5"> Est = THETA(x)/OMEGA(i,j)/SIGMA(i,j) reported in the .ext file</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="5"> BT = back-transformed estimate = 1/(1 + exp(-Est))</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="5"> SE = Standard Error, z = z-score for CI level</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="5"> CV and RSE formulas are multiplied by 100 to express as percentages</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="5"><span class="gt_footnote_marks" style="white-space:nowrap;font-style:italic;font-weight:normal;line-height:0;"><sup>1</sup></span> SE/|Est| unless otherwise noted</td>
    </tr>
    <tr class="gt_footnotes">
      <td class="gt_footnote" colspan="5"><span class="gt_footnote_marks" style="white-space:nowrap;font-style:italic;font-weight:normal;line-height:0;"><sup>2</sup></span> Est ± z·SE unless otherwise noted</td>
    </tr>
  </tfoot>
</table>
</div>
```

:::
:::

