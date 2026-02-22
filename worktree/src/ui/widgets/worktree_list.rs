use crate::app::model::AppMode;
use crate::domain::repository::Worktree;
use crate::ui::theme::{CyberTheme, Icons};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, StatefulWidget, Table, TableState, Widget,
    },
};
use std::borrow::Cow;
use std::sync::OnceLock;

static SPINNER_COMBINATIONS: OnceLock<Vec<String>> = OnceLock::new();

fn get_spinner_icon(is_selected: bool, spinner_tick: usize, icon_idx: usize) -> &'static str {
    // Cyber-style cursor animation
    // ▊, ▋, ▌, ▍, ▎, ▏
    const SPINNER_PREFIXES: [&str; 6] = [" ▊ ", " ▋ ", " ▌ ", " ▍ ", " ▌ ", " ▋ "];
    const ICONS: [&str; 3] = [Icons::HUB, Icons::DETACHED, Icons::WORKTREE];

    let combinations = SPINNER_COMBINATIONS.get_or_init(|| {
        let mut v = Vec::with_capacity((SPINNER_PREFIXES.len() + 1) * ICONS.len());
        // Add selected variations (prefix + icon)
        for prefix in SPINNER_PREFIXES {
            for icon_str in ICONS {
                v.push(format!("{}{}", prefix, icon_str));
            }
        }
        // Add unselected variations ("   " + icon)
        for icon_str in ICONS {
            v.push(format!("   {}", icon_str));
        }
        v
    });

    // Ensure icon_idx is valid, fallback to WORKTREE (index 2)
    let safe_icon_idx = if icon_idx < ICONS.len() { icon_idx } else { 2 };

    if is_selected {
        let spinner_idx = (spinner_tick / 2) % SPINNER_PREFIXES.len();
        let idx = spinner_idx * ICONS.len() + safe_icon_idx;
        &combinations[idx]
    } else {
        let idx = SPINNER_PREFIXES.len() * ICONS.len() + safe_icon_idx;
        &combinations[idx]
    }
}

pub struct WorktreeListWidget<'a> {
    worktrees: &'a [Worktree],
    indices: Option<&'a [usize]>,
    is_dimmed: bool,
    spinner_tick: usize,
    filter_query: Option<&'a str>,
    mode: Option<AppMode>,
}

impl<'a> WorktreeListWidget<'a> {
    pub const fn new(worktrees: &'a [Worktree], indices: Option<&'a [usize]>) -> Self {
        Self {
            worktrees,
            indices,
            is_dimmed: false,
            spinner_tick: 0,
            filter_query: None,
            mode: None,
        }
    }

    pub const fn dimmed(mut self, is_dimmed: bool) -> Self {
        self.is_dimmed = is_dimmed;
        self
    }

    pub const fn tick(mut self, tick: usize) -> Self {
        self.spinner_tick = tick;
        self
    }

    pub const fn with_filter(mut self, query: Option<&'a str>) -> Self {
        self.filter_query = query;
        self
    }

    pub const fn with_mode(mut self, mode: AppMode) -> Self {
        self.mode = Some(mode);
        self
    }
}

impl StatefulWidget for WorktreeListWidget<'_> {
    type State = TableState;

    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer, state: &mut Self::State) {
        let theme = CyberTheme::default();

        let (mode_color, mode_title, border_color) = if let Some(mode) = self.mode {
            match mode {
                AppMode::Normal => (theme.primary, "ACTIVE WORKTREES", theme.border),
                AppMode::Manage => (theme.secondary, "MANAGE WORKTREES", theme.secondary),
                AppMode::Git => (theme.success, "GIT WORKTREES", theme.success),
                AppMode::Filter => (theme.warning, "FILTERING...", theme.warning),
            }
        } else {
            (theme.primary, "ACTIVE WORKTREES", theme.border)
        };

        let border_style = if self.is_dimmed {
            Style::default()
                .fg(theme.subtle)
                .add_modifier(Modifier::DIM)
        } else {
            Style::default().fg(border_color)
        };

        let title_style = if self.is_dimmed {
            Style::default()
                .fg(theme.subtle)
                .add_modifier(Modifier::DIM)
        } else {
            Style::default().fg(mode_color).add_modifier(Modifier::BOLD)
        };

        let total_count = self.indices.map_or(self.worktrees.len(), |i| i.len());

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .title(Span::styled(
                format!(" {} ({}) ", mode_title, total_count),
                title_style,
            ));

        let inner_area = block.inner(area);

        if total_count == 0 {
            block.render(area, buf);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(2), // Height for 2 lines of text
                    Constraint::Min(0),
                ])
                .split(inner_area);

            let text = if let Some(query) = self.filter_query
                && !query.is_empty()
            {
                vec![
                    Line::from(vec![
                        Span::styled("No worktrees match '", Style::default().fg(theme.subtle)),
                        Span::styled(
                            query,
                            Style::default()
                                .fg(theme.warning)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled("'.", Style::default().fg(theme.subtle)),
                    ]),
                    Line::from(Span::styled(
                        "Press [Esc] to clear filter.",
                        Style::default().fg(theme.subtle),
                    )),
                ]
            } else {
                vec![
                    Line::from(Span::styled(
                        "No worktrees found.",
                        Style::default().fg(theme.subtle),
                    )),
                    Line::from(Span::styled(
                        "Press [A] to add a worktree.",
                        Style::default().fg(theme.secondary),
                    )),
                ]
            };

            Paragraph::new(text)
                .alignment(Alignment::Center)
                .render(chunks[1], buf);
            return;
        }

        // Optimization: Manual Virtualization (Windowing)
        // Only render rows that are visible in the current area.
        // This significantly reduces allocations for large lists.
        let header_height = 2; // 1 for text + 1 bottom margin
        let available_height = inner_area.height.saturating_sub(header_height) as usize;

        let start_index = state.offset().min(total_count.saturating_sub(1));
        let end_index = (start_index + available_height).min(total_count);

        // Iterate over the visible range and retrieve worktrees either directly or via indices
        let rows = (start_index..end_index).map(|i| {
            let actual_index = start_index + (i - start_index); // Redundant but explicit
            let wt_index = self.indices.map_or(actual_index, |idxs| idxs[actual_index]);

            // Safety: indices are derived from worktrees, so this should be valid.
            // Fallback to safe get to prevent panic if indices are stale (though they shouldn't be).
            let wt = self
                .worktrees
                .get(wt_index)
                .expect("Invalid worktree index");

            let is_selected = Some(actual_index) == state.selected();

            // icon_idx: 0=HUB, 1=DETACHED, 2=WORKTREE
            let (icon_idx, branch_style) = if wt.is_bare {
                (
                    0,
                    if self.is_dimmed {
                        Style::default().fg(theme.subtle)
                    } else {
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD)
                    },
                )
            } else if wt.is_detached {
                (1, Style::default().fg(theme.error))
            } else {
                (
                    2,
                    if self.is_dimmed {
                        Style::default().fg(theme.subtle)
                    } else {
                        Style::default().fg(theme.primary)
                    },
                )
            };

            let intent_str = if wt.is_bare {
                Cow::Borrowed("MAIN")
            } else if let Some(meta) = &wt.metadata
                && let Some(purpose) = &meta.purpose
            {
                Cow::Borrowed(purpose.as_str())
            } else {
                std::path::Path::new(&wt.path).file_name().map_or_else(
                    || Cow::Borrowed(wt.branch.as_str()),
                    |n| n.to_string_lossy(),
                )
            };

            let mut row_style = Style::default().fg(theme.text);

            if self.is_dimmed {
                row_style = row_style.add_modifier(Modifier::DIM);
            }

            if is_selected {
                row_style = row_style
                    .bg(theme.selection_bg)
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD);

                // Remove dim modifier if selected, to make it pop even when filtering
                if self.is_dimmed {
                    row_style = row_style.remove_modifier(Modifier::DIM);
                }
            }

            let status_cell = wt.status_summary.as_ref().map_or_else(
                || Cell::from("-"),
                |summary| {
                    let (color, icon) = if summary == "clean" {
                        (theme.success, Icons::CLEAN)
                    } else {
                        (theme.warning, Icons::DIRTY)
                    };

                    let style = if self.is_dimmed && !is_selected {
                        Style::default().fg(theme.subtle)
                    } else {
                        Style::default().fg(color)
                    };

                    let summary_text = if summary == "clean" {
                        "CLEAN"
                    } else {
                        summary.as_str()
                    };
                    Cell::from(Line::from(vec![
                        Span::raw(icon),
                        Span::raw(" "),
                        Span::raw(summary_text),
                    ]))
                    .style(style)
                },
            );

            let mut cell_style = Style::default();
            if self.is_dimmed && !is_selected {
                cell_style = cell_style.fg(theme.subtle);
            }

            // Optimization: Use cached spinner strings to avoid Vec allocation per row
            let spinner_icon = get_spinner_icon(is_selected, self.spinner_tick, icon_idx);

            Row::new([
                Cell::from(spinner_icon),
                Cell::from(intent_str).style(if is_selected {
                    branch_style
                } else {
                    cell_style
                }),
                Cell::from(wt.branch.as_str()).style(if is_selected {
                    Style::default().fg(theme.primary)
                } else {
                    cell_style
                }),
                status_cell,
                Cell::from(format_size(wt.size_bytes)).style(Style::default().fg(theme.subtle)),
                Cell::from(&wt.commit[..wt.commit.len().min(7)])
                    .style(Style::default().fg(theme.subtle)),
            ])
            .style(row_style)
            .height(1)
        });

        let table = Table::new(
            rows,
            [
                Constraint::Length(6), // Reduced width for icon/cursor
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Length(12),
                Constraint::Length(10), // Size column
                Constraint::Length(10), // Commit column
            ],
        )
        .header(
            Row::new(["", "INTENT", "BRANCH", "STATUS", "SIZE", "COMMIT"])
                .style(if self.is_dimmed {
                    Style::default()
                        .fg(theme.subtle)
                        .add_modifier(Modifier::DIM)
                } else {
                    Style::default()
                        .fg(theme.secondary)
                        .add_modifier(Modifier::BOLD)
                })
                .bottom_margin(1),
        )
        .block(block)
        .row_highlight_style(Style::default().add_modifier(Modifier::BOLD)); // Handled manually in row mapping, but keeping basic highlight

        // Create a temporary state for rendering the slice
        let mut temp_state = TableState::default();
        if let Some(selected) = state.selected()
            && selected >= start_index
            && selected < end_index
        {
            temp_state.select(Some(selected - start_index));
        }
        // Offset is always 0 because we are feeding exactly what needs to be rendered from top
        *temp_state.offset_mut() = 0;

        StatefulWidget::render(table, area, buf, &mut temp_state);

        if total_count > inner_area.height as usize {
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .track_symbol(None)
                .thumb_symbol("▐")
                .thumb_style(if self.is_dimmed {
                    Style::default()
                        .fg(theme.subtle)
                        .add_modifier(Modifier::DIM)
                } else {
                    Style::default().fg(theme.secondary)
                });

            let mut scrollbar_state = ScrollbarState::new(total_count)
                .position(state.offset())
                .viewport_content_length(inner_area.height as usize);

            StatefulWidget::render(scrollbar, inner_area, buf, &mut scrollbar_state);
        }
    }
}

fn format_size(bytes: u64) -> Cow<'static, str> {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes == 0 {
        return Cow::Borrowed("0 B");
    }

    let s = if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    };
    Cow::Owned(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn test_render_empty_worktree_list() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = TableState::default();
        let worktrees = vec![];
        let widget = WorktreeListWidget::new(&worktrees, None);

        terminal
            .draw(|f| {
                let area = f.area();
                f.render_stateful_widget(widget, area, &mut state);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();

        assert!(content.contains("No worktrees found."));
        assert!(content.contains("Press [A] to add a worktree."));
        assert!(content.contains("ACTIVE WORKTREES (0)"));
    }

    #[test]
    fn test_render_empty_worktree_list_with_filter() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = TableState::default();
        let worktrees = vec![];
        let widget = WorktreeListWidget::new(&worktrees, None).with_filter(Some("foobar"));

        terminal
            .draw(|f| {
                let area = f.area();
                f.render_stateful_widget(widget, area, &mut state);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();

        assert!(content.contains("No worktrees match 'foobar'."));
        assert!(content.contains("Press [Esc] to clear filter."));
        assert!(content.contains("ACTIVE WORKTREES (0)"));
    }

    #[test]
    fn test_render_worktree_list_with_mode() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = TableState::default();
        let worktrees = vec![];
        let widget = WorktreeListWidget::new(&worktrees, None).with_mode(AppMode::Manage);

        terminal
            .draw(|f| {
                let area = f.area();
                f.render_stateful_widget(widget, area, &mut state);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();

        assert!(content.contains("MANAGE WORKTREES (0)"));
    }
}
