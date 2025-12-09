use crate::model::mapper::{Input, PLAY_KEY};
use std::time::Duration;
use anyhow::anyhow;

pub mod windows;
mod scheduler;

pub trait Engine: Send + Sync + Sized {
    /// Atomically emit keydown(s) for this input.
    fn key_down(&self, input: &Input) -> anyhow::Result<()>;

    /// Atomically emit keyup(s) for this input.
    fn key_up(&self, input: &Input) -> anyhow::Result<()>;

    fn key_press(&self, input: &Input, hold_ms: u64, release_ms: u64) -> anyhow::Result<()> {
        if hold_ms == 0 {
            return Err(anyhow!("hold_ms must be greater than 0..!"));
        }

        let play_input = Input {
            keys: &[PLAY_KEY],
            note_label: "play_key",
        };

        // First we hold down the keys that pick the note we want to play
        self.key_down(input)?;
        spin_sleep::sleep(Duration::from_millis(1));

        // Then we explicitly press the play key to begin producing sound
        self.key_down(&play_input)?;
        spin_sleep::sleep(Duration::from_millis(hold_ms));

        // Always release the play key first before releasing any other keys
        self.key_up(&play_input)?;
        spin_sleep::sleep(Duration::from_millis(1));

        // This avoids accidental wrong notes from incidental keypress races
        self.key_up(input)?;
        if release_ms > 0 {
            spin_sleep::sleep(Duration::from_millis(release_ms));
        }

        Ok(())
    }
}
