use crate::app::model::AppState;
use crate::ui::theme::CyberTheme;
use crate::ui::widgets::worktree_list::WorktreeListWidget;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

pub fn render_status(
    f: &mut Frame,
    branch: &str,
    status: &crate::app::model::StatusViewState,
    prev_state: &Box<AppState>,
    area: Rect,
) {
    let theme = CyberTheme::default();

    // Create a side-panel layout
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(area);

    // Render background list from prev_state for context
    if let AppState::ListingWorktrees {
        worktrees,
        table_state,
        ..
    } = &**prev_state
    {
        let mut ts = table_state.clone();
        f.render_stateful_widget(WorktreeListWidget::new(worktrees), body_chunks[0], &mut ts);
    }

    let area = body_chunks[1];
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(theme.primary))
        .title(Span::styled(
            format!("  GIT STATUS: {} ", branch),
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ));

    let inner_area = outer_block.inner(area);
    f.render_widget(outer_block, area);

    let status_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner_area);

    // --- STAGED COLUMN ---
    let mut staged_items = Vec::new();
    for (i, file) in status.staged.iter().enumerate() {
        let is_selected = i == status.selected_index;
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
            Span::styled(file, style),
        ]));
    }
    let staged_list = Paragraph::new(staged_items).block(
        Block::default()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(theme.border))
            .title(Span::styled(
                " 󰄬 STAGED CHANGES ",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(staged_list, status_chunks[0]);

    // --- UNSTAGED COLUMN ---
    let mut unstaged_items = Vec::new();
    let unstaged_start = status.staged.len();
    for (i, file) in status.unstaged.iter().enumerate() {
        let is_selected = (i + unstaged_start) == status.selected_index;
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
            Span::styled(file, style),
        ]));
    }

    let untracked_start = status.staged.len() + status.unstaged.len();
    for (i, file) in status.untracked.iter().enumerate() {
        let is_selected = (i + untracked_start) == status.selected_index;
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
            Span::styled(file, style),
        ]));
    }

    let unstaged_list = Paragraph::new(unstaged_items).block(
        Block::default().title(Span::styled(
            " 󱇨 UNSTAGED / UNTRACKED ",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )),
    );
    f.render_widget(unstaged_list, status_chunks[1]);

    // --- HELPER FOOTER ---
    let footer_area = Rect::new(area.x + 2, area.y + area.height - 1, area.width - 4, 1);
    let help_text = Line::from(vec![
        Span::styled(
            " [SPACE]",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" TOGGLE  "),
        Span::styled(
            " [A]",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" STAGE ALL  "),
        Span::styled(
            " [U]",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" UNSTAGE ALL  "),
        Span::styled(
            " [C]",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" COMMIT MENU  "),
        Span::styled(
            " [ESC/Q]",
            Style::default()
                .fg(theme.subtle)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" BACK "),
    ]);
    f.render_widget(
        Paragraph::new(help_text).alignment(Alignment::Center),
        footer_area,
    );
}
