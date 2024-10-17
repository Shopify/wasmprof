use serde::Serialize;
use std::collections::HashMap;
use crate::profile_data::{ProfileData};
use crate::WeightUnit;

#[derive(Serialize)]
pub struct SpeedscopeFile {
    #[serde(rename = "$schema")]
    schema: String,
    profiles: Vec<Profile>,
    shared: Shared,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    exporter: String,
    #[serde(rename = "activeProfileIndex")]
    active_profile_index: usize,
}

#[derive(Serialize)]
pub struct Profile {
    #[serde(rename = "type")]
    profile_type: String,
    name: String,
    unit: String,
    #[serde(rename = "startValue")]
    start_value: u128,
    #[serde(rename = "endValue")]
    end_value: u128,
    events: Vec<Event>,
}

#[derive(Serialize)]
struct Event {
    #[serde(rename = "type")]
    event_type: String,
    frame: usize,
    at: u128,
}

#[derive(Serialize)]
struct Shared {
    frames: Vec<Frame>,
}

#[derive(Serialize)]
pub struct Frame {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    col: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    module: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    func_index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    func_offset: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    module_offset: Option<usize>,
}

impl SpeedscopeFile {
    pub fn new(profile_data: &ProfileData, name: Option<String>) -> Self {
        let mut frame_map = HashMap::new();
        let mut shared_frames = Vec::new();

        for (index, frame) in profile_data.frames().iter().enumerate() {
            frame_map.insert(&frame.name, index);
            shared_frames.push(Frame {
                name: frame.name.clone(),
                file: frame.file.clone().or_else(|| frame.module.clone()),
                line: frame.line,
                col: frame.column,
                module: frame.module.clone(),
                func_index: Some(frame.func_index),
                func_offset: frame.func_offset,
                module_offset: frame.module_offset,
            });
        }

        let mut events = Vec::new();
        let mut current_value = 0;

        for (sample, &weight) in profile_data.samples().iter().zip(profile_data.weights().iter()) {
            for &frame in sample.iter().rev() {
                events.push(Event {
                    event_type: "O".to_string(),
                    frame,
                    at: current_value,
                });
            }
            current_value += weight;
            for &frame in sample.iter() {
                events.push(Event {
                    event_type: "C".to_string(),
                    frame,
                    at: current_value,
                });
            }
        }

        let unit = match profile_data.weight_unit() {
            WeightUnit::Nanoseconds => "nanoseconds",
            WeightUnit::Fuel => "fuel",
        };

        let profile = Profile {
            profile_type: "evented".to_string(),
            name: "CPU".to_string(),
            unit: unit.to_string(),
            start_value: 0,
            end_value: current_value,
            events,
        };

        SpeedscopeFile {
            schema: "https://www.speedscope.app/file-format-schema.json".to_string(),
            profiles: vec![profile],
            shared: Shared {
                frames: shared_frames,
            },
            name,
            exporter: "wasmprof".to_string(),
            active_profile_index: 0,
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}
