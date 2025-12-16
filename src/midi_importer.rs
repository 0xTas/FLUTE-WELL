use crate::model::song::*;
use anyhow::{Result, anyhow};
use log::{debug, warn};
use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

const EPSILON_MS: f64 = 2.0;
const DEFAULT_MPQN: u32 = 500_000;
const MICROSECONDS_PER_MINUTE: f64 = 60_000_000.0;

/// Simple policy for converting polyphonic MIDI to a single monophonic flute line.
#[derive(Debug, Clone, Copy, Default)]
pub enum PolyPolicy {
    /// Pick the highest active pitch for a given set of overlapping events.
    #[default]
    Highest,

    /// Pick the lowest active pitch for a given set of overlapping events.
    Lowest,

    /// Pick the highest velocity note for a given set of overlapping events.
    Loudest,

    /// Pick notes exclusively from the track with the highest overall note density.
    Densest,
}

struct NoteInterval {
    pub midi: u8,
    pub start_tick: u64,
    pub end_tick: u64,
    pub velocity: u8,
    pub _channel: u8,
}

#[derive(Debug, Clone)]
struct TempoSegment {
    pub mpqn: u32,
    pub start_tick: u64,
    pub ms_at_start: f64,
}

#[derive(Debug, Clone)]
struct Point {
    time_ms: f64,
    is_start: bool,
    midi: u8,
    velocity: u8,
    duration_ms: f64,
}

pub fn import_midi_file<P: AsRef<Path>>(
    path: P,
    transpose_semitones: i32,
    policy: PolyPolicy,
    merge: bool,
    clip_to_range: Option<(u8, u8)>,
) -> Result<Song> {
    let bytes = fs::read(path.as_ref()).map_err(|e| {
        anyhow!(
            "Failed to read MIDI file {}: {}",
            path.as_ref().display(),
            e
        )
    })?;

    midi_bytes_to_song(
        &bytes,
        path.as_ref(),
        transpose_semitones,
        policy,
        merge,
        clip_to_range,
    )
}

fn midi_bytes_to_song(
    bytes: &[u8],
    source_path: &Path,
    transpose_semitones: i32,
    policy: PolyPolicy,
    merge: bool,
    clip_to_range: Option<(u8, u8)>,
) -> Result<Song> {
    let smf = Smf::parse(bytes).map_err(|e| anyhow!("Failed to parse MIDI: {:?}", e))?;

    let ticks_per_quarter = match smf.header.timing {
        Timing::Metrical(t) => t.as_int() as u64,
        Timing::Timecode(_fps, _subframe) => {
            return Err(anyhow!(
                "SMPTE timecode midi timing is not currently supported..!"
            ));
        }
    };

    let mut track_name = String::new();

    debug!("Ticks per quarter note: {}", ticks_per_quarter);
    debug!(
        "MIDI format: {:?}, tracks: {}",
        smf.header.format,
        smf.tracks.len()
    );

    let mut tempo_changes: Vec<(u64, u32)> = Vec::new();
    tempo_changes.push((0u64, DEFAULT_MPQN)); // default tempo to ~120bpm until a tempo meta appears

    let mut intervals: Vec<NoteInterval> = Vec::new();
    let mut open_notes: HashMap<(u8, u8), Vec<(u64, u8)>> = HashMap::new();

    for (track_idx, track) in smf.tracks.iter().enumerate() {
        let mut abs_tick: u64 = 0;
        for event in track.iter() {
            abs_tick = abs_tick.saturating_add(event.delta.as_int() as u64);

            match &event.kind {
                TrackEventKind::Meta(meta) => match meta {
                    MetaMessage::Tempo(micro) => {
                        let mpqn: u32 = micro.as_int();
                        tempo_changes.push((abs_tick, mpqn));
                        debug!(
                            "Tempo change at tick {} -> {} us/qn (track {})",
                            abs_tick, mpqn, track_idx
                        );
                    }
                    MetaMessage::TrackName(bytes) => {
                        if track_name.is_empty() {
                            track_name = String::from_utf8(bytes.to_vec())?;
                            debug!("Track name: {}", track_name);
                        }
                    }
                    _ => {}
                },
                TrackEventKind::Midi { channel, message } => {
                    let ch: u8 = channel.as_int();

                    match message {
                        MidiMessage::NoteOn { key, vel } => {
                            let velocity: u8 = vel.as_int();

                            if velocity == 0 {
                                close_note(
                                    &mut open_notes,
                                    &mut intervals,
                                    ch,
                                    key.as_int(),
                                    abs_tick,
                                );
                            } else {
                                open_notes
                                    .entry((ch, key.as_int()))
                                    .or_default()
                                    .push((abs_tick, velocity));
                            }
                        }
                        MidiMessage::NoteOff { key, vel: _ } => {
                            close_note(&mut open_notes, &mut intervals, ch, key.as_int(), abs_tick);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    let last_tick_estimate = intervals
        .iter()
        .map(|interval| interval.end_tick)
        .max()
        .unwrap_or(0)
        .max(
            tempo_changes
                .iter()
                .map(|(tempo, _)| *tempo)
                .max()
                .unwrap_or(0),
        );

    for ((ch, key), stack) in open_notes.into_iter() {
        for (start_tick, start_vel) in stack {
            let end_tick = if last_tick_estimate > start_tick {
                last_tick_estimate
            } else {
                start_tick + ticks_per_quarter
            };

            intervals.push(NoteInterval {
                midi: key,
                start_tick,
                end_tick,
                velocity: start_vel,
                _channel: ch,
            });

            warn!(
                "Unclosed NoteOn for {}, channel: {} at tick: {} auto-closing at: {}..!",
                key, ch, start_tick, end_tick
            );
        }
    }

    let mut last_tick: u64 = 0;
    let mut ms_accum: f64 = 0.0;
    let mut last_mpqn: u32 = DEFAULT_MPQN;
    let mut tempo_segments: Vec<TempoSegment> = Vec::new();

    tempo_changes.sort_unstable_by_key(|(tick, _)| *tick);

    for (tick, mpqn) in tempo_changes.into_iter() {
        if tick < last_tick {
            continue;
        }

        if tick > last_tick {
            let delta_ticks = (tick - last_tick) as f64;
            ms_accum += delta_ticks * (last_mpqn as f64) / (ticks_per_quarter as f64) / 1000.0;
        }

        // ms_at_start reflects the ms accumulated up to this tick
        tempo_segments.push(TempoSegment {
            start_tick: tick,
            mpqn,
            ms_at_start: ms_accum,
        });

        last_tick = tick;
        last_mpqn = mpqn;
    }

    let ticks_to_ms = |tick: u64| -> f64 {
        if tempo_segments.is_empty() {
            // default 120bpm
            return (tick as f64) * DEFAULT_MPQN as f64 / (ticks_per_quarter as f64) / 1000.0;
        }

        let segment = match tempo_segments.iter().rfind(|seg| seg.start_tick <= tick) {
            Some(s) => s,
            None => &tempo_segments[0],
        };

        let delta_ticks = (tick - segment.start_tick) as f64;
        segment.ms_at_start
            + delta_ticks * (segment.mpqn as f64) / (ticks_per_quarter as f64) / 1000.0
    };

    let mut raw_events: Vec<Event> = Vec::new();
    for interval in intervals.into_iter() {
        let mut note_id = interval.midi as i32 + transpose_semitones;

        if let Some((min_id, max_id)) = clip_to_range {
            let min_id = min_id as i32;
            let max_id = max_id as i32;

            let mut attempts = 0;
            while (note_id < min_id || note_id > max_id) && attempts < 8 {
                if note_id < min_id {
                    note_id += 12;
                } else if note_id > max_id {
                    note_id -= 12;
                }
                attempts += 1;
            }

            if note_id < min_id || note_id > max_id {
                warn!(
                    "Dropping note {} (during octave transpose) as it was not in range [{}..={}]..!",
                    interval.midi, min_id, max_id
                );
                continue;
            }
        }

        if !(0..=127).contains(&note_id) {
            warn!("Dropping out-of-range MIDI {} after transpose..!", note_id);
            continue;
        }

        let start_ms = ticks_to_ms(interval.start_tick);
        let end_ms = ticks_to_ms(interval.end_tick);

        if end_ms <= start_ms {
            debug!(
                "Skipping zero/negative duration midi note {}, start: {} end: {}..!",
                interval.midi, start_ms, end_ms
            );
            continue;
        } else if end_ms - start_ms < EPSILON_MS {
            warn!(
                "Culling a tiny event to prevent audible artifacting..! Duration: {}ms",
                end_ms - start_ms
            );
            continue;
        }

        let event = Event {
            note: Note {
                midi: note_id as u8,
                velocity: interval.velocity,
            },
            time_ms: start_ms,
            duration_ms: end_ms - start_ms,
        };

        raw_events.push(event);
    }

    raw_events.sort_by(|a, b| a.time_ms.total_cmp(&b.time_ms));

    let final_events = reduce_to_monophonic(raw_events, policy, merge)
        .into_iter()
        .filter(|event| {
            if event.duration_ms < EPSILON_MS {
                warn!(
                    "Culling final event with a duration below the allowed epsilon..! [{:.3}ms]",
                    event.duration_ms
                );
                return false;
            }
            true
        })
        .collect::<Vec<_>>();

    // skipping first segment because it was built from our default mpqn
    let tempo_bpm = if let Some(tempo) = tempo_segments.get(1) {
        Some(MICROSECONDS_PER_MINUTE / (tempo.mpqn as f64))
    } else {
        Some(MICROSECONDS_PER_MINUTE / (DEFAULT_MPQN as f64))
    };

    let song = Song {
        metadata: Metadata {
            title: source_path
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string()),
            tempo_bpm,
        },
        events: final_events,
    };

    Ok(song)
}

fn close_note(
    open_notes: &mut HashMap<(u8, u8), Vec<(u64, u8)>>,
    intervals: &mut Vec<NoteInterval>,
    ch: u8,
    midi_num: u8,
    abs_tick: u64,
) {
    if let Some(stack) = open_notes.get_mut(&(ch, midi_num)) {
        if let Some((start_tick, start_vel)) = stack.pop() {
            intervals.push(NoteInterval {
                midi: midi_num,
                start_tick,
                end_tick: abs_tick,
                velocity: start_vel,
                _channel: ch,
            });
        } else {
            debug!(
                "Orphaned NoteOff for {} ch{} at tick {}..!",
                midi_num, ch, abs_tick
            );
        }
    } else {
        debug!(
            "Orphaned NoteOff for {} ch{} at tick {}..!",
            midi_num, ch, abs_tick
        );
    }
}

/// Given a possibly-overlapping set of events, reduce to a single monophonic sequence according
/// to the specified policy. The events emitted by this function should not overlap.
///
/// Basic approach: create a sorted set of time points where something changes (start or end), and
/// at each point decide which note should be active using the policy.
fn reduce_to_monophonic(events: Vec<Event>, policy: PolyPolicy, merge: bool) -> Vec<Event> {
    if events.is_empty() {
        return events;
    }

    let mut points: Vec<Point> = Vec::new();
    for ev in events.into_iter() {
        points.push(Point {
            time_ms: ev.time_ms,
            is_start: true,
            midi: ev.note.midi,
            velocity: ev.note.velocity,
            duration_ms: ev.duration_ms,
        });
        points.push(Point {
            time_ms: ev.time_ms + ev.duration_ms,
            is_start: false,
            midi: ev.note.midi,
            velocity: ev.note.velocity,
            duration_ms: ev.duration_ms,
        });
    }

    // Events with `is_start == false` come before `is_start == true` for the same time_ms,
    // so that notes ending at some time_ms `t` will not be counted active for another event starting at the same `t`.
    points.sort_by(|a, b| {
        a.time_ms
            .partial_cmp(&b.time_ms)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| (a.is_start as u8).cmp(&(b.is_start as u8)))
    });

    let mut result: Vec<Event> = Vec::new();
    let mut current_note: Option<u8> = None;
    let mut current_start: Option<f64> = None;
    let mut active: BTreeMap<u8, f64> = BTreeMap::new();
    let mut note_velocity_lookup: HashMap<u8, u8> = HashMap::new();

    let mut reduced = false;
    for pt in points.into_iter() {
        if pt.is_start {
            note_velocity_lookup.insert(pt.midi, pt.velocity);
            active.insert(pt.midi, pt.time_ms + pt.duration_ms);
        } else {
            active.remove(&pt.midi);
            note_velocity_lookup.remove(&pt.midi);
        }

        let chosen: Option<u8> = match policy {
            PolyPolicy::Highest => active.keys().next_back().copied(),
            PolyPolicy::Lowest => active.keys().next().copied(),
            PolyPolicy::Loudest => active
                .keys()
                .filter_map(|note| note_velocity_lookup.get(note).map(|&vel| (vel, *note)))
                .max_by_key(|(vel, _)| *vel)
                .map(|(_, note)| note),
            PolyPolicy::Densest => {
                todo!("Not yet implemented..!");
            }
        };

        if active.len() > 1 && !reduced {
            reduced = true;
            warn!(
                "MIDI contains multiple overlapping events, so reducing to monophony according to the chosen policy [{:?}]...",
                policy
            );
        }

        if chosen != current_note {
            if let (Some(cn), Some(cs)) = (current_note, current_start)
                && pt.time_ms > cs + EPSILON_MS
            {
                result.push(Event {
                    note: Note {
                        midi: cn,
                        velocity: pt.velocity,
                    },
                    time_ms: cs,
                    duration_ms: pt.time_ms - cs,
                });
            }

            if let Some(ch) = chosen {
                current_note = Some(ch);
                current_start = Some(pt.time_ms);
            } else {
                current_note = None;
                current_start = None;
            }
        }
    }

    let mut n = 0;
    let mut merged: Vec<Event> = Vec::new();
    for ev in result.into_iter() {
        if let Some(last) = merged.last_mut()
            && merge
            && last.note == ev.note
            && ((last.time_ms + last.duration_ms) - ev.time_ms).abs() <= EPSILON_MS
        {
            n += 1;
            let new_end = (last.time_ms + last.duration_ms).max(ev.time_ms + ev.duration_ms);
            last.duration_ms = new_end - last.time_ms;
            continue;
        }

        merged.push(ev);
    }

    if merge && n > 0 {
        warn!(
            "Merged {} consecutive timeline event(s) during monophonic reduction..!",
            n
        );
    }

    merged
}

#[cfg(test)]
mod test {
    use super::*;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() <= EPSILON_MS
    }

    fn create_event(midi: u8, velocity: u8, start: f64, dur: f64) -> Event {
        Event {
            note: Note { midi, velocity },
            time_ms: start,
            duration_ms: dur,
        }
    }

    #[test]
    fn midi_file_import() {
        env_logger::try_init().unwrap_or(());

        let song = import_midi_file(
            "./resources/songs/Twinkle_Twinkle_Little_Star.mid",
            0,
            PolyPolicy::Highest,
            false,
            Some((69, 93)),
        );

        if song.is_err() {
            warn!("{:?}", song);
        }

        assert!(song.is_ok());
        assert_eq!(song.unwrap().events.len(), 42);
    }

    #[test]
    fn midi_semitone_transpose() {
        env_logger::try_init().unwrap_or(());

        let transpose = 2;
        let song_default = import_midi_file(
            "./resources/songs/Twinkle_Twinkle_Little_Star.mid",
            0,
            PolyPolicy::Highest,
            false,
            Some((69, 93)),
        );
        let song_transposed = import_midi_file(
            "./resources/songs/Twinkle_Twinkle_Little_Star.mid",
            transpose,
            PolyPolicy::Highest,
            false,
            Some((69, 93)),
        );

        if song_default.is_err() {
            warn!("{:?}", song_default);
        }
        if song_transposed.is_err() {
            warn!("{:?}", song_transposed);
        }

        assert!(song_default.is_ok());
        assert!(song_transposed.is_ok());
        let events_default = song_default.unwrap().events;
        let events_transposed = song_transposed.unwrap().events;

        assert_eq!(events_default.len(), 42);
        assert_eq!(events_transposed.len(), 42);

        assert_eq!(events_default.first().unwrap().note.midi, 69);
        assert_eq!(events_default.last().unwrap().note.midi, 69);

        assert_eq!(events_transposed.first().unwrap().note.midi, 71);
        assert_eq!(events_transposed.last().unwrap().note.midi, 71);
    }

    #[test]
    fn midi_octave_transpose() {
        env_logger::try_init().unwrap_or(());

        let range = 45..=69;
        let transpose = (*range.start(), *range.end());

        let song = import_midi_file(
            "./resources/songs/Twinkle_Twinkle_Little_Star.mid",
            0,
            PolyPolicy::Highest,
            false,
            Some(transpose),
        );

        if song.is_err() {
            warn!("{:?}", song);
        }

        assert!(song.is_ok());
        let events = song.unwrap().events;

        assert_eq!(events.len(), 42);
        assert!(
            events
                .iter()
                .map(|e| e.note.midi)
                .all(|midi| range.contains(&midi))
        );
    }

    #[test]
    fn highest_policy_overlap() {
        env_logger::try_init().unwrap_or(());

        let input = vec![
            create_event(69, 255, 0.0, 1000.0),
            create_event(77, 255, 500.0, 1000.0),
        ];

        let out = reduce_to_monophonic(input, PolyPolicy::Highest, false);
        assert_eq!(out.len(), 2);

        assert_eq!(out[0].note.midi, 69);
        assert!(approx_eq(out[0].time_ms, 0.0));
        assert!(approx_eq(out[0].duration_ms, 500.0));

        assert_eq!(out[1].note.midi, 77);
        assert!(approx_eq(out[1].time_ms, 500.0));
        assert!(approx_eq(out[1].duration_ms, 1000.0));
    }

    #[test]
    fn lowest_policy_overlap() {
        env_logger::try_init().unwrap_or(());

        let input = vec![
            create_event(77, 255, 0.0, 1000.0),
            create_event(69, 255, 500.0, 1000.0),
        ];

        let out = reduce_to_monophonic(input, PolyPolicy::Lowest, false);
        assert_eq!(out.len(), 2);

        assert_eq!(out[0].note.midi, 77);
        assert!(approx_eq(out[0].time_ms, 0.0));
        assert!(approx_eq(out[0].duration_ms, 500.0));

        assert_eq!(out[1].note.midi, 69);
        assert!(approx_eq(out[1].time_ms, 500.0));
        assert!(approx_eq(out[1].duration_ms, 1000.0));
    }

    #[test]
    fn loudest_policy_overlap() {
        env_logger::try_init().unwrap_or(());

        let input = vec![
            create_event(77, 128, 0.0, 1000.0),
            create_event(69, 255, 500.0, 1000.0),
        ];

        let out = reduce_to_monophonic(input, PolyPolicy::Loudest, false);
        assert_eq!(out.len(), 2);

        assert_eq!(out[0].note.midi, 77);
        assert!(approx_eq(out[0].time_ms, 0.0));
        assert!(approx_eq(out[0].duration_ms, 500.0));

        assert_eq!(out[1].note.midi, 69);
        assert!(approx_eq(out[1].time_ms, 500.0));
        assert!(approx_eq(out[1].duration_ms, 1000.0));
    }

    #[test]
    fn densest_policy_overlap() {
        todo!("Take events exclusively from the midi track with the highest note density.")
    }

    #[test]
    fn merge_adjacent_within_epsilon() {
        env_logger::try_init().unwrap_or(());

        let input = vec![
            create_event(60, 255, 0.0, 500.0),
            create_event(60, 255, 501.0, 500.0),
        ];

        let out = reduce_to_monophonic(input, PolyPolicy::Lowest, true);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].note.midi, 60);

        assert!(approx_eq(out[0].time_ms, 0.0));
        assert!((out[0].duration_ms - 1001.0).abs() <= EPSILON_MS);
    }

    #[test]
    fn cull_insufficient_length() {
        env_logger::try_init().unwrap_or(());

        let input = vec![
            create_event(61, 255, 0.0, 150.0),
            create_event(61, 255, 150.0, 1.337),
            create_event(61, 255, 155.0, 1.937),
            create_event(61, 255, 160.0, EPSILON_MS),
        ];

        let out = reduce_to_monophonic(input, PolyPolicy::Highest, true);
        assert!(
            out.iter()
                .all(|e| !(e.note.midi == 61 && e.duration_ms.abs() <= EPSILON_MS))
        );
    }
}
