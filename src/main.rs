use clap::Parser;
use crossterm::{cursor::*, terminal::*, ExecutableCommand};
use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::new_debouncer;
use std::io::{stdout, Write};
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    single: bool,
    main: String,
    paths: Vec<String>,
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    if cli.paths.is_empty() {
        panic!("Please provide a path to watch");
    }

    let mut stdout = stdout();

    run_elm_make(&cli, None, &mut stdout);

    if let Err(error) = watch(&cli, &mut stdout) {
        log::error!("Error: {error:?}");
    }
}

fn watch(cli: &Cli, stdout: &mut std::io::Stdout) -> notify::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();

    let mut debouncer = new_debouncer(Duration::from_millis(50), None, tx)?;

    let watcher = debouncer.watcher();

    for path in cli.paths.iter() {
        watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;
    }

    let cwd = std::env::current_dir().expect("Failed to get current directory");

    for result in rx {
        match result {
            Ok(events) => {
                let paths = events
                    .iter()
                    .flat_map(|event| event.event.paths.iter())
                    .map(|path| {
                        path.strip_prefix(&cwd)
                            .unwrap_or(path)
                            .display()
                            .to_string()
                    })
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(", ");

                run_elm_make(cli, Some(paths), stdout);
            }
            Err(errors) => errors.iter().for_each(|error| log::error!("{error:?}")),
        }
    }

    Ok(())
}

// TODO
// - [ ] capture full screen
// - [ ] print the colors as well
// - [x] single error mode (only show the first error)
// - [x] take the path as an argument

fn run_elm_make(cli: &Cli, changed: Option<String>, stdout: &mut std::io::Stdout) {
    use std::process::Command;

    let output = Command::new("elm")
        .arg("make")
        .arg(cli.main.clone())
        .arg("--output=/dev/null")
        .output()
        .expect("Failed to run elm make");

    stdout
        .execute(Clear(ClearType::All))
        .unwrap()
        .execute(MoveTo(0, 0))
        .unwrap();

    stdout
        .write_all(format!("Main: {}. Path(s): {}\n", cli.main, cli.paths.join(", ")).as_bytes())
        .unwrap();

    if let Some(changed) = changed {
        stdout
            .write_all(format!("Changed: {changed}\n").as_bytes())
            .unwrap();
    }

    let output = if output.stderr.is_empty() {
        &output.stdout
    } else {
        &output.stderr
    };

    let output = String::from_utf8_lossy(output);

    write_colored_output(
        stdout,
        if cli.single {
            output.split("\n\n\n").next().unwrap()
        } else {
            &output
        },
    );
}

fn write_colored_output(stdout: &mut std::io::Stdout, output: &str) {
    stdout.write_all(output.as_bytes()).unwrap();
    stdout.flush().unwrap();
}
