use crate::engine::InputEngine;
use crate::model::mappings::Input;
use anyhow::Result;
use log::debug;
use spin_sleep::{SpinSleeper, SpinStrategy};
use std::mem::size_of;
use std::time::Duration;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput,
};

#[derive(Clone, Debug)]
pub struct WindowsInputEngine {
    sleeper: SpinSleeper,
    pub articulation: f64,
    pub elevate_thread_priority: bool,
}

impl WindowsInputEngine {
    pub fn new(articulation: f64) -> Self {
        let sleeper = SpinSleeper::default().with_spin_strategy(SpinStrategy::YieldThread);
        Self {
            sleeper,
            articulation,
            elevate_thread_priority: true,
        }
    }

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

                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 { ki },
                }
            })
            .collect()
    }

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

    fn send_inputs_batch(inputs: &mut [INPUT]) -> Result<()> {
        unsafe {
            let sent = SendInput(inputs, size_of::<INPUT>() as i32);
            if sent == inputs.len() as u32 {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "SendInput failed: requested {}, sent {}..!",
                    inputs.len(),
                    sent
                ))
            }
        }
    }
}

impl InputEngine for WindowsInputEngine {
    fn get_articulation(&self) -> f64 {
        self.articulation
    }

    fn sleep(&self, duration_ms: Duration) {
        self.sleeper.sleep(duration_ms);
    }

    fn key_up(&self, combo: &Input) -> Result<()> {
        let mut inputs = Self::build_keyup_inputs(combo);

        debug!(
            "WindowsInputEngine::key_up for {} => keys {:?}",
            combo.note_label, combo.keys
        );

        Self::send_inputs_batch(&mut inputs)
    }

    fn key_down(&self, combo: &Input) -> Result<()> {
        let mut inputs = Self::build_keydown_inputs(combo);

        debug!(
            "WindowsInputEngine::key_down for {} => keys {:?}",
            combo.note_label, combo.keys
        );

        Self::send_inputs_batch(&mut inputs)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::model::mappings::{Input, MAPPINGS, PLAY_KEY, input_for_midi};
    use crate::util::ensure_active_window;
    use log::info;

    #[test]
    fn press_play_key() {
        let art = 1.0;
        let engine = WindowsInputEngine::new(art);

        let input = Input {
            keys: &[PLAY_KEY],
            note_label: "test_play",
        };

        ensure_active_window();
        assert!(engine.key_press(&input, 2000.0, art).is_ok());
    }

    #[test]
    fn play_full_range() {
        env_logger::try_init().unwrap_or(());

        let art = 0.95;
        let engine = WindowsInputEngine::new(art);

        let mut delay_ms = 840.0;
        for n in 0..5 {
            if n > 0 {
                delay_ms /= 2.0;
                info!("Speeding up..!");
            }
            for entry in MAPPINGS {
                ensure_active_window();
                info!("Playing note: \"{}\"", entry.1.note_label);
                assert!(engine.key_press(&entry.1, delay_ms, art).is_ok());
            }
            for entry in MAPPINGS.iter().rev() {
                ensure_active_window();
                info!("Playing note: \"{}\"", entry.1.note_label);
                assert!(engine.key_press(&entry.1, delay_ms, art).is_ok());
            }
        }
    }

    #[test]
    fn play_teleport_tune() {
        env_logger::try_init().unwrap_or(());

        let art = 0.69;
        let mut inputs: Vec<Input> = Vec::new();
        let engine = WindowsInputEngine::new(art);

        for n in 0..8 {
            let midi = match n {
                0..=1 => 69, // A4
                2..=3 => 76, // E5
                4..=5 => 73, // C#5
                6..=7 => 80, // G#5
                _ => unreachable!(),
            };

            inputs.push(
                input_for_midi(midi)
                    .expect("Midi values should be in range..!")
                    .to_owned(),
            );
        }

        ensure_active_window();
        for input in inputs {
            info!("Playing note: \"{}\"", input.note_label);
            assert!(engine.key_press(&input, 150.0, art).is_ok());
        }
    }
}
