use gtk::cairo::{Format, ImageSurface};
use std::ffi::{c_int, c_uchar, c_uint, c_ulong, c_void};
use std::mem;
use std::ptr;
use std::slice;
use x11::{xlib, xshm};

const IPC_PRIVATE: c_int = 0;
const IPC_CREAT: c_int = 0o1000;
const IPC_RMID: c_int = 0;

unsafe extern "C" {
    fn shmget(key: c_int, size: usize, flags: c_int) -> c_int;
    fn shmat(id: c_int, address: *const c_void, flags: c_int) -> *mut c_void;
    fn shmdt(address: *const c_void) -> c_int;
    fn shmctl(id: c_int, command: c_int, buffer: *mut c_void) -> c_int;
}

#[link(name = "Xext")]
unsafe extern "C" {
    fn XShmQueryExtension(display: *mut xlib::Display) -> xlib::Bool;
    fn XShmCreateImage(
        display: *mut xlib::Display,
        visual: *mut xlib::Visual,
        depth: c_uint,
        format: c_int,
        data: *mut i8,
        segment: *mut xshm::XShmSegmentInfo,
        width: c_uint,
        height: c_uint,
    ) -> *mut xlib::XImage;
    fn XShmAttach(display: *mut xlib::Display, segment: *mut xshm::XShmSegmentInfo) -> xlib::Bool;
    fn XShmDetach(display: *mut xlib::Display, segment: *mut xshm::XShmSegmentInfo) -> xlib::Bool;
    fn XShmGetImage(
        display: *mut xlib::Display,
        drawable: xlib::Drawable,
        image: *mut xlib::XImage,
        x: c_int,
        y: c_int,
        plane_mask: u64,
    ) -> xlib::Bool;
}

const CAPTURE_MARGIN: i32 = 48;
const BLUR_RADIUS: usize = 6;
const REFRACTION_DEPTH: f32 = 35.0;
const REFRACTION_INDEX: f32 = 1.7;
const REFRACTION_SCALE: f32 = 65.0;
const CHROMA_STRENGTH: f32 = 0.30;
const CORNER_RADIUS: f32 = 24.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DesktopContext {
    active_window: c_ulong,
    workspace: c_ulong,
}

pub(crate) struct DesktopMonitor {
    display: *mut xlib::Display,
    root: xlib::Window,
    active_window_atom: xlib::Atom,
    workspace_atom: xlib::Atom,
}

impl DesktopMonitor {
    pub(crate) fn new() -> Option<Self> {
        unsafe {
            let display = xlib::XOpenDisplay(ptr::null());
            if display.is_null() {
                return None;
            }
            let screen = xlib::XDefaultScreen(display);
            Some(Self {
                display,
                root: xlib::XRootWindow(display, screen),
                active_window_atom: xlib::XInternAtom(
                    display,
                    c"_NET_ACTIVE_WINDOW".as_ptr(),
                    xlib::False,
                ),
                workspace_atom: xlib::XInternAtom(
                    display,
                    c"_NET_CURRENT_DESKTOP".as_ptr(),
                    xlib::False,
                ),
            })
        }
    }

    pub(crate) fn context(&self) -> Option<DesktopContext> {
        Some(DesktopContext {
            active_window: self.property(self.active_window_atom)?,
            workspace: self.property(self.workspace_atom).unwrap_or(0),
        })
    }

    fn property(&self, property: xlib::Atom) -> Option<c_ulong> {
        unsafe {
            let mut actual_type = 0;
            let mut actual_format = 0;
            let mut item_count = 0;
            let mut bytes_after = 0;
            let mut data: *mut c_uchar = ptr::null_mut();
            let status = xlib::XGetWindowProperty(
                self.display,
                self.root,
                property,
                0,
                1,
                xlib::False,
                xlib::AnyPropertyType as c_ulong,
                &mut actual_type,
                &mut actual_format,
                &mut item_count,
                &mut bytes_after,
                &mut data,
            );
            if status != xlib::Success as c_int
                || actual_format != 32
                || item_count != 1
                || data.is_null()
            {
                if !data.is_null() {
                    xlib::XFree(data.cast());
                }
                return None;
            }
            let value = *data.cast::<c_ulong>();
            xlib::XFree(data.cast());
            Some(value)
        }
    }
}

impl Drop for DesktopMonitor {
    fn drop(&mut self) {
        unsafe {
            xlib::XCloseDisplay(self.display);
        }
    }
}

#[derive(Clone, Copy, Default)]
struct Pixel {
    red: f32,
    green: f32,
    blue: f32,
}

pub(crate) struct DesktopSnapshot {
    width: i32,
    height: i32,
    pixels: Vec<[u8; 3]>,
}

impl DesktopSnapshot {
    pub(crate) fn capture() -> Option<Self> {
        unsafe {
            let display = xlib::XOpenDisplay(ptr::null());
            if display.is_null() {
                return None;
            }
            let screen = xlib::XDefaultScreen(display);
            let width = xlib::XDisplayWidth(display, screen);
            let height = xlib::XDisplayHeight(display, screen);
            let root = xlib::XRootWindow(display, screen);
            if XShmQueryExtension(display) == 0 {
                xlib::XCloseDisplay(display);
                return None;
            }

            let mut segment: xshm::XShmSegmentInfo = mem::zeroed();
            let image = XShmCreateImage(
                display,
                xlib::XDefaultVisual(display, screen),
                xlib::XDefaultDepth(display, screen) as u32,
                xlib::ZPixmap,
                ptr::null_mut(),
                &mut segment,
                width as u32,
                height as u32,
            );
            if image.is_null() {
                xlib::XCloseDisplay(display);
                return None;
            }

            let allocation_size = ((*image).bytes_per_line * (*image).height) as usize;
            segment.shmid = shmget(IPC_PRIVATE, allocation_size, IPC_CREAT | 0o600);
            if segment.shmid < 0 {
                xlib::XDestroyImage(image);
                xlib::XCloseDisplay(display);
                return None;
            }
            segment.shmaddr = shmat(segment.shmid, ptr::null(), 0).cast();
            if segment.shmaddr as isize == -1 {
                shmctl(segment.shmid, IPC_RMID, ptr::null_mut());
                xlib::XDestroyImage(image);
                xlib::XCloseDisplay(display);
                return None;
            }
            segment.readOnly = 0;
            (*image).data = segment.shmaddr;
            if XShmAttach(display, &mut segment) == 0 {
                (*image).data = ptr::null_mut();
                xlib::XDestroyImage(image);
                shmdt(segment.shmaddr.cast());
                shmctl(segment.shmid, IPC_RMID, ptr::null_mut());
                xlib::XCloseDisplay(display);
                return None;
            }
            xlib::XSync(display, 0);
            shmctl(segment.shmid, IPC_RMID, ptr::null_mut());
            let captured = XShmGetImage(display, root, image, 0, 0, !0) != 0;

            let pixels = captured.then(|| copy_pixels(image, width, height));
            XShmDetach(display, &mut segment);
            xlib::XSync(display, 0);
            (*image).data = ptr::null_mut();
            xlib::XDestroyImage(image);
            shmdt(segment.shmaddr.cast());
            xlib::XCloseDisplay(display);
            pixels.map(|pixels| Self {
                width,
                height,
                pixels,
            })
        }
    }

    pub(crate) fn render(
        &self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        scale: i32,
    ) -> Option<(ImageSurface, f64)> {
        let scale = scale.max(1);
        let pixel_width = width * scale;
        let pixel_height = height * scale;
        let margin = CAPTURE_MARGIN * scale;
        let expanded_width = pixel_width + margin * 2;
        let expanded_height = pixel_height + margin * 2;
        let mut crop = Vec::with_capacity((expanded_width * expanded_height) as usize);
        for local_y in 0..expanded_height {
            for local_x in 0..expanded_width {
                crop.push(self.pixel(x * scale + local_x - margin, y * scale + local_y - margin));
            }
        }

        let blurred = gaussian_blur(
            &crop,
            expanded_width as usize,
            expanded_height as usize,
            BLUR_RADIUS * scale as usize,
        );
        let stride = Format::ARgb32.stride_for_width(pixel_width as u32).ok()?;
        let mut output = vec![0_u8; (stride * pixel_height) as usize];
        let mut luminance = 0.0_f64;
        let refraction_depth = REFRACTION_DEPTH * scale as f32;
        for output_y in 0..pixel_height {
            for output_x in 0..pixel_width {
                let (normal_x, normal_y, depth) = glass_normal(
                    output_x as f32 + 0.5,
                    output_y as f32 + 0.5,
                    pixel_width as f32,
                    pixel_height as f32,
                    CORNER_RADIUS * scale as f32,
                );
                let edge = (1.0 - depth / refraction_depth).clamp(0.0, 1.0);
                let sin_incident = edge * edge;
                let incident = sin_incident.asin();
                let transmitted = (sin_incident / REFRACTION_INDEX).asin();
                let displacement = (incident - transmitted).tan() * REFRACTION_SCALE * scale as f32;
                let refracted_x = output_x as f32 + margin as f32 - normal_x * displacement;
                let refracted_y = output_y as f32 + margin as f32 - normal_y * displacement;
                let chroma = CHROMA_STRENGTH * refraction_depth * 0.35 * edge;
                let red = sample(
                    &blurred,
                    expanded_width as usize,
                    expanded_height as usize,
                    refracted_x + normal_x * chroma,
                    refracted_y + normal_y * chroma,
                )
                .red;
                let center = sample(
                    &blurred,
                    expanded_width as usize,
                    expanded_height as usize,
                    refracted_x,
                    refracted_y,
                );
                let blue = sample(
                    &blurred,
                    expanded_width as usize,
                    expanded_height as usize,
                    refracted_x - normal_x * chroma,
                    refracted_y - normal_y * chroma,
                )
                .blue;
                luminance += (0.2126 * f64::from(red)
                    + 0.7152 * f64::from(center.green)
                    + 0.0722 * f64::from(blue))
                    / 255.0;
                write_argb32(
                    &mut output,
                    output_y as usize * stride as usize + output_x as usize * 4,
                    red,
                    center.green,
                    blue,
                );
            }
        }
        let surface = ImageSurface::create_for_data(
            output,
            Format::ARgb32,
            pixel_width,
            pixel_height,
            stride,
        )
        .ok()?;
        let pixel_count = f64::from(pixel_width) * f64::from(pixel_height);
        Some((surface, luminance / pixel_count))
    }

    fn pixel(&self, x: i32, y: i32) -> Pixel {
        let x = x.clamp(0, self.width - 1) as usize;
        let y = y.clamp(0, self.height - 1) as usize;
        let [red, green, blue] = self.pixels[y * self.width as usize + x];
        Pixel {
            red: red as f32,
            green: green as f32,
            blue: blue as f32,
        }
    }
}

unsafe fn copy_pixels(image: *mut xlib::XImage, width: i32, height: i32) -> Vec<[u8; 3]> {
    let source = slice::from_raw_parts(
        (*image).data.cast::<u8>(),
        ((*image).bytes_per_line * height) as usize,
    );
    let bytes_per_pixel = ((*image).bits_per_pixel as usize).div_ceil(8);
    let mut pixels = Vec::with_capacity((width * height) as usize);
    for y in 0..height as usize {
        for x in 0..width as usize {
            let offset = y * (*image).bytes_per_line as usize + x * bytes_per_pixel;
            let bytes = &source[offset..offset + bytes_per_pixel];
            let value = if (*image).byte_order == xlib::LSBFirst {
                bytes
                    .iter()
                    .enumerate()
                    .fold(0_u64, |value, (shift, byte)| {
                        value | (u64::from(*byte) << (shift * 8))
                    })
            } else {
                bytes
                    .iter()
                    .fold(0_u64, |value, byte| (value << 8) | u64::from(*byte))
            };
            pixels.push([
                channel(value, (*image).red_mask),
                channel(value, (*image).green_mask),
                channel(value, (*image).blue_mask),
            ]);
        }
    }
    pixels
}

fn channel(pixel: u64, mask: u64) -> u8 {
    if mask == 0 {
        return 0;
    }
    let shift = mask.trailing_zeros();
    let maximum = mask >> shift;
    (((pixel & mask) >> shift) * 255 / maximum) as u8
}

fn gaussian_blur(source: &[Pixel], width: usize, height: usize, radius: usize) -> Vec<Pixel> {
    let mut output = source.to_vec();
    for _ in 0..3 {
        output = box_blur(&output, width, height, radius);
    }
    output
}

fn box_blur(source: &[Pixel], width: usize, height: usize, radius: usize) -> Vec<Pixel> {
    let divisor = (radius * 2 + 1) as f32;
    let mut horizontal = vec![Pixel::default(); source.len()];
    for y in 0..height {
        let mut sum = Pixel::default();
        for offset in -(radius as isize)..=radius as isize {
            let sample_x = offset.clamp(0, width as isize - 1) as usize;
            add_pixel(&mut sum, source[y * width + sample_x]);
        }
        for x in 0..width {
            horizontal[y * width + x] = divided(sum, divisor);
            if x + 1 < width {
                let leaving = (x as isize - radius as isize).clamp(0, width as isize - 1);
                let entering = (x + radius + 1).min(width - 1);
                subtract_pixel(&mut sum, source[y * width + leaving as usize]);
                add_pixel(&mut sum, source[y * width + entering]);
            }
        }
    }

    let mut output = vec![Pixel::default(); source.len()];
    for x in 0..width {
        let mut sum = Pixel::default();
        for offset in -(radius as isize)..=radius as isize {
            let sample_y = offset.clamp(0, height as isize - 1) as usize;
            add_pixel(&mut sum, horizontal[sample_y * width + x]);
        }
        for y in 0..height {
            output[y * width + x] = divided(sum, divisor);
            if y + 1 < height {
                let leaving = (y as isize - radius as isize).clamp(0, height as isize - 1);
                let entering = (y + radius + 1).min(height - 1);
                subtract_pixel(&mut sum, horizontal[leaving as usize * width + x]);
                add_pixel(&mut sum, horizontal[entering * width + x]);
            }
        }
    }
    output
}

fn add_pixel(target: &mut Pixel, source: Pixel) {
    target.red += source.red;
    target.green += source.green;
    target.blue += source.blue;
}

fn subtract_pixel(target: &mut Pixel, source: Pixel) {
    target.red -= source.red;
    target.green -= source.green;
    target.blue -= source.blue;
}

fn divided(pixel: Pixel, divisor: f32) -> Pixel {
    Pixel {
        red: pixel.red / divisor,
        green: pixel.green / divisor,
        blue: pixel.blue / divisor,
    }
}

fn rounded_rect_distance(x: f32, y: f32, width: f32, height: f32, radius: f32) -> f32 {
    let radius = radius.min(width / 2.0).min(height / 2.0);
    let qx = (x - width / 2.0).abs() - (width / 2.0 - radius);
    let qy = (y - height / 2.0).abs() - (height / 2.0 - radius);
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radius
}

fn glass_normal(x: f32, y: f32, width: f32, height: f32, radius: f32) -> (f32, f32, f32) {
    let distance = rounded_rect_distance(x, y, width, height, radius);
    let dx = rounded_rect_distance(x + 0.5, y, width, height, radius)
        - rounded_rect_distance(x - 0.5, y, width, height, radius);
    let dy = rounded_rect_distance(x, y + 0.5, width, height, radius)
        - rounded_rect_distance(x, y - 0.5, width, height, radius);
    let length = dx.hypot(dy);
    let (normal_x, normal_y) = if length > 0.0001 {
        (dx / length, dy / length)
    } else {
        (0.0, 0.0)
    };
    (normal_x, normal_y, (-distance).max(0.0))
}

fn sample(pixels: &[Pixel], width: usize, height: usize, x: f32, y: f32) -> Pixel {
    let x = x.clamp(0.0, width.saturating_sub(1) as f32);
    let y = y.clamp(0.0, height.saturating_sub(1) as f32);
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let top = mix(pixels[y0 * width + x0], pixels[y0 * width + x1], tx);
    let bottom = mix(pixels[y1 * width + x0], pixels[y1 * width + x1], tx);
    mix(top, bottom, ty)
}

fn mix(first: Pixel, second: Pixel, amount: f32) -> Pixel {
    Pixel {
        red: first.red + (second.red - first.red) * amount,
        green: first.green + (second.green - first.green) * amount,
        blue: first.blue + (second.blue - first.blue) * amount,
    }
}

fn write_argb32(target: &mut [u8], offset: usize, red: f32, green: f32, blue: f32) {
    let red = red.round().clamp(0.0, 255.0) as u8;
    let green = green.round().clamp(0.0, 255.0) as u8;
    let blue = blue.round().clamp(0.0, 255.0) as u8;
    #[cfg(target_endian = "little")]
    target[offset..offset + 4].copy_from_slice(&[blue, green, red, 255]);
    #[cfg(target_endian = "big")]
    target[offset..offset + 4].copy_from_slice(&[255, red, green, blue]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blur_preserves_a_flat_color() {
        let color = Pixel {
            red: 23.0,
            green: 97.0,
            blue: 211.0,
        };
        let result = gaussian_blur(&vec![color; 35], 7, 5, 3);
        for pixel in result {
            assert!((pixel.red - color.red).abs() < 0.01);
            assert!((pixel.green - color.green).abs() < 0.01);
            assert!((pixel.blue - color.blue).abs() < 0.01);
        }
    }
}
