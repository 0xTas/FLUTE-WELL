use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Note {
    pub midi: u8,
    pub is_dotted: bool,
    pub articulation: Articulation,
    pub base_duration: BeatDuration,
}

impl Note {
    pub fn with_default_style(midi: u8, is_dotted: bool) -> Self {
        Note {
            midi, is_dotted,
            base_duration: BeatDuration::Whole,
            articulation: Articulation::Portamento
        }
    }

    pub fn new(midi: u8, is_dotted: bool, base_duration: BeatDuration, articulation: Articulation) -> Self {
        Note {
            midi,
            is_dotted,
            articulation,
            base_duration
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeatDuration {
    Whole, Half, Quarter, Eighth, Sixteenth, ThirtySecond
}

impl BeatDuration {
    fn transform(&self, hold_ms: f64) -> f64 {
        match self {
            BeatDuration::Whole => hold_ms * 4.0,
            BeatDuration::Half => hold_ms * 2.0,
            BeatDuration::Eighth => hold_ms * 0.5,
            BeatDuration::Sixteenth => hold_ms * 0.25,
            BeatDuration::ThirtySecond => hold_ms * 0.125,
            _ => hold_ms
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Articulation {
    Legato, Portamento, Staccato, Staccatissimo
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Event {
    pub bpm: f64,
    pub note: Note,
    pub hold_ms: u64,
    pub release_ms: u64,
}

impl Event {
    pub fn new(bpm: f64, note: Note) -> Self {
        let beat_ms = 60_000.0 / bpm;
        let subdivision = beat_ms / 4.0;
        let mut hold_ms = note.base_duration.transform(beat_ms);

        if note.is_dotted {
            hold_ms *= 1.5;
        }

        let mut release_ms = 0u64;
        match note.articulation {
            Articulation::Portamento => {
                hold_ms -= subdivision;
                release_ms = subdivision.round() as u64;
            }
            Articulation::Staccato => {
                hold_ms -= subdivision * 2.0;
                release_ms = (subdivision * 2.0).round() as u64;
            }
            Articulation::Staccatissimo => {
                hold_ms -= subdivision * 3.0;
                release_ms = (subdivision * 3.0).round() as u64;
            }
            _ => {}
        }

        let hold_ms = hold_ms.round().max(1.0) as u64;

        Event {
            bpm,
            note,
            hold_ms,
            release_ms
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Song {
    pub metadata: Metadata,
    pub events: Vec<Event>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Metadata {
    pub title: Option<String>,
    pub tempo_bpm: Option<f64>,
    pub tick_resolution: Option<u32>, // Used for NBS conversion
    pub calibration_ms: Option<i64>,  // todo: delete if possible
}
