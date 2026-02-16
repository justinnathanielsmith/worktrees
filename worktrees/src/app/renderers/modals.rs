use crate::app::model::AppState;
use crate::domain::repository::ProjectRepository;
use crate::ui::theme::CyberTheme;
use crate::ui::widgets::{details::DetailsWidget, worktree_list::WorktreeListWidget};
use ratatui::{
    Frame,
    layout::{Alignment, Layout, Rect},
    style::{Color as RatatuiColor, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, TableState},
};

use super::helpers::centered_rect;

pub fn render_modals<R: ProjectRepository>(
    f: &mut Frame,
    repo: &R,
    state: &mut AppState,
    spinner_tick: usize,
) {
    let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner = spinner_chars[spinner_tick % spinner_chars.len()];
    let _chunks = Layout::default().margin(1).split(f.area()); // Fallback chunks if not passed?  
    // Actually view.rs calculates chunks in draw(). We might need them or calculate them here.
    // But RenderModals is usually an overlay.
    // However, for Error and Welcome, it renders widgets in the background too.
    // Let's assume we recalculate chunks or pass them.
    // For simplicity, I'll calculate chunks here as they are standard.

    let main_chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .margin(1)
        .constraints(
            [
                ratatui::layout::Constraint::Length(3), // Header
                ratatui::layout::Constraint::Min(5),    // Table
                ratatui::layout::Constraint::Length(6), // Details
                ratatui::layout::Constraint::Length(3), // Footer
            ]
            .as_ref(),
        )
        .split(f.area());

    let context = repo.detect_context(std::path::Path::new("."));
    let standard_area = centered_rect(60, 20, f.area());

    match state {
        AppState::Syncing { branch, .. } => {
            let theme = CyberTheme::default();
            render_info_modal(
                f,
                standard_area,
                Line::from(vec![Span::styled(
                    format!(" {} SYNCING CONFIGURATIONS ", spinner),
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                )]),
                vec![Line::from(vec![
                    Span::raw("Branch: "),
                    Span::styled(
                        branch.as_str(),
                        Style::default()
                            .fg(theme.secondary)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])],
                Style::default().fg(theme.primary),
            );
        }
        AppState::SyncComplete { branch, .. } => {
            render_info_modal(
                f,
                standard_area,
                Line::from(vec![Span::styled(
                    " ✅ SYNC COMPLETE ",
                    Style::default()
                        .fg(RatatuiColor::Green)
                        .add_modifier(Modifier::BOLD),
                )]),
                vec![Line::from(vec![
                    Span::raw("Successfully synced: "),
                    Span::styled(
                        branch.as_str(),
                        Style::default()
                            .fg(RatatuiColor::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])],
                Style::default(),
            );
        }
        AppState::Pushing { branch, .. } => {
            let theme = CyberTheme::default();
            render_info_modal(
                f,
                standard_area,
                Line::from(vec![Span::styled(
                    format!(" {} PUSHING TO REMOTE ", spinner),
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                )]),
                vec![Line::from(vec![
                    Span::raw("Pushing branch: "),
                    Span::styled(
                        branch.as_str(),
                        Style::default()
                            .fg(theme.secondary)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])],
                Style::default().fg(theme.primary),
            );
        }
        AppState::PushComplete { branch, .. } => {
            render_info_modal(
                f,
                standard_area,
                Line::from(vec![Span::styled(
                    " ✅ PUSH COMPLETE ",
                    Style::default()
                        .fg(RatatuiColor::Green)
                        .add_modifier(Modifier::BOLD),
                )]),
                vec![Line::from(vec![
                    Span::raw("Successfully pushed: "),
                    Span::styled(
                        branch.as_str(),
                        Style::default()
                            .fg(RatatuiColor::Magenta)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])],
                Style::default(),
            );
        }
        AppState::Help { .. } => {
            let theme = CyberTheme::default();
            let area = centered_rect(80, 70, f.area());
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Thick)
                .border_style(Style::default().fg(theme.primary))
                .title(Span::styled(
                    " 󰛵 COMMAND PROTOCOLS ",
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ));

            let inner_area = block.inner(area);
            f.render_widget(block, area);

            let help_content = vec![
                Line::from(vec![
                    Span::styled(
                        " [A] ",
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" Add a new worktree for a specific feature or intent"),
                ]),
                Line::from(vec![
                    Span::styled(
                        " [D/X] ",
                        Style::default()
                            .fg(theme.error)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" Remove the selected worktree (requires confirmation)"),
                ]),
                Line::from(vec![
                    Span::styled(
                        " [O] ",
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" Open the selected worktree in your preferred editor"),
                ]),
                Line::from(vec![
                    Span::styled(
                        " [G] ",
                        Style::default()
                            .fg(theme.secondary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" View detailed Git status and stage/commit changes"),
                ]),
                Line::from(vec![
                    Span::styled(
                        " [L] ",
                        Style::default()
                            .fg(theme.secondary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" View recent commit history for the worktree"),
                ]),
                Line::from(vec![
                    Span::styled(
                        " [B] ",
                        Style::default()
                            .fg(theme.secondary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" Switch the selected worktree to a different branch"),
                ]),
                Line::from(vec![
                    Span::styled(
                        " [F] ",
                        Style::default()
                            .fg(theme.secondary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" Fetch all updates from the remote repository"),
                ]),
                Line::from(vec![
                    Span::styled(
                        " [S] ",
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" Sync IDE and project configurations to worktree"),
                ]),
                Line::from(vec![
                    Span::styled(
                        " [P] ",
                        Style::default()
                            .fg(theme.secondary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" Push committed changes to the remote repository"),
                ]),
                Line::from(vec![
                    Span::styled(
                        " [U] ",
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" Run canonical setup (creates 'main' and 'dev')"),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        " [?/H] ",
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" Toggle this help protocol view"),
                ]),
                Line::from(vec![
                    Span::styled(
                        " [Q/ESC] ",
                        Style::default()
                            .fg(theme.subtle)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" Return to previous view or exit system"),
                ]),
            ];

            f.render_widget(
                Paragraph::new(help_content).alignment(Alignment::Left),
                inner_area,
            );
        }
        AppState::Fetching { branch, .. } => {
            let theme = CyberTheme::default();
            render_info_modal(
                f,
                standard_area,
                Line::from(vec![Span::styled(
                    format!(" {} FETCHING FROM REMOTE ", spinner),
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                )]),
                vec![Line::from(vec![
                    Span::raw("Fetching for: "),
                    Span::styled(
                        branch.as_str(),
                        Style::default()
                            .fg(theme.secondary)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])],
                Style::default().fg(theme.primary),
            );
        }
        AppState::Confirming { title, message, .. } => {
            let theme = CyberTheme::default();
            let area = centered_rect(60, 30, f.area());
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Thick)
                .border_style(Style::default().fg(theme.warning))
                .title(Span::styled(
                    title.as_str(),
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                ));

            let inner_area = block.inner(area);
            f.render_widget(block, area);

            let items = vec![
                Line::from(""),
                Line::from(vec![Span::styled(
                    format!("  {}  ", message),
                    Style::default().fg(theme.text),
                )]),
                Line::from(""),
                Line::from(vec![
                    Span::raw("  Confirm with "),
                    Span::styled(
                        "[ENTER/Y]",
                        Style::default()
                            .fg(theme.success)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" or Cancel with "),
                    Span::styled(
                        "[ESC/N]",
                        Style::default()
                            .fg(theme.error)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
            ];

            f.render_widget(
                Paragraph::new(items).alignment(Alignment::Center),
                inner_area,
            );
        }
        AppState::OpeningEditor { branch, editor, .. } => {
            let theme = CyberTheme::default();
            render_info_modal(
                f,
                standard_area,
                Line::from(vec![Span::styled(
                    format!(" {} OPENING IN EDITOR ", spinner),
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                )]),
                vec![
                    Line::from(vec![
                        Span::raw("Branch: "),
                        Span::styled(
                            branch.as_str(),
                            Style::default()
                                .fg(theme.secondary)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(vec![
                        Span::raw("Editor: "),
                        Span::styled(
                            editor.as_str(),
                            Style::default()
                                .fg(theme.accent)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]),
                ],
                Style::default().fg(theme.primary),
            );
        }
        AppState::Welcome => {
            let theme = CyberTheme::default();
            let welcome_text = vec![
                Line::from(""),
                Line::from(vec![Span::styled(
                    " 󰚚 SYSTEM OFFLINE: NO REPOSITORY DETECTED ",
                    Style::default()
                        .fg(RatatuiColor::Red)
                        .add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
                Line::from(vec![
                    Span::raw(" This tool is designed to manage worktrees in a "),
                    Span::styled(
                        "Bare Hub Architecture",
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("."),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::raw(" Press "),
                    Span::styled(
                        " [I] ",
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" to INITIALIZE a new repository from a URL"),
                ]),
                Line::from(vec![
                    Span::raw(" Press "),
                    Span::styled(
                        " [Q] ",
                        Style::default()
                            .fg(theme.error)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" to EXIT the manager"),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        " PRO-TIP: ",
                        Style::default()
                            .fg(theme.secondary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("You can also run 'worktrees init <url>' from your shell."),
                ]),
            ];
            let p = Paragraph::new(welcome_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Thick)
                        .border_style(Style::default().fg(theme.border))
                        .title(Span::styled(
                            " 󰙅 ONBOARDING PROTOCOL ",
                            Style::default().add_modifier(Modifier::BOLD),
                        )),
                )
                .alignment(Alignment::Center);
            f.render_widget(p, main_chunks[1]);
            f.render_widget(DetailsWidget::new(None, context), main_chunks[2]);
        }
        AppState::Error(msg, _) => {
            let table = WorktreeListWidget::new(&[]);
            f.render_stateful_widget(table, main_chunks[1], &mut TableState::default());
            f.render_widget(DetailsWidget::new(None, context), main_chunks[2]);

            let area = centered_rect(70, 50, f.area());
            f.render_widget(Clear, area);

            let mut error_lines = vec![
                Line::from(vec![Span::styled(
                    " 󰅚 SYSTEM ERROR ",
                    Style::default()
                        .fg(RatatuiColor::Red)
                        .add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
            ];

            // Wrap error message
            for line in msg.lines() {
                error_lines.push(Line::from(vec![
                    Span::styled("  ! ", Style::default().fg(RatatuiColor::Red)),
                    Span::raw(line),
                ]));
            }
            error_lines.push(Line::from(""));

            // Actionable Tips
            error_lines.push(Line::from(vec![Span::styled(
                " 💡 ACTIONABLE TIPS: ",
                Style::default()
                    .fg(RatatuiColor::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]));

            if msg.contains("API key not found") {
                error_lines.push(Line::from(
                    "  • Press [C] then select 'SET API KEY' to configure Gemini.",
                ));
            } else if msg.contains("network") || msg.contains("connection") {
                error_lines.push(Line::from(
                    "  • Check your internet connection and SSH/HTTP credentials.",
                ));
            } else if msg.contains("permission") || msg.contains("denied") {
                error_lines.push(Line::from(
                    "  • Ensure you have write access to the directory and repo.",
                ));
            } else if msg.contains("worktree") && msg.contains("exists") {
                error_lines.push(Line::from(
                    "  • Use a different name or remove the existing worktree first.",
                ));
            } else {
                error_lines.push(Line::from(
                    "  • Check 'git status' manually in the worktree directory.",
                ));
                error_lines.push(Line::from(
                    "  • Ensure no other processes are locking the git index.",
                ));
            }

            error_lines.push(Line::from(""));
            error_lines.push(Line::from(vec![
                Span::raw(" Press "),
                Span::styled(
                    "[ENTER/ESC/Q]",
                    Style::default()
                        .fg(RatatuiColor::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" to dismiss "),
            ]));

            let p = Paragraph::new(error_lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Thick)
                        .border_style(Style::default().fg(RatatuiColor::Red))
                        .title(Span::styled(
                            " 󰅚 ERROR ",
                            Style::default()
                                .fg(RatatuiColor::Red)
                                .add_modifier(Modifier::BOLD),
                        )),
                )
                .alignment(Alignment::Left);
            f.render_widget(p, area);
        }
        AppState::Initializing { .. }
        | AppState::AddingWorktree { .. }
        | AppState::SettingUpDefaults => {
            let theme = CyberTheme::default();
            let title_text = if matches!(state, AppState::Initializing { .. }) {
                "INITIALIZING REPOSITORY"
            } else if matches!(state, AppState::AddingWorktree { .. }) {
                "ADDING WORKTREE"
            } else {
                "CONFIGURING DEFAULTS"
            };

            render_info_modal(
                f,
                standard_area,
                Line::from(vec![Span::styled(
                    format!(" {} {} ", spinner, title_text),
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                )]),
                vec![],
                Style::default().fg(theme.primary),
            );
        }
        AppState::SetupComplete => {
            render_info_modal(
                f,
                standard_area,
                Line::from(vec![Span::styled(
                    " ✅ SETUP COMPLETE ",
                    Style::default()
                        .fg(RatatuiColor::Green)
                        .add_modifier(Modifier::BOLD),
                )]),
                vec![Line::from("Default worktrees 'main' and 'dev' are ready.")],
                Style::default(),
            );
        }
        _ => {}
    }
}

fn render_info_modal(
    f: &mut Frame,
    area: Rect,
    title: Line<'_>,
    details: Vec<Line<'_>>,
    border_style: Style,
) {
    f.render_widget(Clear, area);
    let mut lines = vec![title];
    if !details.is_empty() {
        lines.push(Line::from(""));
        lines.extend(details);
    }
    let p = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_style),
        )
        .alignment(Alignment::Center);
    f.render_widget(p, area);
}
