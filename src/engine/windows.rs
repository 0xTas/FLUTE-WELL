use crate::engine::Engine;
use crate::model::mapper::Input;
use anyhow::Result;
use log::{debug, error};
use std::convert::TryInto;
use std::mem::size_of;
use std::ptr::null_mut;
use std::sync::Arc;
use std::time::Duration;
use windows::Win32::System::Threading::{
    GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_HIGHEST,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_SCANCODE, SendInput, VIRTUAL_KEY,
};

#[derive(Clone, Debug)]
pub struct WindowsEngine {
    // nothing stateful for now; kept as Arc in scheduler for shared usage
    pub elevate_thread_priority: bool,
}

impl WindowsEngine {
    pub fn new() -> Self {
        Self {
            elevate_thread_priority: true,
        }
    }

    /// Helper to set the current thread priority higher for playback.
    /// Call this from the playback thread before starting the timing loop.
    pub fn set_playback_thread_high_priority(&self) {
        if !self.elevate_thread_priority {
            return;
        }

        unsafe {
            // GetCurrentThread and SetThreadPriority are used to raise priority.
            // We use THREAD_PRIORITY_HIGHEST (not time-critical) to avoid system starvation.
            let h = GetCurrentThread();
            let ok = SetThreadPriority(h, THREAD_PRIORITY_HIGHEST);

            if let Err(why) = ok {
                println!("Failed to set thread priority");
            } else {
                println!("Set playback thread priority to HIGHEST");
            }
        }
    }

    /// Build an INPUT array of INPUT entries for the provided VK keys as keydown (no KEYUP flag).
    fn build_keydown_inputs(combo: &Input) -> Vec<INPUT> {
        combo
            .keys
            .iter()
            .map(|&vk| {
                let ki = KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: KEYBD_EVENT_FLAGS(0), // keydown
                    time: 0,
                    dwExtraInfo: 0,
                };
                // Construct the union-bearing INPUT and set type and anonymous union field
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 { ki },
                }
            })
            .collect()
    }

    /// Build an INPUT array of KEYBDINPUT entries for keyup (KEYEVENTF_KEYUP flag).
    fn build_keyup_inputs(combo: &Input) -> Vec<INPUT> {
        combo
            .keys
            .iter()
            .map(|&vk| {
                let ki = KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                };
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 { ki },
                }
            })
            .collect()
    }

    /// Low-level wrapper around SendInput: sends a slice of INPUTs and checks the result.
    fn send_inputs_batch(inputs: &mut [INPUT]) -> Result<()> {
        unsafe {
            let sent = SendInput(inputs, size_of::<INPUT>() as i32);
            if sent == inputs.len() as u32 {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "SendInput failed: requested {}, sent {}",
                    inputs.len(),
                    sent
                ))
            }
        }
    }
}

impl Engine for WindowsEngine {
    fn key_down(&self, combo: &Input) -> Result<()> {
        // Build inputs for all keys in combo and send them in single SendInput call.
        let mut inputs = Self::build_keydown_inputs(combo);
        // Diagnostics: log which keys we will press.
        debug!(
            "WindowsBackend::key_down for {} => keys {:?}",
            combo.note_label, combo.keys
        );

        Self::send_inputs_batch(&mut inputs)
    }

    fn key_up(&self, combo: &Input) -> Result<()> {
        let mut inputs = Self::build_keyup_inputs(combo);
        debug!(
            "WindowsBackend::key_up for {} => keys {:?}",
            combo.note_label, combo.keys
        );

        Self::send_inputs_batch(&mut inputs)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::model::mapper::{
        DIR_1_RIGHT, DIR_3_DOWN, DIR_5_LEFT, DIR_7_UP, Input, MAPPINGS, PLAY_KEY,
    };
    use log::info;
    use std::time::Instant;
    use windows::Win32::UI::Input::KeyboardAndMouse::VK_1;

    #[test]
    fn smoke_press_play_key() {
        let engine = WindowsEngine::new();
        let input = Input {
            keys: &[PLAY_KEY],
            note_label: "test_play",
        };

        let now = Instant::now();

        println!("Checking that ANIMAL WELL is the active window...");

        loop {
            let active_window = active_win_pos_rs::get_active_window();

            if !active_window.is_ok() {
                continue;
            }

            let title = active_window.unwrap().title;

            println!("Active window: \"{}\"", title);
            if title == String::from("ANIMAL WELL") {
                break;
            } else {
                let elapsed = now.elapsed();
                if elapsed > Duration::from_secs(30) {
                    panic!("Active window title was never ANIMAL WELL..! (waited 30 seconds.)")
                }
            }

            spin_sleep::sleep(Duration::from_millis(50));
        }

        assert!(engine.key_press(&input, 2000, 0).is_ok());
    }

    #[test]
    fn play_full_range() {
        let engine = WindowsEngine::new();

        let mut delay_ms = 840;
        for n in 0..5 {
            if n > 0 {
                delay_ms /= 2;
            }
            for entry in MAPPINGS {
                let now = Instant::now();

                println!("Checking that ANIMAL WELL is the active window...");

                loop {
                    let active_window = active_win_pos_rs::get_active_window();

                    if !active_window.is_ok() {
                        continue;
                    }

                    let title = active_window.unwrap().title;

                    println!("Active window: \"{}\"", title);
                    if title == String::from("ANIMAL WELL") {
                        break;
                    } else {
                        let elapsed = now.elapsed();
                        if elapsed > Duration::from_secs(30) {
                            panic!(
                                "Active window title was never ANIMAL WELL..! (waited 30 seconds.)"
                            )
                        }
                    }

                    spin_sleep::sleep(Duration::from_millis(50));
                }

                println!("Playing note: \"{}\"", entry.1.note_label);
                assert!(engine.key_press(&entry.1, delay_ms, 0).is_ok());
            }
        }
    }

    #[test]
    fn play_teleport_tune() {
        let engine = WindowsEngine::new();
        let now = Instant::now();

        println!("Checking that ANIMAL WELL is the active window...");

        // todo: create better window handling (maybe in scheduler though)
        loop {
            let active_window = active_win_pos_rs::get_active_window();

            if !active_window.is_ok() {
                continue;
            }

            let title = active_window.unwrap().title;

            println!("Active window: \"{}\"", title);
            if title == String::from("ANIMAL WELL") {
                break;
            } else {
                let elapsed = now.elapsed();
                if elapsed > Duration::from_secs(30) {
                    panic!("Active window title was never ANIMAL WELL..! (waited 30 seconds.)")
                }
            }

            spin_sleep::sleep(Duration::from_millis(50));
        }

        let mut inputs: Vec<Input> = Vec::new();

        inputs.push(Input {
            keys: &[PLAY_KEY, DIR_1_RIGHT, VK_1],
            note_label: "A5",
        });
        inputs.push(Input {
            keys: &[PLAY_KEY, DIR_1_RIGHT, VK_1],
            note_label: "A5",
        });
        inputs.push(Input {
            keys: &[PLAY_KEY, DIR_5_LEFT, VK_1],
            note_label: "E6",
        });
        inputs.push(Input {
            keys: &[PLAY_KEY, DIR_5_LEFT, VK_1],
            note_label: "E6",
        });
        inputs.push(Input {
            keys: &[PLAY_KEY, DIR_3_DOWN, VK_1],
            note_label: "C#6",
        });
        inputs.push(Input {
            keys: &[PLAY_KEY, DIR_3_DOWN, VK_1],
            note_label: "C#6",
        });
        inputs.push(Input {
            keys: &[PLAY_KEY, DIR_7_UP, VK_1],
            note_label: "G#6",
        });
        inputs.push(Input {
            keys: &[PLAY_KEY, DIR_7_UP, VK_1],
            note_label: "G#6",
        });

        for input in inputs {
            println!("Playing note: \"{}\"", input.note_label);
            assert!(engine.key_press(&input, 150, 69).is_ok());
        }
    }
}
