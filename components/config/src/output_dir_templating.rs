use anyhow::Result;
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
}
