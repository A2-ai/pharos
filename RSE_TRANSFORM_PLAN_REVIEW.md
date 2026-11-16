# Review: transform-aware RSE plan

Reviewed against the current codebase on 2026-10-11. This is a design review of
[RSE_TRANSFORM_PLAN.md](RSE_TRANSFORM_PLAN.md), not an implementation or a test report.

## Recommended decisions

1. Separate raw estimate extraction from summary annotation: option 1(b).
2. Error on unknown THETA columns when model declarations are known; skip undeclared OMEGA/SIGMA cells.
3. Preserve the existing summary JSON and its label-only `parameter_names` map.
4. Keep `$MSFI → None` for now; defer a richer representation until a representative run demonstrates the need.

## 1. Raw parameter identity and display labels are currently conflated

**Finding:** `extract_parameters_from_table` overwrites
`ThetaEstimate.name` and `RandomEffectEstimate.name` with parsed labels.
Meanwhile, `copy::read_estimates` uses those same fields as keys for
`update_initial_estimates`. Copy works because it passes no label map.

References:

- [Estimate extraction and renaming](components/nonmem/src/output_files/ext.rs)
- [Copy estimate reading and model updating](components/nonmem/src/copy.rs)

**Recommendation:** Make raw estimates retain NONMEM names such as `THETA1`
and `OMEGA(2,1)`. Construct the summary's annotated parameter table in a separate
step, applying labels, transforms, RSE, shrinkage, and display filtering there.

Use a distinct internal raw representation, or the existing raw table where
practical, rather than allowing the same estimate object to change the meaning
of its name. The existing serialized estimate structs can remain summary output
types, preserving their current display-name behavior without adding a JSON
`label` field.

Membership validation and filtering must be reusable by both summary and copy.
Moving annotation out of `ext` must not cause copy to retain phantom parameters
or check their NaN values before filtering.

**Tradeoff:** This is a larger change than threading the metadata map through
the existing extraction function. It resolves an existing dependency between
copy correctness and the absence of labels, and gives transform application a
clear home.

## 2. Summary annotation should happen after final-table selection

**Finding:** `get_summary` collects minimization results from every selected
estimation table but takes parameter estimates only from the last one.
Currently, every table receives labels, RSE, and shrinkage during extraction.

Reference: [Summary assembly](components/nonmem/src/output_files/mod.rs).

**Recommendation:** Retain raw results and minimization metadata while resolving
estimation files and table order, then annotate the final parameter table once.

**Important constraint:** Preserve the existing shrinkage matching by estimation
method and occurrence. `get_estimation_results` explicitly handles repeated
methods, such as ITS → IMP → IMP. Moving the join must not accidentally select
the first shrinkage block for the final occurrence. Carry the matched shrinkage
association through selection or reproduce the current association deliberately.

## 3. Unknown THETAs should not be silently discarded

**Finding:** Current declaration filtering only governs OMEGA/SIGMA. Replacing it
with unconditional map membership would newly drop unexpected THETAs.

A mismatch can indicate an edited control stream or mismatched output.
`ModelLayout::from_output_dir` resolves the model using the recorded run name;
it does not verify that the model contents are those used for the run.

Reference: [Output-directory model resolution](components/nonmem/src/model_resolution.rs).

**Recommendation:**

- With known declarations, return an error for an unexpected THETA.
- Include the THETA key, model path, and output path in the diagnostic.
- Skip undeclared OMEGA/SIGMA cells, as needed for placeholders and cells between separate blocks.
- With unknown declarations (`None`, including MSFI), retain parameters without membership rejection.
- Share this policy between summary and copy so neither silently produces partial results.

**Validation gap:** Prior-model control streams exist, including
[the nmexample model](components/nonmem/models/nmexample/nmexample.mod), but no
matching prior-model run-output fixture was found in this review.
The parser explicitly distinguishes `$THETAP` from `$THETA`; that alone does
not establish which THETA columns a real prior-model run emits.
Add representative output coverage before considering strict matching fully verified.

## 4. The proposed JSON replacement has an existing-field collision

**Finding:** `Summary` already contains both:

- `parameters: TableParameters`, containing estimates;
- `parameter_names: BTreeMap<String, Option<String>>`, containing labels.

The plan's proposed replacement named `parameters` would collide with the
existing estimates field. The CLI directly serializes `Summary`, so changing
these fields affects the external JSON contract.

References:

- [Summary definition](components/nonmem/src/output_files/mod.rs)
- [CLI JSON output](src/main.rs)

**Recommendation:** Preserve `parameter_names` as a label-only projection of
the internal metadata map. Under unknown declarations, emit an empty map.
Do not expose transform metadata as part of this bug fix.

If downstream consumers later need transforms, introduce a deliberate schema
extension with an unambiguous name such as `parameter_metadata`.

## 5. MSFI support has insufficient evidence for a richer representation

**Finding:** Existing `declared_random_effects` treats MSFI declarations as
unknown and retains both OMEGA and SIGMA columns. The proposed `None` preserves
that filtering intent.

The synthetic
[everything.mod parser fixture](components/nonmem-parser/test_data/everything.mod)
contains MSFI alongside parameter declarations. It demonstrates parser acceptance,
not a working run with reliable comment-to-output correspondence.

Reference: [Current declaration and label handling](components/nonmem-parser/src/model/parameters.rs).

**Recommendation:** Keep `Option<BTreeMap<...>>` and return `None` for MSFI.
Document that this means keeping all parameters without comment-derived labels
or transforms. Retain the distinction between `None` and `Some(empty)`.

**Compatibility detail:** The current label-building function does not special-case
MSFI. Therefore, returning `None` could remove labels for a control stream that
combines MSFI with commented declarations. That potential behavior change should
be explicit, even though no representative run demonstrating the need was found.

Revisit the richer enum only when a real run establishes both the need and the
correct mapping rules. The codebase alone cannot answer the plan's question about
usage in practice.

## 6. Derive ETA/EPS identity and shrinkage indices from coordinates

**Finding:** `get_random_effect_label` and `get_shrinkage_data` count previously
retained diagonal estimates. This assumes earlier diagonals are always present.
Membership filtering introduces a way to remove individual diagonals.

Reference: [Random-effect labels and shrinkage lookup](components/nonmem/src/output_files/ext.rs).

**Recommendation:** Bring this into scope rather than leaving it as a general
cleanup item. For a diagonal `OMEGA(i,i)` or `SIGMA(i,i)`, derive the ETA/EPS
number from `i` and use the corresponding zero-based shrinkage index.
Validate coordinates before converting to an index.

Test a case where an earlier diagonal is absent or filtered, ensuring a later
parameter keeps its original identity and shrinkage association.

## Suggested verification before merging

- Summary-level tests for log and logit THETAs, including a zero log-scale estimate.
- Default/untagged behavior and unchanged OMEGA/SIGMA RSE behavior.
- Membership independent of comment parsing, including no comment type and invalid tags.
- Unknown THETA errors with useful source context, exercised through summary and copy.
- Phantom random-effect filtering before copy's invalid-estimate check.
- Stable copy keys even when summary labels differ from NONMEM names.
- Repeated estimation methods retaining the correct shrinkage association.
- Coordinate-based ETA/EPS numbering and shrinkage lookup after filtering.
- MSFI retaining all output parameters, plus explicit coverage of its metadata policy.
- A representative prior-model output fixture for strict THETA matching.
- Summary JSON retaining its existing fields and label-only projection.
- Correlation filtering for cells between separate random-effect blocks.
- Reviewed snapshot diffs; do not automatically accept them.

No implementation changes or test executions were performed for this review.
