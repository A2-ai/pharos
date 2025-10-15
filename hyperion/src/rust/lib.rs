use extendr_api::prelude::*;
use std::cell::RefCell;

pub mod init;
pub mod model;
pub mod output_files;
pub mod utils;

// Thread-local storage for clean error message from suppressed extendr panic
thread_local! {
    static STORED_ERROR: RefCell<Option<String>> = RefCell::new(None);
}

#[extendr]
pub fn set_panic_message() {
    std::panic::set_hook(Box::new(|x| {
        // Check if this is an extendr internal panic
        let is_extendr_internal = if let Some(location) = x.location() {
            location.file().contains(".cargo/registry") && location.file().contains("extendr-api")
        } else {
            false
        };

        // Extract the panic message
        let message = if let Some(s) = x.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = x.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic payload type".to_string()
        };

        if is_extendr_internal {
            // This is an extendr internal panic - extract clean message and suppress
            if let Some(clean_msg) = extract_clean_message(&message) {
                STORED_ERROR.with(|stored| {
                    *stored.borrow_mut() = Some(clean_msg);
                });
            }
            // Suppress this panic completely - print nothing
        } else {
            // This is a user code panic - show combined message
            if let Some(location) = x.location() {
                rprintln!("Error occurred in Hyperion, {}", location);
            } else {
                rprintln!("Error occurred in Hyperion");
            }

            // Print the stored clean message if available
            STORED_ERROR.with(|stored| {
                if let Some(ref clean_msg) = *stored.borrow() {
                    let indented_msg = clean_msg.replace("\n", "\n\t");
                    reprintln!("Reason:\n\t{}\n", indented_msg);
                }
            });
        }
    }));
}

/// Extract clean message from Error::Other("...") format
fn extract_clean_message(panic_msg: &str) -> Option<String> {
    if panic_msg.starts_with("called `Result::unwrap()` on an `Err` value: Other(\"") {
        let start = "called `Result::unwrap()` on an `Err` value: Other(\"".len();
        if let Some(end) = panic_msg.rfind("\")") {
            let content = &panic_msg[start..end];
            // Convert escape sequences to readable format
            return Some(content.replace("\\n", "\n").replace("\\\"", "\""));
        }
    }
    None
}

// Macro to generate exports.
// This ensures exported functions are registered with R.
// See corresponding C code in `entrypoint.c`.
extendr_module! {
    mod hyperion;

    use output_files;
    use model;
    use init;

    fn set_panic_message;
}
