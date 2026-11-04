//! The JSON Schemas `statglass schema` publishes: one per `--json` result, the error
//! envelope and the request.

use schemars::{JsonSchema, Schema, schema_for};

use crate::{
    CliErrorEnvelope, DoctorReport, EventsAnswer, EventsSummary, HistoryAnswer, HostsAnswer,
    MetricsAnswer, NowAnswer, QueryRequest, StatusAnswer, TopAnswer,
};

#[derive(Clone, Debug)]
pub struct SchemaEntry {
    /// The name `statglass schema <name>` takes. For answers, the [`crate::Answer::name`].
    pub name: &'static str,
    /// One line on what the schema describes.
    pub describes: &'static str,
    pub schema: Schema,
}

fn entry<T: JsonSchema>(name: &'static str, describes: &'static str) -> SchemaEntry {
    SchemaEntry {
        name,
        describes,
        schema: schema_for!(T),
    }
}

/// Every published schema, in `statglass schema` listing order.
pub fn schemas() -> Vec<SchemaEntry> {
    vec![
        entry::<StatusAnswer>(
            "status",
            "The running app's status, as the status request answers it",
        ),
        entry::<HostsAnswer>("hosts", "statglass hosts --json"),
        entry::<NowAnswer>("now", "statglass now --json"),
        entry::<MetricsAnswer>("metrics", "statglass metrics --json"),
        entry::<HistoryAnswer>("history", "statglass history --json"),
        entry::<EventsAnswer>("events", "statglass events --json"),
        entry::<EventsSummary>("events-summary", "statglass events --summary --json"),
        entry::<TopAnswer>("top", "statglass top --json"),
        entry::<DoctorReport>("doctor", "statglass doctor --json"),
        entry::<CliErrorEnvelope>(
            "error",
            "The error envelope: the last stderr line on failure under --json",
        ),
        entry::<QueryRequest>("request", "A request on the query socket"),
    ]
}
