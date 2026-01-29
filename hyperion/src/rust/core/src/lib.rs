use extendr_api::Result;
use extendr_api::prelude::*;
use std::path::PathBuf;

//pharos config crate
use config::{CONFIG_FILENAME, find_config_dir as pharos_find_config_dir};

// Trait extensions for mapping error to extendr_api::Error::Other
// with custom message preceding the error message.
pub trait ResultExt<T> {
    fn map_to_extendr_err(self, message: impl Into<String>) -> Result<T>;
}

impl<T, E: std::fmt::Debug> ResultExt<T> for std::result::Result<T, E> {
    fn map_to_extendr_err(self, message: impl Into<String>) -> Result<T> {
        self.map_err(|x| extendr_api::Error::Other(format!("{}: {x:?}", message.into())))
    }
}

pub trait OptionExt<T> {
    fn ok_or_extendr_err(self, message: impl Into<String>) -> Result<T>;
}

impl<T> OptionExt<T> for Option<T> {
    fn ok_or_extendr_err(self, message: impl Into<String>) -> Result<T> {
        self.ok_or_else(|| Error::Other(message.into()))
    }
}

#[macro_export]
macro_rules! extendr_err {
    ($($arg:tt)*) => {
        Error::Other(format!($($arg)*))
    };
}

pub fn find_config_dir() -> Result<Option<PathBuf>> {
    pharos_find_config_dir().map_to_extendr_err("Failed to find config dir")
}

fn extract_clean_message(panic_msg: &str) -> Option<String> {
    if let Some(start) = panic_msg.find("called `Result::unwrap()` on an `Err` value: ") {
        let content = &panic_msg[start + "called `Result::unwrap()` on an `Err` value: ".len()..];
        if let Some(inner) = content
            .strip_prefix("Other(\"")
            .and_then(|s| s.strip_suffix("\")"))
        {
            return Some(inner.replace("\\n", "\n").replace("\\\"", "\""));
        }
        return Some(content.to_string());
    }
    None
}

fn is_extendr_location(location: &std::panic::Location<'_>) -> bool {
    let file = location.file();
    file.contains("extendr-api") || file.contains("/.cargo/git/checkouts/extendr")
}

#[extendr]
pub fn set_panic_message() {
    std::panic::set_hook(Box::new(|x| {
        // Extract the panic message
        let message = if let Some(s) = x.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = x.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic payload type".to_string()
        };

        let clean_message = extract_clean_message(&message);
        if let Some(location) = x.location() {
            if is_extendr_location(location) {
                rprintln!("Error occurred in Hyperion");
            } else {
                rprintln!("Error occurred in Hyperion, {}", location);
            }
        } else {
            rprintln!("Error occurred in Hyperion");
        }

        if let Some(clean_message) = clean_message {
            let indented_msg = clean_message.replace("\n", "\n\t");
            reprintln!("Reason:\n\t{}\n", indented_msg);
        } else {
            reprintln!("{message}");
        }
    }));
}

#[extendr]
pub fn find_pharos_config_file() -> Result<Robj> {
    let config_dir = find_config_dir()?;

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
