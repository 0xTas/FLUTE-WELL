#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub use windows::Input as Input;
#[cfg(target_os = "windows")]
pub use windows::PLAY_KEY as PLAY_KEY;
#[cfg(target_os = "windows")]
pub use windows::MAPPINGS as MAPPINGS;
#[cfg(target_os = "windows")]
pub use windows::input_for_midi;
