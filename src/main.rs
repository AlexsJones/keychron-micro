mod config;
mod pad;
mod run;
mod status;
mod tap;
mod via;
mod web;

use status::Status;
use std::path::PathBuf;
use std::time::Duration;

/// The repo this binary was built in. Scripts are named relative to it so the
/// whole setup stays portable: clone anywhere, and the config still resolves.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Walk through each status colour so the mapping can be eyeballed on the pad,
/// then put the lighting back exactly as it was found.
fn demo(v: &mut via::Via) -> std::io::Result<()> {
    let saved = v.snapshot()?;
    println!(
        "saved lighting: effect={} brightness={} hue={} sat={}",
        saved.effect, saved.brightness, saved.hue, saved.sat
    );

    v.set_effect(via::EFFECT_SOLID)?;
    v.set_brightness(255)?;

    for s in [
        Status::Idle,
        Status::Thinking,
        Status::NeedsInput,
        Status::Complete,
        Status::Error,
    ] {
        let (h, sat) = s.hue_sat();
        println!("  {}", s.label());
        v.set_color(h, sat)?;
        std::thread::sleep(Duration::from_millis(900));
    }

    v.restore(saved)?;
    println!("restored.");
    Ok(())
}

fn usage() {
    eprintln!(
        "keychron-micro

  run      grab the pad and run config.toml's scripts on keypress
  learn    grab the pad and print what each key sends
  lights   cycle the status colours, then restore
  probe    report what the pad exposes and whether we can reach it"
    );
}

fn probe() -> std::io::Result<()> {
    match pad::find() {
        Ok((path, dev)) => println!(
            "keys:   {} [{}] -- readable",
            dev.name().unwrap_or("unknown"),
            path.display()
        ),
        Err(e) => println!("keys:   UNAVAILABLE -- {e}"),
    }
    match via::Via::open() {
        Ok(mut v) => {
            let ver = v.protocol_version()?;
            let l = v.snapshot()?;
            println!("lights: VIA protocol {ver} -- reachable");
            println!(
                "        effect={} brightness={} hue={} sat={}",
                l.effect, l.brightness, l.hue, l.sat
            );
        }
        Err(e) => println!("lights: UNAVAILABLE -- {e}"),
    }
    Ok(())
}

fn dispatch() -> std::io::Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("run") => {
            let root = repo_root();
            run::run(&root.join("config.toml"), &root)
        }
        Some("learn") => pad::learn(),
        Some("lights") => {
            let mut v = via::Via::open()?;
            demo(&mut v)
        }
        Some("probe") | None => probe(),
        Some(other) => {
            eprintln!("unknown command: {other}\n");
            usage();
            std::process::exit(2);
        }
    }
}

fn main() {
    // Report failures as the sentence they were written as, rather than letting
    // the default Termination impl print io::Error's Debug form.
    if let Err(e) = dispatch() {
        eprintln!("keychron-micro: {e}");
        std::process::exit(1);
    }
}
