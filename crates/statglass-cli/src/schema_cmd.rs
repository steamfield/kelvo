//! `statglass schema [name]`: the JSON Schemas `statglass-query` publishes.

use serde_json::json;
use statglass_query::{schemas, suggest};

use crate::{Output, failure::Failure, format};

pub fn run(name: Option<&str>, json: bool) -> Result<Output, Failure> {
    let all = schemas();
    let Some(name) = name else {
        if json {
            let list: Vec<_> = all
                .iter()
                .map(|e| json!({ "name": e.name, "describes": e.describes }))
                .collect();
            return Output::json(&json!({ "schemas": list }));
        }
        let rows: Vec<Vec<String>> = all
            .iter()
            .map(|e| vec![e.name.to_owned(), e.describes.to_owned()])
            .collect();
        return Ok(Output::Text(format::table(&rows, true, "")));
    };
    let lower = name.to_lowercase();
    let entry = all.iter().find(|e| e.name == lower).ok_or_else(|| {
        Failure::usage(
            Some(name),
            "unknown schema name; run statglass schema to list them",
            suggest::closest(name, all.iter().map(|e| e.name)),
        )
    })?;
    if json {
        return Output::json(&entry.schema);
    }
    let text = serde_json::to_string_pretty(&entry.schema)
        .map_err(|e| Failure::defect(format!("schema {} did not serialize: {e}", entry.name)))?;
    Ok(Output::Text(text + "\n"))
}
