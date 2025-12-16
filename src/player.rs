use crate::engine::InputEngine;
use crate::model::mappings::{Input, input_for_midi};
use crate::model::song::Song;
use anyhow::bail;
use log::{debug, info, warn};
use spin_sleep::{SpinSleeper, SpinStrategy};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

enum ControlMsg {
    Stop,
}

#[derive(Debug, Clone)]
pub struct ScheduledEvent {
    time_ms: f64,
    duration_ms: f64,
    input: &'static Input,
}

#[derive(Debug)]
pub struct Player<E: InputEngine> {
    delay: u64,
    verbose: bool,
    engine: Arc<E>,
    schedule: Mutex<Vec<ScheduledEvent>>,
    control_tx: Mutex<Option<Sender<ControlMsg>>>,
    worker_handle: Mutex<Option<JoinHandle<()>>>,
}

impl<E: InputEngine + 'static> Player<E> {
    pub fn new(engine: E, verbose: bool, delay: u64) -> Self {
        Self {
            delay,
            verbose,
            engine: Arc::new(engine),
            schedule: Mutex::new(Vec::new()),
            control_tx: Mutex::new(None),
            worker_handle: Mutex::new(None),
        }
    }

    pub fn load_song(&self, song: Song) -> anyhow::Result<()> {
        let mut events: Vec<ScheduledEvent> = Vec::new();

        for e in song.events.into_iter() {
            let midi = e.note.midi;
            let input = input_for_midi(midi);

            if let Some(input) = input {
                events.push(ScheduledEvent {
                    time_ms: e.time_ms,
                    duration_ms: e.duration_ms,
                    input,
                });
            } else {
                warn!(
                    "No mapping for MIDI {}: skipping event at {}ms..!",
                    midi, e.time_ms
                );
                continue;
            }
        }

        events.sort_by(|a, b| {
            a.time_ms
                .partial_cmp(&b.time_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let Ok(mut schedule_lock) = self.schedule.lock() else {
            bail!("Failed to lock the schedule..!");
        };
        *schedule_lock = events;

        info!(
            "Loaded song: '{}' with {} scheduled events..!",
            song.metadata.title.unwrap_or(String::from("No Title")),
            schedule_lock.len()
        );

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
            bail!("Failed to lock schedule..!")
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

        let delay = self.delay;
        let verbose = self.verbose;
        let handle = thread::spawn(move || {
            let ctrl_rx = rx;

            #[cfg(target_os = "windows")]
            {
                use windows::Win32::System::Threading::{
                    GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_HIGHEST,
                };
                unsafe {
                    let h = GetCurrentThread();
                    let ok = SetThreadPriority(h, THREAD_PRIORITY_HIGHEST);

                    if ok.is_ok() {
                        debug!("Playback thread priority set to HIGHEST..!");
                    } else {
                        warn!("Failed to set playback thread priority..!");
                    }
                }
            }

            let mut stamp = Instant::now();
            info!("Waiting at most 30 SECONDS for the active window to be ANIMAL WELL..!");

            loop {
                if ctrl_rx.try_recv().is_ok() {
                    warn!("Playback stopped during active window check..!");
                    return;
                }

                let active_window = active_win_pos_rs::get_active_window();

                if active_window.is_err() {
                    continue;
                }

                let title = active_window.expect("Active window should be Ok..!").title;

                debug!("Active window: \"{}\"", title);
                if title == "ANIMAL WELL" {
                    break;
                } else {
                    let elapsed = stamp.elapsed();
                    if elapsed > Duration::from_secs(30) {
                        panic!("Active window title was never ANIMAL WELL..!")
                    }
                }

                spin_sleep::sleep(Duration::from_millis(50));
            }

            let mut was_ok = true;
            info!(
                "Active window is ANIMAL WELL, starting playback {}..!",
                if delay > 0 {
                    format!("in {} seconds", delay)
                } else {
                    "now".to_owned()
                }
            );

            let sleeper = SpinSleeper::new(100_000).with_spin_strategy(SpinStrategy::YieldThread);

            if delay > 0 {
                sleeper.sleep(Duration::from_secs(delay));
            }

            let start = Instant::now();
            const MAX_SLEEP_CHUNK_S: f64 = 0.050;

            for event in schedule.into_iter() {
                if ctrl_rx.try_recv().is_ok() {
                    engine.all_keys_up().expect("Error cancelling input..!");
                    warn!(
                        "Playback stopped via control message after {} seconds..!",
                        start.elapsed().as_secs()
                    );
                    return;
                }

                let target = if event.time_ms < 0.0 {
                    start
                } else {
                    start + Duration::from_secs_f64(event.time_ms / 1000.0)
                };

                loop {
                    if ctrl_rx.try_recv().is_ok() {
                        engine.all_keys_up().expect("Error cancelling input..!");
                        warn!("Playback stopped during wait..!");
                        return;
                    }

                    let now = Instant::now();
                    if now >= target {
                        break;
                    }
                    let remaining = (target - now).as_secs_f64();

                    let chunk = if remaining > MAX_SLEEP_CHUNK_S {
                        MAX_SLEEP_CHUNK_S
                    } else {
                        remaining
                    };

                    sleeper.sleep(Duration::from_secs_f64(chunk));
                }

                loop {
                    if ctrl_rx.try_recv().is_ok() {
                        engine.all_keys_up().expect("Error cancelling input..!");
                        warn!("Playback stopped during active window check..!");
                        return;
                    }

                    let active_window = active_win_pos_rs::get_active_window();

                    if active_window.is_err() {
                        continue;
                    }

                    let title = active_window.expect("Active window should be Ok..!").title;

                    if title == "ANIMAL WELL" {
                        was_ok = true;
                        break;
                    } else {
                        if was_ok {
                            stamp = Instant::now();
                            engine.all_keys_up().expect("Error cancelling input..!");
                        }
                        let elapsed = stamp.elapsed();
                        if elapsed > Duration::from_secs(30) {
                            panic!("Active window title was never ANIMAL WELL..!")
                        }
                    }

                    spin_sleep::sleep(Duration::from_millis(50));
                }

                let emit_time = Instant::now();
                let emitted_at_ms = emit_time.duration_since(start).as_secs_f64() * 1000.0;

                if verbose {
                    let info = format!("Sending inputs for {} ", event.input.note_label);
                    info!(
                        "{:30} | at {:>13.3}ms | scheduled for: {:>13.3}ms | duration: {:>9.3}ms",
                        info, emitted_at_ms, event.time_ms, event.duration_ms
                    );
                }

                if let Err(why) =
                    engine.key_press(event.input, event.duration_ms, engine.get_articulation())
                {
                    warn!(
                        "Input error for {} at {:.3}ms | why: {:?}",
                        event.input.note_label, emitted_at_ms, why
                    );
                }
            }

            info!("Playback thread finished all events..!");
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
}

#[cfg(test)]
mod test {
    use log::warn;
    use crate::util::ensure_active_window;
    use crate::{import_midi_file, DefaultInputEngine, Event, Metadata, Note, Player, PolyPolicy, Song};

    #[test]
    fn mimic_cuckoo_clock() {
        env_logger::try_init().unwrap_or(());

        let art = 0.75;
        let engine = DefaultInputEngine::new(art);

        const E6: u8 = 88;
        const CS6: u8 = 85;
        const REST_MS: f64 = 1000.0;
        const DURATION_MS: f64 = 200.0;
        const NOTE_GROUPS: &[&[u8]] = &[
            &[E6, E6, CS6],
            &[E6, E6, CS6],
            &[E6, E6, CS6],
            &[E6, E6, CS6],
            &[E6, E6],
        ];

        let mut time_ms = 0.0;
        let mut raw_events: Vec<(u8, f64)> = Vec::new();
        for (n, group) in NOTE_GROUPS.iter().enumerate() {
            if n > 0 {
                time_ms += REST_MS;
            }

            for &midi in *group {
                raw_events.push((midi, time_ms));
                time_ms += DURATION_MS;
            }
        }

        let song = Song {
            metadata: Metadata {
                title: Some(String::from("Cuckoo Clock")),
                tempo_bpm: None
            },
            events: raw_events
                .iter()
                .map(|&(midi, start_time_ms)| Event {
                    note: Note {
                        midi,
                        velocity: 255,
                    },
                    time_ms: start_time_ms,
                    duration_ms: DURATION_MS,
                })
                .collect(),
        };

        let player = Player::new(engine, true, 0);

        ensure_active_window();
        assert!(player.load_song(song).is_ok());
        assert!(player.play(true).is_ok());
    }

    #[test]
    fn play_from_midi_file() {
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

        let art = 0.55;
        let engine = DefaultInputEngine::new(art);
        let player = Player::new(engine, true, 0);

        ensure_active_window();
        assert!(player.load_song(song.unwrap()).is_ok());
        assert!(player.play(true).is_ok());
    }
}
