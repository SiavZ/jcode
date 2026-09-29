//! Docked info column: the info box as a fixed panel on the right of the chat.
//!
//! The floating layout (`info_widget_layout`) fits boxes into the gaps between
//! chat lines, so they move whenever the chat scrolls or streams and shrink to
//! whatever gap is free. The dock instead reserves a column: the transcript is
//! wrapped narrower so it never runs under the panel, and the panel is drawn at
//! a fixed position that depends only on the terminal size and the panel's own
//! content, never on the scroll position or the chat text.

use super::*;

/// Narrowest docked column (borders included).
const DOCK_MIN_WIDTH: u16 = 34;
/// Widest docked column (borders included).
const DOCK_MAX_WIDTH: u16 = 60;
/// Columns the chat keeps beside the dock. Below this the dock is not shown.
const DOCK_MIN_CHAT_WIDTH: u16 = 60;

#[cfg(test)]
thread_local! {
    static DOCK_DISABLED_FOR_TEST: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Transcript-geometry tests measure how the chat wraps at a given terminal
/// width. The dock would take part of that width, so they turn it off.
#[cfg(test)]
pub fn set_dock_disabled_for_test(disabled: bool) {
    DOCK_DISABLED_FOR_TEST.with(|flag| flag.set(disabled));
}
/// Width used to measure natural line widths. Wide enough that no section
/// shortens its text to fit.
const MEASURE_WIDTH: u16 = 200;

/// Sections shown in the dock, top to bottom.
fn dock_sections(data: &InfoWidgetData) -> Vec<WidgetKind> {
    let mut kinds = Vec::new();
    if data.has_data_for(WidgetKind::Overview) || overview_has_content(data) {
        kinds.push(WidgetKind::Overview);
    }
    for kind in [
        WidgetKind::MemoryActivity,
        WidgetKind::SwarmStatus,
        WidgetKind::BackgroundTasks,
        WidgetKind::Compaction,
        WidgetKind::AmbientMode,
    ] {
        if data.has_data_for(kind) {
            kinds.push(kind);
        }
    }
    kinds
}

fn overview_has_content(data: &InfoWidgetData) -> bool {
    !overview_lines(data, Rect::new(0, 0, MEASURE_WIDTH, u16::MAX / 2)).is_empty()
}

/// Every overview line at `inner` width. Unlike the floating Overview there is
/// no paging: the dock shows all sections at once.
fn overview_lines(data: &InfoWidgetData, inner: Rect) -> Vec<Line<'static>> {
    let mut overview = data.clone();
    overview.memory_info = None;
    overview.diagrams.clear();
    render_sections(&overview, inner, Some(InfoPageKind::TodosExpanded))
}

fn section_lines(kind: WidgetKind, data: &InfoWidgetData, inner: Rect) -> Vec<Line<'static>> {
    match kind {
        WidgetKind::Overview => overview_lines(data, inner),
        other => render_widget_content(other, data, inner),
    }
}

/// Docked column width for this terminal and content, or `None` when the
/// chat would be too narrow to spare a column.
///
/// The workspace map and margin diagrams are drawn as pictures by the floating
/// layout. The dock only stacks text sections, so when either is present the
/// dock stands aside and the floating layout shows everything.
pub fn dock_width(data: &InfoWidgetData, chat_width: u16) -> Option<u16> {
    #[cfg(test)]
    if DOCK_DISABLED_FOR_TEST.with(|flag| flag.get()) {
        return None;
    }
    if data.has_data_for(WidgetKind::WorkspaceMap) || data.has_data_for(WidgetKind::Diagrams) {
        return None;
    }
    let available = chat_width.checked_sub(DOCK_MIN_CHAT_WIDTH)?;
    if available < DOCK_MIN_WIDTH {
        return None;
    }
    let kinds = dock_sections(data);
    if kinds.is_empty() {
        return None;
    }
    let measure = Rect::new(0, 0, MEASURE_WIDTH, u16::MAX / 2);
    let natural = kinds
        .iter()
        .flat_map(|&kind| section_lines(kind, data, measure))
        .map(|line| line.width() as u16)
        .max()
        .unwrap_or(0);
    // +2 for the border.
    Some(
        natural
            .saturating_add(2)
            .clamp(DOCK_MIN_WIDTH, DOCK_MAX_WIDTH)
            .min(available),
    )
}

/// Draw the dock into `area` (the reserved column). Sections are stacked in a
/// single bordered panel, top-aligned, separated by a dim rule.
pub fn render_dock(frame: &mut Frame, area: Rect, data: &InfoWidgetData) {
    if area.width < 3 || area.height < 3 {
        return;
    }
    let inner_width = area.width - 2;
    let inner_probe = Rect::new(area.x + 1, area.y + 1, inner_width, area.height - 2);
    let mut lines: Vec<Line<'static>> = Vec::new();
    for kind in dock_sections(data) {
        let section = section_lines(kind, data, inner_probe);
        if section.is_empty() {
            continue;
        }
        if !lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "─".repeat(inner_width as usize),
                Style::default().fg(rgb(55, 55, 65)),
            )));
        }
        lines.extend(section);
    }
    if lines.is_empty() {
        return;
    }
    let height = (lines.len() as u16 + 2).min(area.height);
    let rect = Rect::new(area.x, area.y, area.width, height);
    lines.truncate(height.saturating_sub(2) as usize);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(rgb(70, 70, 80)).dim());
    let inner = block.inner(rect);
    frame.render_widget(ratatui::widgets::Clear, rect);
    frame.render_widget(block, rect);
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Lines the dock would draw at `width` (borders included), for tests and
/// debug captures.
pub fn dock_text_lines(data: &InfoWidgetData, width: u16) -> Vec<String> {
    let inner = Rect::new(0, 0, width.saturating_sub(2), u16::MAX / 2);
    dock_sections(data)
        .into_iter()
        .flat_map(|kind| section_lines(kind, data, inner))
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect()
}
