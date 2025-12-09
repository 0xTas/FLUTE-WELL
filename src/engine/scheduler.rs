use crate::engine::Engine;
use crate::model::song::Song;
use std::sync::{mpsc, Arc, Mutex};
use std::sync::mpsc::Sender;
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use anyhow::{anyhow, bail};
use log::{debug, info, warn};
use crate::engine::windows::WindowsEngine;
use crate::model::mapper::{input_for_midi, Input, PLAY_KEY};

#[derive(Debug)]
pub struct Scheduler {
    engine: Arc<WindowsEngine>,

    schedule: Mutex<Vec<Event>>,

    calibration_offset_ms: Mutex<i64>, // todo: delete if possible

    control_tx: Mutex<Option<Sender<ControlMsg>>>,

    worker_handle: Mutex<Option<JoinHandle<()>>>,
}

impl Scheduler {
    pub fn new(engine: WindowsEngine) -> Self {
        Self {
            engine: Arc::new(engine),
            schedule: Mutex::new(Vec::new()),
            calibration_offset_ms: Mutex::new(0),
            control_tx: Mutex::new(None),
            worker_handle: Mutex::new(None),
        }
    }

    pub fn load_song(&self, song: Song) -> anyhow::Result<()> {
        let mut events: Vec<Event> = Vec::new();

        for e in song.events.into_iter() {
            let midi = e.note.midi;
            let input = input_for_midi(midi);

            if let Some(input) = input {
                let t_down = e.hold_ms;
                let t_up = e.release_ms;

                events.push(
                    Event {
                        hold_ms: t_down,
                        release_ms: t_up,
                        input
                    });
                // events.push(
                //     Event {
                //         time_ms: t_up,
                //         is_key_down: false,
                //         input
                //     });
            } else {
                warn!("No mapping for MIDI {}: skipping event at {}ms", midi, e.hold_ms);
                continue;
            }
        }

        let Ok(mut schedule_lock) = self.schedule.lock() else {
            bail!("Failed to acquire scheduler lock..!");
        };
        *schedule_lock = events;

        println!("Loaded song: \"{:?}\" with {} scheduled events..!", song.metadata.title.unwrap_or(String::from("No Title")), schedule_lock.len());

        Ok(())
    }

    pub fn play(&self, join: bool) -> anyhow::Result<()> {
        {
            let Ok(guard) = self.worker_handle.lock() else {
                bail!("Failed to lock worker handle..!")
            };

            if guard.is_some() {
                bail!("Playback already running..!")
            }
        }

        let Ok(schedule) = self.schedule.lock() else {
            return Err(anyhow!("Failed to lock schedule..!"));
        };

        let schedule = schedule.clone();

        if schedule.is_empty() {
            bail!("No song loaded..!")
        }

        let engine = Arc::clone(&self.engine);
        let (tx, rx) = mpsc::channel::<ControlMsg>();

        {
            let Ok(mut ctl) = self.control_tx.lock() else {
                bail!("Failed to lock control_tx..!")
            };

            *ctl = Some(tx);
        }

        let handle = thread::spawn(move || {
            let ctrl_rx = rx;

            #[cfg(target_os = "windows")]
            {
                use windows::Win32::System::Threading::{
                    GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_HIGHEST
                };
                unsafe {
                    let h = GetCurrentThread();
                    let ok = SetThreadPriority(h, THREAD_PRIORITY_HIGHEST);

                    if ok.is_ok() {
                        debug!("Playback thread priority set to HIGHEST..1");
                    } else {
                        warn!("Failed to set playback thread priority..!");
                    }
                }
            }

            let stamp = Instant::now();
            println!("Waiting at most 30 SECONDS for the active window to be ANIMAL WELL..!");

            loop {
                let active_window = active_win_pos_rs::get_active_window();

                if !active_window.is_ok() {
                    continue;
                }

                let title = active_window.expect("Active window should be Ok..!").title;

                debug!("Active window: \"{}\"", title);
                if title == String::from("ANIMAL WELL") {
                    break;
                } else {
                    let elapsed = stamp.elapsed();
                    if elapsed > Duration::from_secs(30) {
                        panic!("Active window title was never ANIMAL WELL..!")
                    }
                }

                spin_sleep::sleep(Duration::from_millis(50));
            }

            println!("Active window is ANIMAL WELL, starting playback now..!");

            let start = Instant::now();

            for event in schedule.into_iter() {
                if ctrl_rx.try_recv().is_ok() {
                    println!("Playback stopped via control message after {} seconds", start.elapsed().as_secs());
                    return;
                }

                const MAX_SLEEP_CHUNK_MS: u64 = 50;
                const PLAY_KEY_INPUT: Input = Input { keys: &[PLAY_KEY], note_label: "play_key" };

                if ctrl_rx.try_recv().is_ok() {
                    println!("Playback stopped during wait");
                    return;
                }

                let now = Instant::now();

                if let Err(why) = engine.key_press(
                    &event.input, event.hold_ms, event.release_ms
                ) {
                    println!("Failed to process key: {:?}", why);
                }

                let emit_time = Instant::now();
                println!("Sent key_down for {} at {:?}.", event.input.note_label, emit_time);
            }

            println!("Playback thread finished all events..!");
        });

        if join {
            handle.join().unwrap();
        } else {
            let Ok(mut wh) = self.worker_handle.lock() else {
                bail!("Failed to lock worker handle..!")
            };

            *wh = Some(handle);
        }

        Ok(())
    }

    pub fn stop(&self) -> anyhow::Result<()> {
        let tx = {
            let Ok(mut lock) = self.control_tx.lock() else {
                bail!("Failed to lock control_tx..!")
            };
            lock.take()
        };

        if let Some(tx) = tx {
            let _ = tx.send(ControlMsg::Stop);
        } else {
            bail!("No worker is running playback..!")
        }

        let Ok(mut lock) = self.worker_handle.lock() else {
            bail!("Failed to lock worker_handle..!")
        };

        if let Some(handle) = lock.take() {
            let _ = handle.join();
            debug!("Playback thread joined..!");
            info!("Stopped playback thread..!");
        }

        Ok(())
    }

    pub fn set_calibration_offset(&self, offset_ms: i64) {
        let Ok(mut lock) = self.calibration_offset_ms.lock() else {
            warn!("Failed to lock calibration offset for modification..!");
            return;
        };

        *lock = offset_ms;
    }
}

#[derive(Debug, Clone)]
pub struct Event {
    hold_ms: u64,
    release_ms: u64,
    input: &'static Input,
}

enum ControlMsg {
    Stop
}

mod test {
    use crate::engine::windows::WindowsEngine;
    use crate::model::song::{Articulation, BeatDuration, Event, Metadata, Note, Song};
    use crate::engine::scheduler::Scheduler;

    fn make_twinkle_song() -> Song {
        let tempo_bpm = 100.0;
        let mut e = |midi: u8, is_dotted: bool, duration: BeatDuration, articulation: Articulation| {
            let note = Note::new(midi, is_dotted, duration, articulation);
            let ev = Event::new(tempo_bpm, note);

            ev
        };

        // Transposed melody in A major / A tonic (notes in MIDI numbers):
        // Phrase 1: A A E E F# F# E
        // Phrase 2: D D C# C# B B A
        // Phrase 3: E E D D C# C# B
        // Phrase 4: A A E E F# F# E  D D C# C# B B A  (final repeat)
        //
        // Using semitone offsets from A4 (69):
        // A  = 69
        // B  = 71
        // C# = 73
        // D  = 74
        // E  = 76
        // F# = 78

        let events = vec![
            // Phrase 1
            e(69, false, BeatDuration::Quarter, Articulation::Portamento), // A
            e(69, false, BeatDuration::Quarter, Articulation::Portamento), // A
            e(76, false, BeatDuration::Quarter, Articulation::Portamento), // E
            e(76, false, BeatDuration::Quarter, Articulation::Portamento), // E
            e(78, false, BeatDuration::Quarter, Articulation::Portamento), // F#
            e(78, false, BeatDuration::Quarter, Articulation::Portamento), // F#
            e(76, false, BeatDuration::Half, Articulation::Legato), // E

            // Phrase 2
            e(74, false, BeatDuration::Quarter, Articulation::Portamento), // D
            e(74, false, BeatDuration::Quarter, Articulation::Portamento), // D
            e(73, false, BeatDuration::Quarter, Articulation::Portamento), // C#
            e(73, false, BeatDuration::Quarter, Articulation::Portamento), // C#
            e(71, false, BeatDuration::Quarter, Articulation::Portamento), // B
            e(71, false, BeatDuration::Quarter, Articulation::Portamento), // B
            e(69, false, BeatDuration::Half, Articulation::Legato), // A

            // Phrase 3
            e(76, false, BeatDuration::Quarter, Articulation::Portamento), // E
            e(76, false, BeatDuration::Quarter, Articulation::Portamento), // E
            e(74, false, BeatDuration::Quarter, Articulation::Portamento), // D
            e(74, false, BeatDuration::Quarter, Articulation::Portamento), // D
            e(73, false, BeatDuration::Quarter, Articulation::Portamento), // C#
            e(73, false, BeatDuration::Quarter, Articulation::Portamento), // C#
            e(71, false, BeatDuration::Half, Articulation::Legato), // B

            // Repeat Phrase 3
            e(76, false, BeatDuration::Quarter, Articulation::Portamento), // E
            e(76, false, BeatDuration::Quarter, Articulation::Portamento), // E
            e(74, false, BeatDuration::Quarter, Articulation::Portamento), // D
            e(74, false, BeatDuration::Quarter, Articulation::Portamento), // D
            e(73, false, BeatDuration::Quarter, Articulation::Portamento), // C#
            e(73, false, BeatDuration::Quarter, Articulation::Portamento), // C#
            e(71, false, BeatDuration::Half, Articulation::Legato), // B

            // Repeat Phrase 1
            e(69, false, BeatDuration::Quarter, Articulation::Portamento), // A
            e(69, false, BeatDuration::Quarter, Articulation::Portamento), // A
            e(76, false, BeatDuration::Quarter, Articulation::Portamento), // E
            e(76, false, BeatDuration::Quarter, Articulation::Portamento), // E
            e(78, false, BeatDuration::Quarter, Articulation::Portamento), // F#
            e(78, false, BeatDuration::Quarter, Articulation::Portamento), // F#
            e(76, false, BeatDuration::Half, Articulation::Legato), // E

            // Repeat Phrase 2
            e(74, false, BeatDuration::Quarter, Articulation::Portamento), // D
            e(74, false, BeatDuration::Quarter, Articulation::Portamento), // D
            e(73, false, BeatDuration::Quarter, Articulation::Portamento), // C#
            e(73, false, BeatDuration::Quarter, Articulation::Portamento), // C#
            e(71, false, BeatDuration::Quarter, Articulation::Portamento), // B
            e(71, false, BeatDuration::Quarter, Articulation::Portamento), // B
            e(69, false, BeatDuration::Half, Articulation::Legato), // A
        ];

        Song {
            metadata: Metadata {
                title: Some("Twinkle Twinkle Little Star (in A)".to_string()),
                tempo_bpm: Some(tempo_bpm),
                tick_resolution: None,
                calibration_ms: None,
            },
            events,
        }
    }

    #[test]
    fn play_song() {
        let engine = WindowsEngine::new();
        let scheduler = Scheduler::new(engine);

        if let Err(why) = scheduler.load_song(make_twinkle_song()) {
            panic!("Error loading song: {:?}", why);
        }

        println!("Playing song..!");

        if let Err(why) = scheduler.play(true) {
            panic!("Error playing song: {:?}", why);
        }
    }
}
