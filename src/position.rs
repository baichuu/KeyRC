use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

pub(crate) fn path() -> Option<PathBuf> {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".cache"))
        })
        .map(|cache| cache.join("keyrc/position"))
}

fn parse(contents: &str) -> Option<(i32, i32)> {
    let mut lines = contents.lines();
    let x = lines.next()?.trim().parse().ok()?;
    let y = lines.next()?.trim().parse().ok()?;
    if lines.next().is_some() {
        return None;
    }
    Some((x, y))
}

pub(crate) fn read(path: Option<&Path>) -> Option<(i32, i32)> {
    path.and_then(|path| fs::read_to_string(path).ok())
        .and_then(|contents| parse(&contents))
}

fn save(path: &Path, position: (i32, i32)) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("tmp.{}", std::process::id()));
    fs::write(&temporary, format!("{}\n{}\n", position.0, position.1))?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}

pub(crate) fn start_writer(path: PathBuf) -> Sender<(i32, i32)> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        while let Ok(mut position) = receiver.recv() {
            while let Ok(next) = receiver.recv_timeout(Duration::from_millis(150)) {
                position = next;
            }
            if let Err(error) = save(&path, position) {
                eprintln!("Could not save KeyRC position: {error}");
            }
        }
    });
    sender
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_position_strictly() {
        assert_eq!(parse("12\n-8\n"), Some((12, -8)));
        assert_eq!(parse("12\n-8\nextra\n"), None);
    }
}
