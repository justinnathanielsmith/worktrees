use crate::app::model::AppState;
use crate::ui::theme::CyberTheme;
use crate::ui::widgets::worktree_list::WorktreeListWidget;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

pub fn render_status(
    f: &mut Frame,
    branch: &str,
    status: &crate::app::model::StatusViewState,
    prev_state: &AppState,
    area: Rect,
) {
    let theme = CyberTheme::default();

    // Create a side-panel layout
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
        .split(area);

    // Render background list from prev_state for context
    if let AppState::ListingWorktrees {
        worktrees,
        table_state,
        ..
    } = prev_state
    {
        let mut ts = table_state.clone();
        f.render_stateful_widget(WorktreeListWidget::new(worktrees), body_chunks[0], &mut ts);
    }

    let main_area = body_chunks[1];

    // Split main area into status view and diff preview
    let main_chunks = if status.show_diff {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(main_area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(100)])
            .split(main_area)
    };

    // --- STATUS VIEW ---
    let status_area = main_chunks[0];
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(theme.primary))
        .title(Span::styled(
            format!("  󰊢 GIT STATUS: {} ", branch),
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ));

    let inner_area = outer_block.inner(status_area);
    f.render_widget(outer_block, status_area);

    // Calculate statistics
    let total_files = status.total();
    let staged_count = status.staged.len();
    let unstaged_count = status.unstaged.len();
    let untracked_count = status.untracked.len();

    // Split into header stats and file lists
    let status_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(inner_area);

    // --- STATISTICS HEADER ---
    let stats_text = vec![
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                format!("󰄬 {} Staged", staged_count),
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  │  ", Style::default().fg(theme.border)),
            Span::styled(
                format!("󱇨 {} Modified", unstaged_count),
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  │  ", Style::default().fg(theme.border)),
            Span::styled(
                format!("󰡯 {} Untracked", untracked_count),
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  │  ", Style::default().fg(theme.border)),
            Span::styled(
                format!(" {} Total", total_files),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];

    let stats_widget = Paragraph::new(stats_text).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(theme.border)),
    );
    f.render_widget(stats_widget, status_layout[0]);

    // --- FILE LISTS ---
    let file_area = status_layout[1];
    let file_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(file_area);

    // --- STAGED COLUMN ---
    let mut staged_items = Vec::new();
    if status.staged.is_empty() {
        staged_items.push(Line::from(vec![
            Span::styled("   ", Style::default()),
            Span::styled("No staged changes", Style::default().fg(theme.subtle)),
        ]));
    } else {
        for (i, file) in status.staged.iter().enumerate() {
            let is_selected = i == status.selected_index;
            let (icon, file_type_color) = get_file_icon_and_color(file, &theme);

            let style = if is_selected {
                Style::default()
                    .bg(theme.selection_bg)
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.success)
            };

            let prefix = if is_selected { " ▶ " } else { "   " };
            staged_items.push(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled("󰄬 ", style),
                Span::styled(icon, Style::default().fg(file_type_color)),
                Span::styled(" ", style),
                Span::styled(file, style),
            ]));
        }
    }

    let staged_list = Paragraph::new(staged_items).block(
        Block::default()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(theme.border))
            .title(Span::styled(
                format!(" 󰄬 STAGED ({}) ", staged_count),
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(staged_list, file_chunks[0]);

    // --- UNSTAGED/UNTRACKED COLUMN ---
    let mut unstaged_items = Vec::new();

    if status.unstaged.is_empty() && status.untracked.is_empty() {
        unstaged_items.push(Line::from(vec![
            Span::styled("   ", Style::default()),
            Span::styled("No unstaged changes", Style::default().fg(theme.subtle)),
        ]));
    } else {
        let unstaged_start = status.staged.len();
        for (i, file) in status.unstaged.iter().enumerate() {
            let is_selected = (i + unstaged_start) == status.selected_index;
            let (icon, file_type_color) = get_file_icon_and_color(file, &theme);

            let style = if is_selected {
                Style::default()
                    .bg(theme.selection_bg)
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.warning)
            };

            let prefix = if is_selected { " ▶ " } else { "   " };
            unstaged_items.push(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled("󱇨 ", style),
                Span::styled(icon, Style::default().fg(file_type_color)),
                Span::styled(" ", style),
                Span::styled(file, style),
            ]));
        }

        let untracked_start = status.staged.len() + status.unstaged.len();
        for (i, file) in status.untracked.iter().enumerate() {
            let is_selected = (i + untracked_start) == status.selected_index;
            let (icon, file_type_color) = get_file_icon_and_color(file, &theme);

            let style = if is_selected {
                Style::default()
                    .bg(theme.selection_bg)
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.error)
            };

            let prefix = if is_selected { " ▶ " } else { "   " };
            unstaged_items.push(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled("󰡯 ", style),
                Span::styled(icon, Style::default().fg(file_type_color)),
                Span::styled(" ", style),
                Span::styled(file, style),
            ]));
        }
    }

    let unstaged_list = Paragraph::new(unstaged_items).block(
        Block::default().title(Span::styled(
            format!(
                " 󱇨 UNSTAGED / UNTRACKED ({}) ",
                unstaged_count + untracked_count
            ),
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )),
    );
    f.render_widget(unstaged_list, file_chunks[1]);

    // --- DIFF PREVIEW ---
    if status.show_diff && main_chunks.len() > 1 {
        let diff_area = main_chunks[1];
        let selected_file = status.selected_file().unwrap_or("No file selected");

        let diff_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.accent))
            .title(Span::styled(
                format!("  DIFF PREVIEW: {} ", selected_file),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ));

        let diff_inner = diff_block.inner(diff_area);
        f.render_widget(diff_block, diff_area);

        let diff_lines = if let Some(ref diff) = status.diff_preview {
            parse_diff_with_colors(diff, &theme)
        } else {
            vec![Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled("Press ", Style::default().fg(theme.subtle)),
                Span::styled(
                    "[D]",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" to view diff", Style::default().fg(theme.subtle)),
            ])]
        };

        let diff_widget = Paragraph::new(diff_lines).wrap(Wrap { trim: false });
        f.render_widget(diff_widget, diff_inner);
    }

    // --- ENHANCED FOOTER ---
    let footer_area = Rect::new(
        status_area.x + 2,
        status_area.y + status_area.height - 1,
        status_area.width - 4,
        1,
    );
    let help_text = Line::from(vec![
        Span::styled(
            " [SPACE]",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Toggle  "),
        Span::styled(
            "[A]",
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" All  "),
        Span::styled(
            "[U]",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Unstage  "),
        Span::styled(
            "[D]",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Diff  "),
        Span::styled(
            "[C]",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Commit  "),
        Span::styled(
            "[R]",
            Style::default()
                .fg(theme.secondary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Refresh  "),
        Span::styled(
            "[ESC]",
            Style::default()
                .fg(theme.subtle)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Back"),
    ]);
    f.render_widget(
        Paragraph::new(help_text).alignment(Alignment::Center),
        footer_area,
    );
}

/// Get file icon and color based on file extension
fn get_file_icon_and_color(
    filename: &str,
    theme: &CyberTheme,
) -> (&'static str, ratatui::style::Color) {
    let ext = filename.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => ("", theme.warning),
        "toml" => ("", theme.accent),
        "md" => ("", theme.primary),
        "json" => ("", theme.success),
        "yaml" | "yml" => ("", theme.secondary),
        "js" | "ts" => ("", theme.warning),
        "py" => ("", theme.primary),
        "go" => ("", theme.primary),
        "java" | "kt" => ("", theme.error),
        "html" | "css" => ("", theme.accent),
        "txt" => ("", theme.subtle),
        "lock" => ("", theme.subtle),
        _ => ("", theme.text),
    }
}

/// Parse diff output and colorize it
fn parse_diff_with_colors<'a>(diff: &'a str, theme: &CyberTheme) -> Vec<Line<'a>> {
    diff.lines()
        .take(50) // Limit to 50 lines for performance
        .map(|line| {
            if line.starts_with('+') && !line.starts_with("+++") {
                Line::from(Span::styled(
                    format!(" {}", line),
                    Style::default().fg(theme.success),
                ))
            } else if line.starts_with('-') && !line.starts_with("---") {
                Line::from(Span::styled(
                    format!(" {}", line),
                    Style::default().fg(theme.error),
                ))
            } else if line.starts_with("@@") {
                Line::from(Span::styled(
                    format!(" {}", line),
                    Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
                ))
            } else if line.starts_with("diff") || line.starts_with("index") {
                Line::from(Span::styled(
                    format!(" {}", line),
                    Style::default().fg(theme.subtle),
                ))
            } else {
                Line::from(Span::styled(
                    format!(" {}", line),
                    Style::default().fg(theme.text),
                ))
            }
        })
        .collect()
}
