const BASE_GITIGNORE: &str = r#"background.set
compile.lnk
FCON
FDATA
FDATA.csv
FMSG
FREPORT
FSIZES
FSTREAM
FSUBS
FSUBS.0
FSUBS.o
FSUBS_MU.F90
FSUBS.f90
fsubs.f90
FSUBS2
gfortran.txt
GFCOMPILE.BAT
INTER
licfile.set
linkc.lnk
LINK.LNK
LINKC.LNK
locfile.set
maxlim.set
newline
nmexec.set
nmpathlist.txt
nmprd4p.mod
nobuild.set
parafile.set
parafprint.set
prcompile.set
prdefault.set
prsame.set
PRSIZES.f90
rundir.set
runpdir.set
simparon.set
temp_dir
tprdefault.set
trskip.set
worker.set
xmloff.set
fort.2001
fort.2002
flushtime.set
nonmem
FPWARN
condorarguments.set
condoropenmpiscript.set
condor.set
mpiloc
nmmpi.sh
temp.out
trashfile.xxx
WK_[0-9]*
*.pnm
"#;

pub const INITIAL_GITIGNORE: &str = r#"*
!.gitignore
"#;

pub fn get_final_gitignore(model_name: &str) -> String {
    let mut output = BASE_GITIGNORE.to_string();

    output.push_str(&format!("\n{model_name}"));
    output.push_str(&format!("\n{model_name}_ETAS"));
    output.push_str(&format!("\n{model_name}_RMAT"));
    output.push_str(&format!("\n{model_name}_SMAT"));
    output.push_str(&format!("\n{model_name}.msf"));
    output.push_str(&format!("\n{model_name}_ETAS.msf"));
    output.push_str(&format!("\n{model_name}_RMAT.msf"));
    output.push_str(&format!("\n{model_name}_SMAT.msf"));

    output
}
