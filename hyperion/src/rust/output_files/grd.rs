use crate::utils::{find_output_file, try_parse_model, get_comment_type};
use extendr_api::prelude::*;
use nonmem::{estimation::EstimationMethod, output_files::grd::GrdReader};

fn create_grd_reader(only_method: Option<&str>, only_last: Option<bool>) -> Result<GrdReader> {
    let mut reader = GrdReader::default();

    if let Some(method_str) = only_method {
        // Handle common aliases
        let normalized_method = match method_str.to_lowercase().as_str() {
            "importance" => "imp",
            "focei" => "foce",
            _ => method_str,
        };
        let method = normalized_method
            .parse::<EstimationMethod>()
            .map_err(|_| Error::Other(format!("Invalid estimation method: {}", method_str)))?;
        reader = reader.only_method(method);
    } else if let Some(last) = only_last {
        if last {
            reader = reader.only_last();
        } else {
            reader = reader.keep_all_tables();
        }
    }

    Ok(reader)
}

/// Gets gradients of pararmeters during modeling
///
/// @param path path to model file, model output directory, grd file or metadata json file.
/// @param only_method character, filter for getting estimates from specified method only.
/// Available methods are Fo, Foce, Saems, Bayes, Imp, ImpMap, Its, Nuts
/// @param only_last boolean, for grabbing only last estimation method parameters
///
/// @return data.frame of gradients
/// @export
///
/// @examples \dontrun{
/// get_gradients("model/nonmem/run001/run001.grd")
/// }
#[extendr]
pub fn get_gradients(
    path: &str,
    #[default = "NULL"] only_method: Option<&str>,
    #[default = "TRUE"] only_last: Option<bool>,
) -> Result<Robj> {
    let grd_reader = create_grd_reader(only_method, only_last)?;
    let grd_path = find_output_file(path, "grd")?;

    let mut model = try_parse_model(&path);

    // Load config and extract comment type
    let comment_type = get_comment_type();

    let tables = grd_reader
        .parse_file(grd_path, model.as_mut(), comment_type)
        .map_err(|e| Error::Other(e.to_string()))?;

    if tables.is_empty() {
        return Err(Error::Other("No tables found in grd file".to_string()));
    }

    // Get gradient parameter names from the first table (skip ITERATION column)
    let gradient_names: Vec<String> = tables[0].parameters.iter().skip(1).cloned().collect();

    let flat_data: Vec<(i32, String, Vec<f64>)> = tables
        .into_iter()
        .flat_map(|table| {
            let method_name = table
                .method
                .map(|m| m.to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            table
                .rows
                .into_iter()
                .map(move |row| (row.iteration as i32, method_name.clone(), row.gradients))
        })
        .collect();

    // Extract columns
    let iterations: Vec<i32> = flat_data.iter().map(|(iter, _, _)| *iter).collect();
    let methods: Vec<String> = flat_data
        .iter()
        .map(|(_, method, _)| method.clone())
        .collect();

    // Build column pairs
    let mut pairs = vec![
        ("iteration", iterations.into_robj()),
        ("method", methods.into_robj()),
    ];

    // Add gradient columns dynamically
    for (grad_idx, grad_name) in gradient_names.iter().enumerate() {
        let values: Vec<f64> = flat_data
            .iter()
            .map(|(_, _, row_gradients)| row_gradients.get(grad_idx).copied().unwrap_or(f64::NAN))
            .collect();
        pairs.push((grad_name.as_str(), values.into_robj()));
    }

    let list = List::from_pairs(pairs);
    let df = data_frame!(list);

    Ok(df)
}

extendr_module! {
    mod grd;
    fn get_gradients;
}
