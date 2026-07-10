use std::path::{Component, Path};

use anyhow::{Result, bail};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use tera::{Context, Tera};

pub fn render_output_dir_template(template: &str, model_name: &str) -> Result<String> {
    let mut context = Context::new();
    context.insert("name", model_name);
    let ts = Timestamp::now();
    context.insert("unix_timestamp", &ts.as_second().to_string());
    context.insert(
        "timestamp",
        &ts.to_zoned(TimeZone::UTC)
            .strftime("%Y-%m-%dT%H_%M_%S%z")
            .to_string(),
    );

    let res = Tera::one_off(template, &context, false)?;

    // Reject rendered names that would escape the model directory once joined
    // onto a base dir and possibly passed to remove_dir_all under --overwrite.
    let p = Path::new(&res);
    if res.is_empty() {
        bail!("output_dir must not be empty");
    }

    if p.has_root() {
        bail!("output_dir must be a relative path, got '{res}'");
    }
    if p.components().any(|c| c == Component::ParentDir) {
        bail!("output_dir must not contain '..' (got '{res}')");
    }

    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_render_output_simple() {
        assert_eq!(
            "run2.dir",
            render_output_dir_template("run2.dir", "run1").unwrap()
        );
        assert_eq!(
            "run1.dir",
            render_output_dir_template("{{name}}.dir", "run1").unwrap()
        );
    }

    #[test]
    fn allows_nested_relative_names() {
        assert_eq!(
            "runs/run001",
            render_output_dir_template("runs/{{name}}", "run001").unwrap()
        );
    }

    #[test]
    fn rejects_unsafe_output_dirs() {
        for template in ["", "..", "../x", "a/../../b", "/abs/path"] {
            assert!(
                render_output_dir_template(template, "run1").is_err(),
                "expected '{template}' to be rejected"
            );
        }
    }
}
