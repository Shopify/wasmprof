use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SpeedscopeFile {
    #[serde(rename = "$schema")]
    schema: Schema,
    #[serde(rename = "activeProfileIndex")]
    active_profile_index: Option<f64>,
    exporter: Option<String>,
    name: Option<String>,
    profiles: Vec<Profile>,
    shared: Shared,
}

// Duration takes 64 bits to store seconds and 32 bits to store nanoseconds
fn from_u128_to_duration(d: &u128) -> Duration {
    let secs = d / 1_000_000_000;
    let nanos = d % 1_000_000_000;
    Duration::new(secs as u64, nanos as u32)
}

impl SpeedscopeFile {
    pub fn new(frames: Vec<String>, samples: Vec<Vec<usize>>, weights: Option<Vec<u128>>) -> Self {
        let end_value = samples.len();
        let samples_len = samples.len();
        let weights = weights.map(|w| {
            w.iter()
                .map(|d| from_u128_to_duration(d).as_secs_f64())
                .collect::<Vec<f64>>()
        });
        SpeedscopeFile {
            // This is always the same
            schema: Schema::HttpsWwwSpeedscopeAppFileFormatSchemaJson,

            active_profile_index: None,

            name: Some("wasmprof".to_string()),

            exporter: Some(format!("wasmprof@{}", env!("CARGO_PKG_VERSION"))),

            profiles: vec![Profile {
                profile_type: ProfileType::Sampled,

                name: "wasmprof profile".to_string(),

                unit: FileFormatValueUnit::Seconds,

                start_value: 0.0,
                end_value: end_value as f64,

                samples: Some(samples),
                weights: match weights {
                    Some(w) => Some(w),
                    None => Some(vec![1.0; samples_len]),
                },
                events: None,
            }],

            shared: Shared {
                frames: frames
                    .iter()
                    .map(|name| FileFormatFrame {
                        name: name.to_string(),
                        col: None,
                        file: None,
                        line: None,
                    })
                    .collect(),
            },
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Serialize, Deserialize)]
pub struct Profile {
    #[serde(rename = "endValue")]
    end_value: f64,
    events: Option<Vec<Event>>,
    name: String,
    #[serde(rename = "startValue")]
    start_value: f64,
    #[serde(rename = "type")]
    profile_type: ProfileType,
    unit: FileFormatValueUnit,
    samples: Option<Vec<Vec<usize>>>,
    weights: Option<Vec<f64>>,
}

#[derive(Serialize, Deserialize)]
pub struct Event {
    at: f64,
    frame: f64,
    #[serde(rename = "type")]
    event_type: EventType,
}

#[derive(Serialize, Deserialize)]
pub struct Shared {
    frames: Vec<FileFormatFrame>,
}

#[derive(Serialize, Deserialize)]
pub struct FileFormatFrame {
    col: Option<f64>,
    file: Option<String>,
    line: Option<f64>,
    name: String,
}

#[derive(Serialize, Deserialize)]
pub enum EventType {
    C,
    O,
}

#[derive(Serialize, Deserialize)]
pub enum ProfileType {
    #[serde(rename = "evented")]
    Evented,
    #[serde(rename = "sampled")]
    Sampled,
}

#[derive(Serialize, Deserialize)]
pub enum FileFormatValueUnit {
    #[serde(rename = "bytes")]
    Bytes,
    #[serde(rename = "microseconds")]
    Microseconds,
    #[serde(rename = "milliseconds")]
    Milliseconds,
    #[serde(rename = "nanoseconds")]
    Nanoseconds,
    #[serde(rename = "none")]
    None,
    #[serde(rename = "seconds")]
    Seconds,
}

#[derive(Serialize, Deserialize)]
pub enum Schema {
    #[serde(rename = "https://www.speedscope.app/file-format-schema.json")]
    HttpsWwwSpeedscopeAppFileFormatSchemaJson,
}
