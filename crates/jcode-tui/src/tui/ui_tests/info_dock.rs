use super::*;
use ratatui::backend::TestBackend;
use ratatui::{Terminal, layout::Rect};

/// Model, context, usage, KV cache and git: the facts the info box shows.
fn dock_data() -> info_widget::InfoWidgetData {
    info_widget::InfoWidgetData {
        model: Some("claude-opus-5-5".to_string()),
        provider_name: Some("claude".to_string()),
        auth_method: info_widget::AuthMethod::AnthropicOAuth,
        session_count: Some(0),
        session_name: Some("jaguar".to_string()),
        context_info: Some(crate::prompt::ContextInfo {
            system_prompt_chars: 20_000,
            total_chars: 900_000,
            ..Default::default()
        }),
        observed_context_tokens: Some(236_000),
        context_limit: Some(1_000_000),
        usage_info: Some(info_widget::UsageInfo {
            provider: info_widget::UsageProvider::Anthropic,
            primary_limit_label: Some("5-hour".to_string()),
            five_hour: 0.15,
            five_hour_resets_at: Some("2099-01-01T00:00:00Z".to_string()),
            secondary_limit_label: Some("Weekly".to_string()),
            seven_day: 0.73,
            seven_day_resets_at: Some("2099-01-04T00:00:00Z".to_string()),
            available: true,
            ..Default::default()
        }),
        git_info: Some(info_widget::GitInfo {
            branch: "siavz".to_string(),
            modified: 10,
            ahead: 36,
            behind: 162,
            untracked: 1,
            staged: 0,
            dirty_files: vec!["crates/jcode-tui/src/tui/ui.rs".to_string()],
        }),
        ..Default::default()
    }
}

/// A long, ragged transcript: short and full-width lines mixed, like real chat.
fn ragged_chat() -> Vec<DisplayMessage> {
    (0..60)
        .map(|i| DisplayMessage {
            role: if i % 5 == 0 { "user" } else { "assistant" }.to_string(),
            content: if i % 3 == 0 {
                format!("message {i}: {}", "long line of reply text ".repeat(12))
            } else {
                format!("message {i}: short")
            },
            tool_calls: vec![],
            duration_secs: None,
            title: None,
            tool_data: None,
        })
        .collect()
}

fn render(state: &TestState, width: u16, height: u16) -> Vec<String> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| crate::tui::ui::draw(frame, state))
        .unwrap();
    let buf = terminal.backend().buffer();
    (0..height)
        .map(|y| (0..width).map(|x| buf[(x, y)].symbol()).collect::<String>())
        .collect()
}

/// `(row, column, width)` of the first box drawn in the right third.
fn right_box(lines: &[String]) -> Option<(usize, usize, usize)> {
    for (row, line) in lines.iter().enumerate() {
        let cells: Vec<&str> = line_cells(line);
        let Some(left) = cells.iter().position(|c| *c == "╭") else {
            continue;
        };
        if left < cells.len() / 2 {
            continue;
        }
        let right = cells.iter().rposition(|c| *c == "╮")?;
        return Some((row, left, right - left + 1));
    }
    None
}

/// Split a rendered row into cells (one grapheme per terminal column).
fn line_cells(line: &str) -> Vec<&str> {
    let mut cells = Vec::new();
    let mut rest = line;
    while let Some(c) = rest.chars().next() {
        let len = c.len_utf8();
        cells.push(&rest[..len]);
        rest = &rest[len..];
    }
    cells
}

/// The info box must not move with the chat: the same rows and columns at
/// every scroll position (the reported bug: it rode the transcript and jumped
/// between gaps).
#[test]
fn docked_info_box_does_not_move_while_scrolling() {
    let _lock = viewport_snapshot_test_lock();
    let mut state = TestState {
        display_messages: ragged_chat(),
        info_widget_data: dock_data(),
        ..Default::default()
    };
    let anchor = right_box(&render(&state, 160, 40)).expect("info box drawn at the bottom");
    for top in (0..200).step_by(7) {
        state.scroll_top_line = Some(top);
        let lines = render(&state, 160, 40);
        assert_eq!(
            right_box(&lines),
            Some(anchor),
            "box moved at scroll line {top}:\n{}",
            lines.join("\n")
        );
    }
}

/// Streaming text appends lines under the box; it must stay put.
#[test]
fn docked_info_box_does_not_move_while_streaming() {
    let _lock = viewport_snapshot_test_lock();
    let mut state = TestState {
        display_messages: ragged_chat(),
        info_widget_data: dock_data(),
        status: ProcessingStatus::Streaming,
        ..Default::default()
    };
    let mut seen = None;
    for words in 0..30 {
        state.streaming_text = "streamed words that wrap across lines ".repeat(words);
        let lines = render(&state, 160, 40);
        let now = right_box(&lines).expect("info box drawn while streaming");
        assert_eq!(
            *seen.get_or_insert(now),
            now,
            "box moved after {words} chunks"
        );
    }
}

/// Chat text never runs under the docked box: the transcript wraps to the
/// column left of it.
#[test]
fn transcript_never_runs_under_the_docked_box() {
    let _lock = viewport_snapshot_test_lock();
    let mut state = TestState {
        display_messages: ragged_chat(),
        info_widget_data: dock_data(),
        ..Default::default()
    };
    for top in [None, Some(0), Some(40), Some(90)] {
        state.scroll_top_line = top;
        let lines = render(&state, 160, 40);
        let (_, left, _) = right_box(&lines).expect("info box drawn");
        for (row, line) in lines.iter().enumerate().take(30) {
            let tail: String = line_cells(line)[left..].concat();
            assert!(
                !tail.contains("reply text"),
                "row {row} at {top:?}: chat text inside the dock column: {tail:?}"
            );
        }
    }
}

/// Everything the box shows fits: nothing is cut off at the right edge.
#[test]
fn docked_info_box_shows_every_line_in_full() {
    let data = dock_data();
    let width = info_widget::dock_width(&data, 160).expect("dock fits in 160 columns");
    let natural = info_widget::dock_text_lines(&data, 200);
    let docked = info_widget::dock_text_lines(&data, width);
    assert_eq!(natural, docked, "lines changed when fitted into the dock");
    for line in &docked {
        assert!(
            unicode_width::UnicodeWidthStr::width(line.as_str()) <= usize::from(width - 2),
            "line wider than the dock: {line:?}"
        );
    }
    for needle in ["Opus", "OAuth", "Context", "5-hour", "Weekly", "siavz"] {
        assert!(
            docked.iter().any(|line| line.contains(needle)),
            "{needle} missing: {docked:#?}"
        );
    }
}

/// On a terminal too narrow to spare a column, the dock is not used and the
/// chat keeps its full width.
#[test]
fn narrow_terminal_has_no_dock() {
    assert_eq!(info_widget::dock_width(&dock_data(), 80), None);
    assert!(info_widget::dock_width(&dock_data(), 100).is_some());
}

/// No data, no dock: the chat keeps its full width.
#[test]
fn empty_info_has_no_dock() {
    assert_eq!(
        info_widget::dock_width(&info_widget::InfoWidgetData::default(), 160),
        None
    );
}

#[test]
fn dock_rect_is_right_aligned_and_starts_at_top() {
    let _lock = viewport_snapshot_test_lock();
    let state = TestState {
        display_messages: ragged_chat(),
        info_widget_data: dock_data(),
        ..Default::default()
    };
    let lines = render(&state, 160, 40);
    let (row, left, width) = right_box(&lines).expect("info box drawn");
    assert_eq!(row, 0, "box starts at the top of the chat");
    assert_eq!(left + width, 160, "box is flush with the right edge");
    let _ = Rect::default();
}
