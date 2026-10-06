use gtk::gdk;
use gtk::glib::translate::ToGlibPtr;
use std::ffi::{c_int, c_uchar, c_ulong};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::ptr;
use std::thread;
use std::time::{Duration, Instant};
use x11::xlib;

const OWNER_PROPERTY: &[u8] = b"_KEYRC_COMPOSITOR_ACTIVE\0";
const GLASS_PROPERTY: &[u8] = b"_KEYRC_LIQUID_GLASS\0";

#[link(name = "gdk-3")]
unsafe extern "C" {
    fn gdk_x11_window_get_xid(window: *mut gdk::ffi::GdkWindow) -> c_ulong;
    fn prctl(option: c_int, argument: c_ulong) -> c_int;
}

const PR_SET_PDEATHSIG: c_int = 1;
const SIGTERM: c_ulong = 15;

#[derive(Clone, Copy, Eq, PartialEq)]
enum CompositorOwner {
    None,
    Keyrc,
    Other,
}

pub(crate) struct NativeCompositor {
    child: Option<Child>,
    active: bool,
}

impl NativeCompositor {
    pub(crate) fn start(enabled: bool) -> Self {
        let mut compositor = Self {
            child: None,
            active: false,
        };
        compositor.ensure(enabled);
        compositor
    }

    pub(crate) fn ensure(&mut self, enabled: bool) -> bool {
        if !enabled {
            if let Some(mut child) = self.child.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
            self.active = false;
            return false;
        }
        match compositor_owner() {
            CompositorOwner::Keyrc => {
                self.active = true;
                return true;
            }
            CompositorOwner::Other => {
                self.active = false;
                return false;
            }
            CompositorOwner::None => {}
        }

        let Some(helper) = helper_path() else {
            self.active = false;
            return false;
        };
        let mut command = Command::new(helper);
        command.stdin(Stdio::null()).stdout(Stdio::null());
        unsafe {
            command.pre_exec(|| {
                if prctl(PR_SET_PDEATHSIG, SIGTERM) == 0 {
                    Ok(())
                } else {
                    Err(std::io::Error::last_os_error())
                }
            });
        }
        let Ok(child) = command.spawn() else {
            self.active = false;
            return false;
        };
        self.child = Some(child);

        let deadline = Instant::now() + Duration::from_millis(600);
        while Instant::now() < deadline {
            if compositor_owner() == CompositorOwner::Keyrc {
                self.active = true;
                return true;
            }
            if self
                .child
                .as_mut()
                .and_then(|child| child.try_wait().ok())
                .flatten()
                .is_some()
            {
                break;
            }
            thread::sleep(Duration::from_millis(15));
        }
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.active = false;
        false
    }

    pub(crate) fn active(&self) -> bool {
        self.active
    }
}

impl Drop for NativeCompositor {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub(crate) fn mark_window(window: &gdk::Window, enabled: bool) {
    unsafe {
        let xid = gdk_x11_window_get_xid(window.to_glib_none().0);
        if xid == 0 {
            return;
        }
        let display = xlib::XOpenDisplay(ptr::null());
        if display.is_null() {
            return;
        }
        let property = xlib::XInternAtom(display, GLASS_PROPERTY.as_ptr().cast(), xlib::False);
        let value: c_ulong = if enabled { 1 } else { 0 };
        xlib::XChangeProperty(
            display,
            xid,
            property,
            xlib::XA_CARDINAL,
            32,
            xlib::PropModeReplace,
            (&value as *const c_ulong).cast::<c_uchar>(),
            1,
        );
        xlib::XFlush(display);
        xlib::XCloseDisplay(display);
    }
}

fn helper_path() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let directory = executable.parent()?;
    let sibling = directory.join("keyrc-compositor");
    sibling.is_file().then_some(sibling)
}

fn compositor_owner() -> CompositorOwner {
    unsafe {
        let display = xlib::XOpenDisplay(ptr::null());
        if display.is_null() {
            return CompositorOwner::None;
        }
        let screen = xlib::XDefaultScreen(display);
        let selection_name = format!("_NET_WM_CM_S{screen}\0");
        let selection = xlib::XInternAtom(display, selection_name.as_ptr().cast(), xlib::False);
        let owner = xlib::XGetSelectionOwner(display, selection);
        if owner == 0 {
            xlib::XCloseDisplay(display);
            return CompositorOwner::None;
        }
        let property = xlib::XInternAtom(display, OWNER_PROPERTY.as_ptr().cast(), xlib::False);
        let mut actual_type = 0;
        let mut actual_format: c_int = 0;
        let mut item_count: c_ulong = 0;
        let mut bytes_after: c_ulong = 0;
        let mut data: *mut c_uchar = ptr::null_mut();
        let status = xlib::XGetWindowProperty(
            display,
            owner,
            property,
            0,
            1,
            xlib::False,
            xlib::XA_CARDINAL,
            &mut actual_type,
            &mut actual_format,
            &mut item_count,
            &mut bytes_after,
            &mut data,
        );
        let is_keyrc = status == xlib::Success as c_int
            && actual_type == xlib::XA_CARDINAL
            && actual_format == 32
            && item_count == 1
            && !data.is_null()
            && *(data.cast::<c_ulong>()) != 0;
        if !data.is_null() {
            xlib::XFree(data.cast());
        }
        xlib::XCloseDisplay(display);
        if is_keyrc {
            CompositorOwner::Keyrc
        } else {
            CompositorOwner::Other
        }
    }
}
