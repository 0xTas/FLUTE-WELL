use windows::Win32::UI::Input::KeyboardAndMouse::*;

/// A combination of keypresses to represent a note
#[derive(Debug, Clone, Copy)]
pub struct Input {
    /// Win32 virtual-key codes to press simultaneously for this note.
    /// Order doesn't matter for playback; both key-downs are sent
    /// together (we'll batch them in SendInput).
    pub keys: &'static [VIRTUAL_KEY],
    /// human-readable note info for debugging
    pub note_label: &'static str,
}

// Win32 Virtual Key constants used here
// pub const VK_NUMPAD0: VIRTUAL_KEY = VIRTUAL_KEY(0x60);
// pub const VK_NUMPAD1: VIRTUAL_KEY = VIRTUAL_KEY(0x61);
// pub const VK_NUMPAD2: VIRTUAL_KEY = VIRTUAL_KEY(0x62);
// pub const VK_NUMPAD3: VIRTUAL_KEY = VIRTUAL_KEY(0x63);
// pub const VK_NUMPAD4: VIRTUAL_KEY = VIRTUAL_KEY(0x64);
// pub const VK_NUMPAD5: VIRTUAL_KEY = VIRTUAL_KEY(0x65); // play/hold
// pub const VK_NUMPAD6: VIRTUAL_KEY = VIRTUAL_KEY(0x66);
// pub const VK_NUMPAD7: VIRTUAL_KEY = VIRTUAL_KEY(0x67);
// pub const VK_NUMPAD8: VIRTUAL_KEY = VIRTUAL_KEY(0x68);
// pub const VK_NUMPAD9: VIRTUAL_KEY = VIRTUAL_KEY(0x69);
// pub const VK_KEY_1: VIRTUAL_KEY = VIRTUAL_KEY(0x31); // '1' (octave-drop modifier)
// pub const VK_KEY_3: VIRTUAL_KEY = VIRTUAL_KEY(0x33); // '3' (semitone/sharp modifier)

// Direction layout (explicit mapping of direction slots -> numpad keys).
// This order is: 1 = right, 2 = down-right, 3 = down, 4 = down-left,
// 5 = left, 6 = up-left, 7 = up, 8 = up-right (clockwise).
pub const DIR_1_RIGHT: VIRTUAL_KEY = VK_NUMPAD6;
pub const DIR_2_DOWNRIGHT: VIRTUAL_KEY = VK_NUMPAD3;
pub const DIR_3_DOWN: VIRTUAL_KEY = VK_NUMPAD2;
pub const DIR_4_DOWNLEFT: VIRTUAL_KEY = VK_NUMPAD1;
pub const DIR_5_LEFT: VIRTUAL_KEY = VK_NUMPAD4;
pub const DIR_6_UPLEFT: VIRTUAL_KEY = VK_NUMPAD7;
pub const DIR_7_UP: VIRTUAL_KEY = VK_NUMPAD8;
pub const DIR_8_UPRIGHT: VIRTUAL_KEY = VK_NUMPAD9;
pub const PLAY_KEY: VIRTUAL_KEY = VK_NUMPAD5;

// -----------------------------------------------------------------------------
// Hardcoded mapping: MIDI 69 (A4) .. MIDI 93 (A6 inclusive)
//
// Rules encoded:
// - base whole-tone mapping: each pair of semitones shares the same direction.
//   i.e., semitone offsets 0 & 1 -> same direction, 2 & 3 -> next direction, etc.
// - if the semitone offset is odd -> include VK_KEY_3 (semitone modifier).
// - if MIDI <= 80 (A4..G#5) -> include VK_KEY_1 (octave-drop modifier).
// - every combo also includes PLAY_KEY.
// -----------------------------------------------------------------------------

pub const MAPPINGS: &[(u8, Input)] = &[
    // Lower octave (A4 .. G#5) include VK_KEY_1 (octave drop)
    (
        69,
        Input {
            keys: &[VK_1, DIR_1_RIGHT],
            note_label: "A4 (69)",
        },
    ),
    (
        70,
        Input {
            keys: &[VK_1, DIR_1_RIGHT, VK_3],
            note_label: "A#4 (70)",
        },
    ),
    (
        71,
        Input {
            keys: &[VK_1, DIR_2_DOWNRIGHT],
            note_label: "B4 (71)",
        },
    ),
    (
        72,
        Input {
            keys: &[VK_1, DIR_2_DOWNRIGHT, VK_3],
            note_label: "C5 (72)",
        },
    ),
    (
        73,
        Input {
            keys: &[VK_1, DIR_3_DOWN],
            note_label: "C#5 (73)",
        },
    ),
    (
        74,
        Input {
            keys: &[VK_1, DIR_3_DOWN, VK_3],
            note_label: "D5 (74)",
        },
    ),
    (
        75,
        Input {
            keys: &[VK_1, DIR_4_DOWNLEFT, VK_3],
            note_label: "D#5 (75)",
        },
    ),
    (
        76,
        Input {
            keys: &[VK_1, DIR_5_LEFT],
            note_label: "E5 (76)",
        },
    ),
    (
        77,
        Input {
            keys: &[VK_1, DIR_5_LEFT, VK_3],
            note_label: "F5 (77)",
        },
    ),
    (
        78,
        Input {
            keys: &[VK_1, DIR_6_UPLEFT],
            note_label: "F#5 (78)",
        },
    ),
    (
        79,
        Input {
            keys: &[VK_1, DIR_6_UPLEFT, VK_3],
            note_label: "G5 (79)",
        },
    ),
    (
        80,
        Input {
            keys: &[VK_1, DIR_7_UP],
            note_label: "G#5 (80)",
        },
    ),
    // Upper octave (A5 .. A6) — no octave-drop modifier (VK_KEY_1 not included)

    // (81, Input { keys: &[PLAY_KEY, DIR_7_UP, VK_3],            note_label: "A5 (82)" }),
    // (83, Input { keys: &[PLAY_KEY, DIR_8_UPRIGHT],               note_label: "A#5 (83)" }),
    // (84, Input { keys: &[PLAY_KEY, DIR_8_UPRIGHT, VK_3],       note_label: "C6 (84)" }),
    (
        81,
        Input {
            keys: &[DIR_1_RIGHT],
            note_label: "A5 (81)",
        },
    ),
    (
        82,
        Input {
            keys: &[DIR_1_RIGHT, VK_3],
            note_label: "A#5 (82)",
        },
    ),
    (
        83,
        Input {
            keys: &[DIR_2_DOWNRIGHT],
            note_label: "B5 (83)",
        },
    ),
    (
        84,
        Input {
            keys: &[DIR_2_DOWNRIGHT, VK_3],
            note_label: "C6 (84)",
        },
    ),
    (
        85,
        Input {
            keys: &[DIR_3_DOWN],
            note_label: "C#6 (85)",
        },
    ),
    (
        86,
        Input {
            keys: &[DIR_3_DOWN, VK_3],
            note_label: "D6 (86)",
        },
    ),
    (
        87,
        Input {
            keys: &[DIR_4_DOWNLEFT, VK_3],
            note_label: "D#6 (87)",
        },
    ),
    (
        88,
        Input {
            keys: &[DIR_5_LEFT],
            note_label: "E6 (88)",
        },
    ),
    (
        89,
        Input {
            keys: &[DIR_5_LEFT, VK_3],
            note_label: "F6 (89)",
        },
    ),
    (
        90,
        Input {
            keys: &[DIR_6_UPLEFT],
            note_label: "F#6 (90)",
        },
    ),
    (
        91,
        Input {
            keys: &[DIR_6_UPLEFT, VK_3],
            note_label: "G6 (91)",
        },
    ),
    (
        92,
        Input {
            keys: &[DIR_7_UP],
            note_label: "G#6 (92)",
        },
    ),
    (
        93,
        Input {
            keys: &[DIR_7_UP, VK_3],
            note_label: "A6 (93)",
        },
    ),
];

/// Return a reference to the Input for the given MIDI note number, if present.
///
/// Example:
/// ```ignore
/// if let Some(inp) = input_for_midi(69) {
///     println!("MIDI 69 -> {} keys: {:?}", inp.note_label, inp.keys);
/// }
/// ```
pub fn input_for_midi(midi: u8) -> Option<&'static Input> {
    MAPPINGS
        .iter()
        .find(|(m, _)| *m == midi)
        .map(|(_, input)| input)
}

/// Convenience: return just the key slice for a MIDI note (if present).
pub fn keys_for_midi(midi: u8) -> Option<&'static [VIRTUAL_KEY]> {
    input_for_midi(midi).map(|inp| inp.keys)
}

/* Optional small example usage (for tests or quick runs):

fn main() {
    let midi = 69u8;
    match input_for_midi(midi) {
        Some(inp) => {
            println!("MIDI {} -> {} (VKs: {:?})", midi, inp.note_label, inp.keys);
        }
        None => {
            println!("No mapping for MIDI {}", midi);
        }
    }
}

*/
