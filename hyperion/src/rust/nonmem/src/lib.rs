use extendr_api::prelude::*;

pub mod model;
pub mod output_files;
pub mod utils;

// Re-export all public items for easy access
pub use model::*;
pub use output_files::*;
pub use utils::*;

// Generate extendr module for R integration
extendr_module! {
    mod hyperion_nonmem;
    use model;
    use output_files;
    use utils;
}