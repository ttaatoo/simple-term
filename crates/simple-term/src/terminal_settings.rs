//! Terminal settings for simple-term
//!
//! This module provides terminal configuration using a JSON config file.

use alacritty_terminal::vte::ansi::{
    CursorShape as AlacCursorShape, CursorStyle as AlacCursorStyle,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Alternate scroll mode
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AlternateScroll {
    /// Alternate scroll enabled (default)
    #[default]
    On,
    /// Alternate scroll disabled
    Off,
}

/// Terminal and chrome theme preset.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TerminalTheme {
    /// Atom One Dark-aligned dark palette.
    #[default]
    #[serde(rename = "atom_one_dark", alias = "microterm")]
    AtomOneDark,
    /// Warm, contrast-friendly dark palette.
    GruvboxDark,
    /// Cool dark palette with vivid accents.
    TokyoNight,
    /// Soft pastel dark palette.
    CatppuccinMocha,
    /// Muted polar dark palette.
    Nord,
    /// Solarized dark palette.
    SolarizedDark,
}

/// Cursor shape
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CursorShape {
    /// Cursor is a block like `█`.
    #[default]
    Block,
    /// Cursor is an underscore like `_`.
    Underline,
    /// Cursor is a vertical bar like `⎸`.
    Bar,
    /// Cursor is a hollow box like `▯`.
    Hollow,
}

impl From<CursorShape> for AlacCursorShape {
    fn from(value: CursorShape) -> Self {
        match value {
            CursorShape::Block => AlacCursorShape::Block,
            CursorShape::Underline => AlacCursorShape::Underline,
            CursorShape::Bar => AlacCursorShape::Beam,
            CursorShape::Hollow => AlacCursorShape::HollowBlock,
        }
    }
}

impl From<CursorShape> for AlacCursorStyle {
    fn from(value: CursorShape) -> Self {
        AlacCursorStyle {
            shape: value.into(),
            blinking: false,
        }
    }
}

/// Blinking behavior
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Blinking {
    /// Blinking off
    Off,
    /// Blinking controlled by terminal
    #[default]
    TerminalControlled,
    /// Blinking on
    On,
}

impl Blinking {
    pub fn uses_terminal_control(self) -> bool {
        matches!(self, Blinking::TerminalControlled)
    }

    pub fn default_enabled(self) -> bool {
        matches!(self, Blinking::On | Blinking::TerminalControlled)
    }
}

/// Line height setting
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum LineHeight {
    /// Comfortable line height (1.618 ratio)
    #[default]
    Comfortable,
    /// Standard line height (1.3 ratio)
    Standard,
    /// Custom line height ratio
    Custom {
        /// Custom ratio value
        value: f32,
    },
}

impl LineHeight {
    pub fn to_ratio(&self) -> f32 {
        match self {
            LineHeight::Comfortable => 1.618,
            LineHeight::Standard => 1.3,
            LineHeight::Custom { value } => *value,
        }
    }
}

/// Working directory strategy
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum WorkingDirectory {
    /// Use current file's directory
    #[default]
    CurrentFileDirectory,
    /// Use current project's directory
    CurrentProjectDirectory,
    /// Use first project's directory
    FirstProjectDirectory,
    /// Always use home directory
    AlwaysHome,
    /// Always use a specific directory
    Always {
        /// The directory path
        directory: PathBuf,
    },
}

/// Shell configuration
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ShellConfig {
    /// Use system default shell
    #[default]
    System,
    /// Use a specific program
    Program {
        /// Path to the shell program
        program: String,
    },
    /// Use a program with arguments
    WithArguments {
        /// Path to the shell program
        program: String,
        /// Arguments to pass to the shell
        args: Vec<String>,
    },
}

impl ShellConfig {
    pub fn to_shell(&self) -> super::Shell {
        match self {
            ShellConfig::System => super::Shell::System,
            ShellConfig::Program { program } => super::Shell::Program(program.clone()),
            ShellConfig::WithArguments { program, args } => super::Shell::WithArguments {
                program: program.clone(),
                args: args.clone(),
                title_override: None,
            },
        }
    }
}

/// Terminal settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerminalSettings {
    /// Shell configuration
    #[serde(default)]
    pub shell: ShellConfig,
    /// Working directory strategy
    #[serde(default)]
    pub working_directory: WorkingDirectory,
    /// Font size in pixels
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    /// Font family name
    #[serde(default = "default_font_family")]
    pub font_family: String,
    /// Font fallback family names (monospace preferred)
    #[serde(default = "default_font_fallbacks")]
    pub font_fallbacks: Vec<String>,
    /// Line height setting
    #[serde(default)]
    pub line_height: LineHeight,
    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Cursor shape
    #[serde(default)]
    pub cursor_shape: CursorShape,
    /// Blinking behavior
    #[serde(default)]
    pub blinking: Blinking,
    /// Alternate scroll mode
    #[serde(default)]
    pub alternate_scroll: AlternateScroll,
    /// Treat option key as meta
    #[serde(default)]
    pub option_as_meta: bool,
    /// Copy on select
    #[serde(default)]
    pub copy_on_select: bool,
    /// Keep selection after copy
    #[serde(default = "default_true")]
    pub keep_selection_on_copy: bool,
    /// Show terminal button in status bar
    #[serde(default = "default_true")]
    pub button: bool,
    /// Terminal and app-chrome theme preset
    #[serde(default)]
    pub theme: TerminalTheme,
    /// Global hotkey string
    #[serde(default = "default_global_hotkey")]
    pub global_hotkey: String,
    /// Pin/unpin hotkey string for always-show mode
    #[serde(default = "default_pin_hotkey")]
    pub pin_hotkey: String,
    /// Auto-hide when clicking outside the terminal window
    #[serde(default = "default_true")]
    pub auto_hide_on_outside_click: bool,
    /// Distance from menubar bottom to terminal panel top (pixels)
    #[serde(default = "default_panel_top_inset")]
    pub panel_top_inset: f32,
    /// Default terminal width
    #[serde(default = "default_width")]
    pub default_width: u32,
    /// Default terminal height
    #[serde(default = "default_height")]
    pub default_height: u32,
    /// Maximum scrollback lines
    #[serde(default = "default_scrollback")]
    pub max_scroll_history_lines: Option<usize>,
    /// Scroll multiplier
    #[serde(default = "default_scroll_multiplier")]
    pub scroll_multiplier: f32,
    /// Minimum contrast ratio
    #[serde(default = "default_minimum_contrast")]
    pub minimum_contrast: f32,
    /// Path hyperlink regex patterns
    #[serde(default)]
    pub path_hyperlink_regexes: Vec<String>,
    /// Path hyperlink timeout in milliseconds
    #[serde(default = "default_hyperlink_timeout")]
    pub path_hyperlink_timeout_ms: u64,
    /// Last known window placement per monitor key (macOS app shell).
    #[serde(default)]
    pub monitor_window_positions: HashMap<String, MonitorWindowPlacement>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct MonitorWindowPlacement {
    pub x: f32,
    pub y: f32,
    #[serde(default)]
    pub width: Option<f32>,
    #[serde(default)]
    pub height: Option<f32>,
}

impl MonitorWindowPlacement {
    pub fn approximately_equals(&self, other: &Self, tolerance: f32) -> bool {
        fn approx(left: f32, right: f32, tolerance: f32) -> bool {
            (left - right).abs() <= tolerance
        }

        fn approx_opt(left: Option<f32>, right: Option<f32>, tolerance: f32) -> bool {
            match (left, right) {
                (Some(left), Some(right)) => approx(left, right, tolerance),
                (None, None) => true,
                _ => false,
            }
        }

        approx(self.x, other.x, tolerance)
            && approx(self.y, other.y, tolerance)
            && approx_opt(self.width, other.width, tolerance)
            && approx_opt(self.height, other.height, tolerance)
    }
}

fn default_font_size() -> f32 {
    14.0
}

const MIN_FONT_SIZE: f32 = 6.0;
const MAX_FONT_SIZE: f32 = 72.0;
const MIN_LINE_HEIGHT_RATIO: f32 = 0.5;
const MAX_LINE_HEIGHT_RATIO: f32 = 3.0;
const MAX_DEFAULT_WIDTH: u32 = 8192;
const MAX_DEFAULT_HEIGHT: u32 = 4320;
const MAX_PANEL_TOP_INSET: f32 = 64.0;

fn default_font_family() -> String {
    if cfg!(target_os = "macos") {
        "Menlo".to_string()
    } else if cfg!(target_os = "windows") {
        "Consolas".to_string()
    } else {
        "DejaVu Sans Mono".to_string()
    }
}

fn default_true() -> bool {
    true
}

fn default_font_fallbacks() -> Vec<String> {
    if cfg!(target_os = "macos") {
        vec![
            "SF Mono".to_string(),
            "Monaco".to_string(),
            "Courier".to_string(),
            "Noto Sans Mono".to_string(),
        ]
    } else if cfg!(target_os = "windows") {
        vec![
            "Cascadia Mono".to_string(),
            "Courier New".to_string(),
            "Lucida Console".to_string(),
            "Noto Sans Mono".to_string(),
        ]
    } else {
        vec![
            "Liberation Mono".to_string(),
            "Noto Sans Mono".to_string(),
            "Ubuntu Mono".to_string(),
            "Monospace".to_string(),
        ]
    }
}

fn default_width() -> u32 {
    640
}

fn default_global_hotkey() -> String {
    "command+F4".to_string()
}

fn default_pin_hotkey() -> String {
    "command+Backquote".to_string()
}

fn default_panel_top_inset() -> f32 {
    8.0
}

fn default_height() -> u32 {
    320
}

fn default_scrollback() -> Option<usize> {
    Some(10_000)
}

fn default_scroll_multiplier() -> f32 {
    3.0
}

fn default_minimum_contrast() -> f32 {
    45.0
}

fn default_hyperlink_timeout() -> u64 {
    500
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            shell: ShellConfig::default(),
            working_directory: WorkingDirectory::default(),
            font_size: default_font_size(),
            font_family: default_font_family(),
            font_fallbacks: default_font_fallbacks(),
            line_height: LineHeight::default(),
            env: HashMap::new(),
            cursor_shape: CursorShape::default(),
            blinking: Blinking::default(),
            alternate_scroll: AlternateScroll::default(),
            option_as_meta: false,
            copy_on_select: false,
            keep_selection_on_copy: true,
            button: true,
            theme: TerminalTheme::default(),
            global_hotkey: default_global_hotkey(),
            pin_hotkey: default_pin_hotkey(),
            auto_hide_on_outside_click: true,
            panel_top_inset: default_panel_top_inset(),
            default_width: default_width(),
            default_height: default_height(),
            max_scroll_history_lines: default_scrollback(),
            scroll_multiplier: default_scroll_multiplier(),
            minimum_contrast: default_minimum_contrast(),
            path_hyperlink_regexes: Vec::new(),
            path_hyperlink_timeout_ms: default_hyperlink_timeout(),
            monitor_window_positions: HashMap::new(),
        }
    }
}

impl TerminalSettings {
    pub fn default_cursor_style(&self) -> AlacCursorStyle {
        AlacCursorStyle {
            shape: self.cursor_shape.into(),
            blinking: self.blinking.default_enabled(),
        }
    }

    fn sanitize(mut self) -> Self {
        if !self.font_size.is_finite() || self.font_size <= 0.0 {
            self.font_size = default_font_size();
        } else {
            self.font_size = self.font_size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
        }

        let line_height = std::mem::take(&mut self.line_height);
        self.line_height = match line_height {
            LineHeight::Custom { value } if !value.is_finite() || value <= 0.0 => {
                LineHeight::default()
            }
            LineHeight::Custom { value } => LineHeight::Custom {
                value: value.clamp(MIN_LINE_HEIGHT_RATIO, MAX_LINE_HEIGHT_RATIO),
            },
            other => other,
        };

        if self.default_width == 0 {
            self.default_width = default_width();
        } else {
            self.default_width = self.default_width.min(MAX_DEFAULT_WIDTH);
        }

        if self.default_height == 0 {
            self.default_height = default_height();
        } else {
            self.default_height = self.default_height.min(MAX_DEFAULT_HEIGHT);
        }

        if self.global_hotkey.trim().is_empty() {
            self.global_hotkey = default_global_hotkey();
        }
        if self.pin_hotkey.trim().is_empty() {
            self.pin_hotkey = default_pin_hotkey();
        }

        if !self.panel_top_inset.is_finite() || self.panel_top_inset < 0.0 {
            self.panel_top_inset = default_panel_top_inset();
        } else {
            self.panel_top_inset = self.panel_top_inset.min(MAX_PANEL_TOP_INSET);
        }

        for placement in self.monitor_window_positions.values_mut() {
            if placement
                .width
                .is_some_and(|width| !width.is_finite() || width <= 0.0)
            {
                placement.width = None;
            }
            if placement
                .height
                .is_some_and(|height| !height.is_finite() || height <= 0.0)
            {
                placement.height = None;
            }
        }
        self.monitor_window_positions.retain(|key, placement| {
            !key.trim().is_empty() && placement.x.is_finite() && placement.y.is_finite()
        });

        self
    }

    /// Load settings from a JSON file
    pub fn load(config_path: &PathBuf) -> Self {
        if config_path.exists() {
            match std::fs::read_to_string(config_path) {
                Ok(contents) => {
                    if let Ok(settings) = serde_json::from_str::<Self>(&contents) {
                        return settings.sanitize();
                    }
                }
                Err(e) => {
                    log::warn!("Failed to read config file: {}", e);
                }
            }
        }

        // Return defaults if file doesn't exist or parsing fails
        Self::default()
    }

    /// Load settings from a JSON file and create a default file when missing.
    pub fn load_or_create(config_path: &PathBuf) -> Self {
        let settings = Self::load(config_path);
        if !config_path.exists() {
            if let Err(err) = settings.save(config_path) {
                log::warn!(
                    "failed to initialize settings file at {}: {err}",
                    config_path.display()
                );
            }
        }
        settings
    }

    /// Save settings to a JSON file.
    pub fn save(&self, config_path: &PathBuf) -> std::io::Result<()> {
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let serialized = serde_json::to_string_pretty(self)
            .map_err(|err| std::io::Error::other(format!("failed to serialize settings: {err}")))?;
        std::fs::write(config_path, serialized)
    }

    /// Get the config directory path
    pub fn config_dir() -> PathBuf {
        directories::BaseDirs::new()
            .map(|dirs| dirs.home_dir().join(".simple-term"))
            .unwrap_or_else(|| PathBuf::from("./.simple-term"))
    }

    /// Get the default config file path
    pub fn config_path() -> PathBuf {
        Self::config_dir().join("settings.json")
    }
}

#[cfg(test)]
mod tests {
    use super::{
        default_font_fallbacks, default_font_family, Blinking, CursorShape, LineHeight,
        MonitorWindowPlacement, ShellConfig, TerminalSettings, TerminalTheme,
    };
    use crate::Shell;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_file(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        path.push(format!("simple-term-{name}-{stamp}.json"));
        path
    }

    #[test]
    fn default_font_family_prefers_platform_monospace() {
        let family = default_font_family();
        #[cfg(target_os = "macos")]
        assert_eq!(family, "Menlo");
        #[cfg(target_os = "windows")]
        assert_eq!(family, "Consolas");
        #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
        assert_eq!(family, "DejaVu Sans Mono");
    }

    #[test]
    fn default_font_fallbacks_are_present() {
        assert!(!default_font_fallbacks().is_empty());
    }

    #[test]
    fn terminal_settings_default_uses_fallback_list() {
        let settings = TerminalSettings::default();
        assert!(!settings.font_fallbacks.is_empty());
        assert_eq!(settings.theme, TerminalTheme::AtomOneDark);
        assert_eq!(settings.global_hotkey, "command+F4");
        assert_eq!(settings.pin_hotkey, "command+Backquote");
    }

    #[test]
    fn default_cursor_style_uses_configured_shape_and_blinking_override() {
        let settings = TerminalSettings {
            cursor_shape: CursorShape::Bar,
            blinking: Blinking::On,
            ..TerminalSettings::default()
        };

        let style = settings.default_cursor_style();
        assert_eq!(style.shape, super::AlacCursorShape::Beam);
        assert!(style.blinking);
    }

    #[test]
    fn terminal_controlled_blinking_defaults_to_blinking_style() {
        let settings = TerminalSettings {
            cursor_shape: CursorShape::Underline,
            blinking: Blinking::TerminalControlled,
            ..TerminalSettings::default()
        };

        let style = settings.default_cursor_style();
        assert_eq!(style.shape, super::AlacCursorShape::Underline);
        assert!(style.blinking);
    }

    #[test]
    fn line_height_ratios_match_presets_and_custom_value() {
        assert_eq!(LineHeight::Comfortable.to_ratio(), 1.618);
        assert_eq!(LineHeight::Standard.to_ratio(), 1.3);
        assert_eq!(LineHeight::Custom { value: 1.9 }.to_ratio(), 1.9);
    }

    #[test]
    fn shell_config_converts_to_runtime_shell_variants() {
        assert!(matches!(ShellConfig::System.to_shell(), Shell::System));
        assert!(matches!(
            ShellConfig::Program {
                program: "/bin/zsh".to_string()
            }
            .to_shell(),
            Shell::Program(program) if program == "/bin/zsh"
        ));
        assert!(matches!(
            ShellConfig::WithArguments {
                program: "/bin/bash".to_string(),
                args: vec!["-l".to_string(), "-i".to_string()],
            }
            .to_shell(),
            Shell::WithArguments { program, args, title_override }
                if program == "/bin/bash"
                    && args == vec!["-l".to_string(), "-i".to_string()]
                    && title_override.is_none()
        ));
    }

    #[test]
    fn load_returns_defaults_when_file_is_missing() {
        let path = unique_temp_file("missing");
        let settings = TerminalSettings::load(&path);
        assert_eq!(settings.font_size, TerminalSettings::default().font_size);
    }

    #[test]
    fn load_uses_valid_json_configuration() {
        let path = unique_temp_file("valid");
        let json = r#"{
            "font_size": 18.5,
            "font_family": "JetBrains Mono",
            "env": {"FOO": "BAR"},
            "blinking": "off",
            "theme": "tokyo_night"
        }"#;
        std::fs::write(&path, json).expect("write test settings");

        let settings = TerminalSettings::load(&path);
        std::fs::remove_file(path).ok();

        assert_eq!(settings.font_size, 18.5);
        assert_eq!(settings.font_family, "JetBrains Mono");
        assert_eq!(settings.env.get("FOO").map(String::as_str), Some("BAR"));
        assert_eq!(settings.blinking, Blinking::Off);
        assert_eq!(settings.theme, TerminalTheme::TokyoNight);
    }

    #[test]
    fn load_falls_back_to_defaults_for_invalid_json() {
        let path = unique_temp_file("invalid");
        std::fs::write(&path, "{ this is not valid json").expect("write invalid settings");

        let settings = TerminalSettings::load(&path);
        std::fs::remove_file(path).ok();

        assert_eq!(settings.font_size, TerminalSettings::default().font_size);
        assert_eq!(
            settings.font_family,
            TerminalSettings::default().font_family
        );
    }

    #[test]
    fn load_sanitizes_non_positive_font_and_line_height_values() {
        let path = unique_temp_file("invalid-metrics");
        let json = r#"{
            "font_size": 0,
            "line_height": {"type": "custom", "value": -1.0}
        }"#;
        std::fs::write(&path, json).expect("write test settings");

        let settings = TerminalSettings::load(&path);
        std::fs::remove_file(path).ok();

        assert_eq!(settings.font_size, 14.0);
        assert_eq!(settings.line_height, LineHeight::Comfortable);
    }

    #[test]
    fn load_sanitizes_zero_default_window_dimensions() {
        let path = unique_temp_file("invalid-window-size");
        let json = r#"{
            "default_width": 0,
            "default_height": 0
        }"#;
        std::fs::write(&path, json).expect("write test settings");

        let settings = TerminalSettings::load(&path);
        std::fs::remove_file(path).ok();

        assert_eq!(settings.default_width, 640);
        assert_eq!(settings.default_height, 320);
    }

    #[test]
    fn load_sanitizes_empty_hotkeys_to_defaults() {
        let path = unique_temp_file("invalid-hotkeys");
        let json = r#"{
            "global_hotkey": " ",
            "pin_hotkey": ""
        }"#;
        std::fs::write(&path, json).expect("write test settings");

        let settings = TerminalSettings::load(&path);
        std::fs::remove_file(path).ok();

        assert_eq!(settings.global_hotkey, "command+F4");
        assert_eq!(settings.pin_hotkey, "command+Backquote");
    }

    #[test]
    fn load_sanitizes_invalid_monitor_window_positions() {
        let path = unique_temp_file("invalid-monitor-window-positions");
        let json = r#"{
            "monitor_window_positions": {
                "": { "x": 1.0, "y": 2.0 },
                "primary": { "x": 44.0, "y": 66.0, "width": 860.0, "height": 480.0 },
                "bad_size": { "x": 9.0, "y": 10.0, "width": -1.0, "height": 0.0 }
            }
        }"#;
        std::fs::write(&path, json).expect("write test settings");

        let settings = TerminalSettings::load(&path);
        std::fs::remove_file(path).ok();

        assert_eq!(settings.monitor_window_positions.len(), 2);
        assert_eq!(
            settings.monitor_window_positions.get("primary"),
            Some(&MonitorWindowPlacement {
                x: 44.0,
                y: 66.0,
                width: Some(860.0),
                height: Some(480.0),
            })
        );
        assert_eq!(
            settings.monitor_window_positions.get("bad_size"),
            Some(&MonitorWindowPlacement {
                x: 9.0,
                y: 10.0,
                width: None,
                height: None,
            })
        );
    }

    #[test]
    fn monitor_window_placement_approximate_equality_respects_tolerance() {
        let base = MonitorWindowPlacement {
            x: 120.0,
            y: 80.0,
            width: Some(640.0),
            height: Some(320.0),
        };
        let near = MonitorWindowPlacement {
            x: 120.3,
            y: 79.8,
            width: Some(640.4),
            height: Some(319.7),
        };
        let far = MonitorWindowPlacement {
            x: 121.0,
            y: 80.0,
            width: Some(640.0),
            height: Some(320.0),
        };

        assert!(base.approximately_equals(&near, 0.5));
        assert!(!base.approximately_equals(&far, 0.5));
    }

    #[test]
    fn load_clamps_extreme_font_line_height_and_window_size_values() {
        let path = unique_temp_file("extreme-values");
        let json = r#"{
            "font_size": 10000.0,
            "line_height": {"type": "custom", "value": 100.0},
            "default_width": 50000,
            "default_height": 50000,
            "panel_top_inset": 1000.0
        }"#;
        std::fs::write(&path, json).expect("write test settings");

        let settings = TerminalSettings::load(&path);
        std::fs::remove_file(path).ok();

        assert_eq!(settings.font_size, 72.0);
        assert_eq!(settings.line_height, LineHeight::Custom { value: 3.0 });
        assert_eq!(settings.default_width, 8192);
        assert_eq!(settings.default_height, 4320);
        assert_eq!(settings.panel_top_inset, 64.0);
    }

    #[test]
    fn config_path_points_to_settings_json_file() {
        let path = TerminalSettings::config_path();
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("settings.json")
        );
    }

    #[test]
    fn config_dir_points_to_simple_term_folder_in_home_directory() {
        let path = TerminalSettings::config_dir();
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some(".simple-term")
        );
    }

    #[test]
    fn load_or_create_creates_settings_file_when_missing() {
        let path = unique_temp_file("load-or-create-missing");
        std::fs::remove_file(&path).ok();

        let settings = TerminalSettings::load_or_create(&path);

        assert!(path.exists(), "settings file should be created");
        assert_eq!(settings.font_size, TerminalSettings::default().font_size);

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn save_writes_json_that_round_trips_with_load() {
        let path = unique_temp_file("save-roundtrip");
        let settings = TerminalSettings {
            font_size: 18.0,
            font_family: "JetBrains Mono".to_string(),
            ..TerminalSettings::default()
        };

        settings.save(&path).expect("save settings");
        let loaded = TerminalSettings::load(&path);
        std::fs::remove_file(path).ok();

        assert_eq!(loaded.font_size, 18.0);
        assert_eq!(loaded.font_family, "JetBrains Mono");
    }

    #[test]
    fn save_creates_missing_parent_directories() {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "simple-term-save-parent-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time should move forward")
                .as_nanos()
        ));
        path.push("nested");
        path.push("settings.json");

        let settings = TerminalSettings::default();
        settings.save(&path).expect("save settings into nested dir");
        assert!(path.exists(), "settings file should be created");

        std::fs::remove_file(&path).ok();
        if let Some(parent) = path.parent() {
            std::fs::remove_dir_all(parent).ok();
        }
    }

    #[test]
    fn saved_settings_do_not_include_dock_mode_field() {
        let path = unique_temp_file("save-no-dock-mode");
        let settings = TerminalSettings::default();
        settings.save(&path).expect("save settings");

        let contents = std::fs::read_to_string(&path).expect("read saved settings");
        std::fs::remove_file(path).ok();

        assert!(
            !contents.contains("\"dock_mode\""),
            "dock_mode should not be serialized in settings.json"
        );
    }
}
