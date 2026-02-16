use crate::domain::repository::{ProjectContext, Worktree};
use crate::ui::theme::CyberTheme;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};

pub struct DetailsWidget<'a> {
    worktree: Option<&'a Worktree>,
    all_worktrees: &'a [Worktree],
    context: ProjectContext,
}

impl<'a> DetailsWidget<'a> {
    pub fn new(
        worktree: Option<&'a Worktree>,
        all_worktrees: &'a [Worktree],
        context: ProjectContext,
    ) -> Self {
        Self {
            worktree,
            all_worktrees,
            context,
        }
    }
}

impl<'a> Widget for DetailsWidget<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let theme = CyberTheme::default();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(Span::styled(
                " WORKTREE // DETAILS ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ));

        let details_text = if let Some(wt) = self.worktree {
            let mut lines = Vec::new();

            if wt.is_bare {
                lines.clear();
                lines.push(Line::from(Span::styled(
                    "HUB PROJECT OVERVIEW ─────",
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                )));

                let active_wt = self
                    .all_worktrees
                    .iter()
                    .filter(|w| !w.is_bare)
                    .collect::<Vec<_>>();
                let dirty_count = active_wt
                    .iter()
                    .filter(|w| {
                        w.status_summary
                            .as_ref()
                            .map(|s| s != "clean")
                            .unwrap_or(false)
                    })
                    .count();
                let clean_count = active_wt.len() - dirty_count;

                lines.push(Line::from(vec![
                    Span::styled(" TOTAL WORKTREES : ", Style::default().fg(theme.secondary)),
                    Span::styled(
                        format!("{}", active_wt.len()),
                        Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::styled(" CLEAN ENV      : ", Style::default().fg(theme.secondary)),
                    Span::styled(
                        format!("{}", clean_count),
                        Style::default().fg(theme.success),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::styled(" DIRTY ENV      : ", Style::default().fg(theme.secondary)),
                    Span::styled(
                        format!("{}", dirty_count),
                        Style::default().fg(if dirty_count > 0 {
                            theme.warning
                        } else {
                            theme.subtle
                        }),
                    ),
                ]));

                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "CONFIGURATION ──────────",
                    Style::default().fg(theme.subtle),
                )));
                lines.push(Line::from(vec![
                    Span::styled(" PROJECT HUB    : ", Style::default().fg(theme.secondary)),
                    Span::styled(
                        &wt.path,
                        Style::default()
                            .fg(theme.subtle)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]));
                if self.context == ProjectContext::KmpAndroid {
                    lines.push(Line::from(vec![
                        Span::styled(" CONTEXT        : ", Style::default().fg(theme.secondary)),
                        Span::styled("KMP / ANDROID", Style::default().fg(theme.accent)),
                    ]));
                }

                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "SYSTEM ACTIONS ─────────",
                    Style::default().fg(theme.subtle),
                )));
                lines.push(Line::from(vec![
                    Span::styled(
                        " [O] ",
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("Open Project Root", Style::default().fg(theme.text)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled(
                        " [F] ",
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("Global Fetch", Style::default().fg(theme.text)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled(
                        " [C] ",
                        Style::default()
                            .fg(theme.secondary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("Prune Stale Metadata", Style::default().fg(theme.text)),
                ]));
            } else {
                // -- SECTION 1: IDENTITY --
                lines.push(Line::from(Span::styled(
                    "IDENTITY ───────────────",
                    Style::default().fg(theme.subtle),
                )));
                lines.push(Line::from(vec![
                    Span::styled(" INTENT : ", Style::default().fg(theme.secondary)),
                    Span::styled(
                        std::path::Path::new(&wt.path)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "UNKNOWN".to_string()),
                        Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
                    ),
                ]));
                if let Some(meta) = &wt.metadata
                    && let Some(purpose) = &meta.purpose
                {
                    lines.push(Line::from(vec![
                        Span::styled(" PURPOSE: ", Style::default().fg(theme.secondary)),
                        Span::styled(purpose, Style::default().fg(theme.primary)),
                    ]));
                }
                lines.push(Line::from(vec![
                    Span::styled(" BRANCH : ", Style::default().fg(theme.secondary)),
                    Span::styled(&wt.branch, Style::default().fg(theme.text)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled(" COMMIT : ", Style::default().fg(theme.secondary)),
                    Span::styled(&wt.commit, Style::default().fg(theme.subtle)),
                ]));
                lines.push(Line::from(""));

                // -- SECTION 2: STATUS --
                lines.push(Line::from(Span::styled(
                    "STATUS ─────────────────",
                    Style::default().fg(theme.subtle),
                )));

                let type_str = if wt.is_bare {
                    "BARE (MAIN)"
                } else if wt.is_detached {
                    "DETACHED HEAD"
                } else {
                    "STANDARD WORKTREE"
                };
                let type_color = if wt.is_bare {
                    theme.primary
                } else if wt.is_detached {
                    theme.error
                } else {
                    theme.success
                };

                lines.push(Line::from(vec![
                    Span::styled(" TYPE   : ", Style::default().fg(theme.secondary)),
                    Span::styled(
                        type_str,
                        Style::default().fg(type_color).add_modifier(Modifier::BOLD),
                    ),
                ]));

                if let Some(summary) = &wt.status_summary {
                    let (color, icon) = if summary == "clean" {
                        (theme.success, "✔")
                    } else {
                        (theme.warning, "⚠")
                    };
                    lines.push(Line::from(vec![
                        Span::styled(" STATE  : ", Style::default().fg(theme.secondary)),
                        Span::styled(
                            format!("{} {}", icon, summary.to_uppercase()),
                            Style::default().fg(color),
                        ),
                    ]));
                }
                lines.push(Line::from(""));

                // -- SECTION 3: PHYSICAL --
                lines.push(Line::from(Span::styled(
                    "PHYSICAL ───────────────",
                    Style::default().fg(theme.subtle),
                )));
                lines.push(Line::from(vec![
                    Span::styled(" PATH   : ", Style::default().fg(theme.secondary)),
                    Span::styled(
                        &wt.path,
                        Style::default()
                            .fg(theme.subtle)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]));

                if self.context == ProjectContext::KmpAndroid {
                    lines.push(Line::from(vec![
                        Span::styled(" OPTIM  : ", Style::default().fg(theme.secondary)),
                        Span::styled("GRADLE_CACHE_ACTIVE", Style::default().fg(theme.success)),
                    ]));
                }

                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "ACTIONS ────────────────",
                    Style::default().fg(theme.subtle),
                )));
                lines.push(Line::from(vec![
                    Span::styled(
                        " [ENT] ",
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("Open in Editor", Style::default().fg(theme.subtle)),
                ]));
                if !wt.is_bare {
                    lines.push(Line::from(vec![
                        Span::styled(
                            " [D/X] ",
                            Style::default()
                                .fg(theme.error)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled("Delete Worktree", Style::default().fg(theme.subtle)),
                    ]));
                }
            }

            lines
        } else {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    " NO WORKTREE SELECTED ",
                    Style::default()
                        .fg(theme.subtle)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Use ", Style::default().fg(theme.subtle)),
                    Span::styled("j/k", Style::default().fg(theme.primary)),
                    Span::styled(" to navigate the list.", Style::default().fg(theme.subtle)),
                ]),
            ]
        };

        Paragraph::new(details_text).block(block).render(area, buf);
    }
}
