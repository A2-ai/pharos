use std::collections::HashMap;
use std::sync::LazyLock;

use anyhow::{Result, bail};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use regex::Regex;

static TEMPLATE_VAR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{\s*(\w+)\s*}}").unwrap());

pub fn render_output_template(template: &str, model_name: &str) -> Result<String> {
    let mut vars = HashMap::new();

    vars.insert("name", model_name.to_string());
    let ts = Timestamp::now();
    vars.insert("unix_timestamp", ts.as_second().to_string());
    vars.insert(
        "timestamp",
        ts.to_zoned(TimeZone::UTC)
            .strftime("%Y-%m-%dT%H_%M_%S%z")
            .to_string(),
    );

    let mut unknown_vars = Vec::new();

    // First pass: collect unknown variables
    for caps in TEMPLATE_VAR_RE.captures_iter(template) {
        let var_name = &caps[1];
        if !vars.contains_key(var_name) {
            unknown_vars.push(var_name.to_string());
        }
    }

    if !unknown_vars.is_empty() {
        let supported = ["`name`", "`unix_timestamp`", "`timestamp`"].join(", ");
        bail!(
            "Unknown template variables: `{}` in template `{}`. Only {} are supported.",
            unknown_vars.join("', '"),
            template,
            supported
        );
    }

    let result = TEMPLATE_VAR_RE
        .replace_all(template, |caps: &regex::Captures| {
            let var_name = &caps[1];
            vars.get(var_name).cloned().unwrap()
        })
        .to_string();

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_render_output_simple() {
        assert_eq!(
            "run2.dir",
            render_output_template("run2.dir", "run1").unwrap()
        );
        assert_eq!(
            "run1.dir",
            render_output_template("{{name}}.dir", "run1").unwrap()
        );
    }

    #[test]
    fn error_on_unknown_variable() {
        let err = render_output_template("{{nam}}.dir", "run1").unwrap_err();
        assert_eq!(
            err.to_string(),
            "Unknown template variables: `nam` in template `{{nam}}.dir`. Only `name`, `unix_timestamp`, `timestamp` are supported."
        );
    }
}
