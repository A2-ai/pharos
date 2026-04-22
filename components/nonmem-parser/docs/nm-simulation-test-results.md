# NONMEM $SIMULATION Test Results

## case01_omitted_alone

`OMITTED` flag alone — accepted (the record is skipped, so the seed-source requirement is waived).

**Input $SIM:**

```
$SIMULATION OMITTED
```

**NONMEM output:**

```
WARNINGS AND ERRORS (IF ANY) FOR PROBLEM    1

 (WARNING  2) NM-TRAN INFERS THAT THE DATA ARE POPULATION.

Note: Analytical 2nd Derivatives are constructed in FSUBS but are never used.
      You may insert $ABBR DERIV2=NO after the first $PROB to save FSUBS construction and compilation time
```

## case02_seed1_zero

seed1 = 0 — accepted as the minimum legal value.

**Input $SIM:**

```
$SIMULATION (0) ONLYSIM SUBPROBLEMS=1
```

**NONMEM output:**

```
WARNINGS AND ERRORS (IF ANY) FOR PROBLEM    1

 (WARNING  2) NM-TRAN INFERS THAT THE DATA ARE POPULATION.

Note: Analytical 2nd Derivatives are constructed in FSUBS but are never used.
      You may insert $ABBR DERIV2=NO after the first $PROB to save FSUBS construction and compilation time
```

## case03_seed1_maximum

seed1 = 2147483647 (i32::MAX) — accepted as the maximum legal value.

**Input $SIM:**

```
$SIMULATION (2147483647) ONLYSIM SUBPROBLEMS=1
```

**NONMEM output:**

```
WARNINGS AND ERRORS (IF ANY) FOR PROBLEM    1

 (WARNING  2) NM-TRAN INFERS THAT THE DATA ARE POPULATION.

Note: Analytical 2nd Derivatives are constructed in FSUBS but are never used.
      You may insert $ABBR DERIV2=NO after the first $PROB to save FSUBS construction and compilation time
```

## case04_seed2_maximum

seed2 = 2147483647 — accepted as the maximum legal seed2 value.

**Input $SIM:**

```
$SIMULATION (1 2147483647) ONLYSIM SUBPROBLEMS=1
```

**NONMEM output:**

```
WARNINGS AND ERRORS (IF ANY) FOR PROBLEM    1

 (WARNING  2) NM-TRAN INFERS THAT THE DATA ARE POPULATION.

Note: Analytical 2nd Derivatives are constructed in FSUBS but are never used.
      You may insert $ABBR DERIV2=NO after the first $PROB to save FSUBS construction and compilation time
```

## case05_multi_group_seeds

Five seed groups with a mix of single-seed `(seed1)` and two-seed `(seed1 seed2)` forms, plus `CLOCKSEED=1` — accepted. Exercises multi-group rendering and varied seed-group arities in one record.

**Input $SIM:**

```
$SIM (804831 123) (1234 4321) (804 831) (621) (0217 123) CLOCKSEED=1 ONLYSIM SUBPROBLEMS=1
```

**NONMEM output:**

```
WARNINGS AND ERRORS (IF ANY) FOR PROBLEM    1

 (WARNING  2) NM-TRAN INFERS THAT THE DATA ARE POPULATION.

Note: Analytical 2nd Derivatives are constructed in FSUBS but are never used.
      You may insert $ABBR DERIV2=NO after the first $PROB to save FSUBS construction and compilation time
```

## case06_onlysim_before_seeds

Same seed content as `case05_multi_group_seeds` but with `ONLYSIM` preceding the seed groups — accepted. Documents that record-option position relative to seed groups is flexible.

**Input $SIM:**

```
$SIM ONLYSIM (804831 123) (1234 4321) (804 831) (621) (0217 123) CLOCKSEED=1 SUBPROBLEMS=1
```

**NONMEM output:**

```
WARNINGS AND ERRORS (IF ANY) FOR PROBLEM    1

 (WARNING  2) NM-TRAN INFERS THAT THE DATA ARE POPULATION.

Note: Analytical 2nd Derivatives are constructed in FSUBS but are never used.
      You may insert $ABBR DERIV2=NO after the first $PROB to save FSUBS construction and compilation time
```

## case07_dist_normal_first_source

Explicit `NORMAL` distribution on the first source — accepted. `NORMAL` is NONMEM's default so this is redundant but not an error. Confirms the first-source-must-be-NORMAL rule is satisfied by both the default and an explicit `NORMAL` token.

**Input $SIM:**

```
$SIMULATION (1 NORMAL) ONLYSIM SUBPROBLEMS=1
```

**NONMEM output:**

```
WARNINGS AND ERRORS (IF ANY) FOR PROBLEM    1

 (WARNING  2) NM-TRAN INFERS THAT THE DATA ARE POPULATION.

Note: Analytical 2nd Derivatives are constructed in FSUBS but are never used.
      You may insert $ABBR DERIV2=NO after the first $PROB to save FSUBS construction and compilation time
```

## case08_dist_uniform_second_source

`UNIFORM` distribution on the second source — accepted. The first-source-must-be-NORMAL rule does not apply to sources beyond the first; `UNIFORM` is legal on the 2nd+ source (compare `reject11_dist_uniform_first_source`).

**Input $SIM:**

```
$SIMULATION (1) (2 UNIFORM) ONLYSIM SUBPROBLEMS=1
```

**NONMEM output:**

```
WARNINGS AND ERRORS (IF ANY) FOR PROBLEM    1

 (WARNING  2) NM-TRAN INFERS THAT THE DATA ARE POPULATION.

Note: Analytical 2nd Derivatives are constructed in FSUBS but are never used.
      You may insert $ABBR DERIV2=NO after the first $PROB to save FSUBS construction and compilation time
```

## case09_seed1_seed2_normal_first_source

First source with all three of seed1, seed2, and explicit `NORMAL` — accepted. Exercises the full single-source syntax `(seed1 seed2 distribution)`.

**Input $SIM:**

```
$SIMULATION (1 2 NORMAL) ONLYSIM SUBPROBLEMS=1
```

**NONMEM output:**

```
WARNINGS AND ERRORS (IF ANY) FOR PROBLEM    1

 (WARNING  2) NM-TRAN INFERS THAT THE DATA ARE POPULATION.

Note: Analytical 2nd Derivatives are constructed in FSUBS but are never used.
      You may insert $ABBR DERIV2=NO after the first $PROB to save FSUBS construction and compilation time
```

## case10_seed1_seed2_uniform_second_source

Second source with seed1, seed2, and explicit `UNIFORM` — accepted. Confirms seed2+distribution combinations work on non-first sources.

**Input $SIM:**

```
$SIMULATION (1) (1 2 UNIFORM) ONLYSIM SUBPROBLEMS=1
```

**NONMEM output:**

```
WARNINGS AND ERRORS (IF ANY) FOR PROBLEM    1

 (WARNING  2) NM-TRAN INFERS THAT THE DATA ARE POPULATION.

Note: Analytical 2nd Derivatives are constructed in FSUBS but are never used.
      You may insert $ABBR DERIV2=NO after the first $PROB to save FSUBS construction and compilation time
```

## reject01_bootstrap_neg1_alone

`BOOTSTRAP=-1` without a `(seed)` group — rejected. **BOOTSTRAP is not a random-number source on its own.**

**Input $SIM:**

```
$SIMULATION BOOTSTRAP=-1 SUBPROBLEMS=1
```

**NONMEM error:**

```
WARNINGS AND ERRORS (IF ANY) FOR PROBLEM    1

 (WARNING  2) NM-TRAN INFERS THAT THE DATA ARE POPULATION.

 AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.

  111  $SIMULATE: 1-10 RANDOM NUMBER SOURCES MUST BE SPECIFIED.
```

## reject02_bootstrap_zero_alone

`BOOTSTRAP=0` without a `(seed)` group — rejected (same as `reject01`; BOOTSTRAP value is irrelevant for the seed-source check).

**Input $SIM:**

```
$SIMULATION BOOTSTRAP=0 SUBPROBLEMS=1
```

**NONMEM error:**

```
WARNINGS AND ERRORS (IF ANY) FOR PROBLEM    1

 (WARNING  2) NM-TRAN INFERS THAT THE DATA ARE POPULATION.

 AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.

  111  $SIMULATE: 1-10 RANDOM NUMBER SOURCES MUST BE SPECIFIED.
```

## reject03_clockseed_zero_alone

CLOCKSEED=0 without a `(seed)` group — rejected. CLOCKSEED is a modifier on an existing seed group, never a source on its own.

**Input $SIM:**

```
$SIM CLOCKSEED=0 ONLYSIM SUBPROBLEMS=1
```

**NONMEM error:**

```
WARNINGS AND ERRORS (IF ANY) FOR PROBLEM    1

 (WARNING  2) NM-TRAN INFERS THAT THE DATA ARE POPULATION.

 AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.

  111  $SIMULATE: 1-10 RANDOM NUMBER SOURCES MUST BE SPECIFIED.
```

## reject04_clockseed_one_alone

CLOCKSEED=1 without a `(seed)` group — rejected. Despite `CLOCKSEED=1` meaning "use the system clock," it still requires an underlying `(seed)` group that it then overrides.

**Input $SIM:**

```
$SIM CLOCKSEED=1 ONLYSIM SUBPROBLEMS=1
```

**NONMEM error:**

```
WARNINGS AND ERRORS (IF ANY) FOR PROBLEM    1

 (WARNING  2) NM-TRAN INFERS THAT THE DATA ARE POPULATION.

 AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.

  111  $SIMULATE: 1-10 RANDOM NUMBER SOURCES MUST BE SPECIFIED.
```

## reject05_clockseed_bare

`CLOCKSEED` with no value — rejected (NONMEM requires an integer value on this option).

**Input $SIM:**

```
$SIMULATION (20260420) ONLYSIM SUBPROBLEMS=1 CLOCKSEED
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.

AN ERROR WAS FOUND ON LINE 93 AT THE APPROXIMATE POSITION NOTED:
 $SIMULATION (20260420) ONLYSIM SUBPROBLEMS=1 CLOCKSEED
 X
   34  INTEGER VALUE IS REQUIRED FOR THIS OPTION.
```

## reject06_clockseed_out_of_range

CLOCKSEED value other than 0 or 1 — rejected (CLOCKSEED is a binary on/off switch).

**Input $SIM:**

```
$SIM (804831) CLOCKSEED=2 ONLYSIM SUBPROBLEMS=1
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.

AN ERROR WAS FOUND ON LINE 86 AT THE APPROXIMATE POSITION NOTED:
  $SIM (804831) CLOCKSEED=2 ONLYSIM SUBPROBLEMS=1
                  X
 THE CHARACTERS IN ERROR ARE: CLOCKSEED
   35  VALUE IS TOO LARGE OR UNACCEPTABLE FOR THIS OPTION.
```

## reject07_seed1_minus_one_first_problem

seed1 = -1 on the first (or only) `$PROBLEM` — rejected. `-1` is legal only as a continuation sentinel referencing a prior problem's random stream.

**Input $SIM:**

```
$SIMULATION (-1) ONLYSIM SUBPROBLEMS=1
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.

AN ERROR WAS FOUND ON LINE 94 AT THE APPROXIMATE POSITION NOTED:
   $SIMULATION (-1) ONLYSIM SUBPROBLEMS=1
                X
 THE CHARACTERS IN ERROR ARE: -1
  256  FIRST SEED MAY NOT BE -1 FOR THE FIRST PROBLEM.
```

## reject08_seed1_overflow

seed1 > i32::MAX (2147483647) — rejected at the pharos lowerer before NM-TRAN runs.

**Input $SIM:**

```
$SIMULATION (2147483648) ONLYSIM SUBPROBLEMS=1
```

**Pharos diagnostic (NM-TRAN not reached):**

```
seed1 value '2147483648' is out of range: must be -1 or an integer in [0, 2147483647]
```

## reject09_seed2_negative

seed2 < 0 — rejected at the pharos lowerer. The `-1` continuation sentinel applies only to seed1, not seed2.

**Input $SIM:**

```
$SIMULATION (1 -1) ONLYSIM SUBPROBLEMS=1
```

**Pharos diagnostic (NM-TRAN not reached):**

```
seed2 value '-1' is out of range: must be an integer in [0, 2147483647]
```

## reject10_eleven_groups

Eleven `(seed)` groups — rejected. NM-TRAN's caret lands on the 11th group's opening paren, confirming "1-10" refers to the source count.

**Input $SIM:**

```
$SIM (804831) (2) (3) (4) (5) (6) (7) (8) (9) (10) (11) CLOCKSEED=1 ONLYSIM SUBPROBLEMS=1
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.

AN ERROR WAS FOUND ON LINE 86 AT THE APPROXIMATE POSITION NOTED:
  $SIM (804831) (2) (3) (4) (5) (6) (7) (8) (9) (10) (11) CLOCKSEED=1 ONLYSIM SUBPROBLEMS=1
                                                     X
 THE CHARACTERS IN ERROR ARE: (
  111  $SIMULATE: 1-10 RANDOM NUMBER SOURCES MUST BE SPECIFIED.
```

## reject11_dist_uniform_first_source

Distribution `UNIFORM` on the first source — rejected. NONMEM requires the first source to be `NORMAL` (the default) when ETAs or epsilons are used in the model.

**Input $SIM:**

```
$SIMULATION (1 UNIFORM) ONLYSIM SUBPROBLEMS=1
```

**NONMEM error:**

```
WARNINGS AND ERRORS (IF ANY) FOR PROBLEM    1

 (WARNING  2) NM-TRAN INFERS THAT THE DATA ARE POPULATION.

 AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.

  255  $SIMULATE: WITH ETAS OR EPSILONS, FIRST SOURCE MUST BE NORMAL.
```

## reject12_dist_nonparametric_first_source

`NONPARAMETRIC` distribution on the first source — rejected. NONPARAMETRIC requires a `$MSFI` record anywhere in the control stream.

**Input $SIM:**

```
$SIMULATION (1 NONPARAMETRIC) ONLYSIM SUBPROBLEMS=1
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.

  349  $SIMULATE: NONPARAMETRIC NOT PERMITTED WITHOUT $MSFI.
```

## reject13_dist_nonparametric_second_source

`NONPARAMETRIC` distribution on the second source — rejected for the same reason as `reject12`. The requirement is `$MSFI`-record presence, not source position.

**Input $SIM:**

```
$SIMULATION (1) (2 NONPARAMETRIC) ONLYSIM SUBPROBLEMS=1
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.

  349  $SIMULATE: NONPARAMETRIC NOT PERMITTED WITHOUT $MSFI.
```

## reject14_dist_uniform_new_first_source

`UNIFORM` plus `NEW` token on the first source — rejected by the first-source-must-be-NORMAL rule. The `NEW` token itself is accepted syntactically; the error comes from `UNIFORM` on the first source.

**Input $SIM:**

```
$SIMULATION (1 UNIFORM NEW) ONLYSIM SUBPROBLEMS=1
```

**NONMEM error:**

```
WARNINGS AND ERRORS (IF ANY) FOR PROBLEM    1

 (WARNING  2) NM-TRAN INFERS THAT THE DATA ARE POPULATION.

 AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.

  255  $SIMULATE: WITH ETAS OR EPSILONS, FIRST SOURCE MUST BE NORMAL.
```

## reject15_three_seed_values_in_group

A seed group with three integers — rejected. NONMEM permits exactly one or two seed values per source (seed1 and optional seed2); a third value is an error. Pharos currently ignores extras silently and lets NM-TRAN catch this case.

**Input $SIM:**

```
$SIM ONLYSIM (804831 123) (1234 4321) (804 831) (621) (0217 123 432) CLOCKSEED=1 SUBPROBLEMS=1
```

**NONMEM error:**

```
AN ERROR WAS FOUND IN THE CONTROL STATEMENTS.

AN ERROR WAS FOUND ON LINE 86 AT THE APPROXIMATE POSITION NOTED:
 $SIM ONLYSIM (804831 123) (1234 4321) (804 831) (621) (0217 123 432) CLOCKSEED=1 SUBPROBLEMS=1
                                                                 X
 THE CHARACTERS IN ERROR ARE: 432
  126  ONE OR TWO SEED VALUES MUST BE SPECIFIED FOR EACH SOURCE.
```
