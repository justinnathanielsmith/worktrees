use crate::app::model::AppState;
use crate::ui::theme::CyberTheme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub struct StashListWidget;

impl StashListWidget {
    pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
        if let AppState::ViewingStashes {
            stashes,
            selected_index,
            branch,
            ..
        } = state
        {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(area);

            // Header/Info
            let header_text = vec![Line::from(vec![
                Span::styled(" STASHES ", Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)),
                Span::raw(" for "),
                Span::styled(branch, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ])];
            
            let header = Paragraph::new(header_text)
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
            frame.render_widget(header, chunks[0]);

            // Stash List
            let items: Vec<ListItem> = stashes
                .iter()
                .enumerate()
                .map(|(i, stash)| {
                    let style = if i == *selected_index {
                        Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };

                    let content = vec![Line::from(vec![
                        Span::styled(format!(" stash@{{{}}} ", stash.index), Style::default().fg(Color::Cyan)),
                        Span::raw(" "),
                        Span::raw(&stash.message),
                    ])];

                    ListItem::new(content).style(style)
                })
                .collect();

            let list = List::new(items)
                .block(Block::default()
                    .title(" [↑↓] Navigate | [a] Apply | [p] Pop | [d] Drop | [n] New | [Esc] Back ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)));

            frame.render_widget(list, chunks[1]);
        }
    }
}
