//! Terminal view - renders the terminal using GPUI

use alacritty_terminal::event::WindowSize;
use alacritty_terminal::grid::{Indexed, Scroll};
use alacritty_terminal::index::Side;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::color::Colors as AlacColors;
use alacritty_terminal::vte::ansi::{Color as AlacColor, CursorShape, NamedColor, Rgb as AlacRgb};

use gpui::{
    canvas, div, fill, hsla, point, px, size, App, AsyncWindowContext, Bounds, ClipboardItem,
    ContentMask, Context, FocusHandle, Focusable, Font, FontFallbacks, FontFeatures, FontStyle,
    FontWeight, Hsla, InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Render, Rgba, ScrollDelta,
    ScrollWheelEvent, SharedString, Size, Styled, Subscription, TextRun, TouchPhase, WeakEntity,
    Window,
};
use std::time::{Duration, Instant};

use zed_terminal::mappings::mouse::{
    alt_scroll, grid_point, grid_point_and_side, mouse_button_report, mouse_moved_report,
    scroll_report,
};
use zed_terminal::terminal::{Terminal, TerminalEvent};
use zed_terminal::terminal_hyperlinks::{find_from_grid_point, RegexSearches};
use zed_terminal::terminal_settings::{AlternateScroll, TerminalSettings, WorkingDirectory};
use zed_terminal::{
    AlacPoint, Dimensions, PathStyle, Selection, SelectionType, TermMode, TerminalBounds,
};

pub struct TerminalView {
    terminal: Terminal,
    regex_searches: RegexSearches,
    settings: TerminalSettings,
    focus_handle: FocusHandle,
    font: Font,
    font_size: Pixels,
    cell_size: Size<Pixels>,
    grid_size: Size<u16>,
    pending_scroll_lines: f32,
    suppress_precise_scroll_until: Option<Instant>,
    suppress_precise_scroll_until_ended: bool,
    selection_anchor: Option<(AlacPoint, Side)>,
    scrollbar_drag_offset: Option<Pixels>,
    _resize_subscription: Subscription,
}

impl TerminalView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, settings: TerminalSettings) -> Self {
        let text_system = window.text_system().clone();
        let mut font = Font {
            family: SharedString::from(settings.font_family.clone()),
            features: FontFeatures::default(),
            fallbacks: Some(FontFallbacks::from_fonts(settings.font_fallbacks.clone())),
            weight: FontWeight::NORMAL,
            style: FontStyle::Normal,
        };
        let font_size = px(settings.font_size);

        if !is_monospace_font(&text_system, &font, font_size) {
            if let Some(monospace_fallback) = settings.font_fallbacks.iter().find_map(|family| {
                let candidate = Font {
                    family: SharedString::from(family.clone()),
                    features: FontFeatures::default(),
                    fallbacks: None,
                    weight: FontWeight::NORMAL,
                    style: FontStyle::Normal,
                };
                is_monospace_font(&text_system, &candidate, font_size).then_some(candidate)
            }) {
                log::warn!(
                    "Terminal font '{}' resolved to non-monospace metrics; using fallback '{}'",
                    settings.font_family,
                    monospace_fallback.family
                );
                font = monospace_fallback;
            } else {
                log::warn!(
                    "Terminal font '{}' appears non-monospace and no monospace fallback matched",
                    settings.font_family
                );
            }
        }

        // Calculate cell dimensions from font metrics
        let font_id = text_system.resolve_font(&font);
        let cell_advance = text_system
            .advance(font_id, font_size, 'm')
            .unwrap_or(Size {
                width: px(8.4),
                height: px(17.0),
            });
        let cell_width = cell_advance.width;
        let line_height = font_size * settings.line_height.to_ratio();

        // Compute initial grid size from window dimensions
        let viewport = window.viewport_size();
        let initial_cols = std::cmp::max(
            (f32::from(viewport.width) / f32::from(cell_width)) as u16,
            1,
        );
        let initial_lines = std::cmp::max(
            (f32::from(viewport.height) / f32::from(line_height)) as u16,
            1,
        );

        let window_size = WindowSize {
            num_lines: initial_lines,
            num_cols: initial_cols,
            cell_width: f32::from(cell_width) as u16,
            cell_height: f32::from(line_height) as u16,
        };

        let scrollback_lines = settings
            .max_scroll_history_lines
            .unwrap_or(zed_terminal::config::DEFAULT_SCROLL_HISTORY_LINES);
        let working_directory = resolve_working_directory(&settings.working_directory);
        let terminal = Terminal::new(
            settings.shell.to_shell(),
            working_directory,
            window_size,
            scrollback_lines,
            settings.env.clone(),
        )
        .expect("Failed to spawn terminal");
        let regex_searches = RegexSearches::new(
            &settings.path_hyperlink_regexes,
            settings.path_hyperlink_timeout_ms,
        );

        let focus_handle = cx.focus_handle();

        // Resize terminal when window bounds change
        let resize_subscription =
            cx.observe_window_bounds(window, |this: &mut Self, window, cx| {
                this.handle_resize(window, cx);
            });

        // Poll for terminal events to trigger re-renders
        let events = terminal.events.clone();
        cx.spawn_in(
            window,
            async move |this: WeakEntity<TerminalView>, cx: &mut AsyncWindowContext| {
                while let Ok(event) = events.recv().await {
                    match event {
                        TerminalEvent::Wakeup => {
                            let _ = cx.update(|_window, cx| {
                                let _ = this.update(cx, |_, cx| cx.notify());
                            });
                        }
                        TerminalEvent::TitleChanged(title) => {
                            let _ = cx.update(|window, cx| {
                                window.set_window_title(&title);
                                let _ = this.update(cx, |_, cx| cx.notify());
                            });
                        }
                        TerminalEvent::Bell => {}
                        TerminalEvent::Exit(_) => break,
                    }
                }
            },
        )
        .detach();

        TerminalView {
            terminal,
            regex_searches,
            settings,
            focus_handle,
            font,
            font_size,
            cell_size: Size {
                width: cell_width,
                height: line_height,
            },
            grid_size: Size {
                width: initial_cols,
                height: initial_lines,
            },
            pending_scroll_lines: 0.0,
            suppress_precise_scroll_until: None,
            suppress_precise_scroll_until_ended: false,
            selection_anchor: None,
            scrollbar_drag_offset: None,
            _resize_subscription: resize_subscription,
        }
    }

    fn mode_and_display_offset(&self) -> (TermMode, usize) {
        let term = self.terminal.term.lock();
        (*term.mode(), term.grid().display_offset())
    }

    fn terminal_bounds(&self) -> TerminalBounds {
        TerminalBounds::new(
            self.cell_size.height,
            self.cell_size.width,
            Bounds {
                origin: point(px(0.), px(0.)),
                size: size(
                    self.cell_size.width * self.grid_size.width as f32,
                    self.cell_size.height * self.grid_size.height as f32,
                ),
            },
        )
    }

    fn scrollbar_layout(&self) -> Option<ScrollbarLayout> {
        let term = self.terminal.term.lock();
        if term.mode().contains(TermMode::ALT_SCREEN) {
            return None;
        }

        scrollbar_layout(
            self.terminal_bounds().bounds,
            term.screen_lines(),
            term.history_size(),
            term.grid().display_offset(),
        )
    }

    fn set_display_offset(&mut self, target_offset: usize) -> bool {
        let mut term = self.terminal.term.lock();
        let max_offset = term.history_size();
        let clamped_target = target_offset.min(max_offset);
        let current_offset = term.grid().display_offset();
        let delta = clamped_target as i32 - current_offset as i32;
        if delta == 0 {
            return false;
        }

        term.scroll_display(Scroll::Delta(delta));
        true
    }

    fn handle_resize(&mut self, window: &Window, cx: &mut Context<Self>) {
        let viewport = window.viewport_size();
        let new_cols = (f32::from(viewport.width) / f32::from(self.cell_size.width)) as u16;
        let new_lines = (f32::from(viewport.height) / f32::from(self.cell_size.height)) as u16;

        if new_cols > 0
            && new_lines > 0
            && (new_cols != self.grid_size.width || new_lines != self.grid_size.height)
        {
            let window_size = WindowSize {
                num_cols: new_cols,
                num_lines: new_lines,
                cell_width: f32::from(self.cell_size.width) as u16,
                cell_height: f32::from(self.cell_size.height) as u16,
            };
            self.terminal.resize(window_size);
            self.grid_size = Size {
                width: new_cols,
                height: new_lines,
            };
            cx.notify();
        }
    }

    fn scroll_to_bottom(&mut self) -> bool {
        let mut term = self.terminal.term.lock();
        let was_scrolled = term.grid().display_offset() != 0;
        if was_scrolled {
            term.scroll_display(Scroll::Bottom);
        }
        was_scrolled
    }

    fn begin_terminal_input(&mut self) {
        let was_scrolled = self.scroll_to_bottom();
        prepare_for_terminal_input(
            was_scrolled,
            &mut self.pending_scroll_lines,
            &mut self.suppress_precise_scroll_until,
            &mut self.suppress_precise_scroll_until_ended,
            Instant::now(),
        );
    }
}

fn is_monospace_font(text_system: &gpui::WindowTextSystem, font: &Font, font_size: Pixels) -> bool {
    let font_id = text_system.resolve_font(font);
    let m = match text_system.advance(font_id, font_size, 'm') {
        Ok(size) => size.width,
        Err(_) => return false,
    };
    let i = match text_system.advance(font_id, font_size, 'i') {
        Ok(size) => size.width,
        Err(_) => return false,
    };
    let w = match text_system.advance(font_id, font_size, 'W') {
        Ok(size) => size.width,
        Err(_) => return false,
    };

    // Keep a small tolerance for rasterizer/font metric rounding.
    let tolerance = px(0.5);
    (m - i).abs() <= tolerance && (m - w).abs() <= tolerance
}

fn consume_scroll_lines(pending: &mut f32, delta_lines: f32) -> i32 {
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

fn effective_scroll_multiplier(multiplier: f32) -> f32 {
    if multiplier.is_finite() {
        multiplier.max(0.01)
    } else {
        1.0
    }
}

fn scroll_delta_to_lines(delta: ScrollDelta, line_height: Pixels) -> f32 {
    match delta {
        ScrollDelta::Lines(pt) => pt.y,
        ScrollDelta::Pixels(pt) => f32::from(pt.y) / f32::from(line_height),
    }
}

fn viewport_row_for_line(line: i32, display_offset: usize, viewport_lines: usize) -> Option<usize> {
    let row = line + display_offset as i32;
    if row < 0 {
        return None;
    }

    let row = row as usize;
    (row < viewport_lines).then_some(row)
}

fn prepare_for_terminal_input(
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

fn should_ignore_scroll_event(
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

fn mouse_mode_enabled_for_scroll(mode: TermMode, shift_held: bool) -> bool {
    mode.intersects(TermMode::MOUSE_MODE) && !shift_held
}

fn alternate_scroll_enabled(mode: TermMode, setting: AlternateScroll, shift_held: bool) -> bool {
    !shift_held
        && matches!(setting, AlternateScroll::On)
        && mode.contains(TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL)
}

const INPUT_SCROLL_SUPPRESSION_WINDOW: Duration = Duration::from_millis(180);
const SCROLLBAR_WIDTH: Pixels = px(10.0);
const SCROLLBAR_PADDING: Pixels = px(1.0);
const SCROLLBAR_MIN_THUMB_HEIGHT: Pixels = px(24.0);

#[derive(Clone, Debug)]
struct ScrollbarLayout {
    track: Bounds<Pixels>,
    thumb: Bounds<Pixels>,
    max_offset: usize,
}

fn point_in_bounds(bounds: &Bounds<Pixels>, point: gpui::Point<Pixels>) -> bool {
    point.x >= bounds.origin.x
        && point.x <= bounds.origin.x + bounds.size.width
        && point.y >= bounds.origin.y
        && point.y <= bounds.origin.y + bounds.size.height
}

fn scrollbar_thumb_metrics(
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

fn display_offset_from_thumb_top(
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

fn scrollbar_layout(
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

fn display_offset_from_pointer(
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

fn resolve_working_directory(strategy: &WorkingDirectory) -> Option<std::path::PathBuf> {
    resolve_working_directory_with_fallback(
        strategy,
        std::env::current_dir().ok(),
        dirs::home_dir(),
    )
}

fn resolve_working_directory_with_fallback(
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

fn selection_copy_plan(
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

fn text_to_insert(keystroke: &gpui::Keystroke) -> Option<String> {
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

impl Focusable for TerminalView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

/// Snapshot of a single cell.
#[derive(Clone)]
struct CellSnapshot {
    c: char,
    fg: AlacColor,
    bg: AlacColor,
    flags: Flags,
}

/// Snapshot of the terminal state taken while holding the lock.
struct TerminalSnapshot {
    rows: Vec<Vec<CellSnapshot>>,
    num_cols: usize,
    num_lines: usize,
    history_size: usize,
    display_offset: usize,
    cursor_row: Option<usize>,
    cursor_col: usize,
    show_cursor: bool,
    colors: ColorsSnapshot,
}

fn take_snapshot(terminal: &Terminal) -> TerminalSnapshot {
    let term = terminal.term.lock();
    let content = term.renderable_content();
    let colors = ColorsSnapshot::from_colors(content.colors);
    let cursor = content.cursor;
    let selection = content.selection;
    let num_cols = term.columns();
    let num_lines = term.screen_lines();
    let history_size = term.history_size();
    let display_offset = term.grid().display_offset();

    let default_cell = CellSnapshot {
        c: ' ',
        fg: AlacColor::Named(NamedColor::Foreground),
        bg: AlacColor::Named(NamedColor::Background),
        flags: Flags::empty(),
    };

    let mut rows: Vec<Vec<CellSnapshot>> = (0..num_lines)
        .map(|_| vec![default_cell.clone(); num_cols])
        .collect();

    for indexed in content.display_iter {
        let Indexed { point, cell } = indexed;
        let Some(row) = viewport_row_for_line(point.line.0, display_offset, num_lines) else {
            continue;
        };
        let col = point.column.0;
        if col < num_cols {
            let mut fg = cell.fg;
            let mut bg = cell.bg;

            if cell.flags.contains(Flags::INVERSE) {
                std::mem::swap(&mut fg, &mut bg);
            }

            if let Some(ref sel) = selection {
                if sel.contains(point) {
                    std::mem::swap(&mut fg, &mut bg);
                }
            }

            rows[row][col] = CellSnapshot {
                c: cell.c,
                fg,
                bg,
                flags: cell.flags,
            };
        }
    }

    let cursor_row = viewport_row_for_line(cursor.point.line.0, display_offset, num_lines);
    let cursor_col = cursor.point.column.0;
    let show_cursor = cursor.shape != CursorShape::Hidden && cursor_row.is_some();

    drop(term);

    TerminalSnapshot {
        rows,
        num_cols,
        num_lines,
        history_size,
        display_offset,
        cursor_row,
        cursor_col,
        show_cursor,
        colors,
    }
}

fn strip_line_column_suffix(target: &str) -> &str {
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

impl Render for TerminalView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = take_snapshot(&self.terminal);
        let font = self.font.clone();
        let font_size = self.font_size;
        let cell_size = self.cell_size;

        div()
            .id("terminal")
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(gpui::black())
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, _window, cx| {
                    if let Some(layout) = this.scrollbar_layout() {
                        if point_in_bounds(&layout.track, event.position) {
                            let grab_offset = if point_in_bounds(&layout.thumb, event.position) {
                                event.position.y - layout.thumb.origin.y
                            } else {
                                layout.thumb.size.height / 2.0
                            };
                            let target_offset =
                                display_offset_from_pointer(event.position.y, &layout, grab_offset);
                            this.scrollbar_drag_offset = Some(grab_offset);
                            this.selection_anchor = None;
                            this.set_display_offset(target_offset);
                            cx.notify();
                            return;
                        }
                    }

                    let (mode, display_offset) = this.mode_and_display_offset();

                    if event.modifiers.secondary() {
                        let point =
                            grid_point(event.position, this.terminal_bounds(), display_offset);
                        let term_handle = this.terminal.term.clone();
                        let term = term_handle.lock();
                        if let Some((target, is_url, _match)) = find_from_grid_point(
                            &term,
                            point,
                            &mut this.regex_searches,
                            PathStyle::Unix,
                        ) {
                            drop(term);
                            if is_url {
                                cx.open_url(&target);
                            } else {
                                let file_path = strip_line_column_suffix(&target);
                                let file_url = format!("file://{}", file_path.replace(' ', "%20"));
                                cx.open_url(&file_url);
                            }
                            return;
                        }
                    }

                    if mode.intersects(TermMode::MOUSE_MODE) {
                        let point =
                            grid_point(event.position, this.terminal_bounds(), display_offset);
                        if let Some(bytes) =
                            mouse_button_report(point, event.button, event.modifiers, true, mode)
                        {
                            this.terminal.write(bytes);
                        }
                    } else {
                        let (point, side) = grid_point_and_side(
                            event.position,
                            this.terminal_bounds(),
                            display_offset,
                        );
                        this.selection_anchor = Some((point, side));

                        let mut term = this.terminal.term.lock();
                        term.selection = Some(Selection::new(SelectionType::Simple, point, side));
                        drop(term);

                        cx.notify();
                    }
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, event: &MouseDownEvent, _window, _cx| {
                    let (mode, display_offset) = this.mode_and_display_offset();
                    if mode.intersects(TermMode::MOUSE_MODE) {
                        let point =
                            grid_point(event.position, this.terminal_bounds(), display_offset);
                        if let Some(bytes) =
                            mouse_button_report(point, event.button, event.modifiers, true, mode)
                        {
                            this.terminal.write(bytes);
                        }
                    }
                }),
            )
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(|this, event: &MouseDownEvent, _window, _cx| {
                    let (mode, display_offset) = this.mode_and_display_offset();
                    if mode.intersects(TermMode::MOUSE_MODE) {
                        let point =
                            grid_point(event.position, this.terminal_bounds(), display_offset);
                        if let Some(bytes) =
                            mouse_button_report(point, event.button, event.modifiers, true, mode)
                        {
                            this.terminal.write(bytes);
                        }
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _window, cx| {
                    if this.scrollbar_drag_offset.take().is_some() {
                        cx.notify();
                        return;
                    }

                    let (mode, display_offset) = this.mode_and_display_offset();
                    if mode.intersects(TermMode::MOUSE_MODE) {
                        let point =
                            grid_point(event.position, this.terminal_bounds(), display_offset);
                        if let Some(bytes) =
                            mouse_button_report(point, event.button, event.modifiers, false, mode)
                        {
                            this.terminal.write(bytes);
                        }
                    } else if this.selection_anchor.is_some() {
                        let (point, side) = grid_point_and_side(
                            event.position,
                            this.terminal_bounds(),
                            display_offset,
                        );
                        let mut term = this.terminal.term.lock();
                        if let Some(selection) = term.selection.as_mut() {
                            selection.update(point, side);
                        }

                        let (copy_text, clear_selection) = selection_copy_plan(
                            this.settings.copy_on_select,
                            this.settings.keep_selection_on_copy,
                            term.selection_to_string(),
                        );

                        if clear_selection {
                            term.selection = None;
                        }

                        drop(term);

                        if let Some(text) = copy_text {
                            cx.write_to_clipboard(ClipboardItem::new_string(text));
                        }

                        this.selection_anchor = None;
                        cx.notify();
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(|this, event: &MouseUpEvent, _window, _cx| {
                    let (mode, display_offset) = this.mode_and_display_offset();
                    if mode.intersects(TermMode::MOUSE_MODE) {
                        let point =
                            grid_point(event.position, this.terminal_bounds(), display_offset);
                        if let Some(bytes) =
                            mouse_button_report(point, event.button, event.modifiers, false, mode)
                        {
                            this.terminal.write(bytes);
                        }
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Middle,
                cx.listener(|this, event: &MouseUpEvent, _window, _cx| {
                    let (mode, display_offset) = this.mode_and_display_offset();
                    if mode.intersects(TermMode::MOUSE_MODE) {
                        let point =
                            grid_point(event.position, this.terminal_bounds(), display_offset);
                        if let Some(bytes) =
                            mouse_button_report(point, event.button, event.modifiers, false, mode)
                        {
                            this.terminal.write(bytes);
                        }
                    }
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _window, cx| {
                if let Some(grab_offset) = this.scrollbar_drag_offset {
                    if event.pressed_button == Some(MouseButton::Left) {
                        if let Some(layout) = this.scrollbar_layout() {
                            let target_offset =
                                display_offset_from_pointer(event.position.y, &layout, grab_offset);
                            if this.set_display_offset(target_offset) {
                                cx.notify();
                            }
                        }
                    } else {
                        this.scrollbar_drag_offset = None;
                        cx.notify();
                    }
                    return;
                }

                let (mode, display_offset) = this.mode_and_display_offset();
                if mode.intersects(TermMode::MOUSE_MOTION | TermMode::MOUSE_DRAG) {
                    let point = grid_point(event.position, this.terminal_bounds(), display_offset);
                    if let Some(bytes) =
                        mouse_moved_report(point, event.pressed_button, event.modifiers, mode)
                    {
                        this.terminal.write(bytes);
                    }
                } else if event.pressed_button == Some(MouseButton::Left) {
                    let Some((anchor_point, anchor_side)) = this.selection_anchor else {
                        return;
                    };

                    let (point, side) =
                        grid_point_and_side(event.position, this.terminal_bounds(), display_offset);

                    let mut term = this.terminal.term.lock();
                    if let Some(selection) = term.selection.as_mut() {
                        selection.update(point, side);
                    } else {
                        let mut selection =
                            Selection::new(SelectionType::Simple, anchor_point, anchor_side);
                        selection.update(point, side);
                        term.selection = Some(selection);
                    }
                    drop(term);

                    cx.notify();
                }
            }))
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _window, cx| {
                if should_ignore_scroll_event(
                    event.touch_phase,
                    matches!(event.delta, ScrollDelta::Pixels(_)),
                    &mut this.pending_scroll_lines,
                    &mut this.suppress_precise_scroll_until,
                    &mut this.suppress_precise_scroll_until_ended,
                    Instant::now(),
                ) {
                    return;
                }

                let (mode, display_offset) = this.mode_and_display_offset();
                let lines = scroll_delta_to_lines(event.delta, this.cell_size.height)
                    * effective_scroll_multiplier(this.settings.scroll_multiplier);
                let delta = consume_scroll_lines(&mut this.pending_scroll_lines, lines);
                if delta == 0 {
                    return;
                }

                if mouse_mode_enabled_for_scroll(mode, event.modifiers.shift) {
                    let point = grid_point(event.position, this.terminal_bounds(), display_offset);
                    if let Some(reports) = scroll_report(point, delta, event, mode) {
                        for bytes in reports {
                            this.terminal.write(bytes);
                        }
                    }
                } else if alternate_scroll_enabled(
                    mode,
                    this.settings.alternate_scroll,
                    event.modifiers.shift,
                ) {
                    this.terminal.write(alt_scroll(delta));
                } else {
                    this.terminal
                        .term
                        .lock()
                        .scroll_display(Scroll::Delta(delta));
                }

                // Ensure local scroll actions repaint immediately.
                cx.notify();
            }))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _window, cx| {
                // Cmd+V: paste from clipboard
                if event.keystroke.modifiers.platform && event.keystroke.key == "v" {
                    if let Some(item) = cx.read_from_clipboard() {
                        if let Some(text) = item.text() {
                            this.begin_terminal_input();
                            this.terminal.write_str(&text);
                        }
                    }
                    return;
                }

                let mode = {
                    let term = this.terminal.term.lock();
                    *term.mode()
                };

                // Try escape sequence mapping first
                if let Some(esc) = zed_terminal::mappings::keys::to_esc_str(
                    &event.keystroke,
                    &mode,
                    this.settings.option_as_meta,
                ) {
                    this.begin_terminal_input();
                    this.terminal.write(esc.as_bytes().to_vec());
                    return;
                }

                // Regular text input
                if let Some(text) = text_to_insert(&event.keystroke) {
                    this.begin_terminal_input();
                    this.terminal.write(text.as_bytes().to_vec());
                }
            }))
            .child(
                canvas(
                    move |_bounds, _window, _cx| snapshot,
                    move |bounds, snapshot, window, cx| {
                        let content_bounds = Bounds {
                            origin: bounds.origin,
                            size: size(
                                cell_size.width * snapshot.num_cols as f32,
                                cell_size.height * snapshot.num_lines as f32,
                            ),
                        };
                        let scrollbar = scrollbar_layout(
                            content_bounds,
                            snapshot.num_lines,
                            snapshot.history_size,
                            snapshot.display_offset,
                        );

                        // Paint phase
                        window.with_content_mask(Some(ContentMask { bounds }), |window| {
                            // Paint cell backgrounds
                            for (row_idx, row) in snapshot.rows.iter().enumerate() {
                                for (col_idx, cell) in row.iter().enumerate() {
                                    let bg_color = resolve_color(&cell.bg, &snapshot.colors, false);
                                    if bg_color != gpui::black() {
                                        let cell_bounds = Bounds {
                                            origin: point(
                                                bounds.origin.x + cell_size.width * col_idx as f32,
                                                bounds.origin.y + cell_size.height * row_idx as f32,
                                            ),
                                            size: size(cell_size.width, cell_size.height),
                                        };
                                        window.paint_quad(fill(cell_bounds, bg_color));
                                    }
                                }
                            }

                            // Paint text runs anchored to terminal columns.
                            let text_system = window.text_system().clone();
                            for (row_idx, row) in snapshot.rows.iter().enumerate() {
                                for positioned_run in build_positioned_text_runs(row) {
                                    let fg_color =
                                        resolve_color(&positioned_run.fg, &snapshot.colors, true);
                                    let bg_color =
                                        resolve_color(&positioned_run.bg, &snapshot.colors, false);
                                    let bg_option = if bg_color != gpui::black() {
                                        Some(bg_color)
                                    } else {
                                        None
                                    };
                                    let shaped = text_system.shape_line(
                                        SharedString::from(positioned_run.text.clone()),
                                        font_size,
                                        &[TextRun {
                                            len: positioned_run.text.len(),
                                            font: Font {
                                                weight: if positioned_run.bold {
                                                    FontWeight::BOLD
                                                } else {
                                                    font.weight
                                                },
                                                ..font.clone()
                                            },
                                            color: fg_color,
                                            background_color: bg_option,
                                            underline: None,
                                            strikethrough: None,
                                        }],
                                        Some(cell_size.width),
                                    );
                                    let origin = point(
                                        bounds.origin.x
                                            + cell_size.width * positioned_run.start_col as f32,
                                        bounds.origin.y + cell_size.height * row_idx as f32,
                                    );
                                    let _ = shaped.paint(origin, cell_size.height, window, cx);
                                }
                            }

                            // Paint cursor on top of text.
                            if snapshot.show_cursor && snapshot.cursor_col < snapshot.num_cols {
                                if let Some(cursor_row) = snapshot.cursor_row {
                                    let cursor_bounds = Bounds {
                                        origin: point(
                                            bounds.origin.x
                                                + cell_size.width * snapshot.cursor_col as f32,
                                            bounds.origin.y + cell_size.height * cursor_row as f32,
                                        ),
                                        size: size(cell_size.width, cell_size.height),
                                    };
                                    window.paint_quad(fill(cursor_bounds, hsla(0., 0., 0.8, 0.7)));
                                }
                            }

                            if let Some(layout) = &scrollbar {
                                window.paint_quad(fill(layout.track, hsla(0.0, 0.0, 0.25, 0.35)));
                                window.paint_quad(fill(layout.thumb, hsla(0.0, 0.0, 0.65, 0.8)));
                            }
                        });
                    },
                )
                .size_full(),
            )
    }
}

/// Snapshot of the terminal color palette.
#[derive(Clone)]
struct ColorsSnapshot {
    palette: [AlacRgb; 256],
    foreground: AlacRgb,
    background: AlacRgb,
}

impl ColorsSnapshot {
    fn from_colors(colors: &AlacColors) -> Self {
        let mut palette = [AlacRgb { r: 0, g: 0, b: 0 }; 256];

        let ansi_colors: [(u8, u8, u8); 16] = [
            (0x00, 0x00, 0x00),
            (0xCC, 0x00, 0x00),
            (0x4E, 0x9A, 0x06),
            (0xC4, 0xA0, 0x00),
            (0x34, 0x65, 0xA4),
            (0x75, 0x50, 0x7B),
            (0x06, 0x98, 0x9A),
            (0xD3, 0xD7, 0xCF),
            (0x55, 0x57, 0x53),
            (0xEF, 0x29, 0x29),
            (0x8A, 0xE2, 0x34),
            (0xFC, 0xE9, 0x4F),
            (0x72, 0x9F, 0xCF),
            (0xAD, 0x7F, 0xA8),
            (0x34, 0xE2, 0xE2),
            (0xEE, 0xEE, 0xEC),
        ];

        for (i, &(r, g, b)) in ansi_colors.iter().enumerate() {
            palette[i] = colors[i].unwrap_or(AlacRgb { r, g, b });
        }

        // 216 color cube (16-231)
        for i in 16..232 {
            let idx = i - 16;
            let r = if idx / 36 > 0 {
                (idx / 36) * 40 + 55
            } else {
                0
            };
            let g = if (idx % 36) / 6 > 0 {
                ((idx % 36) / 6) * 40 + 55
            } else {
                0
            };
            let b = if idx % 6 > 0 { (idx % 6) * 40 + 55 } else { 0 };
            palette[i] = colors[i].unwrap_or(AlacRgb {
                r: r as u8,
                g: g as u8,
                b: b as u8,
            });
        }

        // Grayscale ramp (232-255)
        for i in 232..256 {
            let v = ((i - 232) * 10 + 8) as u8;
            palette[i] = colors[i].unwrap_or(AlacRgb { r: v, g: v, b: v });
        }

        let foreground = colors[NamedColor::Foreground as usize].unwrap_or(AlacRgb {
            r: 0xD3,
            g: 0xD7,
            b: 0xCF,
        });
        let background = colors[NamedColor::Background as usize].unwrap_or(AlacRgb {
            r: 0x00,
            g: 0x00,
            b: 0x00,
        });

        ColorsSnapshot {
            palette,
            foreground,
            background,
        }
    }
}

fn resolve_color(color: &AlacColor, colors: &ColorsSnapshot, is_fg: bool) -> Hsla {
    let rgb = match color {
        AlacColor::Named(name) => match name {
            NamedColor::Foreground => colors.foreground,
            NamedColor::Background => colors.background,
            NamedColor::Cursor => colors.foreground,
            _ => {
                let idx = *name as usize;
                if idx < 256 {
                    colors.palette[idx]
                } else if is_fg {
                    colors.foreground
                } else {
                    colors.background
                }
            }
        },
        AlacColor::Spec(rgb) => *rgb,
        AlacColor::Indexed(idx) => {
            if (*idx as usize) < 256 {
                colors.palette[*idx as usize]
            } else if is_fg {
                colors.foreground
            } else {
                colors.background
            }
        }
    };

    alac_rgb_to_hsla(rgb)
}

fn alac_rgb_to_hsla(rgb: AlacRgb) -> Hsla {
    let r = rgb.r as f32 / 255.0;
    let g = rgb.g as f32 / 255.0;
    let b = rgb.b as f32 / 255.0;
    Hsla::from(Rgba { r, g, b, a: 1.0 })
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PositionedTextRun {
    start_col: usize,
    text: String,
    fg: AlacColor,
    bg: AlacColor,
    bold: bool,
}

fn build_positioned_text_runs(row: &[CellSnapshot]) -> Vec<PositionedTextRun> {
    #[derive(Clone)]
    struct PendingRun {
        start_col: usize,
        end_col: usize,
        text: String,
        fg: AlacColor,
        bg: AlacColor,
        bold: bool,
    }

    let mut runs: Vec<PositionedTextRun> = Vec::new();
    let mut current: Option<PendingRun> = None;

    for (col_idx, cell) in row.iter().enumerate() {
        if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
            if let Some(run) = current.take() {
                runs.push(PositionedTextRun {
                    start_col: run.start_col,
                    text: run.text,
                    fg: run.fg,
                    bg: run.bg,
                    bold: run.bold,
                });
            }
            continue;
        }

        let mut fg = cell.fg;
        let mut bg = cell.bg;
        if cell.flags.contains(Flags::INVERSE) {
            std::mem::swap(&mut fg, &mut bg);
        }
        let bold = cell.flags.contains(Flags::BOLD);
        let display_char = if cell.c == '\0' { ' ' } else { cell.c };

        // Preserve exact grid positioning by skipping blank cells and starting
        // new runs at the corresponding column index.
        if display_char == ' ' {
            if let Some(run) = current.take() {
                runs.push(PositionedTextRun {
                    start_col: run.start_col,
                    text: run.text,
                    fg: run.fg,
                    bg: run.bg,
                    bold: run.bold,
                });
            }
            continue;
        }

        match &mut current {
            Some(run)
                if run.fg == fg && run.bg == bg && run.bold == bold && run.end_col == col_idx =>
            {
                run.text.push(display_char);
                run.end_col += 1;
            }
            Some(_) => {
                let old = current.take().expect("run is present");
                runs.push(PositionedTextRun {
                    start_col: old.start_col,
                    text: old.text,
                    fg: old.fg,
                    bg: old.bg,
                    bold: old.bold,
                });
                current = Some(PendingRun {
                    start_col: col_idx,
                    end_col: col_idx + 1,
                    text: display_char.to_string(),
                    fg,
                    bg,
                    bold,
                });
            }
            None => {
                current = Some(PendingRun {
                    start_col: col_idx,
                    end_col: col_idx + 1,
                    text: display_char.to_string(),
                    fg,
                    bg,
                    bold,
                });
            }
        }
    }

    if let Some(run) = current {
        runs.push(PositionedTextRun {
            start_col: run.start_col,
            text: run.text,
            fg: run.fg,
            bg: run.bg,
            bold: run.bold,
        });
    }

    runs
}

#[cfg(test)]
mod tests {
    use super::{
        alternate_scroll_enabled, build_positioned_text_runs, consume_scroll_lines,
        display_offset_from_thumb_top, effective_scroll_multiplier, mouse_mode_enabled_for_scroll,
        prepare_for_terminal_input, resolve_working_directory_with_fallback, scroll_delta_to_lines,
        scrollbar_thumb_metrics, selection_copy_plan, should_ignore_scroll_event,
        strip_line_column_suffix, text_to_insert, viewport_row_for_line, CellSnapshot,
        INPUT_SCROLL_SUPPRESSION_WINDOW,
    };
    use alacritty_terminal::term::cell::Flags;
    use alacritty_terminal::vte::ansi::{Color as AlacColor, NamedColor};
    use gpui::{px, Keystroke, Modifiers, Point, ScrollDelta, TouchPhase};
    use std::path::PathBuf;
    use std::time::Instant;
    use zed_terminal::terminal_settings::WorkingDirectory;
    use zed_terminal::AlternateScroll;
    use zed_terminal::TermMode;

    fn cell(c: char, flags: Flags) -> CellSnapshot {
        CellSnapshot {
            c,
            fg: AlacColor::Named(NamedColor::Foreground),
            bg: AlacColor::Named(NamedColor::Background),
            flags,
        }
    }

    #[test]
    fn strips_path_line_column_suffix() {
        assert_eq!(
            strip_line_column_suffix("/tmp/file.rs:12:34"),
            "/tmp/file.rs"
        );
    }

    #[test]
    fn strips_path_line_suffix() {
        assert_eq!(strip_line_column_suffix("/tmp/file.rs:12"), "/tmp/file.rs");
    }

    #[test]
    fn leaves_plain_paths_unchanged() {
        assert_eq!(strip_line_column_suffix("/tmp/file.rs"), "/tmp/file.rs");
    }

    #[test]
    fn positioned_runs_skip_wide_spacers_without_losing_columns() {
        let row = vec![
            cell('A', Flags::empty()),
            cell(' ', Flags::WIDE_CHAR_SPACER),
            cell('B', Flags::empty()),
        ];

        let runs = build_positioned_text_runs(&row);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].start_col, 0);
        assert_eq!(runs[0].text, "A");
        assert_eq!(runs[1].start_col, 2);
        assert_eq!(runs[1].text, "B");
    }

    #[test]
    fn positioned_runs_preserve_gaps_for_blank_cells() {
        let row = vec![
            cell('A', Flags::empty()),
            cell(' ', Flags::empty()),
            cell('B', Flags::empty()),
        ];

        let runs = build_positioned_text_runs(&row);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].start_col, 0);
        assert_eq!(runs[0].text, "A");
        assert_eq!(runs[1].start_col, 2);
        assert_eq!(runs[1].text, "B");
    }

    #[test]
    fn scroll_lines_accumulate_small_positive_deltas() {
        let mut pending: f32 = 0.0;
        assert_eq!(consume_scroll_lines(&mut pending, 0.2), 0);
        assert_eq!(consume_scroll_lines(&mut pending, 0.2), 0);
        assert_eq!(consume_scroll_lines(&mut pending, 0.2), 0);
        assert_eq!(consume_scroll_lines(&mut pending, 0.2), 0);
        assert_eq!(consume_scroll_lines(&mut pending, 0.2), 1);
        assert!(pending.abs() < f32::EPSILON);
    }

    #[test]
    fn scroll_lines_accumulate_small_negative_deltas() {
        let mut pending: f32 = 0.0;
        assert_eq!(consume_scroll_lines(&mut pending, -0.25), 0);
        assert_eq!(consume_scroll_lines(&mut pending, -0.25), 0);
        assert_eq!(consume_scroll_lines(&mut pending, -0.25), 0);
        assert_eq!(consume_scroll_lines(&mut pending, -0.25), -1);
        assert!(pending.abs() < f32::EPSILON);
    }

    #[test]
    fn scroll_lines_handle_direction_reversal_without_jumps() {
        let mut pending: f32 = 0.0;
        assert_eq!(consume_scroll_lines(&mut pending, 0.7), 0);
        assert_eq!(consume_scroll_lines(&mut pending, -0.2), 0);
        assert_eq!(consume_scroll_lines(&mut pending, -0.6), 0);
        assert!(pending < 0.0);
    }

    #[test]
    fn effective_scroll_multiplier_has_positive_finite_floor() {
        assert_eq!(effective_scroll_multiplier(3.0), 3.0);
        assert_eq!(effective_scroll_multiplier(0.0), 0.01);
        assert_eq!(effective_scroll_multiplier(-2.0), 0.01);
        assert_eq!(effective_scroll_multiplier(f32::INFINITY), 1.0);
        assert_eq!(effective_scroll_multiplier(f32::NAN), 1.0);
    }

    #[test]
    fn shift_disables_mouse_mode_scroll_capture() {
        let mode = TermMode::MOUSE_MODE | TermMode::SGR_MOUSE;
        assert!(mouse_mode_enabled_for_scroll(mode, false));
        assert!(!mouse_mode_enabled_for_scroll(mode, true));
    }

    #[test]
    fn alternate_scroll_requires_mode_bit_and_setting() {
        let mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL;
        assert!(alternate_scroll_enabled(mode, AlternateScroll::On, false));
        assert!(!alternate_scroll_enabled(
            TermMode::ALT_SCREEN,
            AlternateScroll::On,
            false
        ));
        assert!(!alternate_scroll_enabled(mode, AlternateScroll::Off, false));
        assert!(!alternate_scroll_enabled(mode, AlternateScroll::On, true));
    }

    #[test]
    fn working_directory_prefers_explicit_always_path() {
        let configured = PathBuf::from("/tmp/zed-terminal");
        let resolved = resolve_working_directory_with_fallback(
            &WorkingDirectory::Always {
                directory: configured.clone(),
            },
            Some(PathBuf::from("/cwd")),
            Some(PathBuf::from("/home")),
        );

        assert_eq!(resolved, Some(configured));
    }

    #[test]
    fn working_directory_uses_home_for_always_home() {
        let resolved = resolve_working_directory_with_fallback(
            &WorkingDirectory::AlwaysHome,
            Some(PathBuf::from("/cwd")),
            Some(PathBuf::from("/home/test")),
        );

        assert_eq!(resolved, Some(PathBuf::from("/home/test")));
    }

    #[test]
    fn working_directory_falls_back_to_cwd_for_project_variants() {
        let cwd = Some(PathBuf::from("/workspace"));
        let home = Some(PathBuf::from("/home/test"));

        assert_eq!(
            resolve_working_directory_with_fallback(
                &WorkingDirectory::CurrentFileDirectory,
                cwd.clone(),
                home.clone(),
            ),
            cwd
        );
        assert_eq!(
            resolve_working_directory_with_fallback(
                &WorkingDirectory::CurrentProjectDirectory,
                cwd.clone(),
                home.clone(),
            ),
            cwd
        );
        assert_eq!(
            resolve_working_directory_with_fallback(
                &WorkingDirectory::FirstProjectDirectory,
                cwd.clone(),
                home.clone(),
            ),
            cwd
        );
    }

    #[test]
    fn selection_copy_plan_respects_copy_on_select_flag() {
        let (text, clear) = selection_copy_plan(false, false, Some("hello".to_string()));
        assert_eq!(text, None);
        assert!(!clear);
    }

    #[test]
    fn selection_copy_plan_copies_and_keeps_selection_when_configured() {
        let (text, clear) = selection_copy_plan(true, true, Some("hello".to_string()));
        assert_eq!(text.as_deref(), Some("hello"));
        assert!(!clear);
    }

    #[test]
    fn selection_copy_plan_copies_and_clears_selection_when_configured() {
        let (text, clear) = selection_copy_plan(true, false, Some("hello".to_string()));
        assert_eq!(text.as_deref(), Some("hello"));
        assert!(clear);
    }

    #[test]
    fn scrollbar_thumb_metrics_reflect_display_offset() {
        let (top, height, max_offset) =
            scrollbar_thumb_metrics(px(100.0), 20, 80, 40).expect("should be scrollable");
        assert_eq!(max_offset, 80);
        assert_eq!(height, px(24.0));
        assert!((f32::from(top) - 38.0).abs() < 0.001);
    }

    #[test]
    fn scrollbar_thumb_places_bottom_offset_at_track_bottom() {
        let (top, height, _max_offset) =
            scrollbar_thumb_metrics(px(100.0), 20, 80, 0).expect("should be scrollable");
        assert_eq!(height, px(24.0));
        assert!((f32::from(top) - 76.0).abs() < 0.001);
    }

    #[test]
    fn display_offset_maps_track_top_to_history_top() {
        let offset = display_offset_from_thumb_top(px(0.0), px(100.0), px(24.0), 80);
        assert_eq!(offset, 80);
    }

    #[test]
    fn display_offset_round_trips_through_thumb_position() {
        let track_height = px(120.0);
        let thumb_height = px(30.0);
        let max_offset = 90;
        let expected = 54usize;
        let thumb_top = px(36.0);

        let offset =
            display_offset_from_thumb_top(thumb_top, track_height, thumb_height, max_offset);
        assert_eq!(offset, expected);
    }

    #[test]
    fn scroll_delta_lines_preserves_positive_line_direction() {
        let lines = scroll_delta_to_lines(ScrollDelta::Lines(Point { x: 0.0, y: 2.5 }), px(20.0));
        assert_eq!(lines, 2.5);
    }

    #[test]
    fn scroll_delta_lines_converts_pixels_using_line_height() {
        let lines = scroll_delta_to_lines(
            ScrollDelta::Pixels(Point {
                x: px(0.0),
                y: px(30.0),
            }),
            px(10.0),
        );
        assert_eq!(lines, 3.0);
    }

    #[test]
    fn viewport_row_maps_scrollback_lines_into_visible_rows() {
        assert_eq!(viewport_row_for_line(-5, 5, 20), Some(0));
        assert_eq!(viewport_row_for_line(14, 5, 20), Some(19));
    }

    #[test]
    fn viewport_row_hides_lines_outside_visible_range() {
        assert_eq!(viewport_row_for_line(-6, 5, 20), None);
        assert_eq!(viewport_row_for_line(15, 5, 20), None);
    }

    #[test]
    fn terminal_input_prepares_precise_suppression_when_scrolled() {
        let mut pending = 0.75;
        let mut suppress_precise_until = None;
        let mut suppress_precise_until_ended = false;
        let base = Instant::now();

        prepare_for_terminal_input(
            true,
            &mut pending,
            &mut suppress_precise_until,
            &mut suppress_precise_until_ended,
            base,
        );

        assert_eq!(pending, 0.0);
        assert_eq!(
            suppress_precise_until,
            Some(base + INPUT_SCROLL_SUPPRESSION_WINDOW)
        );
        assert!(suppress_precise_until_ended);
    }

    #[test]
    fn terminal_input_does_not_suppress_when_already_at_bottom() {
        let mut pending = 0.75;
        let mut suppress_precise_until = None;
        let mut suppress_precise_until_ended = false;

        prepare_for_terminal_input(
            false,
            &mut pending,
            &mut suppress_precise_until,
            &mut suppress_precise_until_ended,
            Instant::now(),
        );

        assert_eq!(pending, 0.0);
        assert!(suppress_precise_until.is_none());
        assert!(!suppress_precise_until_ended);
    }

    #[test]
    fn consecutive_input_keeps_existing_precise_suppression_until_scroll_end() {
        let base = Instant::now();
        let mut pending = 0.0;
        let mut suppress_precise_until = None;
        let mut suppress_precise_until_ended = false;

        // First key press while scrolled should activate suppression.
        prepare_for_terminal_input(
            true,
            &mut pending,
            &mut suppress_precise_until,
            &mut suppress_precise_until_ended,
            base,
        );
        let first_until = suppress_precise_until.expect("first key should set suppression window");
        assert!(suppress_precise_until_ended);

        // Subsequent key press while already at bottom should not drop existing suppression.
        prepare_for_terminal_input(
            false,
            &mut pending,
            &mut suppress_precise_until,
            &mut suppress_precise_until_ended,
            base + std::time::Duration::from_millis(20),
        );
        assert_eq!(suppress_precise_until, Some(first_until));
        assert!(suppress_precise_until_ended);

        // Residual precise movement should still be ignored to prevent bounce/jitter.
        assert!(should_ignore_scroll_event(
            TouchPhase::Moved,
            true,
            &mut pending,
            &mut suppress_precise_until,
            &mut suppress_precise_until_ended,
            base + INPUT_SCROLL_SUPPRESSION_WINDOW + std::time::Duration::from_millis(60),
        ));
    }

    #[test]
    fn typing_word_after_scrollback_keeps_suppression_until_precise_scroll_ends() {
        let base = Instant::now();
        let mut pending = 0.0;
        let mut suppress_precise_until = None;
        let mut suppress_precise_until_ended = false;

        // User is scrolled up; first typed key jumps to bottom and activates suppression.
        prepare_for_terminal_input(
            true,
            &mut pending,
            &mut suppress_precise_until,
            &mut suppress_precise_until_ended,
            base,
        );
        let suppression_until =
            suppress_precise_until.expect("initial key while scrolled should set suppression");
        assert!(suppress_precise_until_ended);

        // User keeps typing multiple keys rapidly (e.g. a whole word). This must not
        // clear suppression while residual inertial precise scroll is still in-flight.
        for ms in [10_u64, 20, 35, 50] {
            prepare_for_terminal_input(
                false,
                &mut pending,
                &mut suppress_precise_until,
                &mut suppress_precise_until_ended,
                base + std::time::Duration::from_millis(ms),
            );
            assert_eq!(suppress_precise_until, Some(suppression_until));
            assert!(suppress_precise_until_ended);
        }

        assert!(should_ignore_scroll_event(
            TouchPhase::Started,
            true,
            &mut pending,
            &mut suppress_precise_until,
            &mut suppress_precise_until_ended,
            base + std::time::Duration::from_millis(55),
        ));
        assert!(suppress_precise_until_ended);

        // Even past the initial timed window, stale precise movement is ignored until Ended.
        assert!(should_ignore_scroll_event(
            TouchPhase::Moved,
            true,
            &mut pending,
            &mut suppress_precise_until,
            &mut suppress_precise_until_ended,
            base + INPUT_SCROLL_SUPPRESSION_WINDOW + std::time::Duration::from_millis(70),
        ));

        assert!(should_ignore_scroll_event(
            TouchPhase::Ended,
            true,
            &mut pending,
            &mut suppress_precise_until,
            &mut suppress_precise_until_ended,
            base + INPUT_SCROLL_SUPPRESSION_WINDOW + std::time::Duration::from_millis(90),
        ));
        assert!(suppress_precise_until.is_none());
        assert!(!suppress_precise_until_ended);

        // Fresh precise movement after Ended should no longer be blocked.
        assert!(!should_ignore_scroll_event(
            TouchPhase::Moved,
            true,
            &mut pending,
            &mut suppress_precise_until,
            &mut suppress_precise_until_ended,
            base + INPUT_SCROLL_SUPPRESSION_WINDOW + std::time::Duration::from_millis(110),
        ));
    }

    #[test]
    fn scroll_started_event_keeps_precise_suppression_and_is_ignored() {
        let mut pending = 0.3;
        let suppression = Instant::now() + std::time::Duration::from_millis(250);
        let mut suppress_precise_until = Some(suppression);
        let mut suppress_precise_until_ended = true;

        assert!(should_ignore_scroll_event(
            TouchPhase::Started,
            true,
            &mut pending,
            &mut suppress_precise_until,
            &mut suppress_precise_until_ended,
            Instant::now(),
        ));
        assert_eq!(pending, 0.0);
        assert_eq!(suppress_precise_until, Some(suppression));
        assert!(suppress_precise_until_ended);
    }

    #[test]
    fn scroll_started_event_clears_suppression_for_line_scroll() {
        let mut pending = 0.3;
        let mut suppress_precise_until =
            Some(Instant::now() + std::time::Duration::from_millis(250));
        let mut suppress_precise_until_ended = true;

        assert!(should_ignore_scroll_event(
            TouchPhase::Started,
            false,
            &mut pending,
            &mut suppress_precise_until,
            &mut suppress_precise_until_ended,
            Instant::now(),
        ));
        assert_eq!(pending, 0.0);
        assert!(suppress_precise_until.is_none());
        assert!(!suppress_precise_until_ended);
    }

    #[test]
    fn scroll_ended_event_resets_pending_and_is_ignored() {
        let mut pending = 0.3;
        let mut suppress_precise_until =
            Some(Instant::now() + std::time::Duration::from_millis(200));
        let mut suppress_precise_until_ended = true;

        assert!(should_ignore_scroll_event(
            TouchPhase::Ended,
            true,
            &mut pending,
            &mut suppress_precise_until,
            &mut suppress_precise_until_ended,
            Instant::now(),
        ));
        assert_eq!(pending, 0.0);
        assert!(suppress_precise_until.is_none());
        assert!(!suppress_precise_until_ended);
    }

    #[test]
    fn precise_scroll_is_ignored_within_input_suppression_window() {
        let base = Instant::now();
        let mut pending = 0.3;
        let mut suppress_precise_until = Some(base + std::time::Duration::from_millis(300));
        let mut suppress_precise_until_ended = false;

        assert!(should_ignore_scroll_event(
            TouchPhase::Moved,
            true,
            &mut pending,
            &mut suppress_precise_until,
            &mut suppress_precise_until_ended,
            base + std::time::Duration::from_millis(100),
        ));
        assert_eq!(pending, 0.0);
        assert!(suppress_precise_until.is_some());
    }

    #[test]
    fn precise_scroll_started_event_keeps_input_suppression_for_followup_moved_event() {
        let base = Instant::now();
        let mut pending = 0.0;
        let mut suppress_precise_until = None;
        let mut suppress_precise_until_ended = false;

        prepare_for_terminal_input(
            true,
            &mut pending,
            &mut suppress_precise_until,
            &mut suppress_precise_until_ended,
            base,
        );
        assert!(suppress_precise_until.is_some());
        assert!(suppress_precise_until_ended);

        assert!(should_ignore_scroll_event(
            TouchPhase::Started,
            true,
            &mut pending,
            &mut suppress_precise_until,
            &mut suppress_precise_until_ended,
            base + std::time::Duration::from_millis(5),
        ));
        assert!(suppress_precise_until.is_some());
        assert!(suppress_precise_until_ended);

        assert!(should_ignore_scroll_event(
            TouchPhase::Moved,
            true,
            &mut pending,
            &mut suppress_precise_until,
            &mut suppress_precise_until_ended,
            base + std::time::Duration::from_millis(15),
        ));
    }

    #[test]
    fn precise_scroll_remains_ignored_after_window_until_scroll_sequence_ends() {
        let base = Instant::now();
        let mut pending = 0.0;
        let mut suppress_precise_until = None;
        let mut suppress_precise_until_ended = false;

        prepare_for_terminal_input(
            true,
            &mut pending,
            &mut suppress_precise_until,
            &mut suppress_precise_until_ended,
            base,
        );

        assert!(should_ignore_scroll_event(
            TouchPhase::Started,
            true,
            &mut pending,
            &mut suppress_precise_until,
            &mut suppress_precise_until_ended,
            base + std::time::Duration::from_millis(5),
        ));

        // Even after the time window, stale inertial precise movement should stay suppressed
        // until this scroll sequence ends.
        assert!(should_ignore_scroll_event(
            TouchPhase::Moved,
            true,
            &mut pending,
            &mut suppress_precise_until,
            &mut suppress_precise_until_ended,
            base + INPUT_SCROLL_SUPPRESSION_WINDOW + std::time::Duration::from_millis(80),
        ));

        assert!(should_ignore_scroll_event(
            TouchPhase::Ended,
            true,
            &mut pending,
            &mut suppress_precise_until,
            &mut suppress_precise_until_ended,
            base + INPUT_SCROLL_SUPPRESSION_WINDOW + std::time::Duration::from_millis(120),
        ));
        assert!(suppress_precise_until.is_none());
        assert!(!suppress_precise_until_ended);

        // A subsequent precise move should no longer be blocked once the stale sequence ended.
        assert!(!should_ignore_scroll_event(
            TouchPhase::Moved,
            true,
            &mut pending,
            &mut suppress_precise_until,
            &mut suppress_precise_until_ended,
            base + INPUT_SCROLL_SUPPRESSION_WINDOW + std::time::Duration::from_millis(140),
        ));
    }

    #[test]
    fn precise_scroll_is_allowed_after_suppression_expires() {
        let base = Instant::now();
        let mut pending = 0.3;
        let mut suppress_precise_until = Some(base + std::time::Duration::from_millis(100));
        let mut suppress_precise_until_ended = false;

        assert!(!should_ignore_scroll_event(
            TouchPhase::Moved,
            true,
            &mut pending,
            &mut suppress_precise_until,
            &mut suppress_precise_until_ended,
            base + std::time::Duration::from_millis(200),
        ));
        assert!(suppress_precise_until.is_none());
    }

    #[test]
    fn line_scroll_clears_precise_suppression() {
        let mut pending = 0.3;
        let mut suppress_precise_until = Some(Instant::now() + std::time::Duration::from_secs(1));
        let mut suppress_precise_until_ended = true;

        assert!(!should_ignore_scroll_event(
            TouchPhase::Moved,
            false,
            &mut pending,
            &mut suppress_precise_until,
            &mut suppress_precise_until_ended,
            Instant::now(),
        ));
        assert!(suppress_precise_until.is_none());
        assert!(!suppress_precise_until_ended);
    }

    #[test]
    fn text_to_insert_ignores_alt_without_character_data() {
        let ks = Keystroke {
            modifiers: Modifiers {
                control: false,
                alt: true,
                shift: false,
                platform: false,
                function: false,
            },
            key: "D".to_string(),
            key_char: None,
        };
        assert_eq!(text_to_insert(&ks), None);
    }
}
