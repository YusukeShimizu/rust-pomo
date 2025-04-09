// A CLI tool to manage focus and break time. This uses Clap (with derive), Indicatif,
// ctrlc to handle arguments, progress display, and SIGINT signals. Now includes a
// pause feature that toggles Wi-Fi on/off during pauses.
//
// How to use pause:
//   During focus or break, type 'p' (and press ENTER) in the console to pause.
//   If in focus mode (Wi-Fi off), pause will turn Wi-Fi on. When you unpause,
//   Wi-Fi turns off again. Similarly, if in break mode (Wi-Fi on), pause won't
//   change Wi-Fi state (it remains on), but the timer is paused until 'p' is pressed again.
//
// Note:
//  1. This is a simple blocking approach that checks stdin in a separate thread.
//  2. The user must press ENTER after typing 'p' for the toggle to pick up.
//  3. This approach sleeps for 1 second per loop tick, so pause may take up to 1 second
//     to register or unpause.

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    io::{BufRead, BufReader},
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

/// A simple Pomodoro-style focus timer
#[derive(Debug, Parser)]
#[command(name = "focus-timer")]
struct Cli {
    /// Focus time in seconds
    #[arg(long, default_value_t = 1500)]
    focus: u64,

    /// Break time in seconds
    #[arg(long, default_value_t = 300)]
    break_time: u64,

    /// Number of focus/break cycles
    #[arg(long, default_value_t = 1)]
    cycles: u32,
}

/// Global application state
struct AppState {
    paused: AtomicBool,
}

fn main() -> std::io::Result<()> {
    // Set up SIGINT handler
    ctrlc::set_handler(|| {
        eprintln!("SIGINT received. Turning WiFi on and exiting.");
        let _ = set_wifi_power(true);
        std::process::exit(0);
    })
    .expect("Failed to set SIGINT handler.");

    // Parse CLI args
    let cli = Cli::parse();

    // Shared state for pause toggling
    let app_state = Arc::new(AppState {
        paused: AtomicBool::new(false),
    });

    // Spawn a thread to listen for 'p' to toggle pause
    {
        let app_state_clone = Arc::clone(&app_state);
        thread::spawn(move || {
            let stdin = std::io::stdin();
            let reader = BufReader::new(stdin);

            for line in reader.lines() {
                match line {
                    Ok(cmd) => {
                        if cmd.trim() == "p" {
                            // Toggle paused
                            let currently_paused = app_state_clone.paused.load(Ordering::SeqCst);
                            let new_state = !currently_paused;
                            app_state_clone.paused.store(new_state, Ordering::SeqCst);

                            println!(
                                "Pause toggled to {}",
                                if new_state { "PAUSED" } else { "RUNNING" }
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading input: {}", e);
                    }
                }
            }
        });
    }

    for cycle in 1..=cli.cycles {
        println!("=== Cycle {}/{}: Focus time ===", cycle, cli.cycles);

        // Turn WiFi off for focus
        set_wifi_power(false)?;

        // Run focus timer
        run_timer(cli.focus, true, Arc::clone(&app_state));

        println!("=== Break time ===");

        // Turn WiFi on for break
        set_wifi_power(true)?;

        // Run break timer
        run_timer(cli.break_time, false, Arc::clone(&app_state));

        // Send notification at cycle end
        send_notification("Focus Timer", &format!("Cycle {} finished!", cycle))?;
    }

    // Ensure WiFi is on at the end
    set_wifi_power(true)?;
    println!("All cycles finished!");

    Ok(())
}

// Turn WiFi on/off on macOS
fn set_wifi_power(on: bool) -> std::io::Result<()> {
    let status = if on { "on" } else { "off" };
    println!("Setting WiFi {}", status);

    Command::new("networksetup")
        .args(["-setairportpower", "en0", status])
        .status()?;
    Ok(())
}

// Show a countdown in the console using indicatif, checking for pause state
fn run_timer(seconds: u64, focus_mode: bool, app_state: Arc<AppState>) {
    // focus_mode = true => WiFi should be off when not paused
    // focus_mode = false => WiFi should be on when not paused

    println!("Starting timer for {seconds} seconds... (Type 'p' + ENTER to pause)");

    let pb = ProgressBar::new(seconds);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{bar:40.cyan/blue}] {pos}s / {len}s")
            .unwrap()
            .progress_chars("##-"),
    );

    let mut elapsed = 0;
    while elapsed < seconds {
        // If paused, keep WiFi ON if we are in focus mode
        if app_state.paused.load(Ordering::SeqCst) {
            if focus_mode {
                let _ = set_wifi_power(true);
            }
            // Wait in paused state until unpaused
            while app_state.paused.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(500));
            }
            // Once unpaused, if focus_mode, turn WiFi off again
            if focus_mode {
                let _ = set_wifi_power(false);
            }
        }

        pb.set_position(elapsed);
        thread::sleep(Duration::from_secs(1));
        elapsed += 1;
    }
    pb.finish_with_message("Done!");
}

// Show notification on macOS
fn send_notification(title: &str, message: &str) -> std::io::Result<()> {
    let script = format!(
        "display notification \"{}\" with title \"{}\"",
        message, title
    );

    Command::new("osascript").arg("-e").arg(script).status()?;
    Ok(())
}
