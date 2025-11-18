use extendr_api::prelude::*;
use std::cell::RefCell;

//pharos config crate
use config::{CONFIG_FILENAME, find_config_dir};

// Thread-local storage for clean error message from suppressed extendr panic
thread_local! {
    static STORED_ERROR: RefCell<Option<String>> = RefCell::new(None);
}

// Trait extensions for mapping error to extendr_api::Error::Other
// with custom message preceding the error message.
pub trait ResultExt<T> {
    fn map_to_extendr_err(self, message: impl Into<String>) -> Result<T>;
}

impl<T, E: std::fmt::Debug> ResultExt<T> for std::result::Result<T, E> {
    fn map_to_extendr_err(self, message: impl Into<String>) -> extendr_api::Result<T> {
        self.map_err(|x| extendr_api::Error::Other(format!("{}: {x:?}", message.into())))
    }
}

pub trait OptionExt<T> {
    fn ok_or_extendr_err(self, message: impl Into<String>) -> Result<T>;
}

impl<T> OptionExt<T> for Option<T> {
    fn ok_or_extendr_err(self, message: impl Into<String>) -> extendr_api::Result<T> {
        self.ok_or_else(|| Error::Other(message.into()))
    }
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

#[extendr]
pub fn find_pharos_config_file() -> Result<Robj> {
    let config_dir = find_config_dir().map_to_extendr_err("Failed to find_config_dir")?;

    match config_dir {
        Some(d) => Ok(d.join(CONFIG_FILENAME).to_string_lossy().into_robj()),
        None => Ok(
            "No pharos.toml config file found. Please call hyperion::init() to create one"
                .into_robj(),
        ),
    }
}

extendr_module! {
    mod hyperion_core;

    fn set_panic_message;
    fn find_pharos_config_file;
}
