use extendr_api::prelude::*;

pub mod init;
use hyperion_core;
use hyperion_nonmem;
use hyperion_scheduler;

// Macro to generate exports.
// This ensures exported functions are registered with R.
// See corresponding C code in `entrypoint.c`.
extendr_module! {
    mod hyperion;

    use init;
    use hyperion_core;
    use hyperion_nonmem;
    use hyperion_scheduler;
}
