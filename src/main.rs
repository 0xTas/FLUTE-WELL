use FLUTE_WELL::{Args, Player, import_midi_file, input_for_midi, parse_articulation, parse_policy, DefaultInputEngine};
use anyhow::Result;
use clap::Parser;
use log::{debug, info, warn};
use std::sync::Arc;
use std::sync::mpsc;

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();
    let policy = parse_policy(&args.policy);
    let articulation = parse_articulation(&args.articulation_style, args.custom_articulation);

    info!("Importing MIDI file: '{}'...", args.midi.display());
    let song = import_midi_file(
        &args.midi,
        args.transpose,
        policy,
        args.merge_midi,
        Some((69, 93)),
    )?;

    debug!(
        "Imported song '{}' with {} events..!",
        song.metadata
            .title
            .clone()
            .unwrap_or_else(|| "<unknown>".into()),
        song.events.len()
    );

    if args.dry_run {
        info!("Previewing at most {} events..!", args.dry_run_max);
        for (i, ev) in song.events.iter().enumerate() {
            if i >= args.dry_run_max {
                break;
            }
            let midi = ev.note.midi;
            let keys = input_for_midi(midi)
                .map(|inp| format!("{:?}", inp.keys))
                .unwrap_or_else(|| "<no-mapping>".into());

            info!(
                "Event {}: midi={} time_ms={:.3} dur_ms={:.3} keys={}",
                i, midi, ev.time_ms, ev.duration_ms, keys
            );
        }
        return Ok(());
    }

    let player = Player::new(
        DefaultInputEngine::new(articulation),
        args.verbose,
        args.delay_start,
    );

    player.load_song(song)?;
    let player_arc = Arc::new(player);
    let player = Arc::clone(&player_arc);
    let player_for_handler = Arc::clone(&player_arc);
    let (done_tx, _done_rx) = mpsc::channel::<()>();

    ctrlc::set_handler(move || {
        warn!("Ctrl-C received, stopping playback..!");
        let _ = player_for_handler.stop();
        let _ = done_tx.send(());
    })
    .expect("Error setting Ctrl-C handler..!");

    player.play(true)?;
    info!("Playback finished, exiting..!");

    Ok(())
}
