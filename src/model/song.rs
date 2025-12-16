use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Note {
    pub midi: u8,
    pub velocity: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Event {
    pub note: Note,
    pub time_ms: f64,
    pub duration_ms: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Metadata {
    pub title: Option<String>,
    pub tempo_bpm: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Song {
    pub metadata: Metadata,
    pub events: Vec<Event>,
}
