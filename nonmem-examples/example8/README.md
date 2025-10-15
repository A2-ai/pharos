# README

This example provides what will happen with a bayes run


## nuance

* we need to pick up the fact that a `fort.51` and `fort.52` file will be written and to copy them/hash them like other tables
* we need to copy into the run dir the `nonmem_reserved_general` file

## details

```
  IF(BAYES_EXTRA==1 .AND. ITER_REPORT>=0 .AND. TIME==0.0) THEN
"  WRITE(51,98) ITER_REPORT,ID,CL,V1,Q,V2
" 98 FORMAT(I12,1X,F14.0,4(1X,1PG12.5))
```

```
IF(BAYES_EXTRA==1 .AND. ITER_REPORT>=0 ) THEN
" WRITE(52,97) ITER_REPORT,ID,TIME,F
" 97 FORMAT(I12,1X,F14.0,2(1X,1PG12.5))
ENDIF
```

these files will be written as fort.51 and fort.52 respectively.

	•	97 and 98 are just statement labels that name the two FORMAT definitions.
The WRITE(…,97) or WRITE(…,98) calls say “use the layout described by the FORMAT labeled 97 or 98.”
	•	The output files for WRITE(51,…) and WRITE(52,…) (with no prior OPEN assigning filenames) will be the Fortran default files:
	•	fort.51 for unit 51
	•	fort.52 for unit 52


What each FORMAT does
•	98 FORMAT(I12,1X,F14.0,4(1X,1PG12.5)) (used with WRITE(51,98))
    •	I12 → ITER_REPORT as a 12-wide integer
    •	1X → one space
    •	F14.0 → ID in a 14-wide fixed field with 0 decimals
    •	4(1X,1PG12.5) → four values (CL V1 Q V2), each preceded by a space and printed in generalized scientific format (G12.5) with scale factor 1P
•	97 FORMAT(I12,1X,F14.0,2(1X,1PG12.5)) (used with WRITE(52,97))
    •	Same first two fields, then two values (TIME and F) in G12.5.

## File: fort.51

**Source statement:**
```fortran
WRITE(51,98) ITER_REPORT,ID,CL,V1,Q,V2
98 FORMAT(I12,1X,F14.0,4(1X,1PG12.5))
```

Content:
Posterior draws of structural parameters at reporting iterations.

Example line:

          25              101  1.2346E+01  3.2100E+01  1.5000E+01  2.8000E+01

Columns:
	•	ITER_REPORT → iteration number (I12)
	•	ID → subject ID (F14.0)
	•	CL → clearance (G12.5 scientific format)
	•	V1 → central volume
	•	Q → intercompartmental clearance
	•	V2 → peripheral volume


File: fort.52

Source statement:

```fortran
WRITE(52,97) ITER_REPORT,ID,TIME,F
97 FORMAT(I12,1X,F14.0,2(1X,1PG12.5))
```

Content:
Posterior predictive draws of model predictions.

Example line:

          25              101  0.0000E+00  5.6789E+00

Columns:
	•	ITER_REPORT → iteration number (I12)
	•	ID → subject ID (F14.0)
	•	TIME → time point (G12.5 scientific format)
	•	F → model prediction

⸻

Notes
	•	By default, these files are named fort.51 and fort.52 unless an explicit
OPEN statement assigns another filename.
	•	Lines beginning with " in the control stream are comments; remove the "
to activate the WRITE statements.
	•	Formatting uses Fortran rules: fixed column widths and scientific notation
with 1P scaling for floating-point values.

Would you like me to also extend this with a **multi-line example** (several iterations × subjects) so you have a realistic reference block of what an actual run might output?
