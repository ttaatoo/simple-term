//! Terminal view - renders the terminal using GPUI

use alacritty_terminal::event::WindowSize;
use alacritty_terminal::grid::{Indexed, Scroll};
use alacritty_terminal::index::Side;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::color::Colors as AlacColors;
use alacritty_terminal::vte::ansi::{Color as AlacColor, CursorShape, NamedColor, Rgb as AlacRgb};
use global_hotkey::hotkey::HotKey as GlobalHotKey;

use gpui::prelude::FluentBuilder;
use gpui::{
    canvas, div, fill, hsla, point, px, rgb, size, App, AppContext, AsyncWindowContext, Bounds,
    ClipboardItem, ContentMask, Context, FocusHandle, Focusable, Font, FontFallbacks, FontFeatures,
    FontStyle, FontWeight, Hsla, InteractiveElement, IntoElement, KeyDownEvent, Keystroke,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Render, Rgba,
    ScrollDelta, ScrollHandle, ScrollWheelEvent, SharedString, Size, StatefulInteractiveElement,
    Styled, Subscription, TextRun, WeakEntity, Window, WindowControlArea,
};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};

use simple_term::alacritty_terminal::index::Boundary;
use simple_term::alacritty_terminal::term::search::RegexSearch;
use simple_term::mappings::mouse::{
    alt_scroll, grid_point, grid_point_and_side, mouse_button_report, mouse_moved_report,
    scroll_report,
};
use simple_term::terminal::{Terminal, TerminalEvent};
use simple_term::terminal_hyperlinks::{find_from_grid_point, RegexSearches};
use simple_term::terminal_settings::{
    Blinking, CursorShape as SettingsCursorShape, LineHeight, TerminalSettings, TerminalTheme,
};
use simple_term::{
    AlacDirection, AlacPoint, Column, Dimensions, Line, PathStyle, Selection, SelectionType,
    TermMode, TerminalBounds,
};

mod utils;

use utils::{
    alternate_scroll_enabled, common_shortcut_action, consume_scroll_lines,
    display_offset_from_pointer, effective_scroll_multiplier, file_path_to_file_url,
    mouse_mode_enabled_for_scroll, point_in_bounds, prepare_for_terminal_input,
    resolve_working_directory, scroll_delta_to_lines, scrollbar_layout, selection_copy_plan,
    selection_type_for_click_count, should_ignore_scroll_event, strip_line_column_suffix,
    text_to_insert, viewport_row_for_line, CommonShortcutAction, ScrollbarLayout,
};

const TAB_BAR_HEIGHT_PX: f32 = 40.0;
const TAB_ADD_BUTTON_WIDTH_PX: f32 = 30.0;
const TAB_DROPDOWN_BUTTON_WIDTH_PX: f32 = 30.0;
const TAB_BAR_LEFT_DRAG_WIDTH_PX: f32 = 122.0;
const TAB_ITEM_WIDTH_PX: f32 = 152.0;
const TAB_ITEM_HEIGHT_PX: f32 = 28.0;
const TAB_ITEM_INDICATOR_HEIGHT_PX: f32 = 3.0;
const TAB_ITEM_INDICATOR_BOTTOM_GAP_PX: f32 = 2.0;
const TAB_CLOSE_BUTTON_SIZE_PX: f32 = 20.0;
const PIN_INDICATOR_BUTTON_WIDTH_PX: f32 = 30.0;
const SETTINGS_BUTTON_WIDTH_PX: f32 = 30.0;
const FIND_PANEL_MAX_WIDTH_PX: f32 = 760.0;
const FIND_PANEL_MIN_WIDTH_PX: f32 = 240.0;
const FIND_PANEL_RESERVED_SPACE_PX: f32 = 320.0;
const FIND_PANEL_VIEWPORT_MARGIN_PX: f32 = 24.0;
const SETTINGS_DRAWER_WIDTH_PX: f32 = 360.0;
const SETTINGS_DRAWER_MIN_WIDTH_PX: f32 = 280.0;
const SETTINGS_DRAWER_VIEWPORT_MARGIN_PX: f32 = 32.0;
const SETTINGS_DRAWER_HEADER_HEIGHT_PX: f32 = 44.0;
const SETTINGS_DRAWER_SCROLLBAR_WIDTH_PX: f32 = 8.0;
const SETTINGS_DRAWER_SCROLLBAR_TRACK_WIDTH_PX: f32 = 3.0;
const SETTINGS_DRAWER_SCROLLBAR_TRACK_INSET_PX: f32 = 2.0;
const SETTINGS_DRAWER_SCROLLBAR_MIN_THUMB_HEIGHT_PX: f32 = 24.0;
const SETTINGS_DRAWER_SCROLL_CONTENT_PADDING_RIGHT_PX: f32 = 14.0;
const SETTINGS_OVERLAY_BACKDROP_ALPHA: f32 = 0.28;
const SETTINGS_NUMERIC_BUTTON_WIDTH_PX: f32 = 24.0;
const SETTINGS_CONTROL_HEIGHT_PX: f32 = 24.0;
const SETTINGS_MIN_FONT_SIZE: f32 = 6.0;
const SETTINGS_MAX_FONT_SIZE: f32 = 72.0;
const SETTINGS_FONT_SIZE_STEP: f32 = 1.0;
const SETTINGS_MIN_LINE_HEIGHT_RATIO: f32 = 0.5;
const SETTINGS_MAX_LINE_HEIGHT_RATIO: f32 = 3.0;
const SETTINGS_LINE_HEIGHT_STEP: f32 = 0.1;
const SETTINGS_MIN_SCROLL_MULTIPLIER: f32 = 0.01;
const SETTINGS_MAX_SCROLL_MULTIPLIER: f32 = 10.0;
const SETTINGS_SCROLL_MULTIPLIER_STEP: f32 = 0.25;
const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(530);
const CURSOR_BLINK_SUPPRESSION_AFTER_INPUT: Duration = Duration::from_millis(800);
const SELECTION_TINT_ALPHA: f32 = 0.30;
const THEME_PRESETS: [TerminalTheme; 6] = [
    TerminalTheme::AtomOneDark,
    TerminalTheme::GruvboxDark,
    TerminalTheme::TokyoNight,
    TerminalTheme::CatppuccinMocha,
    TerminalTheme::Nord,
    TerminalTheme::SolarizedDark,
];
const SETTINGS_FONT_FAMILY_CANDIDATES: [&str; 11] = [
    "JetBrains Mono",
    "SF Mono",
    "Menlo",
    "Monaco",
    "Fira Code",
    "Hack",
    "Consolas",
    "Cascadia Mono",
    "DejaVu Sans Mono",
    "Liberation Mono",
    "Noto Sans Mono",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SettingsLineHeightMode {
    Comfortable,
    Standard,
    Custom,
}

#[derive(Clone, Copy, Debug)]
struct ThemePalette {
    ui_bg: u32,
    terminal_bg: u32,
    cursor: u32,
    ansi_colors: [(u8, u8, u8); 16],
    foreground: (u8, u8, u8),
    background: (u8, u8, u8),
}

fn theme_palette(theme: TerminalTheme) -> ThemePalette {
    match theme {
        TerminalTheme::AtomOneDark => ThemePalette {
            ui_bg: 0x101010,
            terminal_bg: 0x000000,
            cursor: 0x528bff,
            ansi_colors: [
                (0x3F, 0x44, 0x51),
                (0xE0, 0x55, 0x61),
                (0x8C, 0xC2, 0x65),
                (0xD1, 0x8F, 0x52),
                (0x4A, 0xA5, 0xF0),
                (0xC1, 0x62, 0xDE),
                (0x42, 0xB3, 0xC2),
                (0xD7, 0xDA, 0xE0),
                (0x4F, 0x56, 0x66),
                (0xFF, 0x61, 0x6E),
                (0xA5, 0xE0, 0x75),
                (0xF0, 0xA4, 0x5D),
                (0x4D, 0xC4, 0xFF),
                (0xDE, 0x73, 0xFF),
                (0x4C, 0xD1, 0xE0),
                (0xE6, 0xE6, 0xE6),
            ],
            foreground: (0xE6, 0xE6, 0xE6),
            background: (0x00, 0x00, 0x00),
        },
        TerminalTheme::GruvboxDark => ThemePalette {
            ui_bg: 0x1D2021,
            terminal_bg: 0x282828,
            cursor: 0xFE8019,
            ansi_colors: [
                (0x28, 0x28, 0x28),
                (0xCC, 0x24, 0x1D),
                (0x98, 0x97, 0x1A),
                (0xD7, 0x99, 0x21),
                (0x45, 0x85, 0x88),
                (0xB1, 0x62, 0x86),
                (0x68, 0x9D, 0x6A),
                (0xA8, 0x99, 0x84),
                (0x92, 0x83, 0x74),
                (0xFB, 0x49, 0x34),
                (0xB8, 0xBB, 0x26),
                (0xFA, 0xBD, 0x2F),
                (0x83, 0xA5, 0x98),
                (0xD3, 0x86, 0x9B),
                (0x8E, 0xC0, 0x7C),
                (0xEB, 0xDB, 0xB2),
            ],
            foreground: (0xEB, 0xDB, 0xB2),
            background: (0x28, 0x28, 0x28),
        },
        TerminalTheme::TokyoNight => ThemePalette {
            ui_bg: 0x16161E,
            terminal_bg: 0x1A1B26,
            cursor: 0x7AA2F7,
            ansi_colors: [
                (0x15, 0x16, 0x1E),
                (0xF7, 0x76, 0x8E),
                (0x9E, 0xCE, 0x6A),
                (0xE0, 0xAF, 0x68),
                (0x7A, 0xA2, 0xF7),
                (0xBB, 0x9A, 0xF7),
                (0x7D, 0xCF, 0xFF),
                (0xA9, 0xB1, 0xD6),
                (0x41, 0x48, 0x68),
                (0xF7, 0x76, 0x8E),
                (0x9E, 0xCE, 0x6A),
                (0xE0, 0xAF, 0x68),
                (0x7A, 0xA2, 0xF7),
                (0xBB, 0x9A, 0xF7),
                (0x7D, 0xCF, 0xFF),
                (0xC0, 0xCA, 0xF5),
            ],
            foreground: (0xC0, 0xCA, 0xF5),
            background: (0x1A, 0x1B, 0x26),
        },
        TerminalTheme::CatppuccinMocha => ThemePalette {
            ui_bg: 0x181825,
            terminal_bg: 0x1E1E2E,
            cursor: 0xF5E0DC,
            ansi_colors: [
                (0x45, 0x47, 0x5A),
                (0xF3, 0x8B, 0xA8),
                (0xA6, 0xE3, 0xA1),
                (0xF9, 0xE2, 0xAF),
                (0x89, 0xB4, 0xFA),
                (0xF5, 0xC2, 0xE7),
                (0x94, 0xE2, 0xD5),
                (0xBA, 0xC2, 0xDE),
                (0x58, 0x5B, 0x70),
                (0xF3, 0x8B, 0xA8),
                (0xA6, 0xE3, 0xA1),
                (0xF9, 0xE2, 0xAF),
                (0x89, 0xB4, 0xFA),
                (0xF5, 0xC2, 0xE7),
                (0x94, 0xE2, 0xD5),
                (0xA6, 0xAD, 0xC8),
            ],
            foreground: (0xCD, 0xD6, 0xF4),
            background: (0x1E, 0x1E, 0x2E),
        },
        TerminalTheme::Nord => ThemePalette {
            ui_bg: 0x242933,
            terminal_bg: 0x2E3440,
            cursor: 0x88C0D0,
            ansi_colors: [
                (0x3B, 0x42, 0x52),
                (0xBF, 0x61, 0x6A),
                (0xA3, 0xBE, 0x8C),
                (0xEB, 0xCB, 0x8B),
                (0x81, 0xA1, 0xC1),
                (0xB4, 0x8E, 0xAD),
                (0x88, 0xC0, 0xD0),
                (0xE5, 0xE9, 0xF0),
                (0x4C, 0x56, 0x6A),
                (0xBF, 0x61, 0x6A),
                (0xA3, 0xBE, 0x8C),
                (0xEB, 0xCB, 0x8B),
                (0x81, 0xA1, 0xC1),
                (0xB4, 0x8E, 0xAD),
                (0x8F, 0xBC, 0xBB),
                (0xEC, 0xEF, 0xF4),
            ],
            foreground: (0xD8, 0xDE, 0xE9),
            background: (0x2E, 0x34, 0x40),
        },
        TerminalTheme::SolarizedDark => ThemePalette {
            ui_bg: 0x001F27,
            terminal_bg: 0x002B36,
            cursor: 0x268BD2,
            ansi_colors: [
                (0x07, 0x36, 0x42),
                (0xDC, 0x32, 0x2F),
                (0x85, 0x99, 0x00),
                (0xB5, 0x89, 0x00),
                (0x26, 0x8B, 0xD2),
                (0xD3, 0x36, 0x82),
                (0x2A, 0xA1, 0x98),
                (0xEE, 0xE8, 0xD5),
                (0x00, 0x2B, 0x36),
                (0xCB, 0x4B, 0x16),
                (0x58, 0x6E, 0x75),
                (0x65, 0x7B, 0x83),
                (0x83, 0x94, 0x96),
                (0x6C, 0x71, 0xC4),
                (0x93, 0xA1, 0xA1),
                (0xFD, 0xF6, 0xE3),
            ],
            foreground: (0x83, 0x94, 0x96),
            background: (0x00, 0x2B, 0x36),
        },
    }
}

struct TerminalTab {
    id: u64,
    number: usize,
    title: String,
    terminal: Terminal,
}

#[derive(Clone, Debug)]
struct TabTitleTooltip {
    title: SharedString,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FindMatch {
    start: AlacPoint,
    end: AlacPoint,
}

#[derive(Clone, Debug, Default)]
struct FindState {
    query: String,
    last_match: Option<FindMatch>,
    match_count: usize,
    active_match_index: Option<usize>,
}

pub struct TerminalView {
    tabs: Vec<TerminalTab>,
    active_tab_id: u64,
    hovered_tab_id: Option<u64>,
    next_tab_id: u64,
    pinned: bool,
    on_hide_terminal_requested: Option<Arc<dyn Fn() + Send + Sync>>,
    on_toggle_pin_requested: Option<Arc<dyn Fn() + Send + Sync>>,
    on_hotkeys_updated: Option<Arc<dyn Fn(String, String) + Send + Sync>>,
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
    find_state: Option<FindState>,
    settings_panel_open: bool,
    recording_global_hotkey: bool,
    window_has_been_active: bool,
    cursor_blink_visible: bool,
    suppress_cursor_blink_until: Option<Instant>,
    settings_drawer_scroll_handle: ScrollHandle,
    scrollbar_drag_offset: Option<Pixels>,
    row_text_cache: Vec<CachedRow>,
    previous_frame: Option<FrameCache>,
    perf: PerfInstrumentation,
    _resize_subscription: Subscription,
    _activation_subscription: Subscription,
}

#[derive(Clone, Copy, Debug, Default)]
struct SnapshotTiming {
    total: Duration,
    lock_hold: Duration,
}

#[derive(Clone)]
struct FrameCache {
    rows: Vec<Vec<CellSnapshot>>,
    colors: ColorsSnapshot,
    num_cols: usize,
    num_lines: usize,
    display_offset: usize,
    cursor_row: Option<usize>,
    cursor_col: usize,
    cursor_shape: CursorShape,
    show_cursor: bool,
    cursor_draw_visible: bool,
}

impl FrameCache {
    fn from_snapshot(snapshot: &TerminalSnapshot) -> Self {
        Self {
            rows: snapshot.rows.clone(),
            colors: snapshot.colors.clone(),
            num_cols: snapshot.num_cols,
            num_lines: snapshot.num_lines,
            display_offset: snapshot.display_offset,
            cursor_row: snapshot.cursor_row,
            cursor_col: snapshot.cursor_col,
            cursor_shape: snapshot.cursor_shape,
            show_cursor: snapshot.show_cursor,
            cursor_draw_visible: snapshot.cursor_draw_visible,
        }
    }
}

impl Render for TabTitleTooltip {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .rounded_sm()
            .border_1()
            .border_color(hsla(0.0, 0.0, 1.0, 0.18))
            .bg(hsla(0.0, 0.0, 0.06, 0.96))
            .text_xs()
            .text_color(hsla(0.0, 0.0, 1.0, 0.9))
            .child(self.title.clone())
    }
}

#[derive(Clone, Copy, Debug)]
struct PreviousFrameView {
    num_cols: usize,
    num_lines: usize,
    display_offset: usize,
}

impl PreviousFrameView {
    fn from_frame(frame: &FrameCache) -> Self {
        Self {
            num_cols: frame.num_cols,
            num_lines: frame.num_lines,
            display_offset: frame.display_offset,
        }
    }
}

#[derive(Default)]
struct PerfCounters {
    frames: u64,
    snapshot_total: Duration,
    snapshot_lock_hold: Duration,
    paint_total: Duration,
    dirty_rows: u64,
    total_rows: u64,
    text_row_cache_hits: u64,
    text_row_cache_misses: u64,
    background_row_cache_hits: u64,
    background_row_cache_misses: u64,
}

#[derive(Clone)]
struct PerfInstrumentation {
    enabled: bool,
    counters: Arc<Mutex<PerfCounters>>,
}

impl PerfInstrumentation {
    fn from_env() -> Self {
        let enabled = std::env::var("SIMPLE_TERM_PERF")
            .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
            .unwrap_or(false);
        Self {
            enabled,
            counters: Arc::new(Mutex::new(PerfCounters::default())),
        }
    }

    fn record_frame(
        &self,
        snapshot_timing: SnapshotTiming,
        paint_time: Duration,
        dirty_rows: usize,
        total_rows: usize,
        row_cache_stats: RowCacheStats,
    ) {
        if !self.enabled {
            return;
        }

        let mut counters = self.counters.lock();
        counters.frames += 1;
        counters.snapshot_total += snapshot_timing.total;
        counters.snapshot_lock_hold += snapshot_timing.lock_hold;
        counters.paint_total += paint_time;
        counters.dirty_rows += dirty_rows as u64;
        counters.total_rows += total_rows as u64;
        counters.text_row_cache_hits += row_cache_stats.text_hits as u64;
        counters.text_row_cache_misses += row_cache_stats.text_misses as u64;
        counters.background_row_cache_hits += row_cache_stats.background_hits as u64;
        counters.background_row_cache_misses += row_cache_stats.background_misses as u64;

        const LOG_EVERY_FRAMES: u64 = 120;
        if counters.frames.is_multiple_of(LOG_EVERY_FRAMES) {
            let frames = counters.frames as f32;
            let avg_snapshot_ms = counters.snapshot_total.as_secs_f32() * 1000.0 / frames;
            let avg_lock_ms = counters.snapshot_lock_hold.as_secs_f32() * 1000.0 / frames;
            let avg_paint_ms = counters.paint_total.as_secs_f32() * 1000.0 / frames;
            let dirty_ratio = if counters.total_rows == 0 {
                0.0
            } else {
                counters.dirty_rows as f32 / counters.total_rows as f32
            };
            let text_cache_total = counters.text_row_cache_hits + counters.text_row_cache_misses;
            let text_cache_hit_ratio = if text_cache_total == 0 {
                0.0
            } else {
                counters.text_row_cache_hits as f32 / text_cache_total as f32
            };
            let background_cache_total =
                counters.background_row_cache_hits + counters.background_row_cache_misses;
            let background_cache_hit_ratio = if background_cache_total == 0 {
                0.0
            } else {
                counters.background_row_cache_hits as f32 / background_cache_total as f32
            };

            log::info!(
                "terminal perf: frames={} avg_snapshot_ms={:.3} avg_lock_hold_ms={:.3} avg_paint_ms={:.3} avg_dirty_row_ratio={:.3} text_row_cache_hit_ratio={:.3} background_row_cache_hit_ratio={:.3}",
                counters.frames,
                avg_snapshot_ms,
                avg_lock_ms,
                avg_paint_ms,
                dirty_ratio,
                text_cache_hit_ratio,
                background_cache_hit_ratio,
            );
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct RowCacheStats {
    text_hits: usize,
    text_misses: usize,
    background_hits: usize,
    background_misses: usize,
}

impl RowCacheStats {
    fn record_hit(&mut self) {
        self.text_hits += 1;
        self.background_hits += 1;
    }

    fn record_miss(&mut self) {
        self.text_misses += 1;
        self.background_misses += 1;
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ViewUpdateAction {
    Notify,
    SetTitleAndNotify(String),
    Ignore,
    Exit,
}

fn update_action_for_terminal_event(event: TerminalEvent) -> ViewUpdateAction {
    match event {
        TerminalEvent::Wakeup => ViewUpdateAction::Notify,
        TerminalEvent::TitleChanged(title) => ViewUpdateAction::SetTitleAndNotify(title),
        TerminalEvent::Bell => ViewUpdateAction::Ignore,
        TerminalEvent::Exit(_) => ViewUpdateAction::Exit,
    }
}

impl TerminalView {
    pub(crate) fn focus_terminal(&self, window: &mut Window) {
        self.focus_handle.focus(window);
    }

    fn sanitize_tab_title(raw: &str) -> String {
        let cleaned: String = raw
            .chars()
            .filter(|ch| *ch == '\t' || !ch.is_control())
            .collect();
        let trimmed = cleaned.trim();
        if trimmed.is_empty() {
            "shell".to_string()
        } else {
            trimmed.chars().take(60).collect()
        }
    }

    fn next_tab_number_from_numbers(existing_numbers: &[usize]) -> usize {
        let mut candidate = 1usize;
        loop {
            if existing_numbers.iter().all(|number| *number != candidate) {
                return candidate;
            }
            candidate += 1;
        }
    }

    fn next_tab_number(existing_tabs: &[TerminalTab]) -> usize {
        let existing_numbers: Vec<usize> = existing_tabs.iter().map(|tab| tab.number).collect();
        Self::next_tab_number_from_numbers(&existing_numbers)
    }

    fn next_active_index_after_close(closing_index: usize, new_len: usize) -> usize {
        closing_index.min(new_len.saturating_sub(1))
    }

    fn should_hide_window_when_closing_tab(tab_count: usize) -> bool {
        tab_count <= 1
    }

    fn hovered_tab_id_after_event(
        current: Option<u64>,
        tab_id: u64,
        is_hovered: bool,
    ) -> Option<u64> {
        if is_hovered {
            Some(tab_id)
        } else if current == Some(tab_id) {
            None
        } else {
            current
        }
    }

    fn should_schedule_window_deactivation_hide(
        auto_hide_on_outside_click: bool,
        window_is_active: bool,
        window_has_been_active: bool,
    ) -> bool {
        auto_hide_on_outside_click && !window_is_active && window_has_been_active
    }

    fn schedule_window_deactivation_hide(
        auto_hide_on_outside_click: bool,
        window_is_active: bool,
        window_has_been_active: bool,
        on_window_deactivated: Option<Arc<dyn Fn() + Send + Sync>>,
        mut schedule: impl FnMut(Arc<dyn Fn() + Send + Sync>),
    ) {
        if !Self::should_schedule_window_deactivation_hide(
            auto_hide_on_outside_click,
            window_is_active,
            window_has_been_active,
        ) {
            return;
        }

        if let Some(on_window_deactivated) = on_window_deactivated {
            schedule(on_window_deactivated);
        }
    }

    fn request_hide_terminal_window(&self, cx: &mut Context<Self>) {
        if let Some(on_hide_terminal_requested) = self.on_hide_terminal_requested.as_ref() {
            on_hide_terminal_requested();
        } else {
            cx.hide();
        }
    }

    fn request_toggle_pin(&self) {
        if let Some(on_toggle_pin_requested) = self.on_toggle_pin_requested.as_ref() {
            on_toggle_pin_requested();
        }
    }

    pub(crate) fn set_pinned(&mut self, pinned: bool, cx: &mut Context<Self>) {
        if self.pinned == pinned {
            return;
        }

        self.pinned = pinned;
        cx.notify();
    }

    fn terminal_grid_for_viewport(viewport: Size<Pixels>, cell_size: Size<Pixels>) -> Size<u16> {
        let content_height = if viewport.height > px(TAB_BAR_HEIGHT_PX) {
            viewport.height - px(TAB_BAR_HEIGHT_PX)
        } else {
            px(1.0)
        };

        let cols = std::cmp::max(
            (f32::from(viewport.width) / f32::from(cell_size.width)) as u16,
            1,
        );
        let lines = std::cmp::max(
            (f32::from(content_height) / f32::from(cell_size.height)) as u16,
            1,
        );

        Size {
            width: cols,
            height: lines,
        }
    }

    fn window_size_for_grid(grid_size: Size<u16>, cell_size: Size<Pixels>) -> WindowSize {
        WindowSize {
            num_lines: grid_size.height,
            num_cols: grid_size.width,
            cell_width: f32::from(cell_size.width) as u16,
            cell_height: f32::from(cell_size.height) as u16,
        }
    }

    fn spawn_terminal(
        settings: &TerminalSettings,
        window_size: WindowSize,
    ) -> std::io::Result<Terminal> {
        let scrollback_lines = settings
            .max_scroll_history_lines
            .unwrap_or(simple_term::config::DEFAULT_SCROLL_HISTORY_LINES);
        let working_directory = resolve_working_directory(&settings.working_directory);
        Terminal::new(
            settings.shell.to_shell(),
            working_directory,
            window_size,
            scrollback_lines,
            settings.env.clone(),
            settings.default_cursor_style(),
        )
    }

    fn tab_display_title(tab: &TerminalTab) -> String {
        format!("{}: {}", tab.number, tab.title)
    }

    fn push_unique_font_family(options: &mut Vec<String>, family: &str) {
        let trimmed = family.trim();
        if trimmed.is_empty() {
            return;
        }
        if options
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(trimmed))
        {
            return;
        }
        options.push(trimmed.to_string());
    }

    fn font_family_options_from_settings(settings: &TerminalSettings) -> Vec<String> {
        let mut options = Vec::new();
        Self::push_unique_font_family(&mut options, &settings.font_family);
        for fallback in &settings.font_fallbacks {
            Self::push_unique_font_family(&mut options, fallback);
        }
        for candidate in SETTINGS_FONT_FAMILY_CANDIDATES {
            Self::push_unique_font_family(&mut options, candidate);
        }
        options
    }

    fn next_font_family(current: &str, options: &[String], direction: isize) -> String {
        if options.is_empty() {
            return current.to_string();
        }

        let current_index = options
            .iter()
            .position(|family| family == current)
            .or_else(|| {
                options
                    .iter()
                    .position(|family| family.eq_ignore_ascii_case(current))
            })
            .unwrap_or(0);

        let next_index = (current_index as isize + direction).rem_euclid(options.len() as isize);
        options[next_index as usize].clone()
    }

    fn theme_label(theme: TerminalTheme) -> &'static str {
        match theme {
            TerminalTheme::AtomOneDark => "Atom One Dark",
            TerminalTheme::GruvboxDark => "Gruvbox Dark",
            TerminalTheme::TokyoNight => "Tokyo Night",
            TerminalTheme::CatppuccinMocha => "Catppuccin",
            TerminalTheme::Nord => "Nord",
            TerminalTheme::SolarizedDark => "Solarized Dark",
        }
    }

    fn next_theme(current: TerminalTheme, direction: isize) -> TerminalTheme {
        let current_index = THEME_PRESETS
            .iter()
            .position(|theme| *theme == current)
            .unwrap_or(0);
        let next_index =
            (current_index as isize + direction).rem_euclid(THEME_PRESETS.len() as isize);
        THEME_PRESETS[next_index as usize]
    }

    fn toggled_settings_panel_open(is_open: bool) -> bool {
        !is_open
    }

    fn pin_indicator_symbol(pinned: bool) -> &'static str {
        if pinned {
            "📌"
        } else {
            "○"
        }
    }

    fn find_panel_width_for_viewport(viewport_width: Pixels) -> Pixels {
        let max_width = (viewport_width - px(FIND_PANEL_VIEWPORT_MARGIN_PX)).max(px(0.0));
        let lower_bound = px(FIND_PANEL_MIN_WIDTH_PX).min(max_width);
        let preferred = (viewport_width - px(FIND_PANEL_RESERVED_SPACE_PX))
            .min(px(FIND_PANEL_MAX_WIDTH_PX))
            .max(lower_bound);
        preferred.min(max_width)
    }

    fn settings_drawer_width_for_viewport(viewport_width: Pixels) -> Pixels {
        let max_width = (viewport_width - px(SETTINGS_DRAWER_VIEWPORT_MARGIN_PX)).max(px(0.0));
        let lower_bound = px(SETTINGS_DRAWER_MIN_WIDTH_PX).min(max_width);
        px(SETTINGS_DRAWER_WIDTH_PX).min(max_width).max(lower_bound)
    }

    fn settings_drawer_scrollbar_width() -> Pixels {
        px(SETTINGS_DRAWER_SCROLLBAR_WIDTH_PX)
    }

    fn settings_drawer_scrollbar_thumb_metrics(
        viewport_height: Pixels,
        max_scroll_offset: Pixels,
        scroll_offset: Pixels,
    ) -> Option<(Pixels, Pixels)> {
        if viewport_height <= px(0.0) || max_scroll_offset <= px(0.0) {
            return None;
        }

        let content_height = viewport_height + max_scroll_offset;
        let visible_ratio = (viewport_height / content_height).clamp(0.0, 1.0);
        let thumb_height = (viewport_height * visible_ratio)
            .max(px(SETTINGS_DRAWER_SCROLLBAR_MIN_THUMB_HEIGHT_PX))
            .min(viewport_height);

        let travel = (viewport_height - thumb_height).max(px(0.0));
        let progress = (-scroll_offset / max_scroll_offset).clamp(0.0, 1.0);
        let thumb_top = travel * progress;

        Some((thumb_top, thumb_height))
    }

    fn settings_drawer_scrollbar_metrics(&self) -> Option<(Pixels, Pixels)> {
        let bounds = self.settings_drawer_scroll_handle.bounds();
        Self::settings_drawer_scrollbar_thumb_metrics(
            bounds.size.height,
            self.settings_drawer_scroll_handle.max_offset().height,
            self.settings_drawer_scroll_handle.offset().y,
        )
    }

    fn should_close_settings_panel_for_keystroke(keystroke: &Keystroke) -> bool {
        let modifiers = keystroke.modifiers;
        keystroke.key == "escape" && !modifiers.platform && !modifiers.control && !modifiers.alt
    }

    fn line_height_mode(line_height: &LineHeight) -> SettingsLineHeightMode {
        match line_height {
            LineHeight::Comfortable => SettingsLineHeightMode::Comfortable,
            LineHeight::Standard => SettingsLineHeightMode::Standard,
            LineHeight::Custom { .. } => SettingsLineHeightMode::Custom,
        }
    }

    fn normalized_scroll_multiplier(value: f32) -> f32 {
        if value.is_finite() {
            value.clamp(
                SETTINGS_MIN_SCROLL_MULTIPLIER,
                SETTINGS_MAX_SCROLL_MULTIPLIER,
            )
        } else {
            1.0
        }
    }

    fn global_hotkey_key_token(key: &str) -> Option<String> {
        let normalized = key.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            return None;
        }

        if matches!(
            normalized.as_str(),
            "shift"
                | "control"
                | "ctrl"
                | "alt"
                | "option"
                | "command"
                | "cmd"
                | "super"
                | "meta"
                | "fn"
                | "function"
        ) {
            return None;
        }

        match normalized.as_str() {
            "backquote" | "`" => return Some("Backquote".to_string()),
            "space" => return Some("Space".to_string()),
            "tab" => return Some("Tab".to_string()),
            "enter" | "return" => return Some("Enter".to_string()),
            "escape" => return Some("Escape".to_string()),
            "backspace" => return Some("Backspace".to_string()),
            "delete" => return Some("Delete".to_string()),
            _ => {}
        }

        if normalized.starts_with('f')
            && normalized.len() > 1
            && normalized[1..].chars().all(|ch| ch.is_ascii_digit())
        {
            return Some(format!("F{}", &normalized[1..]));
        }

        if normalized.len() == 1 {
            let ch = normalized.chars().next().expect("single character");
            if ch.is_ascii_alphanumeric() {
                return Some(ch.to_ascii_uppercase().to_string());
            }
        }

        None
    }

    fn global_hotkey_from_keystroke(keystroke: &Keystroke) -> Option<String> {
        let modifiers = keystroke.modifiers;
        if !modifiers.platform && !modifiers.control && !modifiers.alt {
            return None;
        }

        let key_token = Self::global_hotkey_key_token(&keystroke.key)?;
        let mut parts = Vec::with_capacity(5);
        if modifiers.platform {
            parts.push("command".to_string());
        }
        if modifiers.control {
            parts.push("control".to_string());
        }
        if modifiers.alt {
            parts.push("alt".to_string());
        }
        if modifiers.shift {
            parts.push("shift".to_string());
        }
        parts.push(key_token);

        let candidate = parts.join("+");
        candidate.parse::<GlobalHotKey>().ok().map(|_| candidate)
    }

    fn pin_hotkey_matches_keystroke(pin_hotkey: &str, keystroke: &Keystroke) -> bool {
        let Some(candidate) = Self::global_hotkey_from_keystroke(keystroke) else {
            return false;
        };

        match (
            candidate.parse::<GlobalHotKey>(),
            pin_hotkey.parse::<GlobalHotKey>(),
        ) {
            (Ok(candidate), Ok(configured)) => candidate == configured,
            _ => false,
        }
    }

    fn persist_settings(&self) {
        let config_path = TerminalSettings::config_path();
        if let Err(err) = self.settings.save(&config_path) {
            log::warn!(
                "failed to save settings to {}: {err}",
                config_path.display()
            );
        }
    }

    fn resolve_font_and_cell_size(
        window: &Window,
        settings: &TerminalSettings,
    ) -> (Font, Pixels, Size<Pixels>) {
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

        let font_id = text_system.resolve_font(&font);
        let cell_advance = text_system
            .advance(font_id, font_size, 'm')
            .unwrap_or(Size {
                width: px(8.4),
                height: px(17.0),
            });
        let cell_size = Size {
            width: cell_advance.width,
            height: font_size * settings.line_height.to_ratio(),
        };

        (font, font_size, cell_size)
    }

    fn sync_grid_to_viewport(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
        force_resize: bool,
    ) {
        let new_grid_size =
            Self::terminal_grid_for_viewport(window.viewport_size(), self.cell_size);
        if new_grid_size.width == 0 || new_grid_size.height == 0 {
            return;
        }

        let grid_changed = new_grid_size.width != self.grid_size.width
            || new_grid_size.height != self.grid_size.height;
        if !grid_changed && !force_resize {
            return;
        }

        let window_size = Self::window_size_for_grid(new_grid_size, self.cell_size);
        for tab in &self.tabs {
            tab.terminal.resize(window_size);
        }
        self.grid_size = new_grid_size;
        self.reset_active_tab_frame_state();
        cx.notify();
    }

    fn apply_typography_settings(&mut self, window: &Window, cx: &mut Context<Self>) {
        let (font, font_size, cell_size) = Self::resolve_font_and_cell_size(window, &self.settings);
        self.font = font;
        self.font_size = font_size;
        self.cell_size = cell_size;
        self.sync_grid_to_viewport(window, cx, true);
    }

    fn persist_and_notify(&self, cx: &mut Context<Self>) {
        self.persist_settings();
        cx.notify();
    }

    fn cycle_font_family(&mut self, direction: isize, window: &Window, cx: &mut Context<Self>) {
        let options = Self::font_family_options_from_settings(&self.settings);
        let next_font = Self::next_font_family(&self.settings.font_family, &options, direction);
        if next_font == self.settings.font_family {
            return;
        }

        self.settings.font_family = next_font;
        self.apply_typography_settings(window, cx);
        self.persist_settings();
    }

    fn adjust_font_size(&mut self, delta: f32, window: &Window, cx: &mut Context<Self>) {
        let next_size =
            (self.settings.font_size + delta).clamp(SETTINGS_MIN_FONT_SIZE, SETTINGS_MAX_FONT_SIZE);
        if (next_size - self.settings.font_size).abs() < f32::EPSILON {
            return;
        }

        self.settings.font_size = next_size;
        self.apply_typography_settings(window, cx);
        self.persist_settings();
    }

    fn cycle_theme(&mut self, direction: isize, cx: &mut Context<Self>) {
        let next_theme = Self::next_theme(self.settings.theme, direction);
        if next_theme == self.settings.theme {
            return;
        }

        self.settings.theme = next_theme;
        self.persist_and_notify(cx);
    }

    fn line_height_display(line_height: &LineHeight) -> String {
        match line_height {
            LineHeight::Comfortable => "comfortable".to_string(),
            LineHeight::Standard => "standard".to_string(),
            LineHeight::Custom { value } => format!("{value:.2}"),
        }
    }

    fn set_line_height_mode(
        &mut self,
        mode: SettingsLineHeightMode,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let next = match mode {
            SettingsLineHeightMode::Comfortable => LineHeight::Comfortable,
            SettingsLineHeightMode::Standard => LineHeight::Standard,
            SettingsLineHeightMode::Custom => {
                let current = self.settings.line_height.to_ratio().clamp(
                    SETTINGS_MIN_LINE_HEIGHT_RATIO,
                    SETTINGS_MAX_LINE_HEIGHT_RATIO,
                );
                LineHeight::Custom { value: current }
            }
        };
        if self.settings.line_height == next {
            return;
        }
        self.settings.line_height = next;
        self.apply_typography_settings(window, cx);
        self.persist_settings();
    }

    fn adjust_line_height_custom(&mut self, delta: f32, window: &Window, cx: &mut Context<Self>) {
        let current = self.settings.line_height.to_ratio();
        let next = (current + delta).clamp(
            SETTINGS_MIN_LINE_HEIGHT_RATIO,
            SETTINGS_MAX_LINE_HEIGHT_RATIO,
        );
        if (next - current).abs() < f32::EPSILON {
            return;
        }
        self.settings.line_height = LineHeight::Custom { value: next };
        self.apply_typography_settings(window, cx);
        self.persist_settings();
    }

    fn adjust_scroll_multiplier(&mut self, delta: f32, cx: &mut Context<Self>) {
        let current = Self::normalized_scroll_multiplier(self.settings.scroll_multiplier);
        let next = Self::normalized_scroll_multiplier(current + delta);
        if (next - current).abs() < f32::EPSILON {
            return;
        }
        self.settings.scroll_multiplier = next;
        self.persist_and_notify(cx);
    }

    fn apply_global_hotkey_setting(&mut self, hotkey: String, cx: &mut Context<Self>) {
        if self.settings.global_hotkey == hotkey {
            cx.notify();
            return;
        }

        self.settings.global_hotkey = hotkey;
        self.persist_and_notify(cx);
        if let Some(on_hotkeys_updated) = &self.on_hotkeys_updated {
            on_hotkeys_updated(
                self.settings.global_hotkey.clone(),
                self.settings.pin_hotkey.clone(),
            );
        }
    }

    fn set_global_hotkey_recording(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.recording_global_hotkey == enabled {
            return;
        }

        self.recording_global_hotkey = enabled;
        cx.notify();
    }

    fn handle_global_hotkey_recording(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.recording_global_hotkey {
            return false;
        }

        if Self::should_close_settings_panel_for_keystroke(&event.keystroke) {
            self.recording_global_hotkey = false;
            cx.notify();
            return true;
        }

        let Some(recorded_hotkey) = Self::global_hotkey_from_keystroke(&event.keystroke) else {
            return true;
        };

        self.recording_global_hotkey = false;
        self.apply_global_hotkey_setting(recorded_hotkey, cx);
        true
    }

    fn set_copy_on_select(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.settings.copy_on_select == enabled {
            return;
        }
        self.settings.copy_on_select = enabled;
        self.persist_and_notify(cx);
    }

    fn set_keep_selection_on_copy(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.settings.keep_selection_on_copy == enabled {
            return;
        }
        self.settings.keep_selection_on_copy = enabled;
        self.persist_and_notify(cx);
    }

    fn set_option_as_meta(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.settings.option_as_meta == enabled {
            return;
        }
        self.settings.option_as_meta = enabled;
        self.persist_and_notify(cx);
    }

    fn cursor_style_escape(shape: SettingsCursorShape, blinking: Blinking) -> &'static str {
        let blink = blinking.default_enabled();
        match shape {
            SettingsCursorShape::Block => {
                if blink {
                    "\u{1b}[1 q"
                } else {
                    "\u{1b}[2 q"
                }
            }
            SettingsCursorShape::Underline => {
                if blink {
                    "\u{1b}[3 q"
                } else {
                    "\u{1b}[4 q"
                }
            }
            SettingsCursorShape::Bar => {
                if blink {
                    "\u{1b}[5 q"
                } else {
                    "\u{1b}[6 q"
                }
            }
            SettingsCursorShape::Hollow => "\u{1b}[2 q",
        }
    }

    fn apply_cursor_settings(&mut self, cx: &mut Context<Self>) {
        let escape = Self::cursor_style_escape(self.settings.cursor_shape, self.settings.blinking);
        for tab in &self.tabs {
            tab.terminal.write_str(escape);
        }
        self.previous_frame = None;
        self.persist_and_notify(cx);
    }

    fn set_cursor_shape_setting(&mut self, shape: SettingsCursorShape, cx: &mut Context<Self>) {
        if self.settings.cursor_shape == shape {
            return;
        }
        self.settings.cursor_shape = shape;
        self.apply_cursor_settings(cx);
    }

    fn set_blinking_setting(&mut self, blinking: Blinking, cx: &mut Context<Self>) {
        if self.settings.blinking == blinking {
            return;
        }
        self.settings.blinking = blinking;
        self.apply_cursor_settings(cx);
    }

    fn toggle_settings_panel(&mut self, cx: &mut Context<Self>) {
        self.settings_panel_open = Self::toggled_settings_panel_open(self.settings_panel_open);
        if !self.settings_panel_open {
            self.recording_global_hotkey = false;
        }
        cx.notify();
    }

    #[cfg(test)]
    fn tab_item_vertical_footprint_px(_has_separator_in_flow: bool) -> f32 {
        TAB_ITEM_HEIGHT_PX + TAB_ITEM_INDICATOR_HEIGHT_PX + TAB_ITEM_INDICATOR_BOTTOM_GAP_PX
    }

    fn active_tab_index(&self) -> usize {
        self.tabs
            .iter()
            .position(|tab| tab.id == self.active_tab_id)
            .unwrap_or(0)
    }

    fn active_tab(&self) -> &TerminalTab {
        &self.tabs[self.active_tab_index()]
    }

    fn active_terminal(&self) -> &Terminal {
        &self.active_tab().terminal
    }

    fn active_window_title(&self) -> String {
        Self::tab_display_title(self.active_tab())
    }

    fn reset_active_tab_frame_state(&mut self) {
        self.pending_scroll_lines = 0.0;
        self.suppress_precise_scroll_until = None;
        self.suppress_precise_scroll_until_ended = false;
        self.hovered_tab_id = None;
        self.selection_anchor = None;
        self.find_state = None;
        self.cursor_blink_visible = true;
        self.suppress_cursor_blink_until = None;
        self.scrollbar_drag_offset = None;
        self.row_text_cache.clear();
        self.previous_frame = None;
    }

    fn set_active_tab(&mut self, tab_id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if self.active_tab_id == tab_id {
            return;
        }
        if !self.tabs.iter().any(|tab| tab.id == tab_id) {
            return;
        }

        self.active_tab_id = tab_id;
        self.reset_active_tab_frame_state();
        window.set_window_title(&self.active_window_title());
        cx.notify();
    }

    fn set_active_tab_relative(
        &mut self,
        direction: isize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.tabs.is_empty() {
            return;
        }

        let current_index = self.active_tab_index() as isize;
        let tab_count = self.tabs.len() as isize;
        let next_index = (current_index + direction).rem_euclid(tab_count) as usize;
        let next_tab_id = self.tabs[next_index].id;
        self.set_active_tab(next_tab_id, window, cx);
    }

    fn create_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;

        let number = Self::next_tab_number(&self.tabs);
        let terminal = Self::spawn_terminal(
            &self.settings,
            Self::window_size_for_grid(self.grid_size, self.cell_size),
        )
        .expect("Failed to spawn terminal");
        let events = terminal.events.clone();
        let title = number.to_string();

        self.tabs.push(TerminalTab {
            id: tab_id,
            number,
            title,
            terminal,
        });
        self.active_tab_id = tab_id;
        self.reset_active_tab_frame_state();

        Self::spawn_terminal_event_loop(tab_id, events, window, cx);
        window.set_window_title(&self.active_window_title());
        cx.notify();
    }

    fn close_tab(&mut self, tab_id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if Self::should_hide_window_when_closing_tab(self.tabs.len()) {
            self.request_hide_terminal_window(cx);
            return;
        }

        let Some(closing_index) = self.tabs.iter().position(|tab| tab.id == tab_id) else {
            return;
        };
        let was_active = self.active_tab_id == tab_id;
        if self.hovered_tab_id == Some(tab_id) {
            self.hovered_tab_id = None;
        }

        self.tabs.remove(closing_index);
        if was_active {
            let next_active_index =
                Self::next_active_index_after_close(closing_index, self.tabs.len());
            self.active_tab_id = self.tabs[next_active_index].id;
            self.reset_active_tab_frame_state();
            window.set_window_title(&self.active_window_title());
        }

        cx.notify();
    }

    fn update_tab_title(
        &mut self,
        tab_id: u64,
        raw_title: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == tab_id) else {
            return;
        };

        let title = Self::sanitize_tab_title(raw_title);
        if tab.title == title {
            return;
        }
        tab.title = title;

        if self.active_tab_id == tab_id {
            window.set_window_title(&self.active_window_title());
        }
        cx.notify();
    }

    fn spawn_terminal_event_loop(
        tab_id: u64,
        events: smol::channel::Receiver<TerminalEvent>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.spawn_in(
            window,
            async move |this: WeakEntity<TerminalView>, cx: &mut AsyncWindowContext| {
                while let Ok(event) = events.recv().await {
                    match update_action_for_terminal_event(event) {
                        ViewUpdateAction::Notify => {
                            let _ = cx.update(|_window, cx| {
                                let _ = this.update(cx, |this, cx| {
                                    if this.active_tab_id == tab_id {
                                        cx.notify();
                                    }
                                });
                            });
                        }
                        ViewUpdateAction::SetTitleAndNotify(title) => {
                            let _ = cx.update(|window, cx| {
                                let _ = this.update(cx, |this, cx| {
                                    this.update_tab_title(tab_id, &title, window, cx);
                                });
                            });
                        }
                        ViewUpdateAction::Ignore => {}
                        ViewUpdateAction::Exit => break,
                    }
                }
            },
        )
        .detach();
    }

    fn spawn_cursor_blink_loop(window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(
            window,
            async move |this: WeakEntity<TerminalView>, cx: &mut AsyncWindowContext| loop {
                smol::Timer::after(CURSOR_BLINK_INTERVAL).await;

                let updated = cx.update(|_window, cx| {
                    let _ = this.update(cx, |this, cx| {
                        let terminal_blinking = {
                            let term = this.active_terminal().term.lock();
                            term.cursor_style().blinking
                        };
                        let now = Instant::now();
                        let suppress_blink = this.cursor_blink_suppressed(now);
                        let should_blink =
                            cursor_should_blink(this.settings.blinking, terminal_blinking)
                                && !suppress_blink;
                        if should_blink {
                            this.cursor_blink_visible = !this.cursor_blink_visible;
                            this.previous_frame = None;
                            cx.notify();
                        } else if !this.cursor_blink_visible {
                            this.cursor_blink_visible = true;
                            this.previous_frame = None;
                            cx.notify();
                        }
                    });
                });

                if updated.is_err() {
                    break;
                }
            },
        )
        .detach();
    }

    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        settings: TerminalSettings,
        pinned: bool,
        on_hide_terminal_requested: Option<Arc<dyn Fn() + Send + Sync>>,
        on_window_deactivated: Option<Arc<dyn Fn() + Send + Sync>>,
        on_toggle_pin_requested: Option<Arc<dyn Fn() + Send + Sync>>,
        on_hotkeys_updated: Option<Arc<dyn Fn(String, String) + Send + Sync>>,
    ) -> Self {
        let (font, font_size, cell_size) = Self::resolve_font_and_cell_size(window, &settings);

        let grid_size = Self::terminal_grid_for_viewport(window.viewport_size(), cell_size);
        let window_size = Self::window_size_for_grid(grid_size, cell_size);
        let first_terminal =
            Self::spawn_terminal(&settings, window_size).expect("Failed to spawn terminal");
        let first_events = first_terminal.events.clone();

        let regex_searches = RegexSearches::new(
            &settings.path_hyperlink_regexes,
            settings.path_hyperlink_timeout_ms,
        );
        let focus_handle = cx.focus_handle();

        let resize_subscription =
            cx.observe_window_bounds(window, |this: &mut Self, window, cx| {
                this.handle_resize(window, cx);
            });
        let auto_hide_on_outside_click = settings.auto_hide_on_outside_click;
        let activation_subscription =
            cx.observe_window_activation(window, move |this, window, cx| {
                let window_is_active = window.is_window_active();
                this.window_has_been_active = this.window_has_been_active || window_is_active;
                Self::schedule_window_deactivation_hide(
                    auto_hide_on_outside_click,
                    window_is_active,
                    this.window_has_been_active,
                    on_window_deactivated.clone(),
                    |on_window_deactivated| {
                        cx.defer(move |_| {
                            on_window_deactivated();
                        });
                    },
                );
            });

        let view = TerminalView {
            tabs: vec![TerminalTab {
                id: 1,
                number: 1,
                title: "1".to_string(),
                terminal: first_terminal,
            }],
            active_tab_id: 1,
            hovered_tab_id: None,
            next_tab_id: 2,
            pinned,
            on_hide_terminal_requested,
            on_toggle_pin_requested,
            on_hotkeys_updated,
            regex_searches,
            settings,
            focus_handle,
            font,
            font_size,
            cell_size,
            grid_size,
            pending_scroll_lines: 0.0,
            suppress_precise_scroll_until: None,
            suppress_precise_scroll_until_ended: false,
            selection_anchor: None,
            find_state: None,
            settings_panel_open: false,
            recording_global_hotkey: false,
            window_has_been_active: false,
            cursor_blink_visible: true,
            suppress_cursor_blink_until: None,
            settings_drawer_scroll_handle: ScrollHandle::new(),
            scrollbar_drag_offset: None,
            row_text_cache: Vec::new(),
            previous_frame: None,
            perf: PerfInstrumentation::from_env(),
            _resize_subscription: resize_subscription,
            _activation_subscription: activation_subscription,
        };
        window.set_window_title(&view.active_window_title());
        Self::spawn_terminal_event_loop(1, first_events, window, cx);
        Self::spawn_cursor_blink_loop(window, cx);

        view
    }

    fn mode_and_display_offset(&self) -> (TermMode, usize) {
        let term = self.active_terminal().term.lock();
        (*term.mode(), term.grid().display_offset())
    }

    fn terminal_bounds(&self) -> TerminalBounds {
        TerminalBounds::new(
            self.cell_size.height,
            self.cell_size.width,
            Bounds {
                origin: point(px(0.), px(TAB_BAR_HEIGHT_PX)),
                size: size(
                    self.cell_size.width * self.grid_size.width as f32,
                    self.cell_size.height * self.grid_size.height as f32,
                ),
            },
        )
    }

    fn scrollbar_layout(&self) -> Option<ScrollbarLayout> {
        let term = self.active_terminal().term.lock();
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
        let mut term = self.active_terminal().term.lock();
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
        self.sync_grid_to_viewport(window, cx, false);
    }

    fn scroll_to_bottom(&mut self) -> bool {
        let mut term = self.active_terminal().term.lock();
        let was_scrolled = term.grid().display_offset() != 0;
        if was_scrolled {
            term.scroll_display(Scroll::Bottom);
        }
        was_scrolled
    }

    fn begin_terminal_input(&mut self, cx: &mut Context<Self>) {
        let was_scrolled = self.scroll_to_bottom();
        let now = Instant::now();
        prepare_for_terminal_input(
            was_scrolled,
            &mut self.pending_scroll_lines,
            &mut self.suppress_precise_scroll_until,
            &mut self.suppress_precise_scroll_until_ended,
            now,
        );
        self.suppress_cursor_blink_until = Some(now + CURSOR_BLINK_SUPPRESSION_AFTER_INPUT);
        if !self.cursor_blink_visible {
            self.cursor_blink_visible = true;
            self.previous_frame = None;
            cx.notify();
        }
    }

    fn cursor_blink_suppressed(&mut self, now: Instant) -> bool {
        if cursor_blink_is_suppressed(self.suppress_cursor_blink_until, now) {
            return true;
        }

        if self.suppress_cursor_blink_until.is_some() {
            self.suppress_cursor_blink_until = None;
        }

        false
    }

    fn copy_selection_to_clipboard(&mut self, cx: &mut Context<Self>) -> bool {
        let mut term = self.active_terminal().term.lock();
        let Some(text) = term.selection_to_string().filter(|text| !text.is_empty()) else {
            return false;
        };

        let clear_selection = !self.settings.keep_selection_on_copy;
        if clear_selection {
            term.selection = None;
        }
        drop(term);

        cx.write_to_clipboard(ClipboardItem::new_string(text));
        if clear_selection {
            cx.notify();
        }

        true
    }

    fn select_all_terminal_content(&mut self) -> bool {
        let mut term = self.active_terminal().term.lock();
        if term.columns() == 0 || term.screen_lines() == 0 {
            return false;
        }

        let start = AlacPoint::new(term.topmost_line(), Column(0));
        let end = AlacPoint::new(term.bottommost_line(), term.last_column());
        let mut selection = Selection::new(SelectionType::Simple, start, Side::Left);
        selection.update(end, Side::Right);
        term.selection = Some(selection);
        drop(term);
        self.selection_anchor = None;

        true
    }

    fn normalize_find_query(selection: &str) -> Option<String> {
        selection
            .lines()
            .next()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(ToString::to_string)
    }

    fn regex_escape_literal(query: &str) -> String {
        let mut escaped = String::with_capacity(query.len());
        for ch in query.chars() {
            match ch {
                '\\' | '.' | '+' | '*' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$'
                | '|' => {
                    escaped.push('\\');
                    escaped.push(ch);
                }
                _ => escaped.push(ch),
            }
        }
        escaped
    }

    fn collect_find_match_stats(
        term: &alacritty_terminal::term::Term<simple_term::terminal::EventProxy>,
        query: &str,
        active: Option<FindMatch>,
    ) -> (usize, Option<usize>) {
        if query.is_empty() {
            return (0, None);
        }

        let escaped = Self::regex_escape_literal(query);
        let mut regex = match RegexSearch::new(&escaped) {
            Ok(regex) => regex,
            Err(err) => {
                log::warn!("failed to build find regex for '{}': {err}", query);
                return (0, None);
            }
        };

        let mut origin = AlacPoint::new(term.topmost_line(), Column(0));
        let mut count = 0usize;
        let mut active_index = None;
        let mut first_seen = None::<FindMatch>;

        loop {
            let Some(found_range) =
                term.search_next(&mut regex, origin, AlacDirection::Right, Side::Right, None)
            else {
                break;
            };

            let found = FindMatch {
                start: *found_range.start(),
                end: *found_range.end(),
            };
            if first_seen.is_some() && first_seen == Some(found) {
                break;
            }

            if first_seen.is_none() {
                first_seen = Some(found);
            }

            count += 1;
            if active == Some(found) {
                active_index = Some(count);
            }

            let next_origin = found.end.add(term, Boundary::None, 1);
            if next_origin == origin {
                break;
            }
            origin = next_origin;
        }

        (count, active_index)
    }

    fn start_find(&mut self, cx: &mut Context<Self>) {
        let selected_query = {
            let term = self.active_terminal().term.lock();
            term.selection_to_string()
                .as_deref()
                .and_then(Self::normalize_find_query)
        };

        if let Some(query) = selected_query {
            self.find_state = Some(FindState {
                query,
                last_match: None,
                match_count: 0,
                active_match_index: None,
            });
        } else if self.find_state.is_none() {
            self.find_state = Some(FindState::default());
        } else if let Some(state) = self.find_state.as_mut() {
            state.last_match = None;
            state.active_match_index = None;
        }

        let has_query = self
            .find_state
            .as_ref()
            .is_some_and(|state| !state.query.is_empty());
        if has_query {
            if !self.find_next_match(AlacDirection::Right, cx) {
                cx.notify();
            }
            return;
        }

        cx.notify();
    }

    fn find_next_match(&mut self, direction: AlacDirection, cx: &mut Context<Self>) -> bool {
        let query = match self
            .find_state
            .as_ref()
            .map(|state| state.query.as_str())
            .filter(|query| !query.is_empty())
        {
            Some(query) => query,
            None => return false,
        };

        let escaped = Self::regex_escape_literal(query);
        let mut regex = match RegexSearch::new(&escaped) {
            Ok(regex) => regex,
            Err(err) => {
                log::warn!("failed to build find regex for '{}': {err}", query);
                return false;
            }
        };

        let previous_match = self.find_state.as_ref().and_then(|state| state.last_match);
        let side = if matches!(direction, AlacDirection::Right) {
            Side::Right
        } else {
            Side::Left
        };

        let mut term = self.active_terminal().term.lock();
        if term.columns() == 0 || term.screen_lines() == 0 {
            return false;
        }

        let origin = match previous_match {
            Some(found) if matches!(direction, AlacDirection::Right) => {
                found.end.add(&*term, Boundary::None, 1)
            }
            Some(found) => found.start.sub(&*term, Boundary::None, 1),
            None => {
                let display_offset = term.grid().display_offset() as i32;
                AlacPoint::new(Line(-display_offset), Column(0))
            }
        };

        let Some(found) = term.search_next(&mut regex, origin, direction, side, None) else {
            drop(term);
            if let Some(state) = self.find_state.as_mut() {
                state.last_match = None;
                state.match_count = 0;
                state.active_match_index = None;
            }
            return false;
        };

        let start = *found.start();
        let end = *found.end();
        let mut selection = Selection::new(SelectionType::Simple, start, Side::Left);
        selection.update(end, Side::Right);
        term.selection = Some(selection);

        let display_offset = term.grid().display_offset() as i32;
        let screen_lines = term.screen_lines() as i32;
        let line = start.line.0;
        let matched = FindMatch { start, end };
        let (match_count, active_match_index) =
            Self::collect_find_match_stats(&term, query, Some(matched));
        drop(term);

        if let Some(state) = self.find_state.as_mut() {
            state.last_match = Some(matched);
            state.match_count = match_count;
            state.active_match_index = active_match_index;
        }
        self.selection_anchor = None;

        let target_offset = if line + display_offset < 0 {
            (-line) as usize
        } else if line + display_offset >= screen_lines {
            (screen_lines - 1 - line).max(0) as usize
        } else {
            display_offset.max(0) as usize
        };
        let _ = self.set_display_offset(target_offset);

        cx.notify();
        true
    }

    fn handle_find_keybinding(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) -> bool {
        if self.find_state.is_none() {
            return false;
        }

        let modifiers = event.keystroke.modifiers;
        let key = event.keystroke.key.as_str();
        if key == "escape" && !modifiers.platform && !modifiers.control && !modifiers.alt {
            self.find_state = None;
            cx.notify();
            return true;
        }

        if key == "enter" && !modifiers.platform && !modifiers.control && !modifiers.alt {
            let direction = if modifiers.shift {
                AlacDirection::Left
            } else {
                AlacDirection::Right
            };
            let _ = self.find_next_match(direction, cx);
            return true;
        }

        if key == "backspace" && !modifiers.platform && !modifiers.control && !modifiers.alt {
            let should_search = if let Some(state) = self.find_state.as_mut() {
                if state.query.pop().is_some() {
                    state.last_match = None;
                    state.active_match_index = None;
                    !state.query.is_empty()
                } else {
                    state.match_count = 0;
                    state.active_match_index = None;
                    false
                }
            } else {
                false
            };

            if should_search {
                if !self.find_next_match(AlacDirection::Right, cx) {
                    cx.notify();
                }
            } else {
                cx.notify();
            }
            return true;
        }

        if let Some(text) = text_to_insert(&event.keystroke) {
            if let Some(state) = self.find_state.as_mut() {
                state.query.push_str(&text);
                state.last_match = None;
                state.active_match_index = None;
            }
            if !self.find_next_match(AlacDirection::Right, cx) {
                cx.notify();
            }
            return true;
        }

        false
    }

    fn handle_common_shortcut(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) -> bool {
        let Some(action) = common_shortcut_action(&event.keystroke) else {
            return false;
        };

        match action {
            CommonShortcutAction::CopySelection => {
                let _ = self.copy_selection_to_clipboard(cx);
            }
            CommonShortcutAction::Paste => {
                if let Some(item) = cx.read_from_clipboard() {
                    if let Some(text) = item.text() {
                        self.begin_terminal_input(cx);
                        self.active_terminal().write_str(&text);
                    }
                }
            }
            CommonShortcutAction::SelectAll => {
                if self.select_all_terminal_content() {
                    cx.notify();
                }
            }
            CommonShortcutAction::Find => self.start_find(cx),
        }

        true
    }

    fn handle_pin_keybinding(&mut self, event: &KeyDownEvent) -> bool {
        if !Self::pin_hotkey_matches_keystroke(&self.settings.pin_hotkey, &event.keystroke) {
            return false;
        }

        self.request_toggle_pin();
        true
    }

    fn handle_tab_keybinding(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let modifiers = event.keystroke.modifiers;
        let key = event.keystroke.key.as_str();
        if key == "tab" && modifiers.control {
            let direction = if modifiers.shift { -1 } else { 1 };
            self.set_active_tab_relative(direction, window, cx);
            return true;
        }

        if !modifiers.platform {
            return false;
        }

        match key {
            "t" => {
                self.create_tab(window, cx);
                true
            }
            "w" => {
                self.close_tab(self.active_tab_id, window, cx);
                true
            }
            "[" => {
                self.set_active_tab_relative(-1, window, cx);
                true
            }
            "]" => {
                self.set_active_tab_relative(1, window, cx);
                true
            }
            "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                let index = key.parse::<usize>().ok().and_then(|n| n.checked_sub(1));
                let Some(index) = index else {
                    return false;
                };
                if index < self.tabs.len() {
                    let tab_id = self.tabs[index].id;
                    self.set_active_tab(tab_id, window, cx);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn refresh_row_text_cache(
        &mut self,
        snapshot: &TerminalSnapshot,
        dirty_rows: &[bool],
        text_system: &gpui::WindowTextSystem,
    ) -> RowCacheStats {
        let mut stats = RowCacheStats::default();

        if self.row_text_cache.len() != snapshot.num_lines {
            self.row_text_cache = vec![CachedRow::default(); snapshot.num_lines];
        }

        for (row_idx, row) in snapshot.rows.iter().enumerate() {
            if !row_cache_rebuild_required(
                dirty_rows.get(row_idx).copied().unwrap_or(true),
                &self.row_text_cache[row_idx],
            ) {
                stats.record_hit();
                continue;
            }

            stats.record_miss();
            self.row_text_cache[row_idx] = build_cached_row(
                row,
                &snapshot.colors,
                text_system,
                &self.font,
                self.font_size,
                self.cell_size.width,
            );
        }

        stats
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

fn cursor_should_blink(blinking: Blinking, terminal_blinking: bool) -> bool {
    match blinking {
        Blinking::Off => false,
        Blinking::On => true,
        Blinking::TerminalControlled => terminal_blinking,
    }
}

fn cursor_blink_is_suppressed(suppress_until: Option<Instant>, now: Instant) -> bool {
    matches!(suppress_until, Some(until) if now < until)
}

fn beam_cursor_width(cell_width: Pixels) -> Pixels {
    px((f32::from(cell_width) * 0.14).clamp(1.0, 2.0))
}

fn underline_cursor_height(cell_height: Pixels) -> Pixels {
    px((f32::from(cell_height) * 0.12).clamp(1.0, 2.0))
}

fn hollow_cursor_thickness(cell_size: Size<Pixels>) -> Pixels {
    let max_thickness =
        (f32::from(cell_size.width).min(f32::from(cell_size.height)) / 2.0).max(1.0);
    px((f32::from(cell_size.width) * 0.1).clamp(1.0, max_thickness))
}

impl Focusable for TerminalView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

/// Snapshot of a single cell.
#[derive(Clone, PartialEq, Eq)]
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
    cursor_shape: CursorShape,
    cursor_blinking: bool,
    show_cursor: bool,
    cursor_draw_visible: bool,
    colors: ColorsSnapshot,
}

fn take_snapshot(terminal: &Terminal, theme: TerminalTheme) -> (TerminalSnapshot, SnapshotTiming) {
    let total_start = Instant::now();
    let term = terminal.term.lock();
    let lock_acquired_at = Instant::now();
    let content = term.renderable_content();
    let colors = ColorsSnapshot::from_colors(content.colors, theme);
    let selection_tint = selection_tint_rgb(theme);
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
                    bg = selection_background_color(&bg, &colors, selection_tint);
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
    let cursor_shape = cursor.shape;
    let cursor_blinking = term.cursor_style().blinking;
    let show_cursor = cursor.shape != CursorShape::Hidden && cursor_row.is_some();

    let snapshot = TerminalSnapshot {
        rows,
        num_cols,
        num_lines,
        history_size,
        display_offset,
        cursor_row,
        cursor_col,
        cursor_shape,
        cursor_blinking,
        show_cursor,
        cursor_draw_visible: show_cursor,
        colors,
    };
    let lock_hold = lock_acquired_at.elapsed();
    drop(term);

    (
        snapshot,
        SnapshotTiming {
            total: total_start.elapsed(),
            lock_hold,
        },
    )
}

fn mark_row_dirty(dirty_rows: &mut [bool], row: Option<usize>) {
    if let Some(row_idx) = row.filter(|idx| *idx < dirty_rows.len()) {
        dirty_rows[row_idx] = true;
    }
}

fn dirty_rows_for_snapshot(
    snapshot: &TerminalSnapshot,
    previous: Option<&FrameCache>,
) -> Vec<bool> {
    let mut dirty_rows = vec![true; snapshot.num_lines];
    let Some(previous) = previous else {
        return dirty_rows;
    };

    if previous.num_cols != snapshot.num_cols
        || previous.num_lines != snapshot.num_lines
        || previous.colors != snapshot.colors
    {
        return dirty_rows;
    }

    let display_offset_delta = snapshot.display_offset as isize - previous.display_offset as isize;
    if display_offset_delta.unsigned_abs() >= snapshot.num_lines {
        return dirty_rows;
    }

    dirty_rows.fill(false);

    for (new_row_idx, new_row) in snapshot.rows.iter().enumerate() {
        let old_row_idx = new_row_idx as isize - display_offset_delta;
        if old_row_idx < 0 {
            dirty_rows[new_row_idx] = true;
            continue;
        }

        let old_row_idx = old_row_idx as usize;
        if previous.rows.get(old_row_idx) != Some(new_row) {
            dirty_rows[new_row_idx] = true;
        }
    }

    if previous.show_cursor != snapshot.show_cursor
        || previous.cursor_row != snapshot.cursor_row
        || previous.cursor_col != snapshot.cursor_col
        || previous.cursor_shape != snapshot.cursor_shape
        || previous.cursor_draw_visible != snapshot.cursor_draw_visible
    {
        mark_row_dirty(&mut dirty_rows, previous.cursor_row);
        mark_row_dirty(&mut dirty_rows, snapshot.cursor_row);
    }

    dirty_rows
}

fn shift_row_cache_for_display_offset(
    row_cache: &mut [CachedRow],
    previous: Option<PreviousFrameView>,
    snapshot: &TerminalSnapshot,
) {
    let Some(previous) = previous else {
        return;
    };

    if previous.num_cols != snapshot.num_cols || previous.num_lines != snapshot.num_lines {
        return;
    }

    if row_cache.len() != snapshot.num_lines {
        return;
    }

    let display_offset_delta = snapshot.display_offset as isize - previous.display_offset as isize;
    if display_offset_delta == 0 || display_offset_delta.unsigned_abs() >= snapshot.num_lines {
        return;
    }

    let old = row_cache.to_vec();
    for (new_row_idx, slot) in row_cache.iter_mut().enumerate() {
        let old_row_idx = new_row_idx as isize - display_offset_delta;
        if old_row_idx < 0 || old_row_idx as usize >= old.len() {
            *slot = CachedRow::default();
        } else {
            *slot = old[old_row_idx as usize].clone();
        }
    }
}

impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_theme_palette = theme_palette(self.settings.theme);
        let (mut snapshot, snapshot_timing) =
            take_snapshot(self.active_terminal(), self.settings.theme);
        let now = Instant::now();
        let should_blink = cursor_should_blink(self.settings.blinking, snapshot.cursor_blinking)
            && !self.cursor_blink_suppressed(now);
        snapshot.cursor_draw_visible =
            snapshot.show_cursor && (!should_blink || self.cursor_blink_visible);
        let previous_view = self
            .previous_frame
            .as_ref()
            .map(PreviousFrameView::from_frame);
        let dirty_rows = dirty_rows_for_snapshot(&snapshot, self.previous_frame.as_ref());
        let dirty_row_count = dirty_rows.iter().filter(|is_dirty| **is_dirty).count();
        let total_rows = snapshot.num_lines;
        let text_system = window.text_system().clone();
        shift_row_cache_for_display_offset(&mut self.row_text_cache, previous_view, &snapshot);
        let row_cache_stats = self.refresh_row_text_cache(&snapshot, &dirty_rows, &text_system);
        let row_text_cache = self.row_text_cache.clone();
        self.previous_frame = Some(FrameCache::from_snapshot(&snapshot));

        let active_tab_id = self.active_tab_id;
        let hovered_tab_id = self.hovered_tab_id;
        let tab_count = self.tabs.len();
        let has_multiple_tabs = tab_count > 1;
        let viewport_width = window.viewport_size().width;
        let find_panel_width = Self::find_panel_width_for_viewport(viewport_width);
        let settings_drawer_width = Self::settings_drawer_width_for_viewport(viewport_width);
        let settings_control_height = px(SETTINGS_CONTROL_HEIGHT_PX);
        let ui_accent = tab_brand_purple(1.0);
        let tabs_for_render = self
            .tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| (tab.id, Self::tab_display_title(tab), index + 1 == tab_count))
            .collect::<Vec<_>>();
        let find_panel_state = self.find_state.as_ref().map(|state| {
            let query_display = if state.query.is_empty() {
                "Find".to_string()
            } else {
                state.query.chars().take(64).collect::<String>()
            };
            let count_label = match (state.active_match_index, state.match_count) {
                (Some(active), total) if total > 0 => format!("{active}/{total}"),
                _ => format!("0/{}", state.match_count),
            };
            (
                query_display,
                state.query.is_empty(),
                count_label,
                !state.query.is_empty() && state.match_count > 0,
            )
        });
        let settings_panel_open = self.settings_panel_open;
        let pinned = self.pinned;
        let pin_indicator_symbol = Self::pin_indicator_symbol(pinned);
        let pin_indicator_color = if pinned {
            tab_brand_purple(1.0)
        } else {
            hsla(0.0, 0.0, 1.0, 0.52)
        };
        let active_font_family_display = self.settings.font_family.clone();
        let font_size_display = format!("{:.1}", self.settings.font_size);
        let line_height_mode = Self::line_height_mode(&self.settings.line_height);
        let line_height_display = Self::line_height_display(&self.settings.line_height);
        let theme_display = Self::theme_label(self.settings.theme).to_string();
        let scroll_multiplier_value =
            Self::normalized_scroll_multiplier(self.settings.scroll_multiplier);
        let scroll_multiplier_display = format!("{:.2}", scroll_multiplier_value);
        let global_hotkey_display = self.settings.global_hotkey.clone();
        let recording_global_hotkey = self.recording_global_hotkey;
        let cursor_shape_display = match self.settings.cursor_shape {
            SettingsCursorShape::Block => "block",
            SettingsCursorShape::Underline => "underline",
            SettingsCursorShape::Bar => "bar",
            SettingsCursorShape::Hollow => "hollow",
        };
        let blinking_display = match self.settings.blinking {
            Blinking::Off => "off",
            Blinking::TerminalControlled => "terminal",
            Blinking::On => "on",
        };

        let cell_size = self.cell_size;
        let perf = self.perf.clone();

        let terminal_surface = div()
            .id("terminal-surface")
            .track_focus(&self.focus_handle)
            .flex_1()
            .bg(rgb(active_theme_palette.terminal_bg))
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
                        let term_handle = this.active_terminal().term.clone();
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
                                let file_url = file_path_to_file_url(file_path);
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
                            this.active_terminal().write(bytes);
                        }
                    } else {
                        let (point, side) = grid_point_and_side(
                            event.position,
                            this.terminal_bounds(),
                            display_offset,
                        );
                        this.selection_anchor = Some((point, side));

                        let mut term = this.active_terminal().term.lock();
                        let selection_type = selection_type_for_click_count(event.click_count);
                        term.selection = Some(Selection::new(selection_type, point, side));
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
                            this.active_terminal().write(bytes);
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
                            this.active_terminal().write(bytes);
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
                            this.active_terminal().write(bytes);
                        }
                    } else if this.selection_anchor.is_some() {
                        let (point, side) = grid_point_and_side(
                            event.position,
                            this.terminal_bounds(),
                            display_offset,
                        );
                        let mut term = this.active_terminal().term.lock();
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
                            this.active_terminal().write(bytes);
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
                            this.active_terminal().write(bytes);
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
                        this.active_terminal().write(bytes);
                    }
                } else if event.pressed_button == Some(MouseButton::Left) {
                    let Some((anchor_point, anchor_side)) = this.selection_anchor else {
                        return;
                    };

                    let (point, side) =
                        grid_point_and_side(event.position, this.terminal_bounds(), display_offset);

                    let mut term = this.active_terminal().term.lock();
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
                            this.active_terminal().write(bytes);
                        }
                    }
                } else if alternate_scroll_enabled(
                    mode,
                    this.settings.alternate_scroll,
                    event.modifiers.shift,
                ) {
                    this.active_terminal().write(alt_scroll(delta));
                } else {
                    this.active_terminal()
                        .term
                        .lock()
                        .scroll_display(Scroll::Delta(delta));
                }

                cx.notify();
            }))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if this.handle_global_hotkey_recording(event, cx) {
                    return;
                }

                if this.handle_tab_keybinding(event, window, cx) {
                    return;
                }

                if this.settings_panel_open
                    && Self::should_close_settings_panel_for_keystroke(&event.keystroke)
                {
                    this.settings_panel_open = false;
                    this.recording_global_hotkey = false;
                    cx.notify();
                    return;
                }

                if this.handle_find_keybinding(event, cx) {
                    return;
                }

                if this.handle_common_shortcut(event, cx) {
                    return;
                }

                if this.handle_pin_keybinding(event) {
                    return;
                }

                let mode = {
                    let term = this.active_terminal().term.lock();
                    *term.mode()
                };

                if let Some(esc) = simple_term::mappings::keys::to_esc_str(
                    &event.keystroke,
                    &mode,
                    this.settings.option_as_meta,
                ) {
                    this.begin_terminal_input(cx);
                    this.active_terminal().write(esc.as_bytes().to_vec());
                    return;
                }

                if let Some(text) = text_to_insert(&event.keystroke) {
                    this.begin_terminal_input(cx);
                    this.active_terminal().write(text.as_bytes().to_vec());
                }
            }))
            .child(
                canvas(
                    move |_bounds, _window, _cx| snapshot,
                    move |bounds, snapshot, window, cx| {
                        let paint_start = Instant::now();
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

                        window.with_content_mask(Some(ContentMask { bounds }), |window| {
                            window.paint_quad(fill(
                                content_bounds,
                                rgb(active_theme_palette.terminal_bg),
                            ));
                            for (row_idx, cached_row) in row_text_cache.iter().enumerate() {
                                for span in cached_row.background_spans.iter() {
                                    let span_bounds = Bounds {
                                        origin: point(
                                            bounds.origin.x
                                                + cell_size.width * span.start_col as f32,
                                            bounds.origin.y + cell_size.height * row_idx as f32,
                                        ),
                                        size: size(
                                            cell_size.width * span.len as f32,
                                            cell_size.height,
                                        ),
                                    };
                                    window.paint_quad(fill(span_bounds, span.color));
                                }
                            }

                            for (row_idx, cached_row) in row_text_cache.iter().enumerate() {
                                for run in cached_row.text_runs.iter() {
                                    let origin = point(
                                        bounds.origin.x + cell_size.width * run.start_col as f32,
                                        bounds.origin.y + cell_size.height * row_idx as f32,
                                    );
                                    let _ = run.shaped.paint(origin, cell_size.height, window, cx);
                                }
                            }

                            if snapshot.cursor_draw_visible
                                && snapshot.cursor_col < snapshot.num_cols
                            {
                                if let Some(cursor_row) = snapshot.cursor_row {
                                    let cell_bounds = Bounds {
                                        origin: point(
                                            bounds.origin.x
                                                + cell_size.width * snapshot.cursor_col as f32,
                                            bounds.origin.y + cell_size.height * cursor_row as f32,
                                        ),
                                        size: size(cell_size.width, cell_size.height),
                                    };
                                    match snapshot.cursor_shape {
                                        CursorShape::Beam => {
                                            let width = beam_cursor_width(cell_size.width);
                                            let cursor_bounds = Bounds {
                                                origin: cell_bounds.origin,
                                                size: size(width, cell_size.height),
                                            };
                                            window.paint_quad(fill(
                                                cursor_bounds,
                                                rgb(active_theme_palette.cursor),
                                            ));
                                        }
                                        CursorShape::Underline => {
                                            let height = underline_cursor_height(cell_size.height);
                                            let cursor_bounds = Bounds {
                                                origin: point(
                                                    cell_bounds.origin.x,
                                                    cell_bounds.origin.y + cell_size.height
                                                        - height,
                                                ),
                                                size: size(cell_size.width, height),
                                            };
                                            window.paint_quad(fill(
                                                cursor_bounds,
                                                rgb(active_theme_palette.cursor),
                                            ));
                                        }
                                        CursorShape::HollowBlock => {
                                            let stroke = hollow_cursor_thickness(cell_size);
                                            let color = rgb(active_theme_palette.cursor);
                                            let top = Bounds {
                                                origin: cell_bounds.origin,
                                                size: size(cell_bounds.size.width, stroke),
                                            };
                                            let bottom = Bounds {
                                                origin: point(
                                                    cell_bounds.origin.x,
                                                    cell_bounds.origin.y + cell_bounds.size.height
                                                        - stroke,
                                                ),
                                                size: size(cell_bounds.size.width, stroke),
                                            };
                                            let left = Bounds {
                                                origin: cell_bounds.origin,
                                                size: size(stroke, cell_bounds.size.height),
                                            };
                                            let right = Bounds {
                                                origin: point(
                                                    cell_bounds.origin.x + cell_bounds.size.width
                                                        - stroke,
                                                    cell_bounds.origin.y,
                                                ),
                                                size: size(stroke, cell_bounds.size.height),
                                            };
                                            window.paint_quad(fill(top, color));
                                            window.paint_quad(fill(bottom, color));
                                            window.paint_quad(fill(left, color));
                                            window.paint_quad(fill(right, color));
                                        }
                                        _ => {
                                            window.paint_quad(fill(
                                                cell_bounds,
                                                rgb(active_theme_palette.cursor),
                                            ));
                                        }
                                    }
                                }
                            }

                            if let Some(layout) = &scrollbar {
                                window.paint_quad(fill(layout.track, hsla(0.0, 0.0, 0.0, 0.0)));
                                window.paint_quad(fill(
                                    layout.thumb,
                                    hsla(223.0 / 360.0, 0.14, 0.34, 0.6),
                                ));
                            }
                        });

                        perf.record_frame(
                            snapshot_timing,
                            paint_start.elapsed(),
                            dirty_row_count,
                            total_rows,
                            row_cache_stats,
                        );
                    },
                )
                .size_full(),
            );

        let tab_bar = div()
            .id("tab-bar")
            .h(px(TAB_BAR_HEIGHT_PX))
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .bg(rgb(active_theme_palette.ui_bg))
            .border_b_1()
            .border_color(hsla(0.0, 0.0, 1.0, 0.04))
            .child(
                div()
                    .w(px(TAB_BAR_LEFT_DRAG_WIDTH_PX))
                    .flex_none()
                    .h_full()
                    .window_control_area(WindowControlArea::Drag)
                    .border_r_1()
                    .border_color(hsla(0.0, 0.0, 1.0, 0.04)),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .items_center()
                    .overflow_x_hidden()
                    .child(
                        div()
                            .id("tab-items-scroll")
                            .h_full()
                            .w_full()
                            .flex()
                            .flex_row()
                            .items_end()
                            .justify_start()
                            .gap_2()
                            .px_4()
                            .overflow_x_scroll()
                            .scrollbar_width(px(0.0))
                            .children(tabs_for_render.into_iter().map(
                                |(tab_id, tab_title, is_last)| {
                                    let is_active = tab_id == active_tab_id;
                                    let is_hovered = hovered_tab_id == Some(tab_id);
                                    let show_close_button = is_hovered && tab_count > 1;
                                    div()
                                        .flex_none()
                                        .h_full()
                                        .w(px(TAB_ITEM_WIDTH_PX))
                                        .flex()
                                        .flex_col()
                                        .justify_end()
                                        .child(
                                            div()
                                                .id(("tab-item", tab_id))
                                                .h(px(TAB_ITEM_HEIGHT_PX))
                                                .px_2()
                                                .flex()
                                                .flex_row()
                                                .items_center()
                                                .when(!is_last, |this| {
                                                    this.border_r_1()
                                                        .border_color(hsla(0.0, 0.0, 1.0, 0.04))
                                                })
                                                .cursor_pointer()
                                                .bg(hsla(0.0, 0.0, 1.0, 0.0))
                                                .hover(|style| style.bg(tab_brand_purple(0.22)))
                                                .on_hover(cx.listener(
                                                    move |this,
                                                          is_hovered_event: &bool,
                                                          _window,
                                                          cx| {
                                                        let next = Self::hovered_tab_id_after_event(
                                                            this.hovered_tab_id,
                                                            tab_id,
                                                            *is_hovered_event,
                                                        );
                                                        if next != this.hovered_tab_id {
                                                            this.hovered_tab_id = next;
                                                            cx.notify();
                                                        }
                                                    },
                                                ))
                                                .on_mouse_down(
                                                    MouseButton::Left,
                                                    cx.listener(
                                                        move |this,
                                                              _event: &MouseDownEvent,
                                                              window,
                                                              cx| {
                                                            this.set_active_tab(tab_id, window, cx);
                                                        },
                                                    ),
                                                )
                                                .child(
                                                    div()
                                                        .w_full()
                                                        .flex()
                                                        .items_center()
                                                        .gap_2()
                                                        .child(
                                                            div()
                                                                .flex_1()
                                                                .truncate()
                                                                .id(("tab-title", tab_id))
                                                                .tooltip({
                                                                    let tab_title = tab_title.clone();
                                                                    move |_window, cx| {
                                                                        cx.new({
                                                                            let tab_title = tab_title.clone();
                                                                            move |_cx| TabTitleTooltip {
                                                                                title: tab_title.clone().into(),
                                                                            }
                                                                        })
                                                                        .into()
                                                                    }
                                                                })
                                                                .text_xs()
                                                                .text_color(if is_active {
                                                                    hsla(0.0, 0.0, 1.0, 0.9)
                                                                } else {
                                                                    hsla(0.0, 0.0, 1.0, 0.5)
                                                                })
                                                                .child(tab_title),
                                                        )
                                                        .when(show_close_button, |this| {
                                                            this.child(
                                                                div()
                                                                    .id(("tab-close", tab_id))
                                                                    .h(px(TAB_CLOSE_BUTTON_SIZE_PX))
                                                                    .w(px(TAB_CLOSE_BUTTON_SIZE_PX))
                                                                    .flex_none()
                                                                    .flex()
                                                                    .items_center()
                                                                    .justify_center()
                                                                    .rounded_sm()
                                                                    .bg(hsla(0.0, 0.0, 0.0, 0.42))
                                                                    .border_1()
                                                                    .border_color(hsla(0.0, 0.0, 1.0, 0.08))
                                                                    .text_sm()
                                                                    .text_color(hsla(0.0, 0.0, 1.0, 0.84))
                                                                    .cursor_pointer()
                                                                    .hover(|style| {
                                                                        style
                                                                            .bg(tab_brand_purple(1.0))
                                                                            .border_color(tab_brand_purple(0.9))
                                                                            .text_color(hsla(0.0, 0.0, 1.0, 0.94))
                                                                    })
                                                                    .on_mouse_down(
                                                                        MouseButton::Left,
                                                                        cx.listener(
                                                                            move |this,
                                                                                  _event: &MouseDownEvent,
                                                                                  window,
                                                                                  cx| {
                                                                                cx.stop_propagation();
                                                                                this.close_tab(tab_id, window, cx);
                                                                            },
                                                                        ),
                                                                    )
                                                                    .child("✕"),
                                                            )
                                                        }),
                                                ),
                                        )
                                        .child(
                                            div().h(px(TAB_ITEM_INDICATOR_HEIGHT_PX)).w_full().bg(if is_active {
                                                ui_accent
                                            } else {
                                                hsla(0.0, 0.0, 1.0, 0.0)
                                            }),
                                        )
                                        .child(div().h(px(TAB_ITEM_INDICATOR_BOTTOM_GAP_PX)).w_full())
                                },
                            )),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .h_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .px_3()
                    .border_l_1()
                    .border_color(hsla(0.0, 0.0, 1.0, 0.04))
                    .when(find_panel_state.is_some(), |this| {
                        let (query_display, is_placeholder, count_label, has_match) =
                            find_panel_state
                                .clone()
                                .unwrap_or_else(|| (String::new(), true, "0/0".to_string(), false));
                        let hint = if is_placeholder {
                            "Type to search"
                        } else {
                            "Enter next  Shift+Enter prev  Esc close"
                        };
                        this.child(
                            div()
                                .h(px(TAB_ITEM_HEIGHT_PX))
                                .w(find_panel_width)
                                .max_w(find_panel_width)
                                .gap_2()
                                .flex()
                                .items_center()
                                .child(
                                    div()
                                        .h(px(TAB_ITEM_HEIGHT_PX))
                                        .flex_1()
                                        .px_3()
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .justify_between()
                                        .rounded_lg()
                                        .border_1()
                                        .border_color(hsla(0.0, 0.0, 1.0, 0.14))
                                        .bg(hsla(0.0, 0.0, 0.0, 0.75))
                                        .child(
                                            div()
                                                .flex_1()
                                                .truncate()
                                                .text_sm()
                                                .text_color(if is_placeholder {
                                                    hsla(0.0, 0.0, 1.0, 0.38)
                                                } else {
                                                    hsla(0.0, 0.0, 1.0, 0.82)
                                                })
                                                .child(query_display),
                                        )
                                        .child(
                                            div()
                                                .flex_none()
                                                .text_xs()
                                                .text_color(hsla(0.0, 0.0, 1.0, 0.54))
                                                .child(hint),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(hsla(0.0, 0.0, 1.0, 0.58))
                                        .child(count_label),
                                )
                                .child(
                                    div()
                                        .h(px(TAB_ITEM_HEIGHT_PX))
                                        .w(px(24.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded_sm()
                                        .text_sm()
                                        .text_color(if has_match {
                                            hsla(0.0, 0.0, 1.0, 0.72)
                                        } else {
                                            hsla(0.0, 0.0, 1.0, 0.26)
                                        })
                                        .cursor_pointer()
                                        .hover(|style| style.bg(tab_brand_purple(0.18)))
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                                let _ = this.find_next_match(AlacDirection::Right, cx);
                                            }),
                                        )
                                        .child("↓"),
                                )
                                .child(
                                    div()
                                        .h(px(TAB_ITEM_HEIGHT_PX))
                                        .w(px(24.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded_sm()
                                        .text_sm()
                                        .text_color(if has_match {
                                            hsla(0.0, 0.0, 1.0, 0.72)
                                        } else {
                                            hsla(0.0, 0.0, 1.0, 0.26)
                                        })
                                        .cursor_pointer()
                                        .hover(|style| style.bg(tab_brand_purple(0.18)))
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                                let _ = this.find_next_match(AlacDirection::Left, cx);
                                            }),
                                        )
                                        .child("↑"),
                                )
                                .child(
                                    div()
                                        .h(px(TAB_ITEM_HEIGHT_PX))
                                        .w(px(24.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded_sm()
                                        .text_sm()
                                        .text_color(hsla(0.0, 0.0, 1.0, 0.68))
                                        .cursor_pointer()
                                        .hover(|style| style.bg(tab_brand_purple(0.18)))
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                                this.find_state = None;
                                                cx.notify();
                                            }),
                                        )
                                        .child("✕"),
                                ),
                        )
                    })
                    .when(find_panel_state.is_none(), |this| {
                        this.child(
                        div()
                            .h(px(TAB_ITEM_HEIGHT_PX))
                            .w(px(TAB_ADD_BUTTON_WIDTH_PX))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .bg(hsla(0.0, 0.0, 1.0, 0.0))
                            .border_1()
                            .border_color(hsla(0.0, 0.0, 1.0, 0.12))
                            .text_color(hsla(0.0, 0.0, 1.0, 0.4))
                            .text_lg()
                            .cursor_pointer()
                            .text_center()
                            .hover(|style| {
                                style
                                    .bg(tab_brand_purple(0.18))
                                    .border_color(tab_brand_purple(0.9))
                                    .text_color(hsla(0.0, 0.0, 1.0, 0.92))
                            })
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _event: &MouseDownEvent, window, cx| {
                                    this.create_tab(window, cx);
                                }),
                            )
                            .child("+"),
                    )
                    .child(
                        div()
                            .h(px(TAB_ITEM_HEIGHT_PX))
                            .w(px(TAB_DROPDOWN_BUTTON_WIDTH_PX))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .bg(hsla(0.0, 0.0, 1.0, 0.0))
                            .border_1()
                            .border_color(hsla(0.0, 0.0, 1.0, 0.12))
                            .text_xs()
                            .text_color(if has_multiple_tabs {
                                hsla(0.0, 0.0, 1.0, 0.4)
                            } else {
                                hsla(0.0, 0.0, 1.0, 0.2)
                            })
                            .opacity(if has_multiple_tabs { 1.0 } else { 0.65 })
                            .cursor_default()
                            .text_center()
                            .when(has_multiple_tabs, |this| {
                                this.cursor_pointer().hover(|style| {
                                    style
                                        .bg(tab_brand_purple(0.18))
                                        .border_color(tab_brand_purple(0.9))
                                        .text_color(hsla(0.0, 0.0, 1.0, 0.92))
                                })
                            })
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _event: &MouseDownEvent, window, cx| {
                                    if has_multiple_tabs {
                                        this.set_active_tab_relative(1, window, cx);
                                    }
                                }),
                            )
                            .child("▾"),
                    )
                    })
                    .child(
                        div()
                            .h(px(TAB_ITEM_HEIGHT_PX))
                            .w(px(PIN_INDICATOR_BUTTON_WIDTH_PX))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .bg(hsla(0.0, 0.0, 1.0, 0.0))
                            .border_1()
                            .border_color(if pinned {
                                tab_brand_purple(0.8)
                            } else {
                                hsla(0.0, 0.0, 1.0, 0.12)
                            })
                            .text_sm()
                            .text_color(pin_indicator_color)
                            .cursor_pointer()
                            .text_center()
                            .hover(|style| {
                                style
                                    .bg(tab_brand_purple(0.18))
                                    .border_color(tab_brand_purple(0.9))
                                    .text_color(hsla(0.0, 0.0, 1.0, 0.92))
                            })
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _event: &MouseDownEvent, _window, _cx| {
                                    this.request_toggle_pin();
                                }),
                            )
                            .child(pin_indicator_symbol),
                    )
                    .child(
                        div()
                            .h(px(TAB_ITEM_HEIGHT_PX))
                            .w(px(SETTINGS_BUTTON_WIDTH_PX))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .bg(hsla(0.0, 0.0, 1.0, 0.0))
                            .border_1()
                            .border_color(hsla(0.0, 0.0, 1.0, 0.12))
                            .text_sm()
                            .text_color(hsla(0.0, 0.0, 1.0, 0.5))
                            .cursor_pointer()
                            .text_center()
                            .hover(|style| {
                                style
                                    .bg(tab_brand_purple(0.18))
                                    .border_color(tab_brand_purple(0.9))
                                    .text_color(hsla(0.0, 0.0, 1.0, 0.92))
                            })
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                    this.toggle_settings_panel(cx);
                                }),
                            )
                            .child(if settings_panel_open { "✕" } else { "⚙︎" }),
                    )
            );

        let line_height_custom_enabled = line_height_mode == SettingsLineHeightMode::Custom;
        let line_height_comfortable_active =
            line_height_mode == SettingsLineHeightMode::Comfortable;
        let line_height_standard_active = line_height_mode == SettingsLineHeightMode::Standard;
        let line_height_custom_active = line_height_mode == SettingsLineHeightMode::Custom;
        let line_height_custom_display = if line_height_custom_enabled {
            line_height_display.clone()
        } else {
            "--".to_string()
        };
        let keep_selection_enabled = self.settings.copy_on_select;
        let settings_drawer_scrollbar_metrics = self.settings_drawer_scrollbar_metrics();
        let mut settings_drawer = div()
            .id("settings-popup")
            .w(settings_drawer_width)
            .max_w(settings_drawer_width)
            .h_full()
            .min_h(px(0.0))
            .relative()
            .flex()
            .flex_col()
            .occlude()
            .rounded_lg()
            .bg(hsla(0.0, 0.0, 0.0, 0.82))
            .border_1()
            .border_color(hsla(0.0, 0.0, 1.0, 0.07))
            .child(
                div()
                    .h(px(SETTINGS_DRAWER_HEADER_HEIGHT_PX))
                    .w_full()
                    .px_3()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(hsla(0.0, 0.0, 1.0, 0.07))
                    .child(
                        div()
                            .text_sm()
                            .text_color(hsla(0.0, 0.0, 1.0, 0.86))
                            .child("Terminal Settings"),
                    )
                    .child(
                        div()
                            .h(px(22.0))
                            .w(px(22.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .text_xs()
                            .text_color(hsla(0.0, 0.0, 1.0, 0.72))
                            .cursor_pointer()
                            .hover(|style| style.bg(tab_brand_purple(0.22)))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                    this.settings_panel_open = false;
                                    this.recording_global_hotkey = false;
                                    cx.notify();
                                }),
                            )
                            .child("✕"),
                    ),
            )
            .child(
                div()
                    .id("settings-popup-scroll")
                    .flex_1()
                    .min_h(px(0.0))
                    .p_3()
                    .pr(px(SETTINGS_DRAWER_SCROLL_CONTENT_PADDING_RIGHT_PX))
                    .flex()
                    .flex_col()
                    .gap_4()
                    .overflow_y_scroll()
                    .scrollbar_width(Self::settings_drawer_scrollbar_width())
                    .track_scroll(&self.settings_drawer_scroll_handle)
                    .child(
                        div()
                            .text_xs()
                            .text_color(hsla(0.0, 0.0, 1.0, 0.46))
                            .child("Appearance"),
                    )
                    .child(
                        div()
                            .p_3()
                            .rounded_sm()
                            .border_1()
                            .border_color(hsla(0.0, 0.0, 1.0, 0.12))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(hsla(0.0, 0.0, 1.0, 0.72))
                                    .child("Theme"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .h(settings_control_height)
                                            .w(px(SETTINGS_NUMERIC_BUTTON_WIDTH_PX))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded_sm()
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.78))
                                            .cursor_pointer()
                                            .hover(|style| style.bg(tab_brand_purple(0.22)))
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                                    this.cycle_theme(-1, cx);
                                                }),
                                            )
                                            .child("<"),
                                    )
                                    .child(
                                        div()
                                            .w(px(126.0))
                                            .truncate()
                                            .text_center()
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.84))
                                            .child(theme_display),
                                    )
                                    .child(
                                        div()
                                            .h(settings_control_height)
                                            .w(px(SETTINGS_NUMERIC_BUTTON_WIDTH_PX))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded_sm()
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.78))
                                            .cursor_pointer()
                                            .hover(|style| style.bg(tab_brand_purple(0.22)))
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                                    this.cycle_theme(1, cx);
                                                }),
                                            )
                                            .child(">"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .p_3()
                            .rounded_sm()
                            .border_1()
                            .border_color(hsla(0.0, 0.0, 1.0, 0.12))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(hsla(0.0, 0.0, 1.0, 0.72))
                                    .child("Font Family"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .h(settings_control_height)
                                            .w(px(SETTINGS_NUMERIC_BUTTON_WIDTH_PX))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded_sm()
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.78))
                                            .cursor_pointer()
                                            .hover(|style| style.bg(tab_brand_purple(0.22)))
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _event: &MouseDownEvent, window, cx| {
                                                    this.cycle_font_family(-1, window, cx);
                                                }),
                                            )
                                            .child("<"),
                                    )
                                    .child(
                                        div()
                                            .w(px(126.0))
                                            .truncate()
                                            .text_center()
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.84))
                                            .child(active_font_family_display),
                                    )
                                    .child(
                                        div()
                                            .h(settings_control_height)
                                            .w(px(SETTINGS_NUMERIC_BUTTON_WIDTH_PX))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded_sm()
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.78))
                                            .cursor_pointer()
                                            .hover(|style| style.bg(tab_brand_purple(0.22)))
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _event: &MouseDownEvent, window, cx| {
                                                    this.cycle_font_family(1, window, cx);
                                                }),
                                            )
                                            .child(">"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .p_3()
                            .rounded_sm()
                            .border_1()
                            .border_color(hsla(0.0, 0.0, 1.0, 0.12))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(hsla(0.0, 0.0, 1.0, 0.72))
                                    .child("Font Size"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .h(settings_control_height)
                                            .w(px(SETTINGS_NUMERIC_BUTTON_WIDTH_PX))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded_sm()
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.78))
                                            .cursor_pointer()
                                            .hover(|style| style.bg(tab_brand_purple(0.22)))
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _event: &MouseDownEvent, window, cx| {
                                                    this.adjust_font_size(-SETTINGS_FONT_SIZE_STEP, window, cx);
                                                }),
                                            )
                                            .child("-"),
                                    )
                                    .child(
                                        div()
                                            .w(px(64.0))
                                            .text_center()
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.84))
                                            .child(font_size_display),
                                    )
                                    .child(
                                        div()
                                            .h(settings_control_height)
                                            .w(px(SETTINGS_NUMERIC_BUTTON_WIDTH_PX))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded_sm()
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.78))
                                            .cursor_pointer()
                                            .hover(|style| style.bg(tab_brand_purple(0.22)))
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _event: &MouseDownEvent, window, cx| {
                                                    this.adjust_font_size(SETTINGS_FONT_SIZE_STEP, window, cx);
                                                }),
                                            )
                                            .child("+"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .p_3()
                            .rounded_sm()
                            .border_1()
                            .border_color(hsla(0.0, 0.0, 1.0, 0.12))
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(hsla(0.0, 0.0, 1.0, 0.72))
                                    .child("Line Height"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .px_2()
                                            .h(settings_control_height)
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(hsla(0.0, 0.0, 1.0, 0.18))
                                            .bg(if line_height_comfortable_active {
                                                hsla(0.0, 0.0, 1.0, 0.12)
                                            } else {
                                                hsla(0.0, 0.0, 1.0, 0.0)
                                            })
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.78))
                                            .cursor_pointer()
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _event: &MouseDownEvent, window, cx| {
                                                    this.set_line_height_mode(
                                                        SettingsLineHeightMode::Comfortable,
                                                        window,
                                                        cx,
                                                    );
                                                }),
                                            )
                                            .child("comfortable"),
                                    )
                                    .child(
                                        div()
                                            .px_2()
                                            .h(settings_control_height)
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(hsla(0.0, 0.0, 1.0, 0.18))
                                            .bg(if line_height_standard_active {
                                                hsla(0.0, 0.0, 1.0, 0.12)
                                            } else {
                                                hsla(0.0, 0.0, 1.0, 0.0)
                                            })
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.78))
                                            .cursor_pointer()
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _event: &MouseDownEvent, window, cx| {
                                                    this.set_line_height_mode(
                                                        SettingsLineHeightMode::Standard,
                                                        window,
                                                        cx,
                                                    );
                                                }),
                                            )
                                            .child("standard"),
                                    )
                                    .child(
                                        div()
                                            .px_2()
                                            .h(settings_control_height)
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(hsla(0.0, 0.0, 1.0, 0.18))
                                            .bg(if line_height_custom_active {
                                                hsla(0.0, 0.0, 1.0, 0.12)
                                            } else {
                                                hsla(0.0, 0.0, 1.0, 0.0)
                                            })
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.78))
                                            .cursor_pointer()
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _event: &MouseDownEvent, window, cx| {
                                                    this.set_line_height_mode(
                                                        SettingsLineHeightMode::Custom,
                                                        window,
                                                        cx,
                                                    );
                                                }),
                                            )
                                            .child("custom"),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(if line_height_custom_enabled {
                                                hsla(0.0, 0.0, 1.0, 0.72)
                                            } else {
                                                hsla(0.0, 0.0, 1.0, 0.38)
                                            })
                                            .child("Custom"),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .h(settings_control_height)
                                                    .w(px(SETTINGS_NUMERIC_BUTTON_WIDTH_PX))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .rounded_sm()
                                                    .text_xs()
                                                    .text_color(if line_height_custom_enabled {
                                                        hsla(0.0, 0.0, 1.0, 0.78)
                                                    } else {
                                                        hsla(0.0, 0.0, 1.0, 0.34)
                                                    })
                                                    .cursor_default()
                                                    .when(line_height_custom_enabled, |this| {
                                                        this.cursor_pointer()
                                                            .hover(|style| style.bg(tab_brand_purple(0.22)))
                                                            .on_mouse_down(
                                                                MouseButton::Left,
                                                                cx.listener(
                                                                    |this, _event: &MouseDownEvent, window, cx| {
                                                                        this.adjust_line_height_custom(
                                                                            -SETTINGS_LINE_HEIGHT_STEP,
                                                                            window,
                                                                            cx,
                                                                        );
                                                                    },
                                                                ),
                                                            )
                                                    })
                                                    .child("-"),
                                            )
                                            .child(
                                                div()
                                                    .w(px(48.0))
                                                    .text_center()
                                                    .text_xs()
                                                    .text_color(if line_height_custom_enabled {
                                                        hsla(0.0, 0.0, 1.0, 0.84)
                                                    } else {
                                                        hsla(0.0, 0.0, 1.0, 0.38)
                                                    })
                                                    .child(line_height_custom_display),
                                            )
                                            .child(
                                                div()
                                                    .h(settings_control_height)
                                                    .w(px(SETTINGS_NUMERIC_BUTTON_WIDTH_PX))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .rounded_sm()
                                                    .text_xs()
                                                    .text_color(if line_height_custom_enabled {
                                                        hsla(0.0, 0.0, 1.0, 0.78)
                                                    } else {
                                                        hsla(0.0, 0.0, 1.0, 0.34)
                                                    })
                                                    .cursor_default()
                                                    .when(line_height_custom_enabled, |this| {
                                                        this.cursor_pointer()
                                                            .hover(|style| style.bg(tab_brand_purple(0.22)))
                                                            .on_mouse_down(
                                                                MouseButton::Left,
                                                                cx.listener(
                                                                    |this, _event: &MouseDownEvent, window, cx| {
                                                                        this.adjust_line_height_custom(
                                                                            SETTINGS_LINE_HEIGHT_STEP,
                                                                            window,
                                                                            cx,
                                                                        );
                                                                    },
                                                                ),
                                                            )
                                                    })
                                                    .child("+"),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .p_3()
                            .rounded_sm()
                            .border_1()
                            .border_color(hsla(0.0, 0.0, 1.0, 0.12))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(hsla(0.0, 0.0, 1.0, 0.72))
                                    .child("Cursor Shape"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .px_2()
                                            .h(settings_control_height)
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(hsla(0.0, 0.0, 1.0, 0.18))
                                            .bg(if self.settings.cursor_shape == SettingsCursorShape::Block {
                                                hsla(0.0, 0.0, 1.0, 0.12)
                                            } else {
                                                hsla(0.0, 0.0, 1.0, 0.0)
                                            })
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.78))
                                            .cursor_pointer()
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                                    this.set_cursor_shape_setting(SettingsCursorShape::Block, cx);
                                                }),
                                            )
                                            .child("block"),
                                    )
                                    .child(
                                        div()
                                            .px_2()
                                            .h(settings_control_height)
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(hsla(0.0, 0.0, 1.0, 0.18))
                                            .bg(if self.settings.cursor_shape == SettingsCursorShape::Underline {
                                                hsla(0.0, 0.0, 1.0, 0.12)
                                            } else {
                                                hsla(0.0, 0.0, 1.0, 0.0)
                                            })
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.78))
                                            .cursor_pointer()
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                                    this.set_cursor_shape_setting(
                                                        SettingsCursorShape::Underline,
                                                        cx,
                                                    );
                                                }),
                                            )
                                            .child("underline"),
                                    )
                                    .child(
                                        div()
                                            .px_2()
                                            .h(settings_control_height)
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(hsla(0.0, 0.0, 1.0, 0.18))
                                            .bg(if self.settings.cursor_shape == SettingsCursorShape::Bar {
                                                hsla(0.0, 0.0, 1.0, 0.12)
                                            } else {
                                                hsla(0.0, 0.0, 1.0, 0.0)
                                            })
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.78))
                                            .cursor_pointer()
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                                    this.set_cursor_shape_setting(SettingsCursorShape::Bar, cx);
                                                }),
                                            )
                                            .child("bar"),
                                    )
                                    .child(
                                        div()
                                            .px_2()
                                            .h(settings_control_height)
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(hsla(0.0, 0.0, 1.0, 0.18))
                                            .bg(if self.settings.cursor_shape == SettingsCursorShape::Hollow {
                                                hsla(0.0, 0.0, 1.0, 0.12)
                                            } else {
                                                hsla(0.0, 0.0, 1.0, 0.0)
                                            })
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.78))
                                            .cursor_pointer()
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                                    this.set_cursor_shape_setting(SettingsCursorShape::Hollow, cx);
                                                }),
                                            )
                                            .child("hollow"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .p_3()
                            .rounded_sm()
                            .border_1()
                            .border_color(hsla(0.0, 0.0, 1.0, 0.12))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(hsla(0.0, 0.0, 1.0, 0.72))
                                    .child("Blinking"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .px_2()
                                            .h(settings_control_height)
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(hsla(0.0, 0.0, 1.0, 0.18))
                                            .bg(if self.settings.blinking == Blinking::Off {
                                                hsla(0.0, 0.0, 1.0, 0.12)
                                            } else {
                                                hsla(0.0, 0.0, 1.0, 0.0)
                                            })
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.78))
                                            .cursor_pointer()
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                                    this.set_blinking_setting(Blinking::Off, cx);
                                                }),
                                            )
                                            .child("off"),
                                    )
                                    .child(
                                        div()
                                            .px_2()
                                            .h(settings_control_height)
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(hsla(0.0, 0.0, 1.0, 0.18))
                                            .bg(if self.settings.blinking == Blinking::TerminalControlled {
                                                hsla(0.0, 0.0, 1.0, 0.12)
                                            } else {
                                                hsla(0.0, 0.0, 1.0, 0.0)
                                            })
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.78))
                                            .cursor_pointer()
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                                    this.set_blinking_setting(Blinking::TerminalControlled, cx);
                                                }),
                                            )
                                            .child("terminal"),
                                    )
                                    .child(
                                        div()
                                            .px_2()
                                            .h(settings_control_height)
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(hsla(0.0, 0.0, 1.0, 0.18))
                                            .bg(if self.settings.blinking == Blinking::On {
                                                hsla(0.0, 0.0, 1.0, 0.12)
                                            } else {
                                                hsla(0.0, 0.0, 1.0, 0.0)
                                            })
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.78))
                                            .cursor_pointer()
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                                    this.set_blinking_setting(Blinking::On, cx);
                                                }),
                                            )
                                            .child("on"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(hsla(0.0, 0.0, 1.0, 0.46))
                            .child("Behavior"),
                    )
                    .child(
                        div()
                            .p_3()
                            .rounded_sm()
                            .border_1()
                            .border_color(hsla(0.0, 0.0, 1.0, 0.12))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(hsla(0.0, 0.0, 1.0, 0.72))
                                    .child("Copy On Select"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .px_2()
                                            .h(settings_control_height)
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(hsla(0.0, 0.0, 1.0, 0.18))
                                            .bg(if self.settings.copy_on_select {
                                                hsla(0.0, 0.0, 1.0, 0.12)
                                            } else {
                                                hsla(0.0, 0.0, 1.0, 0.0)
                                            })
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.78))
                                            .cursor_pointer()
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                                    this.set_copy_on_select(true, cx);
                                                }),
                                            )
                                            .child("on"),
                                    )
                                    .child(
                                        div()
                                            .px_2()
                                            .h(settings_control_height)
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(hsla(0.0, 0.0, 1.0, 0.18))
                                            .bg(if !self.settings.copy_on_select {
                                                hsla(0.0, 0.0, 1.0, 0.12)
                                            } else {
                                                hsla(0.0, 0.0, 1.0, 0.0)
                                            })
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.78))
                                            .cursor_pointer()
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                                    this.set_copy_on_select(false, cx);
                                                }),
                                            )
                                            .child("off"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .p_3()
                            .rounded_sm()
                            .border_1()
                            .border_color(hsla(0.0, 0.0, 1.0, 0.12))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(hsla(0.0, 0.0, 1.0, 0.72))
                                    .child("Keep Selection On Copy"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .px_2()
                                            .h(settings_control_height)
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(hsla(0.0, 0.0, 1.0, 0.18))
                                            .bg(if self.settings.keep_selection_on_copy {
                                                hsla(0.0, 0.0, 1.0, 0.12)
                                            } else {
                                                hsla(0.0, 0.0, 1.0, 0.0)
                                            })
                                            .text_xs()
                                            .text_color(if keep_selection_enabled {
                                                hsla(0.0, 0.0, 1.0, 0.78)
                                            } else {
                                                hsla(0.0, 0.0, 1.0, 0.36)
                                            })
                                            .cursor_default()
                                            .when(keep_selection_enabled, |this| {
                                                this.cursor_pointer().on_mouse_down(
                                                    MouseButton::Left,
                                                    cx.listener(
                                                        |this, _event: &MouseDownEvent, _window, cx| {
                                                            this.set_keep_selection_on_copy(true, cx);
                                                        },
                                                    ),
                                                )
                                            })
                                            .child("on"),
                                    )
                                    .child(
                                        div()
                                            .px_2()
                                            .h(settings_control_height)
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(hsla(0.0, 0.0, 1.0, 0.18))
                                            .bg(if !self.settings.keep_selection_on_copy {
                                                hsla(0.0, 0.0, 1.0, 0.12)
                                            } else {
                                                hsla(0.0, 0.0, 1.0, 0.0)
                                            })
                                            .text_xs()
                                            .text_color(if keep_selection_enabled {
                                                hsla(0.0, 0.0, 1.0, 0.78)
                                            } else {
                                                hsla(0.0, 0.0, 1.0, 0.36)
                                            })
                                            .cursor_default()
                                            .when(keep_selection_enabled, |this| {
                                                this.cursor_pointer().on_mouse_down(
                                                    MouseButton::Left,
                                                    cx.listener(
                                                        |this, _event: &MouseDownEvent, _window, cx| {
                                                            this.set_keep_selection_on_copy(false, cx);
                                                        },
                                                    ),
                                                )
                                            })
                                            .child("off"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .p_3()
                            .rounded_sm()
                            .border_1()
                            .border_color(hsla(0.0, 0.0, 1.0, 0.12))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(hsla(0.0, 0.0, 1.0, 0.72))
                                    .child("Option As Meta"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .px_2()
                                            .h(settings_control_height)
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(hsla(0.0, 0.0, 1.0, 0.18))
                                            .bg(if self.settings.option_as_meta {
                                                hsla(0.0, 0.0, 1.0, 0.12)
                                            } else {
                                                hsla(0.0, 0.0, 1.0, 0.0)
                                            })
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.78))
                                            .cursor_pointer()
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                                    this.set_option_as_meta(true, cx);
                                                }),
                                            )
                                            .child("on"),
                                    )
                                    .child(
                                        div()
                                            .px_2()
                                            .h(settings_control_height)
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(hsla(0.0, 0.0, 1.0, 0.18))
                                            .bg(if !self.settings.option_as_meta {
                                                hsla(0.0, 0.0, 1.0, 0.12)
                                            } else {
                                                hsla(0.0, 0.0, 1.0, 0.0)
                                            })
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.78))
                                            .cursor_pointer()
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                                    this.set_option_as_meta(false, cx);
                                                }),
                                            )
                                            .child("off"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .p_3()
                            .rounded_sm()
                            .border_1()
                            .border_color(hsla(0.0, 0.0, 1.0, 0.12))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(hsla(0.0, 0.0, 1.0, 0.72))
                                    .child("Scroll Multiplier"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .h(settings_control_height)
                                            .w(px(SETTINGS_NUMERIC_BUTTON_WIDTH_PX))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded_sm()
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.78))
                                            .cursor_pointer()
                                            .hover(|style| style.bg(tab_brand_purple(0.22)))
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                                    this.adjust_scroll_multiplier(
                                                        -SETTINGS_SCROLL_MULTIPLIER_STEP,
                                                        cx,
                                                    );
                                                }),
                                            )
                                            .child("-"),
                                    )
                                    .child(
                                        div()
                                            .w(px(64.0))
                                            .text_center()
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.84))
                                            .child(scroll_multiplier_display),
                                    )
                                    .child(
                                        div()
                                            .h(settings_control_height)
                                            .w(px(SETTINGS_NUMERIC_BUTTON_WIDTH_PX))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded_sm()
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.78))
                                            .cursor_pointer()
                                            .hover(|style| style.bg(tab_brand_purple(0.22)))
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                                    this.adjust_scroll_multiplier(
                                                        SETTINGS_SCROLL_MULTIPLIER_STEP,
                                                        cx,
                                                    );
                                                }),
                                            )
                                            .child("+"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .p_3()
                            .rounded_sm()
                            .border_1()
                            .border_color(hsla(0.0, 0.0, 1.0, 0.12))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(hsla(0.0, 0.0, 1.0, 0.72))
                                    .child("Show/Hide Shortcut"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .w(px(168.0))
                                            .text_center()
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.84))
                                            .child(if recording_global_hotkey {
                                                "Press shortcut...".to_string()
                                            } else {
                                                global_hotkey_display.clone()
                                            }),
                                    )
                                    .child(
                                        div()
                                            .px_2()
                                            .h(settings_control_height)
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded_sm()
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.78))
                                            .cursor_pointer()
                                            .border_1()
                                            .border_color(hsla(0.0, 0.0, 1.0, 0.18))
                                            .bg(if recording_global_hotkey {
                                                hsla(0.0, 0.0, 1.0, 0.16)
                                            } else {
                                                hsla(0.0, 0.0, 1.0, 0.0)
                                            })
                                            .hover(|style| {
                                                style
                                                    .bg(tab_brand_purple(0.22))
                                                    .border_color(tab_brand_purple(0.9))
                                            })
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                                    this.set_global_hotkey_recording(
                                                        !this.recording_global_hotkey,
                                                        cx,
                                                    );
                                                }),
                                            )
                                            .child(if recording_global_hotkey {
                                                "cancel"
                                            } else {
                                                "record"
                                            }),
                                    ),
                            ),
                    )
                    .child(div().text_xs().text_color(hsla(0.0, 0.0, 1.0, 0.46)).child("Advanced"))
                    .child(
                        div()
                            .p_3()
                            .rounded_sm()
                            .border_1()
                            .border_color(hsla(0.0, 0.0, 1.0, 0.12))
                            .text_xs()
                            .text_color(hsla(0.0, 0.0, 1.0, 0.46))
                            .child(format!(
                                "Cursor: {}  •  Blinking: {}",
                                cursor_shape_display, blinking_display
                            ))
                            .child(
                                div()
                                    .mt_2()
                                    .child("Advanced options are available in ~/.simple-term/settings.json"),
                            )
                            .child(
                                div()
                                    .mt_2()
                                    .text_color(hsla(0.0, 0.0, 1.0, 0.56))
                                    .child(
                                        "Includes shell, working directory, env, path regexes, and pin hotkey.",
                                    ),
                            ),
                    ),
            );

        if let Some((thumb_top, thumb_height)) = settings_drawer_scrollbar_metrics {
            settings_drawer = settings_drawer.child(
                div()
                    .absolute()
                    .top(px(SETTINGS_DRAWER_HEADER_HEIGHT_PX))
                    .bottom(px(0.0))
                    .right(px(SETTINGS_DRAWER_SCROLLBAR_TRACK_INSET_PX))
                    .w(px(SETTINGS_DRAWER_SCROLLBAR_TRACK_WIDTH_PX))
                    .rounded_sm()
                    .bg(hsla(0.0, 0.0, 1.0, 0.1))
                    .child(
                        div()
                            .absolute()
                            .left(px(0.0))
                            .right(px(0.0))
                            .top(thumb_top)
                            .h(thumb_height)
                            .rounded_sm()
                            .bg(hsla(0.0, 0.0, 1.0, 0.46)),
                    ),
            );
        }

        let content_row = div()
            .flex_1()
            .w_full()
            .flex()
            .flex_row()
            .px_2()
            .py_1()
            .child(terminal_surface);

        let mut terminal_root = div()
            .id("terminal")
            .track_focus(&self.focus_handle)
            .size_full()
            .relative()
            .bg(rgb(active_theme_palette.terminal_bg))
            .flex()
            .flex_col()
            .child(tab_bar)
            .child(content_row);

        if settings_panel_open {
            terminal_root = terminal_root.child(
                div()
                    .id("settings-popup-overlay")
                    .absolute()
                    .top(px(0.0))
                    .right(px(0.0))
                    .bottom(px(0.0))
                    .left(px(0.0))
                    .occlude()
                    .child(
                        div()
                            .absolute()
                            .top(px(0.0))
                            .right(px(0.0))
                            .bottom(px(0.0))
                            .left(px(0.0))
                            .bg(hsla(0.0, 0.0, 0.0, SETTINGS_OVERLAY_BACKDROP_ALPHA))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                    this.settings_panel_open = false;
                                    this.recording_global_hotkey = false;
                                    cx.notify();
                                }),
                            ),
                    )
                    .child(
                        div()
                            .relative()
                            .size_full()
                            .p_4()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(settings_drawer),
                    ),
            );
        }

        terminal_root
    }
}

/// Snapshot of the terminal color palette.
#[derive(Clone, PartialEq, Eq)]
struct ColorsSnapshot {
    palette: [AlacRgb; 256],
    foreground: AlacRgb,
    background: AlacRgb,
}

impl ColorsSnapshot {
    fn from_colors(colors: &AlacColors, theme: TerminalTheme) -> Self {
        let mut palette = [AlacRgb { r: 0, g: 0, b: 0 }; 256];
        let palette_theme = theme_palette(theme);

        for (i, &(r, g, b)) in palette_theme.ansi_colors.iter().enumerate() {
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
            r: palette_theme.foreground.0,
            g: palette_theme.foreground.1,
            b: palette_theme.foreground.2,
        });
        let background = colors[NamedColor::Background as usize].unwrap_or(AlacRgb {
            r: palette_theme.background.0,
            g: palette_theme.background.1,
            b: palette_theme.background.2,
        });

        ColorsSnapshot {
            palette,
            foreground,
            background,
        }
    }
}

fn resolve_color(color: &AlacColor, colors: &ColorsSnapshot, is_fg: bool) -> Hsla {
    alac_rgb_to_hsla(resolve_alac_rgb(color, colors, is_fg))
}

fn alac_rgb_to_hsla(rgb: AlacRgb) -> Hsla {
    let r = rgb.r as f32 / 255.0;
    let g = rgb.g as f32 / 255.0;
    let b = rgb.b as f32 / 255.0;
    Hsla::from(Rgba { r, g, b, a: 1.0 })
}

fn selection_tint_rgb(theme: TerminalTheme) -> AlacRgb {
    rgb_u32_to_alac_rgb(theme_palette(theme).cursor)
}

fn selection_background_color(
    background: &AlacColor,
    colors: &ColorsSnapshot,
    selection_tint: AlacRgb,
) -> AlacColor {
    let base_bg = resolve_alac_rgb(background, colors, false);
    AlacColor::Spec(blend_rgb(base_bg, selection_tint, SELECTION_TINT_ALPHA))
}

fn resolve_alac_rgb(color: &AlacColor, colors: &ColorsSnapshot, is_fg: bool) -> AlacRgb {
    match color {
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
    }
}

fn rgb_u32_to_alac_rgb(rgb: u32) -> AlacRgb {
    AlacRgb {
        r: ((rgb >> 16) & 0xFF) as u8,
        g: ((rgb >> 8) & 0xFF) as u8,
        b: (rgb & 0xFF) as u8,
    }
}

fn tab_brand_purple(alpha: f32) -> Hsla {
    hsla(272.0 / 360.0, 0.91, 0.65, alpha.clamp(0.0, 1.0))
}

fn blend_rgb(base: AlacRgb, overlay: AlacRgb, overlay_alpha: f32) -> AlacRgb {
    let alpha = overlay_alpha.clamp(0.0, 1.0);
    let mix = |base_channel: u8, overlay_channel: u8| -> u8 {
        let blended = base_channel as f32 * (1.0 - alpha) + overlay_channel as f32 * alpha;
        blended.round().clamp(0.0, 255.0) as u8
    };

    AlacRgb {
        r: mix(base.r, overlay.r),
        g: mix(base.g, overlay.g),
        b: mix(base.b, overlay.b),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PositionedTextRun {
    start_col: usize,
    text: String,
    fg: AlacColor,
    bg: AlacColor,
    bold: bool,
}

#[derive(Clone)]
struct CachedTextRun {
    start_col: usize,
    shaped: gpui::ShapedLine,
}

#[derive(Clone)]
struct CachedBackgroundSpan {
    start_col: usize,
    len: usize,
    color: Hsla,
}

#[derive(Clone)]
struct CachedRow {
    initialized: bool,
    text_runs: Arc<[CachedTextRun]>,
    background_spans: Arc<[CachedBackgroundSpan]>,
}

impl Default for CachedRow {
    fn default() -> Self {
        Self {
            initialized: false,
            text_runs: Arc::from(Vec::<CachedTextRun>::new()),
            background_spans: Arc::from(Vec::<CachedBackgroundSpan>::new()),
        }
    }
}

fn row_cache_rebuild_required(is_dirty: bool, cached_row: &CachedRow) -> bool {
    is_dirty || !cached_row.initialized
}

fn build_cached_row(
    row: &[CellSnapshot],
    colors: &ColorsSnapshot,
    text_system: &gpui::WindowTextSystem,
    font: &Font,
    font_size: Pixels,
    cell_width: Pixels,
) -> CachedRow {
    CachedRow {
        initialized: true,
        text_runs: shape_row_text_runs(row, colors, text_system, font, font_size, cell_width),
        background_spans: build_background_spans(row, colors),
    }
}

fn build_background_spans(
    row: &[CellSnapshot],
    colors: &ColorsSnapshot,
) -> Arc<[CachedBackgroundSpan]> {
    let mut spans = Vec::new();
    let mut current_start: Option<usize> = None;
    let mut current_len = 0usize;
    let mut current_color = gpui::black();

    for (col_idx, cell) in row.iter().enumerate() {
        let bg = resolve_color(&cell.bg, colors, false);
        if bg == gpui::black() {
            if let Some(start_col) = current_start.take() {
                spans.push(CachedBackgroundSpan {
                    start_col,
                    len: current_len,
                    color: current_color,
                });
                current_len = 0;
            }
            continue;
        }

        match current_start {
            Some(_) if bg == current_color => {
                current_len += 1;
            }
            Some(start_col) => {
                spans.push(CachedBackgroundSpan {
                    start_col,
                    len: current_len,
                    color: current_color,
                });
                current_start = Some(col_idx);
                current_len = 1;
                current_color = bg;
            }
            None => {
                current_start = Some(col_idx);
                current_len = 1;
                current_color = bg;
            }
        }
    }

    if let Some(start_col) = current_start {
        spans.push(CachedBackgroundSpan {
            start_col,
            len: current_len,
            color: current_color,
        });
    }

    Arc::from(spans)
}

fn shape_row_text_runs(
    row: &[CellSnapshot],
    colors: &ColorsSnapshot,
    text_system: &gpui::WindowTextSystem,
    font: &Font,
    font_size: Pixels,
    cell_width: Pixels,
) -> Arc<[CachedTextRun]> {
    let mut shaped_runs = Vec::new();

    for positioned_run in build_positioned_text_runs(row) {
        let fg_color = resolve_color(&positioned_run.fg, colors, true);
        let bg_color = resolve_color(&positioned_run.bg, colors, false);
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
            Some(cell_width),
        );

        shaped_runs.push(CachedTextRun {
            start_col: positioned_run.start_col,
            shaped,
        });
    }

    Arc::from(shaped_runs)
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
    use super::utils::{
        common_shortcut_action, display_offset_from_thumb_top,
        resolve_working_directory_with_fallback, scrollbar_thumb_metrics,
        selection_type_for_click_count, CommonShortcutAction, INPUT_SCROLL_SUPPRESSION_WINDOW,
    };
    use super::{
        alternate_scroll_enabled, beam_cursor_width, blend_rgb, build_background_spans,
        build_positioned_text_runs, consume_scroll_lines, cursor_blink_is_suppressed,
        cursor_should_blink, dirty_rows_for_snapshot, display_offset_from_pointer,
        effective_scroll_multiplier, file_path_to_file_url, mouse_mode_enabled_for_scroll,
        point_in_bounds, prepare_for_terminal_input, row_cache_rebuild_required,
        scroll_delta_to_lines, scrollbar_layout, selection_background_color, selection_copy_plan,
        selection_tint_rgb, shift_row_cache_for_display_offset, should_ignore_scroll_event,
        strip_line_column_suffix, tab_brand_purple, text_to_insert, theme_palette,
        underline_cursor_height, update_action_for_terminal_event, viewport_row_for_line,
        CachedRow, CachedTextRun, CellSnapshot, ColorsSnapshot, CursorShape, FrameCache,
        PreviousFrameView, ScrollbarLayout, SettingsLineHeightMode, TerminalSnapshot, TerminalView,
        ViewUpdateAction, FIND_PANEL_MAX_WIDTH_PX, FIND_PANEL_MIN_WIDTH_PX,
        SETTINGS_DRAWER_WIDTH_PX, SETTINGS_OVERLAY_BACKDROP_ALPHA, TAB_BAR_HEIGHT_PX,
        TAB_CLOSE_BUTTON_SIZE_PX, TAB_ITEM_INDICATOR_BOTTOM_GAP_PX, TAB_ITEM_WIDTH_PX,
    };
    use alacritty_terminal::term::cell::Flags;
    use alacritty_terminal::vte::ansi::{Color as AlacColor, NamedColor, Rgb as AlacRgb};
    use gpui::{point, px, size, Bounds, Keystroke, Modifiers, Point, ScrollDelta, TouchPhase};
    use simple_term::terminal::TerminalEvent;
    use simple_term::terminal_settings::{
        Blinking, LineHeight, TerminalSettings, TerminalTheme, WorkingDirectory,
    };
    use simple_term::TermMode;
    use simple_term::{AlternateScroll, SelectionType};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    fn cell(c: char, flags: Flags) -> CellSnapshot {
        CellSnapshot {
            c,
            fg: AlacColor::Named(NamedColor::Foreground),
            bg: AlacColor::Named(NamedColor::Background),
            flags,
        }
    }

    fn test_colors() -> ColorsSnapshot {
        ColorsSnapshot {
            palette: [alacritty_terminal::vte::ansi::Rgb { r: 0, g: 0, b: 0 }; 256],
            foreground: alacritty_terminal::vte::ansi::Rgb {
                r: 0xD3,
                g: 0xD7,
                b: 0xCF,
            },
            background: alacritty_terminal::vte::ansi::Rgb { r: 0, g: 0, b: 0 },
        }
    }

    fn snapshot_from_rows(
        rows: &[&str],
        cursor_row: Option<usize>,
        cursor_col: usize,
        show_cursor: bool,
    ) -> TerminalSnapshot {
        snapshot_from_rows_with_offset(rows, cursor_row, cursor_col, show_cursor, 0)
    }

    fn snapshot_from_rows_with_offset(
        rows: &[&str],
        cursor_row: Option<usize>,
        cursor_col: usize,
        show_cursor: bool,
        display_offset: usize,
    ) -> TerminalSnapshot {
        let num_lines = rows.len();
        let num_cols = rows
            .iter()
            .map(|row| row.chars().count())
            .max()
            .unwrap_or(0);
        let rows = rows
            .iter()
            .map(|line| {
                let mut row: Vec<CellSnapshot> = line
                    .chars()
                    .map(|ch| CellSnapshot {
                        c: ch,
                        fg: AlacColor::Named(NamedColor::Foreground),
                        bg: AlacColor::Named(NamedColor::Background),
                        flags: Flags::empty(),
                    })
                    .collect();
                row.resize(num_cols, cell(' ', Flags::empty()));
                row
            })
            .collect();

        TerminalSnapshot {
            rows,
            num_cols,
            num_lines,
            history_size: 0,
            display_offset,
            cursor_row,
            cursor_col,
            cursor_shape: CursorShape::Block,
            cursor_blinking: false,
            show_cursor,
            cursor_draw_visible: show_cursor,
            colors: test_colors(),
        }
    }

    #[test]
    fn dirty_rows_mark_all_rows_without_previous_frame() {
        let snapshot = snapshot_from_rows(&["abc", "def"], Some(0), 0, true);
        let dirty = dirty_rows_for_snapshot(&snapshot, None);
        assert_eq!(dirty, vec![true, true]);
    }

    #[test]
    fn dirty_rows_mark_only_changed_content_rows() {
        let previous = snapshot_from_rows(&["abc", "def", "ghi"], Some(1), 1, true);
        let current = snapshot_from_rows(&["abc", "dXf", "ghi"], Some(1), 1, true);
        let previous_cache = FrameCache::from_snapshot(&previous);

        let dirty = dirty_rows_for_snapshot(&current, Some(&previous_cache));
        assert_eq!(dirty, vec![false, true, false]);
    }

    #[test]
    fn dirty_rows_mark_cursor_source_and_destination_rows() {
        let previous = snapshot_from_rows(&["abc", "def"], Some(0), 1, true);
        let current = snapshot_from_rows(&["abc", "def"], Some(1), 1, true);
        let previous_cache = FrameCache::from_snapshot(&previous);

        let dirty = dirty_rows_for_snapshot(&current, Some(&previous_cache));
        assert_eq!(dirty, vec![true, true]);
    }

    #[test]
    fn wakeup_event_maps_to_notify_action() {
        assert_eq!(
            update_action_for_terminal_event(TerminalEvent::Wakeup),
            ViewUpdateAction::Notify
        );
    }

    #[test]
    fn title_event_maps_to_title_update_and_notify_action() {
        assert_eq!(
            update_action_for_terminal_event(TerminalEvent::TitleChanged("hello".to_string())),
            ViewUpdateAction::SetTitleAndNotify("hello".to_string())
        );
    }

    #[test]
    fn bell_event_maps_to_ignore_action() {
        assert_eq!(
            update_action_for_terminal_event(TerminalEvent::Bell),
            ViewUpdateAction::Ignore
        );
    }

    #[test]
    fn exit_event_maps_to_exit_action() {
        assert_eq!(
            update_action_for_terminal_event(TerminalEvent::Exit(0)),
            ViewUpdateAction::Exit
        );
    }

    #[test]
    fn sanitize_tab_title_removes_control_characters_and_limits_length() {
        let sanitized = TerminalView::sanitize_tab_title("  \u{0007}hello\tworld\u{001b}[31m  ");
        assert_eq!(sanitized, "hello\tworld[31m");

        let long = "x".repeat(120);
        let truncated = TerminalView::sanitize_tab_title(&long);
        assert_eq!(truncated.len(), 60);
    }

    #[test]
    fn sanitize_tab_title_falls_back_when_empty_after_sanitization() {
        assert_eq!(
            TerminalView::sanitize_tab_title("\u{0000}\u{0001}  "),
            "shell"
        );
    }

    #[test]
    fn next_tab_number_fills_first_available_slot() {
        assert_eq!(TerminalView::next_tab_number_from_numbers(&[]), 1);
        assert_eq!(TerminalView::next_tab_number_from_numbers(&[1, 2, 4]), 3);
        assert_eq!(TerminalView::next_tab_number_from_numbers(&[2, 3]), 1);
    }

    #[test]
    fn next_active_index_after_close_prefers_right_neighbor() {
        assert_eq!(TerminalView::next_active_index_after_close(0, 3), 0);
        assert_eq!(TerminalView::next_active_index_after_close(1, 3), 1);
        assert_eq!(TerminalView::next_active_index_after_close(2, 3), 2);
    }

    #[test]
    fn next_active_index_after_close_clamps_for_rightmost_closure() {
        assert_eq!(TerminalView::next_active_index_after_close(2, 2), 1);
        assert_eq!(TerminalView::next_active_index_after_close(5, 1), 0);
    }

    #[test]
    fn hovered_tab_id_after_event_tracks_enter_and_leave() {
        assert_eq!(
            TerminalView::hovered_tab_id_after_event(None, 3, true),
            Some(3)
        );
        assert_eq!(
            TerminalView::hovered_tab_id_after_event(Some(3), 3, false),
            None
        );
    }

    #[test]
    fn hovered_tab_id_after_event_ignores_leave_for_other_tab() {
        assert_eq!(
            TerminalView::hovered_tab_id_after_event(Some(4), 3, false),
            Some(4)
        );
    }

    #[test]
    fn close_tab_hides_window_when_last_tab_would_be_closed() {
        assert!(TerminalView::should_hide_window_when_closing_tab(0));
        assert!(TerminalView::should_hide_window_when_closing_tab(1));
        assert!(!TerminalView::should_hide_window_when_closing_tab(2));
    }

    #[test]
    fn window_deactivation_hide_scheduling_requires_auto_hide_and_inactive_window() {
        assert!(TerminalView::should_schedule_window_deactivation_hide(
            true, false, true
        ));
        assert!(!TerminalView::should_schedule_window_deactivation_hide(
            true, true, true
        ));
        assert!(!TerminalView::should_schedule_window_deactivation_hide(
            true, false, false
        ));
        assert!(!TerminalView::should_schedule_window_deactivation_hide(
            false, false, true
        ));
    }

    #[test]
    fn window_deactivation_hide_callback_is_deferred() {
        let hide_called = Arc::new(AtomicBool::new(false));
        let hide_called_flag = hide_called.clone();
        let callback: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            hide_called_flag.store(true, Ordering::SeqCst);
        });

        let mut deferred: Option<Arc<dyn Fn() + Send + Sync>> = None;
        TerminalView::schedule_window_deactivation_hide(
            true,
            false,
            true,
            Some(callback),
            |scheduled_callback| {
                deferred = Some(scheduled_callback);
            },
        );

        assert!(deferred.is_some());
        assert!(!hide_called.load(Ordering::SeqCst));

        deferred.expect("callback should be deferred")();
        assert!(hide_called.load(Ordering::SeqCst));
    }

    #[test]
    fn tab_item_vertical_footprint_is_stable_across_tab_positions() {
        let last_tab_height = TerminalView::tab_item_vertical_footprint_px(false);
        let non_last_tab_height = TerminalView::tab_item_vertical_footprint_px(true);
        assert_eq!(last_tab_height, non_last_tab_height);
    }

    #[test]
    fn tab_item_vertical_footprint_fits_tab_bar_height_budget() {
        let tab_height = TerminalView::tab_item_vertical_footprint_px(true);
        assert!(
            tab_height <= TAB_BAR_HEIGHT_PX,
            "tab height {} exceeds bar height {}",
            tab_height,
            TAB_BAR_HEIGHT_PX
        );
    }

    #[test]
    fn tab_spacing_tokens_follow_balanced_compact_spec() {
        assert_eq!(TAB_BAR_HEIGHT_PX, 40.0);
        assert_eq!(TAB_ITEM_WIDTH_PX, 152.0);
        assert_eq!(TAB_ITEM_INDICATOR_BOTTOM_GAP_PX, 2.0);
        assert!(TAB_ITEM_WIDTH_PX > TAB_CLOSE_BUTTON_SIZE_PX);

        let tab_height = TerminalView::tab_item_vertical_footprint_px(true);
        assert!(tab_height <= TAB_BAR_HEIGHT_PX);
    }

    #[test]
    fn font_family_options_include_active_and_unique_entries() {
        let settings = TerminalSettings {
            font_family: "JetBrains Mono".to_string(),
            font_fallbacks: vec![
                "JetBrains Mono".to_string(),
                "Menlo".to_string(),
                "menlo".to_string(),
                "SF Mono".to_string(),
            ],
            ..TerminalSettings::default()
        };

        let options = TerminalView::font_family_options_from_settings(&settings);
        assert_eq!(
            options.iter().take(3).cloned().collect::<Vec<_>>(),
            vec![
                "JetBrains Mono".to_string(),
                "Menlo".to_string(),
                "SF Mono".to_string(),
            ]
        );
        assert_eq!(
            options
                .iter()
                .filter(|family| family.eq_ignore_ascii_case("menlo"))
                .count(),
            1
        );
        assert!(options.iter().any(|family| family == "Fira Code"));
    }

    #[test]
    fn next_font_family_wraps_for_forward_and_backward_navigation() {
        let options = vec![
            "Menlo".to_string(),
            "JetBrains Mono".to_string(),
            "SF Mono".to_string(),
        ];

        assert_eq!(
            TerminalView::next_font_family("SF Mono", &options, 1),
            "Menlo".to_string()
        );
        assert_eq!(
            TerminalView::next_font_family("Menlo", &options, -1),
            "SF Mono".to_string()
        );
    }

    #[test]
    fn next_theme_wraps_for_forward_and_backward_navigation() {
        assert_eq!(
            TerminalView::next_theme(TerminalTheme::SolarizedDark, 1),
            TerminalTheme::AtomOneDark
        );
        assert_eq!(
            TerminalView::next_theme(TerminalTheme::AtomOneDark, -1),
            TerminalTheme::SolarizedDark
        );
    }

    #[test]
    fn atom_one_dark_theme_palette_matches_configured_black_and_white_bias() {
        let palette = theme_palette(TerminalTheme::AtomOneDark);
        assert_eq!(palette.ui_bg, 0x101010);
        assert_eq!(palette.terminal_bg, 0x000000);
        assert_eq!(palette.cursor, 0x528bff);
        assert_eq!(
            palette.ansi_colors,
            [
                (0x3F, 0x44, 0x51),
                (0xE0, 0x55, 0x61),
                (0x8C, 0xC2, 0x65),
                (0xD1, 0x8F, 0x52),
                (0x4A, 0xA5, 0xF0),
                (0xC1, 0x62, 0xDE),
                (0x42, 0xB3, 0xC2),
                (0xD7, 0xDA, 0xE0),
                (0x4F, 0x56, 0x66),
                (0xFF, 0x61, 0x6E),
                (0xA5, 0xE0, 0x75),
                (0xF0, 0xA4, 0x5D),
                (0x4D, 0xC4, 0xFF),
                (0xDE, 0x73, 0xFF),
                (0x4C, 0xD1, 0xE0),
                (0xE6, 0xE6, 0xE6),
            ]
        );
        assert_eq!(palette.foreground, (0xE6, 0xE6, 0xE6));
        assert_eq!(palette.background, (0x00, 0x00, 0x00));
    }

    #[test]
    fn tab_brand_purple_matches_requested_indicator_color() {
        let purple = tab_brand_purple(1.0);
        assert_eq!(purple.h, 272.0 / 360.0);
        assert_eq!(purple.s, 0.91);
        assert_eq!(purple.l, 0.65);
        assert_eq!(purple.a, 1.0);
    }

    #[test]
    fn pin_indicator_symbol_maps_pinned_and_unpinned_states() {
        assert_eq!(TerminalView::pin_indicator_symbol(true), "📌");
        assert_eq!(TerminalView::pin_indicator_symbol(false), "○");
    }

    #[test]
    fn blend_rgb_interpolates_channels() {
        let base = AlacRgb { r: 0, g: 0, b: 0 };
        let overlay = AlacRgb {
            r: 0x52,
            g: 0x8B,
            b: 0xFF,
        };

        let blended = blend_rgb(base, overlay, 0.30);
        assert_eq!(
            blended,
            AlacRgb {
                r: 25,
                g: 42,
                b: 77
            }
        );
    }

    #[test]
    fn selection_background_color_uses_soft_tint_instead_of_foreground_swap() {
        let colors = test_colors();
        let selected = selection_background_color(
            &AlacColor::Named(NamedColor::Background),
            &colors,
            selection_tint_rgb(TerminalTheme::AtomOneDark),
        );

        match selected {
            AlacColor::Spec(rgb) => {
                assert_eq!(
                    rgb,
                    AlacRgb {
                        r: 25,
                        g: 42,
                        b: 77
                    }
                );
                assert_ne!(rgb, colors.foreground);
            }
            _ => panic!("selection background should be a concrete tinted RGB color"),
        }
    }

    #[test]
    fn toggled_settings_panel_open_flips_boolean_state() {
        assert!(TerminalView::toggled_settings_panel_open(false));
        assert!(!TerminalView::toggled_settings_panel_open(true));
    }

    #[test]
    fn global_hotkey_from_keystroke_builds_command_function_shortcut() {
        let keystroke = Keystroke {
            modifiers: Modifiers {
                platform: true,
                ..Modifiers::default()
            },
            key: "f4".to_string(),
            key_char: None,
        };

        assert_eq!(
            TerminalView::global_hotkey_from_keystroke(&keystroke),
            Some("command+F4".to_string())
        );
    }

    #[test]
    fn global_hotkey_from_keystroke_accepts_backquote_toggle_style() {
        let keystroke = Keystroke {
            modifiers: Modifiers {
                platform: true,
                ..Modifiers::default()
            },
            key: "backquote".to_string(),
            key_char: None,
        };

        assert_eq!(
            TerminalView::global_hotkey_from_keystroke(&keystroke),
            Some("command+Backquote".to_string())
        );
    }

    #[test]
    fn global_hotkey_from_keystroke_rejects_shortcuts_without_modifier() {
        let keystroke = Keystroke {
            modifiers: Modifiers::default(),
            key: "f4".to_string(),
            key_char: None,
        };

        assert_eq!(TerminalView::global_hotkey_from_keystroke(&keystroke), None);
    }

    #[test]
    fn global_hotkey_from_keystroke_rejects_modifier_only_input() {
        let keystroke = Keystroke {
            modifiers: Modifiers {
                platform: true,
                ..Modifiers::default()
            },
            key: "shift".to_string(),
            key_char: None,
        };

        assert_eq!(TerminalView::global_hotkey_from_keystroke(&keystroke), None);
    }

    #[test]
    fn pin_hotkey_matches_only_configured_combination() {
        let matched = Keystroke {
            modifiers: Modifiers {
                platform: true,
                ..Modifiers::default()
            },
            key: "backquote".to_string(),
            key_char: None,
        };
        let mismatched = Keystroke {
            modifiers: Modifiers {
                platform: true,
                ..Modifiers::default()
            },
            key: "f4".to_string(),
            key_char: None,
        };

        assert!(TerminalView::pin_hotkey_matches_keystroke(
            "command+Backquote",
            &matched
        ));
        assert!(!TerminalView::pin_hotkey_matches_keystroke(
            "command+Backquote",
            &mismatched
        ));
    }

    #[test]
    fn find_panel_width_scales_with_viewport_space() {
        assert_eq!(
            TerminalView::find_panel_width_for_viewport(px(1400.0)),
            px(FIND_PANEL_MAX_WIDTH_PX)
        );
        assert_eq!(
            TerminalView::find_panel_width_for_viewport(px(900.0)),
            px(580.0)
        );
        assert_eq!(
            TerminalView::find_panel_width_for_viewport(px(320.0)),
            px(FIND_PANEL_MIN_WIDTH_PX)
        );
        assert_eq!(
            TerminalView::find_panel_width_for_viewport(px(180.0)),
            px(156.0)
        );
    }

    #[test]
    fn settings_drawer_width_respects_viewport_margin() {
        assert_eq!(
            TerminalView::settings_drawer_width_for_viewport(px(1000.0)),
            px(SETTINGS_DRAWER_WIDTH_PX)
        );
        assert_eq!(
            TerminalView::settings_drawer_width_for_viewport(px(320.0)),
            px(288.0)
        );
        assert_eq!(
            TerminalView::settings_drawer_width_for_viewport(px(220.0)),
            px(188.0)
        );
    }

    #[test]
    fn theme_label_formats_atom_one_dark_with_spaces() {
        assert_eq!(
            TerminalView::theme_label(TerminalTheme::AtomOneDark),
            "Atom One Dark"
        );
    }

    #[test]
    fn settings_drawer_scrollbar_width_is_non_zero_for_scroll_behavior() {
        assert!(TerminalView::settings_drawer_scrollbar_width() > px(0.0));
    }

    #[test]
    fn settings_overlay_backdrop_alpha_stays_subtle() {
        assert!(SETTINGS_OVERLAY_BACKDROP_ALPHA > 0.0);
        assert!(SETTINGS_OVERLAY_BACKDROP_ALPHA < 0.5);
    }

    #[test]
    fn settings_drawer_scrollbar_thumb_metrics_require_positive_scroll_range() {
        assert_eq!(
            TerminalView::settings_drawer_scrollbar_thumb_metrics(px(320.0), px(0.0), px(0.0)),
            None
        );
        assert_eq!(
            TerminalView::settings_drawer_scrollbar_thumb_metrics(px(0.0), px(400.0), px(0.0)),
            None
        );
    }

    #[test]
    fn settings_drawer_scrollbar_thumb_metrics_map_scroll_offset_to_thumb_position() {
        let (top_at_start, thumb_height) =
            TerminalView::settings_drawer_scrollbar_thumb_metrics(px(240.0), px(480.0), px(0.0))
                .expect("scrollbar should be visible for positive overflow");
        assert_eq!(top_at_start, px(0.0));
        assert!(thumb_height > px(70.0));
        assert!(thumb_height < px(90.0));

        let (top_at_end, _) =
            TerminalView::settings_drawer_scrollbar_thumb_metrics(px(240.0), px(480.0), px(-480.0))
                .expect("scrollbar should remain visible at bottom offset");
        assert!(top_at_end > px(150.0));
        assert!(top_at_end <= px(160.0));
    }

    #[test]
    fn should_close_settings_panel_only_on_plain_escape() {
        let plain_escape = Keystroke::parse("escape").expect("escape keystroke");
        assert!(TerminalView::should_close_settings_panel_for_keystroke(
            &plain_escape
        ));

        let ctrl_escape = Keystroke {
            modifiers: Modifiers {
                control: true,
                ..Modifiers::default()
            },
            key: "escape".to_string(),
            key_char: None,
        };
        assert!(!TerminalView::should_close_settings_panel_for_keystroke(
            &ctrl_escape
        ));

        let plain_character = Keystroke {
            modifiers: Modifiers::default(),
            key: "a".to_string(),
            key_char: Some("a".to_string()),
        };
        assert!(!TerminalView::should_close_settings_panel_for_keystroke(
            &plain_character
        ));
    }

    #[test]
    fn line_height_mode_maps_variants_for_settings_controls() {
        assert_eq!(
            TerminalView::line_height_mode(&LineHeight::Comfortable),
            SettingsLineHeightMode::Comfortable
        );
        assert_eq!(
            TerminalView::line_height_mode(&LineHeight::Standard),
            SettingsLineHeightMode::Standard
        );
        assert_eq!(
            TerminalView::line_height_mode(&LineHeight::Custom { value: 1.75 }),
            SettingsLineHeightMode::Custom
        );
    }

    #[test]
    fn normalized_scroll_multiplier_falls_back_for_invalid_values() {
        assert_eq!(TerminalView::normalized_scroll_multiplier(3.5), 3.5);
        assert_eq!(TerminalView::normalized_scroll_multiplier(0.0), 0.01);
        assert_eq!(TerminalView::normalized_scroll_multiplier(20.0), 10.0);
        assert_eq!(TerminalView::normalized_scroll_multiplier(f32::NAN), 1.0);
        assert_eq!(
            TerminalView::normalized_scroll_multiplier(f32::INFINITY),
            1.0
        );
    }

    #[test]
    fn cursor_should_blink_respects_blinking_mode() {
        assert!(!cursor_should_blink(Blinking::Off, true));
        assert!(cursor_should_blink(Blinking::On, false));
        assert!(cursor_should_blink(Blinking::TerminalControlled, true));
        assert!(!cursor_should_blink(Blinking::TerminalControlled, false));
    }

    #[test]
    fn cursor_blink_is_suppressed_during_recent_input_window() {
        let now = Instant::now();
        assert!(cursor_blink_is_suppressed(
            Some(now + Duration::from_millis(250)),
            now
        ));
        assert!(!cursor_blink_is_suppressed(
            Some(now + Duration::from_millis(250)),
            now + Duration::from_millis(251)
        ));
        assert!(!cursor_blink_is_suppressed(None, now));
    }

    #[test]
    fn beam_cursor_width_is_narrower_than_cell_width() {
        let cell_width = px(10.0);
        let width = beam_cursor_width(cell_width);
        assert!(width < cell_width);
        assert!(width >= px(1.0));
    }

    #[test]
    fn underline_cursor_height_is_narrower_than_cell_height() {
        let cell_height = px(18.0);
        let height = underline_cursor_height(cell_height);
        assert!(height < cell_height);
        assert!(height >= px(1.0));
    }

    #[test]
    fn dirty_rows_mark_row_when_cursor_column_changes() {
        let previous = snapshot_from_rows(&["abc", "def"], Some(1), 0, true);
        let current = snapshot_from_rows(&["abc", "def"], Some(1), 2, true);
        let previous_cache = FrameCache::from_snapshot(&previous);

        let dirty = dirty_rows_for_snapshot(&current, Some(&previous_cache));
        assert_eq!(dirty, vec![false, true]);
    }

    #[test]
    fn dirty_rows_for_offset_shift_marks_only_newly_entered_row() {
        let previous = snapshot_from_rows_with_offset(&["A", "B", "C", "D"], None, 0, false, 0);
        let current = snapshot_from_rows_with_offset(&["X", "A", "B", "C"], None, 0, false, 1);
        let previous_cache = FrameCache::from_snapshot(&previous);

        let dirty = dirty_rows_for_snapshot(&current, Some(&previous_cache));
        assert_eq!(dirty, vec![true, false, false, false]);
    }

    #[test]
    fn dirty_rows_for_large_offset_shift_marks_all_rows() {
        let previous = snapshot_from_rows_with_offset(&["A", "B", "C", "D"], None, 0, false, 0);
        let current = snapshot_from_rows_with_offset(&["W", "X", "Y", "Z"], None, 0, false, 4);
        let previous_cache = FrameCache::from_snapshot(&previous);

        let dirty = dirty_rows_for_snapshot(&current, Some(&previous_cache));
        assert_eq!(dirty, vec![true, true, true, true]);
    }

    #[test]
    fn background_spans_merge_adjacent_cells_with_same_color() {
        let row = vec![
            CellSnapshot {
                c: 'a',
                fg: AlacColor::Named(NamedColor::Foreground),
                bg: AlacColor::Named(NamedColor::Background),
                flags: Flags::empty(),
            },
            CellSnapshot {
                c: 'b',
                fg: AlacColor::Named(NamedColor::Foreground),
                bg: AlacColor::Spec(AlacRgb { r: 200, g: 0, b: 0 }),
                flags: Flags::empty(),
            },
            CellSnapshot {
                c: 'c',
                fg: AlacColor::Named(NamedColor::Foreground),
                bg: AlacColor::Spec(AlacRgb { r: 200, g: 0, b: 0 }),
                flags: Flags::empty(),
            },
            CellSnapshot {
                c: 'd',
                fg: AlacColor::Named(NamedColor::Foreground),
                bg: AlacColor::Named(NamedColor::Background),
                flags: Flags::empty(),
            },
            CellSnapshot {
                c: 'e',
                fg: AlacColor::Named(NamedColor::Foreground),
                bg: AlacColor::Spec(AlacRgb {
                    r: 0,
                    g: 100,
                    b: 200,
                }),
                flags: Flags::empty(),
            },
        ];

        let spans = build_background_spans(&row, &test_colors());
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].start_col, 1);
        assert_eq!(spans[0].len, 2);
        assert_eq!(spans[1].start_col, 4);
        assert_eq!(spans[1].len, 1);
    }

    #[test]
    fn background_spans_ignore_default_background_cells() {
        let row = vec![cell('a', Flags::empty()), cell('b', Flags::empty())];
        let spans = build_background_spans(&row, &test_colors());
        assert!(spans.is_empty());
    }

    #[test]
    fn row_cache_rebuild_required_for_dirty_or_uninitialized_rows() {
        let uninitialized = CachedRow::default();
        assert!(row_cache_rebuild_required(false, &uninitialized));
        assert!(row_cache_rebuild_required(true, &uninitialized));
    }

    #[test]
    fn row_cache_reuse_allowed_for_clean_initialized_rows() {
        let initialized = CachedRow {
            initialized: true,
            ..CachedRow::default()
        };
        assert!(!row_cache_rebuild_required(false, &initialized));
        assert!(row_cache_rebuild_required(true, &initialized));
    }

    #[test]
    fn row_cache_shift_reuses_rows_for_small_display_offset_delta() {
        fn cache_row_with_marker(marker: usize) -> CachedRow {
            CachedRow {
                initialized: true,
                text_runs: Arc::from(vec![CachedTextRun {
                    start_col: marker,
                    shaped: gpui::ShapedLine::default(),
                }]),
                ..CachedRow::default()
            }
        }

        let mut cache = vec![
            cache_row_with_marker(10),
            cache_row_with_marker(20),
            cache_row_with_marker(30),
            cache_row_with_marker(40),
        ];
        let snapshot = snapshot_from_rows_with_offset(&["X", "A", "B", "C"], None, 0, false, 1);

        shift_row_cache_for_display_offset(
            &mut cache,
            Some(PreviousFrameView {
                num_cols: snapshot.num_cols,
                num_lines: snapshot.num_lines,
                display_offset: 0,
            }),
            &snapshot,
        );

        assert!(!cache[0].initialized);
        assert_eq!(cache[1].text_runs[0].start_col, 10);
        assert_eq!(cache[2].text_runs[0].start_col, 20);
        assert_eq!(cache[3].text_runs[0].start_col, 30);
    }

    #[test]
    fn row_cache_shift_clears_rows_when_scrolling_down() {
        fn cache_row_with_marker(marker: usize) -> CachedRow {
            CachedRow {
                initialized: true,
                text_runs: Arc::from(vec![CachedTextRun {
                    start_col: marker,
                    shaped: gpui::ShapedLine::default(),
                }]),
                ..CachedRow::default()
            }
        }

        let mut cache = vec![
            cache_row_with_marker(10),
            cache_row_with_marker(20),
            cache_row_with_marker(30),
            cache_row_with_marker(40),
        ];
        let snapshot = snapshot_from_rows_with_offset(&["B", "C", "D", "E"], None, 0, false, 0);

        shift_row_cache_for_display_offset(
            &mut cache,
            Some(PreviousFrameView {
                num_cols: snapshot.num_cols,
                num_lines: snapshot.num_lines,
                display_offset: 1,
            }),
            &snapshot,
        );

        assert_eq!(cache[0].text_runs[0].start_col, 20);
        assert_eq!(cache[1].text_runs[0].start_col, 30);
        assert_eq!(cache[2].text_runs[0].start_col, 40);
        assert!(!cache[3].initialized);
    }

    #[test]
    #[ignore = "manual perf smoke benchmark"]
    fn paint_path_perf_smoke_benchmark() {
        let rows: Vec<String> = (0..120)
            .map(|i| format!("row-{i:03}-{}", "x".repeat(200)))
            .collect();
        let row_refs: Vec<&str> = rows.iter().map(String::as_str).collect();
        let baseline = snapshot_from_rows_with_offset(&row_refs, Some(10), 3, true, 0);
        let mut next = snapshot_from_rows_with_offset(&row_refs, Some(11), 4, true, 1);
        next.rows[5][7] = cell('Z', Flags::empty());
        let previous_cache = FrameCache::from_snapshot(&baseline);
        let previous_view = PreviousFrameView::from_frame(&previous_cache);
        let mut row_cache = vec![CachedRow::default(); next.num_lines];

        let iterations = 1_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = dirty_rows_for_snapshot(&next, Some(&previous_cache));
            shift_row_cache_for_display_offset(&mut row_cache, Some(previous_view), &next);
        }
        let elapsed = start.elapsed();
        eprintln!(
            "paint_path_perf_smoke_benchmark: iterations={} elapsed_ms={:.2}",
            iterations,
            elapsed.as_secs_f64() * 1000.0
        );

        // Broad guardrail: this should stay comfortably below this threshold on CI/dev hardware.
        assert!(
            elapsed < Duration::from_secs(5),
            "perf smoke benchmark exceeded threshold: {:?}",
            elapsed
        );
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
    fn file_path_to_file_url_encodes_spaces_and_reserved_characters() {
        let url = file_path_to_file_url("/tmp/hello world#frag?.rs");
        assert!(url.starts_with("file://"));
        assert!(url.contains("hello%20world"));
        assert!(url.contains("%23frag%3F.rs"));
    }

    #[test]
    fn file_path_to_file_url_handles_relative_paths() {
        let url = file_path_to_file_url("relative path/with space.txt");
        assert!(url.starts_with("file://"));
        assert!(url.contains("relative%20path"));
        assert!(url.contains("with%20space.txt"));
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
        let configured = PathBuf::from("/tmp/simple-term");
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
    fn selection_type_for_click_count_matches_terminal_conventions() {
        assert_eq!(selection_type_for_click_count(1), SelectionType::Simple);
        assert_eq!(selection_type_for_click_count(2), SelectionType::Semantic);
        assert_eq!(selection_type_for_click_count(3), SelectionType::Lines);
        assert_eq!(selection_type_for_click_count(4), SelectionType::Lines);
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

    #[test]
    fn text_to_insert_returns_key_char_and_single_key_fallback() {
        let with_char = Keystroke {
            modifiers: Modifiers::default(),
            key: "a".to_string(),
            key_char: Some("ä".to_string()),
        };
        assert_eq!(text_to_insert(&with_char).as_deref(), Some("ä"));

        let without_char = Keystroke {
            modifiers: Modifiers::default(),
            key: "x".to_string(),
            key_char: None,
        };
        assert_eq!(text_to_insert(&without_char).as_deref(), Some("x"));
    }

    #[test]
    fn text_to_insert_rejects_control_or_platform_modified_keys() {
        let ctrl = Keystroke::parse("ctrl-a").expect("valid keystroke");
        assert_eq!(text_to_insert(&ctrl), None);

        let platform = Keystroke {
            modifiers: Modifiers {
                platform: true,
                ..Modifiers::default()
            },
            key: "v".to_string(),
            key_char: Some("v".to_string()),
        };
        assert_eq!(text_to_insert(&platform), None);
    }

    #[test]
    fn common_shortcut_action_matches_platform_shortcuts() {
        let platform_copy = Keystroke {
            modifiers: Modifiers {
                platform: true,
                ..Modifiers::default()
            },
            key: "c".to_string(),
            key_char: None,
        };
        assert_eq!(
            common_shortcut_action(&platform_copy),
            Some(CommonShortcutAction::CopySelection)
        );

        let platform_paste = Keystroke {
            modifiers: Modifiers {
                platform: true,
                ..Modifiers::default()
            },
            key: "v".to_string(),
            key_char: None,
        };
        assert_eq!(
            common_shortcut_action(&platform_paste),
            Some(CommonShortcutAction::Paste)
        );

        let platform_select_all = Keystroke {
            modifiers: Modifiers {
                platform: true,
                ..Modifiers::default()
            },
            key: "a".to_string(),
            key_char: None,
        };
        assert_eq!(
            common_shortcut_action(&platform_select_all),
            Some(CommonShortcutAction::SelectAll)
        );

        let platform_find = Keystroke {
            modifiers: Modifiers {
                platform: true,
                ..Modifiers::default()
            },
            key: "f".to_string(),
            key_char: None,
        };
        assert_eq!(
            common_shortcut_action(&platform_find),
            Some(CommonShortcutAction::Find)
        );
    }

    #[test]
    fn common_shortcut_action_matches_ctrl_shift_shortcuts() {
        let ctrl_shift_copy = Keystroke {
            modifiers: Modifiers {
                control: true,
                shift: true,
                ..Modifiers::default()
            },
            key: "c".to_string(),
            key_char: None,
        };
        assert_eq!(
            common_shortcut_action(&ctrl_shift_copy),
            Some(CommonShortcutAction::CopySelection)
        );

        let ctrl_shift_paste = Keystroke {
            modifiers: Modifiers {
                control: true,
                shift: true,
                ..Modifiers::default()
            },
            key: "v".to_string(),
            key_char: None,
        };
        assert_eq!(
            common_shortcut_action(&ctrl_shift_paste),
            Some(CommonShortcutAction::Paste)
        );

        let ctrl_shift_select_all = Keystroke {
            modifiers: Modifiers {
                control: true,
                shift: true,
                ..Modifiers::default()
            },
            key: "a".to_string(),
            key_char: None,
        };
        assert_eq!(
            common_shortcut_action(&ctrl_shift_select_all),
            Some(CommonShortcutAction::SelectAll)
        );

        let ctrl_shift_find = Keystroke {
            modifiers: Modifiers {
                control: true,
                shift: true,
                ..Modifiers::default()
            },
            key: "f".to_string(),
            key_char: None,
        };
        assert_eq!(
            common_shortcut_action(&ctrl_shift_find),
            Some(CommonShortcutAction::Find)
        );
    }

    #[test]
    fn common_shortcut_action_does_not_intercept_terminal_control_keys() {
        let ctrl_c = Keystroke::parse("ctrl-c").expect("valid ctrl-c");
        assert_eq!(common_shortcut_action(&ctrl_c), None);

        let ctrl_v = Keystroke::parse("ctrl-v").expect("valid ctrl-v");
        assert_eq!(common_shortcut_action(&ctrl_v), None);

        let ctrl_f = Keystroke::parse("ctrl-f").expect("valid ctrl-f");
        assert_eq!(common_shortcut_action(&ctrl_f), None);

        let platform_tab = Keystroke {
            modifiers: Modifiers {
                platform: true,
                ..Modifiers::default()
            },
            key: "tab".to_string(),
            key_char: None,
        };
        assert_eq!(common_shortcut_action(&platform_tab), None);
    }

    #[test]
    fn normalize_find_query_uses_first_non_empty_line() {
        assert_eq!(
            TerminalView::normalize_find_query("  hello world  \nsecond"),
            Some("hello world".to_string())
        );
        assert_eq!(TerminalView::normalize_find_query(" \nnext"), None);
    }

    #[test]
    fn regex_escape_literal_escapes_special_characters() {
        let escaped = TerminalView::regex_escape_literal("a+b(c)?[d]{e}|f.^$\\");
        assert_eq!(escaped, r"a\+b\(c\)\?\[d\]\{e\}\|f\.\^\$\\");
    }

    #[test]
    fn selection_copy_plan_ignores_missing_or_empty_selection() {
        assert_eq!(selection_copy_plan(true, true, None), (None, false));
        assert_eq!(
            selection_copy_plan(true, false, Some(String::new())),
            (None, false)
        );
    }

    #[test]
    fn point_in_bounds_is_inclusive_on_edges() {
        let bounds = Bounds {
            origin: point(px(10.0), px(20.0)),
            size: size(px(30.0), px(40.0)),
        };

        assert!(point_in_bounds(&bounds, point(px(10.0), px(20.0))));
        assert!(point_in_bounds(&bounds, point(px(40.0), px(60.0))));
        assert!(!point_in_bounds(&bounds, point(px(9.9), px(20.0))));
        assert!(!point_in_bounds(&bounds, point(px(40.1), px(60.0))));
    }

    #[test]
    fn scrollbar_layout_requires_enough_width_and_history() {
        let tiny = scrollbar_layout(
            Bounds {
                origin: point(px(0.0), px(0.0)),
                size: size(px(8.0), px(100.0)),
            },
            20,
            50,
            10,
        );
        assert!(tiny.is_none());

        let no_history = scrollbar_layout(
            Bounds {
                origin: point(px(0.0), px(0.0)),
                size: size(px(200.0), px(100.0)),
            },
            20,
            0,
            0,
        );
        assert!(no_history.is_none());
    }

    #[test]
    fn display_offset_from_pointer_tracks_thumb_drag_position() {
        let layout = ScrollbarLayout {
            track: Bounds {
                origin: point(px(0.0), px(0.0)),
                size: size(px(10.0), px(100.0)),
            },
            thumb: Bounds {
                origin: point(px(1.0), px(20.0)),
                size: size(px(8.0), px(24.0)),
            },
            max_offset: 80,
        };

        let offset_top = display_offset_from_pointer(px(12.0), &layout, px(12.0));
        let offset_bottom = display_offset_from_pointer(px(88.0), &layout, px(12.0));

        assert_eq!(offset_top, 80);
        assert_eq!(offset_bottom, 0);
    }
}
