use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "FLUTE_WELL",
    about = "Play a MIDI file on the Animal Well flute!"
)]
pub struct Args {
    /// Path to the target MIDI file.
    pub midi: PathBuf,

    /// Transpose in semitones (positive or negative).
    #[arg(short, long, default_value_t = 0)]
    pub transpose: i32,

    /// The articulation style to use for the song. Supports presets Tenuto|Portato|Staccato|Staccatissimo.
    /// Pass 'Custom' along with the flag `--hold-percentage <0.0..=1.0>` to use a custom value.
    #[arg(short, long, default_value = "portato")]
    pub articulation_style: String,

    /// How much of a note's original value to sustain for when using a custom articulation style.
    #[arg(long = "hold-percentage")]
    pub custom_articulation: Option<f64>,

    /// Dry run (print first dry_run_max events and exit).
    #[arg(short, long, default_value_t = false)]
    pub dry_run: bool,

    /// Maximum events to print in dry run.
    #[arg(long, default_value_t = 80)]
    pub dry_run_max: usize,

    /// Polyphony reduction policy: highest|lowest|loudest|first|last.
    #[arg(short, long, default_value = "highest")]
    pub policy: String,

    /// Prints extra information to the terminal.
    #[arg(short, long)]
    pub verbose: bool,

    /// Delays the start of the performance by N seconds after focusing the window.
    #[arg(long = "delay-start", default_value_t = 0)]
    pub delay_start: u64,

    /// Whether to merge consecutive midi events for the same pitch when reducing the tracks to monophony.
    #[arg(short, long, default_value_t = false)]
    pub merge_midi: bool,
}
