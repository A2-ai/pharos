# Pharos

Pharos is a standalone CLI tool to manage and run pharmaceutical models using 
various CLI-based Nonlinear mixed effects modeling software solutions.

Each software solution code will be contained in a sub-package of this repository 
(eg components/nonmem for nonmem) that
you can use directly if you want to use it from Rust.

Pharos was heavily inspired by (P)erl (S)peaks (N)onmem (PSN) and tries to bring similar functionalities to
the CLI, as well as bbr/bbi around additional capabilities for both on the CLI and through R.

Pharos will also "learn" to talk to other software such as Monolix in 2026 and beyond through 
a consistent interface, smoothing out the differences between each software solution and providing
critical traceability and useability features to all software it wraps.

## NONMEM

[NONMEM](https://en.wikipedia.org/wiki/NONMEM) is the first supported software.
There is also a R package, [Hyperion](https://github.com/A2-ai/hyperion) to interact directly from R.

All the commands mentioned below have an `--help` flag to see all the available options.

To get started, run `pharos nonmem init` which will create a `pharos.toml` configuration file like this:

```toml
[nonmem]
# clean_level is what we use to determine which files to keep from nonmem
# 1: ".xml", ".grd", ".shk", ".cor", ".cov", ".ext", ".lst"
# 2: level 1 + ".clt", ".coi", ".cpu", ".shm", ".phi"
# 3: level 2 + ".msf"
# any other number: keep everything
clean_level = 1
# Which versions of nonmem. This should be a key defined in nonmem.versions below
default_version = "nm760"
# If you want specific files to copy along with the model if we run it from a temporary directory
# Useful if you have FORTRAN files or similar that we don't detect
files_to_copy = []

# This contains nonmem command line parameters
[nonmem.options]
prsame = false
prcompile = false
prdefault = false
tprdefault = false
background = false
nobuild = false
maxlim = 2

# You can define the paths to several versions of nonmem and switch between them via the CLI or here
[nonmem.versions]
nm760 = "/opt/nonmem/nm760"

[nonmem.parallel]
# Path to the mpiexec executable
mpiexec_path = "/opt/mpich/bin/mpiexec"
# Whether mpi usage is enabled or not
enabled = false
num_cpus = 4
timeout = 2147483647

# We can parse some comments format into structured data automatically
# More on the supported format below
[nonmem.comments]
# You can choose to error if the comments are not matching the selected type. 
# This will fail before the run.
error_on_invalid = false
# Comment type to match: currently only "type1" supported
type = ""
```

### Running a model

You can first run nmtrans to check a model before running it:

```
pharos nonmem check components/nonmem/models/BQL/bql.mod
```

And then run it

```
pharos nonmem run components/nonmem/models/BQL/bql.mod
```

Use `pharos nonmem run --help` to see all the available flags.

In the output directory, we will create 3 files:

- `pharos_config.json`: contains the actual pharos configuration that was used for this run
- `pharos_start.json`: created before the run, contains the path and hash of model/dataset file
- `pharos_end.json`: created once the run is over contains the start/end datetime, the files path we've rewritten in the model to move it to a temporary directory
  (for example your model might say to output a file to `../2.TAB`, we will rewrite it to `2.TAB`) and the hashes for each output files

After a run, you can get the interesting data from the output files via the `summary` command:

```
pharos nonmem summary components/nonmem/test_data/run_output/run002

# will output something like

=== run002 Summary ===

Problem: Base one-compartment oral absorption model
Records: 240   Observations: 210  Subjects: 30

Estimation Method(s):
 - First Order Conditional Estimation with Interaction

Objective Function Value:
 - -103.468

Condition Number:
 - 29.628

Heuristic Problems Detected:
 - None

THETA Parameters:
+-----------+----------+-------------------+-------+
| Parameter | Estimate | SE (RSE%)         | Fixed |
+-----------+----------+-------------------+-------+
| TVCL      | 1.2468   | 0.12883 (10.333%) | no    |
| TVV       | 40.848   | 3.0272 (7.4109%)  | no    |
| TVKA      | 1.2439   | 0.11341 (9.1170%) | no    |
+-----------+----------+-------------------+-------+

OMEGA Parameters:
+------------+------+----------+----------------------+---------------+-------+----------+
| Parameter  | ETA  | Estimate | SE (RSE%)            | Shrinkage (%) | Fixed | Diagonal |
+------------+------+----------+----------------------+---------------+-------+----------+
| OMEGA(1,1) | ETA1 | 0.130417 | 0.0601864 (46.1492%) | 18.0601       | no    | yes      |
| OMEGA(2,2) | ETA2 | 0.136332 | 0.0397114 (29.1285%) | 4.98590       | no    | yes      |
| OMEGA(3,3) | ETA3 | 0.114408 | 0.0614429 (53.7051%) | 27.1894       | no    | yes      |
+------------+------+----------+----------------------+---------------+-------+----------+

SIGMA Parameters:
+------------+------+------------+-------------------------+---------------+-------+----------+
| Parameter  | EPS  | Estimate   | SE (RSE%)               | Shrinkage (%) | Fixed | Diagonal |
+------------+------+------------+-------------------------+---------------+-------+----------+
| SIGMA(1,1) | EPS1 | 0.03723180 | 0.01159960 (31.155088%) | 15.438400     | no    | yes      |
| SIGMA(2,2) | EPS2 | 0.00660722 | 0.02792070 (422.57863%) | 15.438400     | no    | yes      |
+------------+------+------------+-------------------------+---------------+-------+----------+
```
### Copying a model

Once a model has run, you can decide to branch a new model from it with the `copy` command.

```
pharos copy --from=components/nonmem/models/BQL/bql.mod --to=components/nonmem/models/BQL/bql2.mod
```

This command will not anything other than copy the model file to the `to` destination, as well as creating a `{model_name}_metadata.json`
file to track lineage (more on that later).

However, you can also choose to update the parameters directly from an `ext` file and/or jitter thetas. For example:

```
pharos nonmem copy --from=bql.mod --to=bql2.mod --update=theta,omega --jitter 0.2  --overwrite  --jitter-excluded=THETA1
```

This will make it so the `bql2.mod` file to have the `ext` theta and omega values from the `bql.mod` run, with a 20% jitter on theta (except for THETA1).
It will also overwrite if a `bql2.mod` file already exists.

### Lineage

If you are using `pharos` to copy models, you should automatically get a lineage from the `*_metadata.json` files.
Make sure you create one for the intial model if you want the full lineage, you can copy an empty one like:

```json
{
  "based_on": [],
  "description": "",
  "tags": []
}
```

To view the lineage, you can do the following

```
pharos nonmem lineage components/nonmem/models/BQL/
```

You can specify `--from` and/or `--to` to only show a certain lineage.


### Comments
As mentioned before, some comments on THETA/OMEGA/SIGMA parameters can be parsed if they follow a certain convention.

#### Type 1

One of the following for THETAs (can be mixed in the same file):
- `TVCL (L/h) :LOG` -> parameter: TVCL, unit: L/h, optional parametrization: LOG
- `CRCL cov` -> parameter: CRCL, detected as covariate
- `RES ERR :stdev` -> typ: RES ERR, parameterization: stdev

OMEGAs: `OM<i> <THETA_NAME> :EXP` -> name: OM<i>, theta_name: <THETA_NAME>, optional parametrization :EXP
SIGMAs: `SIG<i> :EXP` -> name: SIG<i>, optional parametrization :EXP


## Previous Inspiration

Learn more about PSN at: [https://github.com/uupharmacometrics/psn](https://github.com/uupharmacometrics/psn)

and bbr/bbi at: [https://github.com/metrumresearchgroup/bbr](https://github.com/metrumresearchgroup/bbr)
and [https://github.com/metrumresearchgroup/bbi](https://github.com/metrumresearchgroup/bbi)
