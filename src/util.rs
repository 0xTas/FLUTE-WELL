use crate::PolyPolicy;
use log::info;

pub fn parse_articulation(input: &str, custom: Option<f64>) -> f64 {
    match input.to_lowercase().as_str() {
        "t" | "tenuto" => 1.0,
        "s" | "staccato" => 0.5,
        "ss" | "staccatissimo" => 0.25,
        "p" | "portato" | "portamento" => 0.75,
        "c" | "custom" => {
            if let Some(hold_perc) = custom.as_ref() {
                hold_perc.clamp(0.0, 1.0)
            } else {
                info!(
                    "No custom articulation given..!\nExample usage: `--hold-percentage 0.42` | Defaulting to 0.75 (Portato)..!"
                );
                0.75
            }
        }
        _ => 0.75,
    }
}

pub fn parse_policy(s: &str) -> PolyPolicy {
    match s.to_lowercase().as_str() {
        "h"|"highest" => PolyPolicy::Highest,
        "lw"|"lowest" => PolyPolicy::Lowest,
        "lu"|"loudest" => PolyPolicy::Loudest,
        "a"|"d"|"auto"|"densest" => PolyPolicy::Densest,
        other => {
            info!("Unknown policy '{}', defaulting to `highest`..!", other);
            PolyPolicy::Highest
        }
    }
}

/// Blocks for 30 seconds while checking that the active window's title is ANIMAL WELL, then panics or returns.
#[cfg(test)]
pub fn ensure_active_window() {
    use log::debug;
    use std::time::{Duration, Instant};

    let now = Instant::now();
    loop {
        let active_window = active_win_pos_rs::get_active_window();

        if active_window.is_err() {
            continue;
        }

        let title = active_window.unwrap().title;

        debug!("Active window: \"{}\"", title);
        if title == "ANIMAL WELL" {
            break;
        } else {
            let elapsed = now.elapsed();
            if elapsed > Duration::from_secs(30) {
                panic!("Active window title was never ANIMAL WELL..! (waited 30 seconds.)")
            }
        }

        spin_sleep::sleep(Duration::from_millis(50));
    }
}
