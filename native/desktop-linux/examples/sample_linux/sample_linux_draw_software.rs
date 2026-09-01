use crate::sample_linux::{DRAG_AND_DROP_LEFT_OF, WindowState};
use desktop_linux::linux::events::SoftwareDrawData;
use desktop_linux::linux::geometry::{LogicalPixelsInt, PhysicalSize, Scale};

const BYTES_PER_PIXEL: usize = 4;

fn between(val: f64, min: f64, max: f64) -> bool {
    val > min && val < max
}

fn scaled(v: LogicalPixelsInt, scale: Scale) -> f64 {
    v.to_raw_physical(scale)
}

#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
const fn pixel_bytes(v: f64) -> usize {
    v.round() as usize * BYTES_PER_PIXEL
}

fn stride_len(v: u32, stride: i32) -> usize {
    usize::try_from(v * stride.unsigned_abs()).unwrap()
}

fn stride_offset(v: i32, stride: i32) -> isize {
    isize::try_from(v * stride).unwrap()
}

pub fn draw_software(data: &SoftwareDrawData, physical_size: PhysicalSize, window_state: &WindowState) {
    let scale = window_state.scale;
    let stride = data.stride;
    let w = f64::from(physical_size.width.raw_physical());
    let h = f64::from(physical_size.height.raw_physical());

    let padding_left = scaled(window_state.frame.padding.left, scale);
    let padding_right = scaled(window_state.frame.padding.right, scale);
    let padding_top = scaled(window_state.frame.padding.top, scale);
    let padding_bottom = scaled(window_state.frame.padding.bottom, scale);

    let line_thickness = scaled(LogicalPixelsInt::new(4), scale);
    let drag_and_drop_left_of = scaled(DRAG_AND_DROP_LEFT_OF, scale) + padding_left;
    let drag_source_indicator_height = scaled(LogicalPixelsInt::new(100), scale) + padding_top;

    let mut horizontal_line = vec![0; stride_len(1, stride)];

    // Order of bytes in `pixel` is [b, g, r, a] (for the Argb8888 format)
    for (pixel, i) in horizontal_line.as_chunks_mut::<BYTES_PER_PIXEL>().0.iter_mut().zip(0..) {
        let x = f64::from(i % physical_size.width.raw_physical());
        if x <= padding_left || x >= w - padding_right {
            pixel.fill(128);
        } else if between(x, drag_and_drop_left_of, drag_and_drop_left_of + line_thickness) {
            *pixel = [0, 0, 0, 255];
        } else if between(x, padding_left + line_thickness, padding_left + (line_thickness * 2.))
            || between(x, w - (line_thickness * 2.) - padding_right, w - line_thickness - padding_right)
        {
            // left and right border
            *pixel = [0, 0, 255, 255];
        } else if x < drag_and_drop_left_of && window_state.drag_and_drop_target {
            *pixel = [128, 0, 0, 255];
        } else if window_state.active {
            pixel.fill(255);
        } else {
            *pixel = [128, 128, 128, 255];
        }
    }

    let content_w = w - padding_left - padding_right;
    let content_h = h - padding_top - padding_bottom;
    let padding_left_bytes = pixel_bytes(padding_left);
    let content_x_end_bytes = pixel_bytes(w - padding_right);
    let half_line_thickness = line_thickness / 2.;

    for y in 0..physical_size.height.raw_physical() {
        let line = unsafe { std::slice::from_raw_parts_mut(data.canvas.offset(stride_offset(y, stride)), pixel_bytes(w)) };
        let y = f64::from(y);

        if y <= padding_top || y >= h - padding_bottom {
            line.fill(128);
            continue;
        }

        line.copy_from_slice(&horizontal_line);

        if between(y, padding_top + line_thickness, padding_top + (line_thickness * 2.))
            || between(y, h - (line_thickness * 2.) - padding_bottom, h - line_thickness - padding_bottom)
        {
            // top and bottom border
            line[padding_left_bytes..content_x_end_bytes]
                .as_chunks_mut::<BYTES_PER_PIXEL>()
                .0
                .fill([0, 0, 255, 255]);
        }

        if window_state.drag_and_drop_source && between(y, drag_source_indicator_height, drag_source_indicator_height + line_thickness) {
            line[padding_left_bytes..pixel_bytes(drag_and_drop_left_of)]
                .as_chunks_mut::<BYTES_PER_PIXEL>()
                .0
                .fill([255, 0, 0, 255]);
        }

        let diagonal_x = ((y - padding_top) * content_w) / content_h;

        let diagonal_start_x_pixel = padding_left_bytes + pixel_bytes((diagonal_x - half_line_thickness).max(0.));
        let diagonal_end_x_pixel = (padding_left_bytes + pixel_bytes(diagonal_x + half_line_thickness)).min(content_x_end_bytes);
        line[diagonal_start_x_pixel..diagonal_end_x_pixel]
            .as_chunks_mut::<BYTES_PER_PIXEL>()
            .0
            .fill([0, 0, 255, 255]);
    }
}
