use windows::Win32::UI::Input::KeyboardAndMouse::*;

/// A combination of keypresses that represent a note
#[derive(Debug, Clone, Copy)]
pub struct Input {
    pub note_label: &'static str,
    pub keys: &'static [VIRTUAL_KEY],
}

pub const DIR_1_RIGHT: VIRTUAL_KEY = VK_NUMPAD6;
pub const DIR_2_DOWNRIGHT: VIRTUAL_KEY = VK_NUMPAD3;
pub const DIR_3_DOWN: VIRTUAL_KEY = VK_NUMPAD2;
pub const DIR_4_DOWNLEFT: VIRTUAL_KEY = VK_NUMPAD1;
pub const DIR_5_LEFT: VIRTUAL_KEY = VK_NUMPAD4;
pub const DIR_6_UPLEFT: VIRTUAL_KEY = VK_NUMPAD7;
pub const DIR_7_UP: VIRTUAL_KEY = VK_NUMPAD8;
pub const DIR_8_UPRIGHT: VIRTUAL_KEY = VK_NUMPAD9;
pub const PLAY_KEY: VIRTUAL_KEY = VK_NUMPAD5;
pub const OCTAVE_MODIFIER: VIRTUAL_KEY = VK_1;
pub const SEMITONE_MODIFIER: VIRTUAL_KEY = VK_3;

pub const MAPPINGS: &[(u8, Input)] = &[
    (
        69,
        Input {
            keys: &[OCTAVE_MODIFIER, DIR_1_RIGHT],
            note_label: "A4 (69)",
        },
    ),
    (
        70,
        Input {
            keys: &[OCTAVE_MODIFIER, DIR_1_RIGHT, SEMITONE_MODIFIER],
            note_label: "A#4 (70)",
        },
    ),
    (
        71,
        Input {
            keys: &[OCTAVE_MODIFIER, DIR_2_DOWNRIGHT],
            note_label: "B4 (71)",
        },
    ),
    (
        72,
        Input {
            keys: &[OCTAVE_MODIFIER, DIR_2_DOWNRIGHT, SEMITONE_MODIFIER],
            note_label: "C5 (72)",
        },
    ),
    (
        73,
        Input {
            keys: &[OCTAVE_MODIFIER, DIR_3_DOWN],
            note_label: "C#5 (73)",
        },
    ),
    (
        74,
        Input {
            keys: &[OCTAVE_MODIFIER, DIR_3_DOWN, SEMITONE_MODIFIER],
            note_label: "D5 (74)",
        },
    ),
    (
        75,
        Input {
            keys: &[OCTAVE_MODIFIER, DIR_4_DOWNLEFT, SEMITONE_MODIFIER],
            note_label: "D#5 (75)",
        },
    ),
    (
        76,
        Input {
            keys: &[OCTAVE_MODIFIER, DIR_5_LEFT],
            note_label: "E5 (76)",
        },
    ),
    (
        77,
        Input {
            keys: &[OCTAVE_MODIFIER, DIR_5_LEFT, SEMITONE_MODIFIER],
            note_label: "F5 (77)",
        },
    ),
    (
        78,
        Input {
            keys: &[OCTAVE_MODIFIER, DIR_6_UPLEFT],
            note_label: "F#5 (78)",
        },
    ),
    (
        79,
        Input {
            keys: &[OCTAVE_MODIFIER, DIR_6_UPLEFT, SEMITONE_MODIFIER],
            note_label: "G5 (79)",
        },
    ),
    (
        80,
        Input {
            keys: &[OCTAVE_MODIFIER, DIR_7_UP],
            note_label: "G#5 (80)",
        },
    ),
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
            keys: &[DIR_1_RIGHT, SEMITONE_MODIFIER],
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
            keys: &[DIR_2_DOWNRIGHT, SEMITONE_MODIFIER],
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
            keys: &[DIR_3_DOWN, SEMITONE_MODIFIER],
            note_label: "D6 (86)",
        },
    ),
    (
        87,
        Input {
            keys: &[DIR_4_DOWNLEFT, SEMITONE_MODIFIER],
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
            keys: &[DIR_5_LEFT, SEMITONE_MODIFIER],
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
            keys: &[DIR_6_UPLEFT, SEMITONE_MODIFIER],
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
            keys: &[DIR_8_UPRIGHT],
            note_label: "A6 (93)",
        },
    ),
];

pub fn input_for_midi(midi: u8) -> Option<&'static Input> {
    MAPPINGS
        .iter()
        .find(|(m, _)| *m == midi)
        .map(|(_, input)| input)
}
