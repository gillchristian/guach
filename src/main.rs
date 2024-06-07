use crossterm::{cursor::*, terminal::*, ExecutableCommand};
use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::new_debouncer;
use std::io::{stdout, Write};
use std::{path::Path, time::Duration};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let main_path = std::env::args()
        .nth(1)
        .expect("Please provide a path to the main file");

    let paths = std::env::args().skip(2).collect::<Vec<_>>();

    if paths.is_empty() {
        panic!("Please provide a path to watch");
    }

    let mut stdout = stdout();

    run_elm_make(&main_path, &paths.join(", "), None, &mut stdout);

    if let Err(error) = watch(&main_path, &paths, &mut stdout) {
        log::error!("Error: {error:?}");
    }
}

fn watch<P: AsRef<Path>>(
    main_path: &P,
    paths: &[P],
    stdout: &mut std::io::Stdout,
) -> notify::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();

    let mut debouncer = new_debouncer(Duration::from_millis(50), None, tx)?;

    let watcher = debouncer.watcher();

    for path in paths {
        watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;
    }

    let formatted_paths = paths
        .iter()
        .map(|path| path.as_ref().display().to_string())
        .collect::<Vec<_>>()
        .join(", ");

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

                run_elm_make(main_path, &formatted_paths, Some(paths), stdout);
            }
            Err(errors) => errors.iter().for_each(|error| log::error!("{error:?}")),
        }
    }

    Ok(())
}

// TODO
// - [ ] capture full screen
// - [ ] print the colors as well
// - [ ] single error mode (only show the first error)
// - [x] take the path as an argument

fn run_elm_make<P: AsRef<Path>>(
    main_path: &P,
    paths: &str,
    changed: Option<String>,
    stdout: &mut std::io::Stdout,
) {
    use std::process::Command;

    let output = Command::new("elm")
        .arg("make")
        .arg(main_path.as_ref())
        .arg("--output=/dev/null")
        .output()
        .expect("Failed to run elm make");

    stdout
        .execute(Clear(ClearType::All))
        .unwrap()
        .execute(MoveTo(0, 0))
        .unwrap();

    stdout
        .write_all(format!("Main: {}. Path(s): {paths}\n", main_path.as_ref().display()).as_bytes())
        .unwrap();

    if let Some(changed) = changed {
        stdout
            .write_all(format!("Changed: {changed}\n").as_bytes())
            .unwrap();
    }

    write_colored_output(
        stdout,
        if output.stderr.is_empty() {
            &output.stdout
        } else {
            &output.stderr
        },
    );
}

fn write_colored_output(stdout: &mut std::io::Stdout, output: &[u8]) {
    let output_str = String::from_utf8_lossy(output);
    stdout.write_all(output_str.as_bytes()).unwrap();
    stdout.flush().unwrap();
}
