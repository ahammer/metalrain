use clap::Parser;

/// Entry describing a demo.
struct DemoEntry {
    name: &'static str,
    run: fn(),
    description: &'static str,
}

use architecture_test::{run_architecture_test, DEMO_NAME as ARCH_DEMO};
use compositor_test::{run_compositor_test, DEMO_NAME as COMPOSITOR_DEMO};
use metaballs_test::{run_metaballs_test, DEMO_NAME as METABALLS_DEMO};
use physics_playground::{run_physics_playground, DEMO_NAME as PHYSICS_DEMO};

static DEMOS: &[DemoEntry] = &[
    DemoEntry { name: ARCH_DEMO, run: run_architecture_test, description: "Minimal architecture integration demo" },
    DemoEntry { name: COMPOSITOR_DEMO, run: run_compositor_test, description: "Compositor + rendering layers stress test" },
    DemoEntry { name: METABALLS_DEMO, run: run_metaballs_test, description: "Metaball renderer clustering / presentation demo" },
    DemoEntry { name: PHYSICS_DEMO, run: run_physics_playground, description: "Interactive physics playground" },
];

#[derive(Parser, Debug)]
#[command(name = "demo-launcher", version, about = "Unified launcher for all demos")]
struct Cli {
    /// Demo name to run (see --list)
    demo: Option<String>,
    /// List available demos and exit
    #[arg(long, short)]
    list: bool,
}

fn main() {
    let cli = Cli::parse();

    if cli.list { list_demos(); return; }

    if let Some(name) = cli.demo.as_ref() {
        launch_by_name(name);
        return;
    }

    interactive_select_and_launch();
}

fn list_demos() {
    println!("Available demos:");
    for (i, d) in DEMOS.iter().enumerate() {
        println!("  [{}] {:20} - {}", i + 1, d.name, d.description);
    }
}

fn launch_by_name(name: &str) {
    if let Some(entry) = DEMOS.iter().find(|d| d.name == name) {
        println!("Launching demo: {}", entry.name);
        run_entry(entry);
    } else {
        eprintln!("Unknown demo '{name}'. Use --list to see options.");
        std::process::exit(1);
    }
}

fn run_entry(entry: &DemoEntry) {
    if let Err(payload) = std::panic::catch_unwind(|| (entry.run)()) {
        eprintln!("Demo '{}' panicked. Aborting.", entry.name);
        if let Some(msg) = payload.downcast_ref::<&str>() { eprintln!("Reason: {msg}"); }
        else if let Some(msg) = payload.downcast_ref::<String>() { eprintln!("Reason: {msg}"); }
        std::process::exit(1);
    }
}

fn interactive_select_and_launch() {
    use std::io::{self, Write};
    loop {
        list_demos();
        println!("Select a demo by number (or 'q' to quit):");
        print!("> ");
        let _ = io::stdout().flush();
        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Ok(0) => { println!("EOF received. Exiting."); return; }
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.eq_ignore_ascii_case("q") { return; }
                if let Ok(idx) = trimmed.parse::<usize>() {
                    if idx >= 1 && idx <= DEMOS.len() {
                        let entry = &DEMOS[idx - 1];
                        println!("Launching demo: {}", entry.name);
                        run_entry(entry);
                        return;
                    }
                }
                println!("Invalid selection '{trimmed}'. Please enter a number 1-{} or 'q'.", DEMOS.len());
            }
            Err(e) => {
                eprintln!("Error reading input: {e}. Exiting.");
                return;
            }
        }
    }
}
