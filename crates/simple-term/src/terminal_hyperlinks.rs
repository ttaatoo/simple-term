//! Terminal hyperlinks - URL and path detection

use std::ops::Index;

use alacritty_terminal::{
    event::EventListener,
    grid::Dimensions,
    index::{Boundary, Column, Direction as AlacDirection, Point as AlacPoint},
    term::{
        cell::Flags,
        search::{Match, RegexIter, RegexSearch},
    },
    Term,
};
use log::{info, warn};
use regex::Regex;
use std::{
    iter::{once, once_with},
    ops::Range,
    time::{Duration, Instant},
};
use url::Url;

use crate::PathStyle;

const URL_REGEX: &str = r#"(ipfs:|ipns:|magnet:|mailto:|gemini://|gopher://|https://|http://|news:|file://|git://|ssh:|ftp://)[^\u{0000}-\u{001F}\u{007F}-\u{009F}<>"\s{-}\^⟨⟩`']+"#;
const WIDE_CHAR_SPACERS: Flags =
    Flags::from_bits(Flags::LEADING_WIDE_CHAR_SPACER.bits() | Flags::WIDE_CHAR_SPACER.bits())
        .unwrap();

pub struct RegexSearches {
    url_regex: RegexSearch,
    path_hyperlink_regexes: Vec<Regex>,
    path_hyperlink_timeout: Duration,
}

impl Default for RegexSearches {
    fn default() -> Self {
        Self {
            url_regex: RegexSearch::new(URL_REGEX).unwrap(),
            path_hyperlink_regexes: Vec::default(),
            path_hyperlink_timeout: Duration::default(),
        }
    }
}

impl RegexSearches {
    pub fn new(
        path_hyperlink_regexes: impl IntoIterator<Item: AsRef<str>>,
        path_hyperlink_timeout_ms: u64,
    ) -> Self {
        Self {
            url_regex: RegexSearch::new(URL_REGEX).unwrap(),
            path_hyperlink_regexes: path_hyperlink_regexes
                .into_iter()
                .filter_map(|regex| {
                    Regex::new(regex.as_ref())
                        .inspect_err(|error| {
                            warn!(
                                "Ignoring path hyperlink regex specified in `terminal.path_hyperlink_regexes`:\n\n\t{}\n\nError: {}",
                                regex.as_ref(),
                                error
                            );
                        })
                        .ok()
                })
                .collect(),
            path_hyperlink_timeout: Duration::from_millis(path_hyperlink_timeout_ms),
        }
    }
}

pub fn find_from_grid_point<T: EventListener>(
    term: &Term<T>,
    point: AlacPoint,
    regex_searches: &mut RegexSearches,
    _path_style: PathStyle,
) -> Option<(String, bool, Match)> {
    let grid = term.grid();
    let link = grid.index(point).hyperlink();
    let found_word = if let Some(ref url) = link {
        let mut min_index = point;
        loop {
            let new_min_index = min_index.sub(term, Boundary::Cursor, 1);
            if new_min_index == min_index || grid.index(new_min_index).hyperlink() != link {
                break;
            } else {
                min_index = new_min_index
            }
        }

        let mut max_index = point;
        loop {
            let new_max_index = max_index.add(term, Boundary::Cursor, 1);
            if new_max_index == max_index || grid.index(new_max_index).hyperlink() != link {
                break;
            } else {
                max_index = new_max_index
            }
        }

        let url: String = url.uri().to_owned();
        let url_match = min_index..=max_index;

        Some((url, true, url_match))
    } else {
        let (line_start, line_end) = (term.line_search_left(point), term.line_search_right(point));
        if let Some((url, url_match)) = RegexIter::new(
            line_start,
            line_end,
            AlacDirection::Right,
            term,
            &mut regex_searches.url_regex,
        )
        .find(|rm| rm.contains(&point))
        .map(|url_match| {
            let url = term.bounds_to_string(*url_match.start(), *url_match.end());
            sanitize_url_punctuation(url, url_match, term)
        }) {
            Some((url, true, url_match))
        } else {
            path_match(
                term,
                line_start,
                line_end,
                point,
                &mut regex_searches.path_hyperlink_regexes,
                regex_searches.path_hyperlink_timeout,
            )
            .map(|(path, path_match)| (path, false, path_match))
        }
    };

    found_word.map(|(maybe_url_or_path, is_url, word_match)| {
        if is_url {
            // Treat "file://" IRIs like file paths
            if maybe_url_or_path.starts_with("file://") {
                if let Ok(url) = Url::parse(&maybe_url_or_path) {
                    if let Ok(path) = url.to_file_path() {
                        return (path.to_string_lossy().into_owned(), false, word_match);
                    }
                }
                // Fallback: strip file:// prefix if URL parsing fails
                let path = maybe_url_or_path
                    .strip_prefix("file://")
                    .unwrap_or(&maybe_url_or_path);
                (path.to_string(), false, word_match)
            } else {
                (maybe_url_or_path, true, word_match)
            }
        } else {
            (maybe_url_or_path, false, word_match)
        }
    })
}

fn sanitize_url_punctuation<T: EventListener>(
    url: String,
    url_match: Match,
    term: &Term<T>,
) -> (String, Match) {
    let mut sanitized_url = url;
    let mut chars_trimmed = 0;

    // Count parentheses in the URL
    let (open_parens, mut close_parens) =
        sanitized_url
            .chars()
            .fold((0, 0), |(opens, closes), c| match c {
                '(' => (opens + 1, closes),
                ')' => (opens, closes + 1),
                _ => (opens, closes),
            });

    // Remove trailing characters that shouldn't be at the end of URLs
    while let Some(last_char) = sanitized_url.chars().last() {
        let should_remove = match last_char {
            '.' | ',' | ':' | ';' => true,
            '(' => true,
            ')' if close_parens > open_parens => {
                close_parens -= 1;
                true
            }
            _ => false,
        };

        if should_remove {
            sanitized_url.pop();
            chars_trimmed += 1;
        } else {
            break;
        }
    }

    if chars_trimmed > 0 {
        let new_end = url_match.end().sub(term, Boundary::Grid, chars_trimmed);
        let sanitized_match = Match::new(*url_match.start(), new_end);
        (sanitized_url, sanitized_match)
    } else {
        (sanitized_url, url_match)
    }
}

fn path_match<T>(
    term: &Term<T>,
    line_start: AlacPoint,
    line_end: AlacPoint,
    hovered: AlacPoint,
    path_hyperlink_regexes: &mut Vec<Regex>,
    path_hyperlink_timeout: Duration,
) -> Option<(String, Match)> {
    if path_hyperlink_regexes.is_empty() || path_hyperlink_timeout.as_millis() == 0 {
        return None;
    }

    let search_start_time = Instant::now();

    let timed_out = || {
        let elapsed_time = Instant::now().saturating_duration_since(search_start_time);
        (elapsed_time > path_hyperlink_timeout)
            .then_some((elapsed_time.as_millis(), path_hyperlink_timeout.as_millis()))
    };

    let mut line = String::with_capacity(
        (line_end.line.0 - line_start.line.0 + 1) as usize * term.grid().columns(),
    );
    let first_cell = &term.grid()[line_start];
    let mut prev_len = 0;
    line.push(first_cell.c);
    let mut prev_char_is_space = first_cell.c == ' ';
    let mut hovered_point_byte_offset = None;
    let mut hovered_word_start_offset = None;
    let mut hovered_word_end_offset = None;

    if line_start == hovered {
        hovered_point_byte_offset = Some(0);
        if first_cell.c != ' ' {
            hovered_word_start_offset = Some(0);
        }
    }

    for cell in term.grid().iter_from(line_start) {
        if cell.point > line_end {
            break;
        }

        if !cell.flags.intersects(WIDE_CHAR_SPACERS) {
            prev_len = line.len();
            match cell.c {
                ' ' | '\t' => {
                    if hovered_point_byte_offset.is_some()
                        && !prev_char_is_space
                        && hovered_word_end_offset.is_none()
                    {
                        hovered_word_end_offset = Some(line.len());
                    }
                    line.push(' ');
                    prev_char_is_space = true;
                }
                c => {
                    if hovered_point_byte_offset.is_none() && prev_char_is_space {
                        hovered_word_start_offset = Some(line.len());
                    }
                    line.push(c);
                    prev_char_is_space = false;
                }
            }
        }

        if cell.point == hovered {
            hovered_point_byte_offset = Some(prev_len);
        }
    }
    let line = line.trim_ascii_end();
    let hovered_point_byte_offset = hovered_point_byte_offset?;
    let hovered_word_range = {
        let word_start_offset = hovered_word_start_offset.unwrap_or(0);
        (word_start_offset != 0)
            .then_some(word_start_offset..hovered_word_end_offset.unwrap_or(line.len()))
    };
    if line.len() <= hovered_point_byte_offset {
        return None;
    }

    let found_from_range = |path_range: Range<usize>,
                            link_range: Range<usize>,
                            position: Option<(u32, Option<u32>)>| {
        let advance_point_by_str = |mut point: AlacPoint, s: &str| {
            for _ in s.chars() {
                point = term
                    .expand_wide(point, AlacDirection::Right)
                    .add(term, Boundary::Grid, 1);
            }

            let flags = term.grid().index(point).flags;
            if flags.contains(Flags::LEADING_WIDE_CHAR_SPACER) {
                AlacPoint::new(point.line + 1, Column(0))
            } else if flags.contains(Flags::WIDE_CHAR_SPACER) {
                AlacPoint::new(point.line, point.column - 1)
            } else {
                point
            }
        };

        let link_start = advance_point_by_str(line_start, &line[..link_range.start]);
        let link_end = advance_point_by_str(link_start, &line[link_range]);
        let link_match = link_start
            ..=term
                .expand_wide(link_end, AlacDirection::Left)
                .sub(term, Boundary::Grid, 1);

        (
            {
                let mut path = line[path_range].to_string();
                position.inspect(|(line, column)| {
                    path += &format!(":{line}");
                    column.inspect(|column| path += &format!(":{column}"));
                });
                path
            },
            link_match,
        )
    };

    for regex in path_hyperlink_regexes {
        let mut path_found = false;

        for (line_start_offset, captures) in once(
            regex
                .captures_iter(line)
                .next()
                .map(|captures| (0, captures)),
        )
        .chain(once_with(|| {
            if let Some(hovered_word_range) = &hovered_word_range {
                regex
                    .captures_iter(&line[hovered_word_range.clone()])
                    .next()
                    .map(|captures| (hovered_word_range.start, captures))
            } else {
                None
            }
        }))
        .flatten()
        {
            path_found = true;
            let match_range = captures.get(0).unwrap().range();
            let (mut path_range, line_column) = if let Some(path) = captures.name("path") {
                let parse = |name: &str| {
                    captures
                        .name(name)
                        .and_then(|capture| capture.as_str().parse().ok())
                };

                (
                    path.range(),
                    parse("line").map(|line| (line, parse("column"))),
                )
            } else {
                (match_range.clone(), None)
            };
            let mut link_range = captures
                .name("link")
                .map_or_else(|| match_range.clone(), |link| link.range());

            path_range.start += line_start_offset;
            path_range.end += line_start_offset;
            link_range.start += line_start_offset;
            link_range.end += line_start_offset;

            if !link_range.contains(&hovered_point_byte_offset) {
                continue;
            }
            let found = found_from_range(path_range, link_range, line_column);

            if found.1.contains(&hovered) {
                return Some(found);
            }
        }

        if path_found {
            return None;
        }

        if let Some((timed_out_ms, timeout_ms)) = timed_out() {
            warn!("Timed out processing path hyperlink regexes after {timed_out_ms}ms");
            info!("{timeout_ms}ms time out specified in `terminal.path_hyperlink_timeout_ms`");
            return None;
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{find_from_grid_point, RegexSearches};
    use crate::PathStyle;
    use alacritty_terminal::index::{Column, Line, Point};
    use alacritty_terminal::term::test::mock_term;

    #[test]
    fn trims_trailing_url_punctuation() {
        let term = mock_term("visit https://example.com/path).");
        let mut searches = RegexSearches::default();
        let point = Point::new(Line(0), Column(12));

        let (target, is_url, _) =
            find_from_grid_point(&term, point, &mut searches, PathStyle::Unix).expect("url found");

        assert_eq!(target, "https://example.com/path");
        assert!(is_url);
    }

    #[test]
    fn file_urls_are_treated_as_paths() {
        let term = mock_term("file:///tmp/readme.md");
        let mut searches = RegexSearches::default();
        let point = Point::new(Line(0), Column(7));

        let (target, is_url, _) =
            find_from_grid_point(&term, point, &mut searches, PathStyle::Unix).expect("path found");

        assert_eq!(target, "/tmp/readme.md");
        assert!(!is_url);
    }

    #[test]
    fn path_regex_match_preserves_path_prefix() {
        let term = mock_term("/tmp/main.rs:12:3");
        let mut searches = RegexSearches::new(
            [r"(?P<link>(?P<path>/tmp/[[:alnum:]_./-]+)(:(?P<line>\d+)(:(?P<column>\d+))?)?)"],
            500,
        );
        let point = Point::new(Line(0), Column(2));

        let (target, is_url, _) =
            find_from_grid_point(&term, point, &mut searches, PathStyle::Unix).expect("path found");

        assert_eq!(target, "/tmp/main.rs:12:3");
        assert!(!is_url);
    }

    #[test]
    fn path_regex_match_includes_first_character_hover() {
        let term = mock_term("/tmp/main.rs:12:3");
        let mut searches = RegexSearches::new(
            [r"(?P<link>(?P<path>/tmp/[[:alnum:]_./-]+)(:(?P<line>\d+)(:(?P<column>\d+))?)?)"],
            500,
        );
        let point = Point::new(Line(0), Column(0));

        let (target, is_url, _) =
            find_from_grid_point(&term, point, &mut searches, PathStyle::Unix)
                .expect("path found at first character");

        assert_eq!(target, "/tmp/main.rs:12:3");
        assert!(!is_url);
    }

    #[test]
    fn invalid_path_regexes_are_ignored() {
        let term = mock_term("/tmp/main.rs:12");
        let mut searches = RegexSearches::new(["("], 500);
        let point = Point::new(Line(0), Column(2));

        let found = find_from_grid_point(&term, point, &mut searches, PathStyle::Unix);
        assert!(found.is_none());
    }

    #[test]
    fn path_detection_is_disabled_when_timeout_is_zero() {
        let term = mock_term("/tmp/main.rs:12");
        let mut searches = RegexSearches::new(
            [r"(?P<link>(?P<path>/tmp/[[:alnum:]_./-]+)(:(?P<line>\d+))?)"],
            0,
        );
        let point = Point::new(Line(0), Column(2));

        let found = find_from_grid_point(&term, point, &mut searches, PathStyle::Unix);
        assert!(found.is_none());
    }

    #[test]
    fn path_regex_match_supports_windows_drive_paths() {
        let term = mock_term(r"C:\Users\mt\project\main.rs:12:8");
        let mut searches = RegexSearches::new(
            [r"(?P<link>(?P<path>[A-Za-z]:\\[^\s:]+)(:(?P<line>\d+)(:(?P<column>\d+))?)?)"],
            500,
        );
        let point = Point::new(Line(0), Column(5));

        let (target, is_url, _) =
            find_from_grid_point(&term, point, &mut searches, PathStyle::Windows)
                .expect("windows path should match");

        assert_eq!(target, r"C:\Users\mt\project\main.rs:12:8");
        assert!(!is_url);
    }

    #[test]
    fn path_regex_match_supports_unc_paths() {
        let term = mock_term(r"\\server\share\dir\file.txt:3");
        let mut searches = RegexSearches::new(
            [r"(?P<link>(?P<path>\\\\[^\s:]+)(:(?P<line>\d+)(:(?P<column>\d+))?)?)"],
            500,
        );
        let point = Point::new(Line(0), Column(4));

        let (target, is_url, _) =
            find_from_grid_point(&term, point, &mut searches, PathStyle::Windows)
                .expect("unc path should match");

        assert_eq!(target, r"\\server\share\dir\file.txt:3");
        assert!(!is_url);
    }

    #[test]
    fn path_regex_match_supports_non_ascii_paths() {
        let term = mock_term("/tmp/你好.rs:9:1");
        let mut searches = RegexSearches::new(
            [r"(?P<link>(?P<path>/tmp/\S+)(:(?P<line>\d+)(:(?P<column>\d+))?)?)"],
            500,
        );
        let point = Point::new(Line(0), Column(6));

        let (target, is_url, _) =
            find_from_grid_point(&term, point, &mut searches, PathStyle::Unix)
                .expect("non-ascii path should match");

        assert_eq!(target, "/tmp/你好.rs:9:1");
        assert!(!is_url);
    }

    #[test]
    fn keeps_balanced_parentheses_in_urls() {
        let term = mock_term("visit https://example.com/path(test)");
        let mut searches = RegexSearches::default();
        let point = Point::new(Line(0), Column(12));

        let (target, is_url, _) =
            find_from_grid_point(&term, point, &mut searches, PathStyle::Unix).expect("url found");

        assert_eq!(target, "https://example.com/path(test)");
        assert!(is_url);
    }

    #[test]
    fn file_urls_decode_percent_encoded_paths() {
        let term = mock_term("file:///tmp/hello%20world.rs");
        let mut searches = RegexSearches::default();
        let point = Point::new(Line(0), Column(10));

        let (target, is_url, _) =
            find_from_grid_point(&term, point, &mut searches, PathStyle::Unix).expect("path found");

        assert_eq!(target, "/tmp/hello world.rs");
        assert!(!is_url);
    }
}
