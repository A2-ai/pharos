//! Canonical-form lookups for NM-TRAN record option keywords.
//!
//! NM-TRAN accepts keyword abbreviations (e.g. `IGN` for `IGNORE`, `NSUB` for
//! `SUBPROBLEMS`).
//! Accepted forms empirically verified against nm760

pub(crate) fn canonicalize_data_option(raw: &str) -> String {
    let upper = raw.to_uppercase();
    let canonical = match upper.as_str() {
        "IGN" | "IGNO" | "IGNOR" => "IGNORE",
        "ACC" | "ACCE" | "ACCEP" => "ACCEPT",
        "REC" | "RECO" | "RECOR" | "RECORD" | "RECS" | "NREC" | "NRECO" | "NRECOR" | "NRECORD"
        | "NRECORDS" | "NRECS" => "RECORDS",
        "NUL" => "NULL",
        "LREC" => "LRECL",
        "MIS" | "MISD" | "MISDA" => "MISDAT",
        "REP" => "REPL",
        "NOW" | "NOWI" | "NOWID" => "NOWIDE",
        "WID" => "WIDE",
        "CHECK" | "CHECKO" | "CHECKOU" | "CHECKDATA" => "CHECKOUT",
        "NOO" | "NOOP" | "NOOPE" => "NOOPEN",
        "PRED_IGNORE" => "PRED_IGNORE_DATA",
        _ => return upper,
    };
    canonical.to_string()
}

pub(crate) fn canonicalize_simulation_option(raw: &str) -> String {
    let upper = raw.to_uppercase();
    let canonical = match upper.as_str() {
        "ONL" | "ONLY" | "ONLYS" | "ONLYSI" | "ONLYSIMU" | "ONLYSIMUL" | "ONLYSIMULA"
        | "ONLYSIMULAT" | "ONLYSIMULATI" | "ONLYSIMULATIO" | "ONLYSIMULATION" => "ONLYSIM",
        "OMI" | "OMIT" | "OMITT" | "OMITTE" => "OMITTED",
        "PRED" | "PREDI" | "PREDIC" | "PREDICT" | "PREDICTI" | "PREDICTIO" => "PREDICTION",
        "NOPR" | "NOPRE" | "NOPRED" | "NOPREDI" | "NOPREDIC" | "NOPREDICT" | "NOPREDICTI"
        | "NOPREDICTIO" => "NOPREDICTION",
        "NOREW" | "NOREWI" | "NOREWIN" => "NOREWIND",
        "REW" | "REWI" | "REWIN" => "REWIND",
        "SUPR" | "SUPRE" | "SUPRES" | "SUPRESE" => "SUPRESET",
        "NOSUPR" | "NOSUPRE" | "NOSUPRES" | "NOSUPRESE" => "NOSUPRESET",
        "REP" | "REPL" | "REPLA" | "REPLAC" => "REPLACE",
        "NOREP" | "NOREPL" | "NOREPLA" | "NOREPLAC" => "NOREPLACE",
        "SUBP" | "SUBPR" | "SUBPRO" | "SUBPROB" | "SUBPROBL" | "SUBPROBLE" | "SUBPROBLEM"
        | "SUBPROBS" | "NSUB" | "NSUBP" | "NSUBPR" | "NSUBPRO" | "NSUBPROB" | "NSUBPROBL"
        | "NSUBPROBLE" | "NSUBPROBLEM" | "NSUBPROBLEMS" | "NSUBPROBS" => "SUBPROBLEMS",
        "BOO" | "BOOT" | "BOOTS" | "BOOTST" | "BOOTSTR" | "BOOTSTRA" => "BOOTSTRAP",
        "SOUR" | "SOURC" | "SOURCE" => "SOURCE_EPS",
        "TR" | "TRU" => "TRUE",
        "STR" | "STRA" => "STRAT",
        "RAN" | "RANM" | "RANME" | "RANMET" | "RANMETH" | "RANMETHO" => "RANMETHOD",
        "PAR" | "PARA" | "PARAF" | "PARAFI" | "PARAFIL" => "PARAFILE",
        "CLOCK" | "CLOCKS" | "CLOCKSE" | "CLOCKSEE" => "CLOCKSEED",
        _ => return upper,
    };
    canonical.to_string()
}
