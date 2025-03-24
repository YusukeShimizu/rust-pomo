// A CLI tool to manage focus and break time. This uses Clap (with derive), Indicatif,
// and ctrlc to handle arguments, progress display, and SIGINT signals.
// This code allows specifying focus time, break time, and repeat cycles,
// then toggles WiFi off/on for each cycle on macOS.
//
// Removed set_message calls to avoid lifetime issues.

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::{thread, time::Duration};

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

    for cycle in 1..=cli.cycles {
        println!("=== Cycle {}/{}: Focus time ===", cycle, cli.cycles);

        // Turn WiFi off for focus
        set_wifi_power(false)?;

        // Run focus timer
        run_timer(cli.focus);

        println!("=== Break time ===");

        // Turn WiFi on for break
        set_wifi_power(true)?;

        // Run break timer
        run_timer(cli.break_time);

        // Send notification at cycle end
        send_notification("Focus Timer", &format!("Cycle {} finished!", cycle))?;
    }

    // Ensure WiFi is on at the end
    set_wifi_power(true)?;
    println!("All cycles finished!");

    Ok(())
}

// Turn WiFi on/off on macOS
// Keep lines short for clarity and maintainability
fn set_wifi_power(on: bool) -> std::io::Result<()> {
    let status = if on { "on" } else { "off" };
    println!("Setting WiFi {}", status);

    std::process::Command::new("networksetup")
        .args(["-setairportpower", "en0", status])
        .status()?;
    Ok(())
}

// Show a countdown in the console using indicatif
fn run_timer(seconds: u64) {
    println!("Starting timer for {seconds} seconds...");

    let pb = ProgressBar::new(seconds);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{bar:40.cyan/blue}] {pos}s / {len}s")
            .unwrap()
            .progress_chars("##-"),
    );

    for i in 0..seconds {
        pb.set_position(i);
        thread::sleep(Duration::from_secs(1));
    }
    pb.finish_with_message("Done!");
}

// Show notification on macOS
fn send_notification(title: &str, message: &str) -> std::io::Result<()> {
    let script = format!(
        "display notification \"{}\" with title \"{}\"",
        message, title
    );

    std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .status()?;
    Ok(())
}
