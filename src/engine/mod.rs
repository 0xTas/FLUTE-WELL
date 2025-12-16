use crate::MAPPINGS;
use crate::model::mappings::{Input, PLAY_KEY};
use anyhow::anyhow;
use std::time::Duration;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::WindowsInputEngine as DefaultInputEngine;

pub trait InputEngine: Send + Sync {
    fn get_articulation(&self) -> f64;

    fn sleep(&self, duration_ms: Duration);

    fn key_up(&self, input: &Input) -> anyhow::Result<()>;

    fn key_down(&self, input: &Input) -> anyhow::Result<()>;

    fn all_keys_up(&self) -> anyhow::Result<()> {
        for (_midi, input) in MAPPINGS {
            self.key_up(input)?;
        }

        Ok(())
    }

    fn key_press(&self, input: &Input, hold_ms: f64, articulation: f64) -> anyhow::Result<()> {
        if hold_ms <= 0.0 {
            return Err(anyhow!("hold_ms must be greater than 0..!"));
        }

        let play_input = Input {
            keys: &[PLAY_KEY],
            note_label: "play_key",
        };

        let mut release_ms = 0.0;
        let mut final_hold_ms = hold_ms;

        if articulation > 0.0 && articulation < 1.0 {
            final_hold_ms *= articulation;
            release_ms = hold_ms * (1.0 - articulation);
        }

        if final_hold_ms <= 0.0 {
            release_ms = 0.0;
            final_hold_ms = hold_ms;
        }

        // Always press & release the play key first before pressing/releasing any other keys.
        // This avoids accidental wrong notes from incidental keypress races.
        self.key_down(input)?;
        self.sleep(Duration::from_millis(1));

        self.key_down(&play_input)?;
        self.sleep(Duration::from_secs_f64(final_hold_ms / 1000.0));

        self.key_up(&play_input)?;
        self.sleep(Duration::from_millis(1));

        self.key_up(input)?;
        if release_ms > 0.0 {
            self.sleep(Duration::from_secs_f64(release_ms / 1000.0));
        }

        Ok(())
    }
}
