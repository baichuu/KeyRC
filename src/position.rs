use gtk::glib::{self, ControlFlow};
use std::cell::{Cell, RefCell};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

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

pub(crate) fn start_writer(path: PathBuf) -> impl Fn((i32, i32)) {
    let pending = Rc::new(RefCell::new(None));
    let scheduled = Rc::new(Cell::new(false));
    move |position| {
        *pending.borrow_mut() = Some((position, Instant::now()));
        if scheduled.replace(true) {
            return;
        }
        let path = path.clone();
        let pending = Rc::clone(&pending);
        let scheduled = Rc::clone(&scheduled);
        glib::timeout_add_local(Duration::from_millis(50), move || {
            let ready = pending
                .borrow()
                .as_ref()
                .is_some_and(|(_, updated)| updated.elapsed() >= Duration::from_millis(150));
            if !ready {
                return ControlFlow::Continue;
            }
            if let Some((position, _)) = pending.borrow_mut().take() {
                if let Err(error) = save(&path, position) {
                    eprintln!("Could not save KeyRC position: {error}");
                }
            }
            scheduled.set(false);
            ControlFlow::Break
        });
    }
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
