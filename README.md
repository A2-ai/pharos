# Pharos

Pharos is a standalone CLI tool to manage and run pharmaceutical models using various software solutions.

Each software solution code will be contained in a sub-package of this repository (eg components/nonmem for nonmem) that
you can use directly if you want to use it from Rust.


## NONMEM

[NONMEM](https://en.wikipedia.org/wiki/NONMEM) is the first supported software.
There is also a R package, [Hyperion](https://github.com/A2-ai/hyperion) wrapping that crate.

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

After a run, you can get the interesting data from the output files via 2 commands:

```
pharos nonmem parameters components/nonmem/models/BQL/bql/bql.ext

# Will output the parameters
```

```
pharos nonmem summary components/nonmem/models/BQL/bql/

# will output something like

=== Summary ===

Problem: RUN# 2 - 2cmpt model - no BQLs 
Records: 2895   Observations: 2702  Subjects: 193

Estimation Method(s):
 - First Order Conditional Estimation with Interaction

Objective Function Value:
 - -14346.006

Heuristic Problems Detected:
 - None

THETA Parameters:
+-----------+----------+-----------+-------+
| Parameter | Estimate | SE (RSE%) | Fixed |
+-----------+----------+-----------+-------+
| THETA1    | 26.49    | N/A       | no    |
| THETA2    | 282.6    | N/A       | no    |
| THETA3    | 297.0    | N/A       | no    |
| THETA4    | 58.75    | N/A       | no    |
| THETA5    | 1.509    | N/A       | no    |
| THETA6    | 0.7500   | N/A       | yes   |
| THETA7    | 1.000    | N/A       | yes   |
| THETA8    | 1.000    | N/A       | yes   |
| THETA9    | 0.7500   | N/A       | yes   |
+-----------+----------+-----------+-------+

OMEGA Parameters:
+------------+------+----------+-----------+---------------+-------+
| Parameter  | ETA  | Estimate | SE (RSE%) | Shrinkage (%) | Fixed |
+------------+------+----------+-----------+---------------+-------+
| OMEGA(1,1) | ETA1 | 0.1006   | N/A       | 0.6662        | no    |
| OMEGA(2,2) | ETA2 | 0.03600  | N/A       | 2.317         | no    |
| OMEGA(3,3) | ETA3 | 0.01117  | N/A       | 18.70         | no    |
+------------+------+----------+-----------+---------------+-------+

SIGMA Parameters:
+------------+------+----------+-----------+---------------+-------+
| Parameter  | EPS  | Estimate | SE (RSE%) | Shrinkage (%) | Fixed |
+------------+------+----------+-----------+---------------+-------+
| SIGMA(1,1) | EPS1 | 0.002451 | N/A       | 9.703         | no    |
+------------+------+----------+-----------+---------------+-------+

```
### Copying a model

Once a model has run, you can decide to branch a new model from it with the `copy` command.

```
pharos copy --from=components/nonmem/models/BQL/bql.mod --to=components/nonmem/models/BQL/bql2.mod
```

This command will not anything other than copy the model file to the `to` destination, as well as creating a `{model_name}_metadata.json`
file to track lineage (more on that later).

However, you can also choose to update the parameters directly from an `ext` file and/or jitter them. For example:

```
pharos nonmem copy --from=bql.mod --to=bql2.mod --update=theta,omega --jitter theta:0.2 --jitter omega:0.3 --overwrite  --jitter-excluded=THETA1
```

This will make it so the `bql2.mod` file to have the `ext` theta and omega values from the `bql.mod` run, with a 20% jitter on theta (except for THETA1)
and 30% for omegas. It will also overwrite if a `bql2.mod` file already exists.

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
