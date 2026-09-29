//! Docked info column: the info box as a fixed panel on the right of the chat.
//!
//! The floating layout (`info_widget_layout`) fits boxes into the gaps between
//! chat lines, so they move whenever the chat scrolls or streams and shrink to
//! whatever gap is free. The dock instead reserves a column: the transcript is
//! wrapped narrower so it never runs under the panel, and the panel is drawn at
//! a fixed position and width that depend only on the terminal size, never on
//! the scroll position, the chat text, or the panel's own content.

use super::*;

/// Narrowest docked column (borders included).
const DOCK_MIN_WIDTH: u16 = 34;
/// Docked column width (borders included) when the terminal has room. Wide
/// enough for every line the sections draw at full length (context and usage
/// bars, model and account lines, KV cache), so nothing is cut off, and fixed
/// so the box never shifts sideways when a line appears or grows.
const DOCK_WIDTH: u16 = 46;
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
/// Width used to check whether the overview has anything to show. Wide
/// enough that no line is shortened.
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
    if dock_sections(data).is_empty() {
        return None;
    }
    // Depends on the terminal width only. Measuring the content made the box
    // one column wider (and its left edge jump) whenever a longer line such as
    // the KV cache line appeared after the first reply.
    Some(DOCK_WIDTH.min(available))
}

/// Every line of the docked box (sections separated by a dim rule) at the
/// given inner width.
fn dock_lines(data: &InfoWidgetData, inner: Rect) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    for kind in dock_sections(data) {
        let section = section_lines(kind, data, inner);
        if section.is_empty() {
            continue;
        }
        if !lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "─".repeat(inner.width as usize),
                Style::default().fg(rgb(55, 55, 65)),
            )));
        }
        lines.extend(section);
    }
    lines
}

/// Height of the docked box (borders included) at column `width`, or 0 when
/// there is nothing to show.
pub fn dock_height(data: &InfoWidgetData, width: u16) -> u16 {
    let inner = Rect::new(0, 0, width.saturating_sub(2), u16::MAX / 2);
    let lines = dock_lines(data, inner).len() as u16;
    if lines == 0 {
        0
    } else {
        lines.saturating_add(2)
    }
}

/// Where the docked box goes. Its top edge sits at a fixed row, a third of the
/// way down the chat column, computed from the column height alone (which only
/// changes with the terminal size). So the box never moves when the chat
/// scrolls or streams, when the input grows, or when its own content changes
/// height (usage limits appearing, todos): it grows and shrinks downward from
/// that row. Only if it would reach `bottom` (the status line) is it pulled
/// up, and clipped if the space is smaller than the box.
pub fn dock_rect(column: Rect, bottom: u16, box_height: u16) -> Option<Rect> {
    let usable = bottom.saturating_sub(column.y);
    if box_height == 0 || usable < 3 {
        return None;
    }
    let height = box_height.min(usable);
    let anchor = column.y + column.height / 3;
    let y = anchor.min(bottom - height).max(column.y);
    Some(Rect::new(column.x, y, column.width, height))
}

/// Draw the docked box into `rect` (from [`dock_rect`]).
pub fn render_dock(frame: &mut Frame, rect: Rect, data: &InfoWidgetData) {
    if rect.width < 3 || rect.height < 3 {
        return;
    }
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(rgb(70, 70, 80)).dim());
    let inner = block.inner(rect);
    let mut lines = dock_lines(data, inner);
    if lines.is_empty() {
        return;
    }
    lines.truncate(inner.height as usize);
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
