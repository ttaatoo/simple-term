use gpui::{point, px, size, Bounds, Pixels, ScrollDelta, TouchPhase};
use simple_term::terminal_settings::{AlternateScroll, WorkingDirectory};
use simple_term::TermMode;
use std::time::{Duration, Instant};
use url::Url;

pub(super) const INPUT_SCROLL_SUPPRESSION_WINDOW: Duration = Duration::from_millis(180);
const SCROLLBAR_WIDTH: Pixels = px(10.0);
const SCROLLBAR_PADDING: Pixels = px(1.0);
const SCROLLBAR_MIN_THUMB_HEIGHT: Pixels = px(24.0);

#[derive(Clone, Debug)]
pub(super) struct ScrollbarLayout {
    pub(super) track: Bounds<Pixels>,
    pub(super) thumb: Bounds<Pixels>,
    pub(super) max_offset: usize,
}

pub(super) fn consume_scroll_lines(pending: &mut f32, delta_lines: f32) -> i32 {
    const EPSILON: f32 = 1e-4;

    *pending += delta_lines;

    let whole_lines = if *pending >= 0.0 {
        (*pending + EPSILON).floor() as i32
    } else {
        (*pending - EPSILON).ceil() as i32
    };

    *pending -= whole_lines as f32;
    if pending.abs() < EPSILON {
        *pending = 0.0;
    }

    whole_lines
}

pub(super) fn effective_scroll_multiplier(multiplier: f32) -> f32 {
    if multiplier.is_finite() {
        multiplier.max(0.01)
    } else {
        1.0
    }
}

pub(super) fn scroll_delta_to_lines(delta: ScrollDelta, line_height: Pixels) -> f32 {
    match delta {
        ScrollDelta::Lines(pt) => pt.y,
        ScrollDelta::Pixels(pt) => f32::from(pt.y) / f32::from(line_height),
    }
}

pub(super) fn viewport_row_for_line(
    line: i32,
    display_offset: usize,
    viewport_lines: usize,
) -> Option<usize> {
    let row = line + display_offset as i32;
    if row < 0 {
        return None;
    }

    let row = row as usize;
    (row < viewport_lines).then_some(row)
}

pub(super) fn prepare_for_terminal_input(
    was_scrolled: bool,
    pending_scroll_lines: &mut f32,
    suppress_precise_scroll_until: &mut Option<Instant>,
    suppress_precise_scroll_until_ended: &mut bool,
    now: Instant,
) {
    *pending_scroll_lines = 0.0;
    if was_scrolled {
        *suppress_precise_scroll_until = Some(now + INPUT_SCROLL_SUPPRESSION_WINDOW);
        *suppress_precise_scroll_until_ended = true;
    }
}

pub(super) fn should_ignore_scroll_event(
    touch_phase: TouchPhase,
    precise: bool,
    pending_scroll_lines: &mut f32,
    suppress_precise_scroll_until: &mut Option<Instant>,
    suppress_precise_scroll_until_ended: &mut bool,
    now: Instant,
) -> bool {
    match touch_phase {
        TouchPhase::Started => {
            *pending_scroll_lines = 0.0;
            if !precise {
                *suppress_precise_scroll_until = None;
                *suppress_precise_scroll_until_ended = false;
            }
            return true;
        }
        TouchPhase::Ended => {
            *pending_scroll_lines = 0.0;
            if precise {
                *suppress_precise_scroll_until_ended = false;
                *suppress_precise_scroll_until = None;
            }
            return true;
        }
        TouchPhase::Moved => {}
    }

    if precise {
        if *suppress_precise_scroll_until_ended {
            *pending_scroll_lines = 0.0;
            return true;
        }

        if let Some(until) = *suppress_precise_scroll_until {
            if now < until {
                *pending_scroll_lines = 0.0;
                return true;
            }
            *suppress_precise_scroll_until = None;
        }
    } else {
        // A line-based wheel interaction indicates a new user gesture.
        *suppress_precise_scroll_until = None;
        *suppress_precise_scroll_until_ended = false;
    }

    false
}

pub(super) fn mouse_mode_enabled_for_scroll(mode: TermMode, shift_held: bool) -> bool {
    mode.intersects(TermMode::MOUSE_MODE) && !shift_held
}

pub(super) fn alternate_scroll_enabled(
    mode: TermMode,
    setting: AlternateScroll,
    shift_held: bool,
) -> bool {
    !shift_held
        && matches!(setting, AlternateScroll::On)
        && mode.contains(TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL)
}

pub(super) fn point_in_bounds(bounds: &Bounds<Pixels>, point: gpui::Point<Pixels>) -> bool {
    point.x >= bounds.origin.x
        && point.x <= bounds.origin.x + bounds.size.width
        && point.y >= bounds.origin.y
        && point.y <= bounds.origin.y + bounds.size.height
}

pub(super) fn scrollbar_thumb_metrics(
    track_height: Pixels,
    viewport_lines: usize,
    history_size: usize,
    display_offset: usize,
) -> Option<(Pixels, Pixels, usize)> {
    if history_size == 0 || viewport_lines == 0 || track_height <= px(0.0) {
        return None;
    }

    let max_offset = history_size;
    let clamped_offset = display_offset.min(max_offset);
    let total_lines = viewport_lines + history_size;
    let visible_ratio = viewport_lines as f32 / total_lines as f32;
    let thumb_height = (track_height * visible_ratio)
        .max(SCROLLBAR_MIN_THUMB_HEIGHT)
        .min(track_height);
    let max_thumb_top = (track_height - thumb_height).max(px(0.0));
    let thumb_top = if max_offset == 0 || max_thumb_top <= px(0.0) {
        px(0.0)
    } else {
        max_thumb_top * (1.0 - clamped_offset as f32 / max_offset as f32)
    };

    Some((thumb_top, thumb_height, max_offset))
}

pub(super) fn display_offset_from_thumb_top(
    thumb_top: Pixels,
    track_height: Pixels,
    thumb_height: Pixels,
    max_offset: usize,
) -> usize {
    if max_offset == 0 {
        return 0;
    }

    let max_thumb_top = (track_height - thumb_height).max(px(0.0));
    if max_thumb_top <= px(0.0) {
        return 0;
    }

    let clamped_thumb_top = thumb_top.max(px(0.0)).min(max_thumb_top);
    let ratio = f32::from(clamped_thumb_top) / f32::from(max_thumb_top);
    ((1.0 - ratio) * max_offset as f32).round() as usize
}

pub(super) fn scrollbar_layout(
    content_bounds: Bounds<Pixels>,
    viewport_lines: usize,
    history_size: usize,
    display_offset: usize,
) -> Option<ScrollbarLayout> {
    if content_bounds.size.width <= SCROLLBAR_WIDTH {
        return None;
    }

    let track = Bounds {
        origin: point(
            content_bounds.origin.x + content_bounds.size.width - SCROLLBAR_WIDTH,
            content_bounds.origin.y,
        ),
        size: size(SCROLLBAR_WIDTH, content_bounds.size.height),
    };

    let (thumb_top, thumb_height, max_offset) = scrollbar_thumb_metrics(
        track.size.height,
        viewport_lines,
        history_size,
        display_offset,
    )?;

    let thumb = Bounds {
        origin: point(
            track.origin.x + SCROLLBAR_PADDING,
            track.origin.y + thumb_top,
        ),
        size: size(
            track.size.width - SCROLLBAR_PADDING * 2.0,
            thumb_height.max(px(1.0)),
        ),
    };

    Some(ScrollbarLayout {
        track,
        thumb,
        max_offset,
    })
}

pub(super) fn display_offset_from_pointer(
    pointer_y: Pixels,
    layout: &ScrollbarLayout,
    grab_offset: Pixels,
) -> usize {
    let thumb_top = pointer_y - layout.track.origin.y - grab_offset;
    display_offset_from_thumb_top(
        thumb_top,
        layout.track.size.height,
        layout.thumb.size.height,
        layout.max_offset,
    )
}

pub(super) fn resolve_working_directory(strategy: &WorkingDirectory) -> Option<std::path::PathBuf> {
    resolve_working_directory_with_fallback(
        strategy,
        std::env::current_dir().ok(),
        dirs::home_dir(),
    )
}

pub(super) fn resolve_working_directory_with_fallback(
    strategy: &WorkingDirectory,
    current_dir: Option<std::path::PathBuf>,
    home_dir: Option<std::path::PathBuf>,
) -> Option<std::path::PathBuf> {
    match strategy {
        WorkingDirectory::Always { directory } => Some(directory.clone()),
        WorkingDirectory::AlwaysHome => home_dir,
        WorkingDirectory::CurrentFileDirectory
        | WorkingDirectory::CurrentProjectDirectory
        | WorkingDirectory::FirstProjectDirectory => current_dir.or(home_dir),
    }
}

pub(super) fn selection_copy_plan(
    copy_on_select: bool,
    keep_selection_on_copy: bool,
    selected_text: Option<String>,
) -> (Option<String>, bool) {
    if !copy_on_select {
        return (None, false);
    }

    let Some(text) = selected_text.filter(|text| !text.is_empty()) else {
        return (None, false);
    };

    (Some(text), !keep_selection_on_copy)
}

pub(super) fn text_to_insert(keystroke: &gpui::Keystroke) -> Option<String> {
    if keystroke.modifiers.control || keystroke.modifiers.platform {
        return None;
    }

    if keystroke.modifiers.alt && keystroke.key_char.is_none() {
        return None;
    }

    if let Some(text) = keystroke.key_char.as_ref() {
        if !text.is_empty() {
            return Some(text.clone());
        }
    }

    if keystroke.key.len() == 1 && !keystroke.modifiers.alt && !keystroke.modifiers.function {
        return Some(keystroke.key.clone());
    }

    None
}

pub(super) fn strip_line_column_suffix(target: &str) -> &str {
    let mut end = target.len();
    let bytes = target.as_bytes();

    while end > 0 && bytes[end - 1].is_ascii_digit() {
        end -= 1;
    }

    if end > 0 && bytes[end - 1] == b':' {
        let mut second = end - 1;
        while second > 0 && bytes[second - 1].is_ascii_digit() {
            second -= 1;
        }

        if second > 0 && bytes[second - 1] == b':' {
            return &target[..second - 1];
        }

        return &target[..end - 1];
    }

    target
}

fn percent_encode_file_path(path: &str) -> String {
    let mut encoded = String::with_capacity(path.len());
    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b':' | b'.' | b'-' | b'_' | b'~' => {
                encoded.push(byte as char)
            }
            _ => {
                encoded.push('%');
                encoded.push_str(&format!("{byte:02X}"));
            }
        }
    }
    encoded
}

pub(super) fn file_path_to_file_url(target: &str) -> String {
    let path = std::path::Path::new(target);
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else if let Ok(cwd) = std::env::current_dir() {
        cwd.join(path)
    } else {
        path.to_path_buf()
    };

    if let Ok(url) = Url::from_file_path(&absolute_path) {
        return url.into();
    }

    format!(
        "file://{}",
        percent_encode_file_path(&absolute_path.to_string_lossy())
    )
}
