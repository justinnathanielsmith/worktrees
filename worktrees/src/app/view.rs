use crate::app::model::{AppState, EditorConfig, PromptType};
use crate::domain::repository::{ProjectRepository, Worktree};
use crate::ui::widgets::{
    details::DetailsWidget, footer::FooterWidget, header::HeaderWidget,
    worktree_list::WorktreeListWidget,
};
use anyhow::Result;
use owo_colors::OwoColorize;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color as RatatuiColor, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, TableState},
    Frame, Terminal,
};
use std::io;
use std::process::Command;
use std::time::Duration;
use comfy_table::Table;

pub struct View;

impl View {
    pub fn render_banner() {
        let lines = [
            r#"██╗    ██╗ ██████╗ ██████╗ ██╗  ██╗████████╗██████╗ ███████╗███████╗███████╗"#,
            r#"██║    ██║██╔═══██╗██╔══██╗██║ ██╔╝╚══██╔══╝██╔══██╗██╔════╝██╔════╝██╔════╝"#,
            r#"██║ █╗ ██║██║   ██║██████╔╝█████╔╝    ██║   ██████╔╝█████╗  █████╗  ███████╗"#,
            r#"██║███╗██║██║   ██║██╔══██╗██╔═██╗    ██║   ██╔══██╗██╔══╝  ██╔══╝  ╚════██║"#,
            r#"╚███╔███╔╝╚██████╔╝██║  ██║██║  ██╗   ██║   ██║  ██║███████╗███████╗███████║"#,
            r#" ╚══╝╚══╝  ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝   ╚═╝   ╚═╝  ╚═╝╚══════╝╚══════╝╚══════╝"#,
        ];

        let colors = [
            (6, 182, 212),   // Cyan
            (34, 158, 234),  // Sky-Blue
            (59, 130, 246),  // Blue
            (99, 102, 241),  // Indigo
            (139, 92, 246),  // Violet
            (168, 85, 247),  // Purple
        ];

        for (i, line) in lines.iter().enumerate() {
            let (r, g, b) = colors.get(i).unwrap_or(&(168, 85, 247));
            println!("{}", line.truecolor(*r, *g, *b).bold());
        }

        println!(
            "                    {}",
            "HI-RES WORKTREE INFRASTRUCTURE".truecolor(6, 182, 212).italic()
        );
        println!("{}\n", "━".repeat(76).truecolor(59, 130, 246).dimmed());
    }

    pub fn render_json<T: serde::Serialize>(data: &T) -> Result<()> {
        let json = serde_json::to_string_pretty(data)?;
        println!("{}", json);
        Ok(())
    }

    pub fn render_listing_table(worktrees: &[Worktree]) {
        let mut table = Table::new();
        table.set_header(vec!["Branch", "Commit", "Path", "Status"]);

        for wt in worktrees {
            let status = if wt.is_bare {
                "Bare"
            } else if wt.is_detached {
                "Detached"
            } else {
                "Active"
            };
            table.add_row(vec![&wt.branch, &wt.commit, &wt.path, status]);
        }

        println!("{}", table);
    }

    pub fn render_feedback_prompt() {
        println!("\n{}", "━".repeat(60).cyan().dimmed());
        println!(
            "{}",
            "Thank you for using the Worktree Manager.".bold()
        );
        println!(
            "{}",
            "Feedback: https://github.com/justin-smith/worktrees/issues"
                .blue()
                .underline()
        );
    }

    pub fn render_tui<R: ProjectRepository>(repo: &R, mut state: AppState) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let res = Self::run_loop(&mut terminal, repo, &mut state);

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        res
    }

    fn run_loop<R: ProjectRepository>(
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        repo: &R,
        state: &mut AppState,
    ) -> Result<()> {
        loop {
            if let AppState::ListingWorktrees { refresh_needed: true, .. } = state
                && let Ok(worktrees) = repo.list_worktrees() {
                    let mut table_state = TableState::default();
                    if !worktrees.is_empty() {
                        table_state.select(Some(0));
                    }
                    *state = AppState::ListingWorktrees {
                        worktrees,
                        table_state,
                        refresh_needed: false,
                    };
                }

            terminal.draw(|f| Self::draw(f, repo, state))?;

            if event::poll(Duration::from_millis(100))?
                && let Event::Key(key) = event::read()? {
                    match state {
                        AppState::ListingWorktrees { worktrees, table_state, .. } => {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                                KeyCode::Down | KeyCode::Char('j') => {
                                    Self::move_selection(table_state, worktrees.len(), 1);
                                }
                                KeyCode::Up | KeyCode::Char('k') => {
                                    Self::move_selection(table_state, worktrees.len(), -1);
                                }
                                KeyCode::Char('a') => {
                                    *state = AppState::Prompting {
                                        prompt_type: PromptType::AddIntent,
                                        input: String::new(),
                                        prev_state: Box::new(state.clone()),
                                    };
                                }
                                KeyCode::Char('d') | KeyCode::Char('x') => {
                                    if let Some(i) = table_state.selected()
                                        && let Some(wt) = worktrees.get(i).filter(|wt| !wt.is_bare) {
                                            let _ = repo.remove_worktree(&wt.branch, false);
                                            state.request_refresh();
                                        }
                                }
                                KeyCode::Char('s') => {
                                    if let Some(i) = table_state.selected()
                                        && let Some(wt) = worktrees.get(i).filter(|wt| !wt.is_bare) {
                                            let branch = wt.branch.clone();
                                            let path = wt.path.clone();
                                            *state = AppState::Syncing { branch: branch.clone() };
                                            terminal.draw(|f| Self::draw(f, repo, state))?;
                                            let _ = repo.sync_configs(&path);
                                            *state = AppState::SyncComplete { branch: branch.clone() };
                                            terminal.draw(|f| Self::draw(f, repo, state))?;
                                            std::thread::sleep(Duration::from_millis(800));
                                            state.request_refresh();
                                        }
                                }
                                KeyCode::Char('S') => {
                                    let targets: Vec<(String, String)> = worktrees.iter()
                                        .filter(|wt| !wt.is_bare)
                                        .map(|wt| (wt.branch.clone(), wt.path.clone()))
                                        .collect();
                                    
                                    *state = AppState::Syncing { branch: "ALL".to_string() };
                                    terminal.draw(|f| Self::draw(f, repo, state))?;
                                    for (_, path) in targets {
                                        let _ = repo.sync_configs(&path);
                                    }
                                    *state = AppState::SyncComplete { branch: "ALL".to_string() };
                                    terminal.draw(|f| Self::draw(f, repo, state))?;
                                    std::thread::sleep(Duration::from_millis(800));
                                    state.request_refresh();
                                }
                                KeyCode::Char('o') => {
                                    if let Some(i) = table_state.selected()
                                        && let Some(wt) = worktrees.get(i).filter(|wt| !wt.is_bare) {
                                            let branch = wt.branch.clone();
                                            let path = wt.path.clone();
                                            
                                            if let Ok(Some(editor)) = repo.get_preferred_editor() {
                                                *state = AppState::OpeningEditor { 
                                                    branch: branch.clone(), 
                                                    editor: editor.clone() 
                                                };
                                                terminal.draw(|f| Self::draw(f, repo, state))?;
                                                let _ = Command::new(&editor).arg(&path).spawn();
                                                std::thread::sleep(Duration::from_millis(800));
                                                state.request_refresh();
                                            } else {
                                                let options = vec![
                                                    EditorConfig { name: "VS Code".into(), command: "code".into() },
                                                    EditorConfig { name: "Cursor".into(), command: "cursor".into() },
                                                    EditorConfig { name: "Zed".into(), command: "zed".into() },
                                                    EditorConfig { name: "Android Studio".into(), command: "studio".into() },
                                                    EditorConfig { name: "IntelliJ IDEA".into(), command: "idea".into() },
                                                    EditorConfig { name: "Vim".into(), command: "vim".into() },
                                                    EditorConfig { name: "Neovim".into(), command: "nvim".into() },
                                                    EditorConfig { name: "Antigravity".into(), command: "antigravity".into() },
                                                ];
                                                *state = AppState::SelectingEditor {
                                                    branch,
                                                    options,
                                                    selected: 0,
                                                    prev_state: Box::new(state.clone()),
                                                };
                                            }
                                        }
                                }
                                KeyCode::Char('O') => {
                                    if let Some(i) = table_state.selected()
                                        && let Some(wt) = worktrees.get(i).filter(|wt| !wt.is_bare) {
                                            let options = vec![
                                                EditorConfig { name: "VS Code".into(), command: "code".into() },
                                                EditorConfig { name: "Cursor".into(), command: "cursor".into() },
                                                EditorConfig { name: "Zed".into(), command: "zed".into() },
                                                EditorConfig { name: "Android Studio".into(), command: "studio".into() },
                                                EditorConfig { name: "IntelliJ IDEA".into(), command: "idea".into() },
                                                EditorConfig { name: "Vim".into(), command: "vim".into() },
                                                EditorConfig { name: "Neovim".into(), command: "nvim".into() },
                                                EditorConfig { name: "Antigravity".into(), command: "antigravity".into() },
                                            ];
                                            *state = AppState::SelectingEditor {
                                                branch: wt.branch.clone(),
                                                options,
                                                selected: 0,
                                                prev_state: Box::new(state.clone()),
                                            };
                                        }
                                }
                                KeyCode::Char('u') => {
                                    let _ = repo.add_worktree("main", "main");
                                    let _ = repo.add_worktree("dev", "dev");
                                    state.request_refresh();
                                }
                                _ => {}
                            }
                        }
                        AppState::SelectingEditor { branch, options, selected, prev_state } => {
                            match key.code {
                                KeyCode::Up | KeyCode::Char('k') => {
                                    if *selected > 0 { *selected -= 1; }
                                    else { *selected = options.len() - 1; }
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    if *selected < options.len() - 1 { *selected += 1; }
                                    else { *selected = 0; }
                                }
                                KeyCode::Enter => {
                                    let editor = options[*selected].command.clone();
                                    let _ = repo.set_preferred_editor(&editor);
                                    
                                    // Now actually open it
                                    let path = if let AppState::ListingWorktrees { worktrees, table_state, .. } = &**prev_state {
                                        table_state.selected().and_then(|i| worktrees.get(i)).map(|wt| wt.path.clone())
                                    } else { None };

                                    if let Some(p) = path {
                                        *state = AppState::OpeningEditor { 
                                            branch: branch.clone(), 
                                            editor: editor.clone() 
                                        };
                                        terminal.draw(|f| Self::draw(f, repo, state))?;
                                        let _ = Command::new(&editor).arg(&p).spawn();
                                        std::thread::sleep(Duration::from_millis(800));
                                    }
                                    state.request_refresh();
                                }
                                KeyCode::Esc => {
                                    *state = *prev_state.clone();
                                }
                                _ => {}
                            }
                        }
                        AppState::Prompting { prompt_type, input, prev_state } => {
                            match key.code {
                                KeyCode::Enter => {
                                    let val = input.trim().to_string();
                                    if !val.is_empty() {
                                        match prompt_type {
                                            PromptType::AddIntent => {
                                                let _ = repo.add_worktree(&val, &val);
                                            }
                                            PromptType::InitUrl => {
                                                let _ = repo.init_bare_repo(&val, "project");
                                            }
                                        }
                                    }
                                    *state = AppState::ListingWorktrees {
                                        worktrees: Vec::new(),
                                        table_state: TableState::default(),
                                        refresh_needed: true,
                                    };
                                }
                                KeyCode::Esc => {
                                    *state = *prev_state.clone();
                                }
                                KeyCode::Char(c) => input.push(c),
                                KeyCode::Backspace => { input.pop(); }
                                _ => {}
                            }
                        }
                        AppState::Welcome => {
                             match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                                KeyCode::Char('i') => {
                                    *state = AppState::Prompting {
                                        prompt_type: PromptType::InitUrl,
                                        input: String::new(),
                                        prev_state: Box::new(AppState::Welcome),
                                    };
                                }
                                _ => {}
                             }
                        }
                        _ => {
                            if let KeyCode::Char('q') | KeyCode::Esc = key.code {
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }

    fn move_selection(state: &mut TableState, len: usize, delta: isize) {
        if len == 0 { return; }
        let i = match state.selected() {
            Some(i) => {
                let next = i as isize + delta;
                if next < 0 {
                    len - 1
                } else if next >= len as isize {
                    0
                } else {
                    next as usize
                }
            }
            None => 0,
        };
        state.select(Some(i));
    }

    fn draw<R: ProjectRepository>(f: &mut Frame, repo: &R, state: &mut AppState) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Length(3), // Header
                    Constraint::Min(5),    // Table
                    Constraint::Length(6), // Details
                    Constraint::Length(3), // Footer
                ]
                .as_ref(),
            )
            .split(f.area());

        let context = repo.detect_context();
        f.render_widget(HeaderWidget { context }, chunks[0]);

        match state {
            AppState::ListingWorktrees {
                worktrees,
                table_state,
                ..
            } => {
                let table = WorktreeListWidget::new(worktrees);
                f.render_stateful_widget(table, chunks[1], table_state);

                let selected_worktree = table_state
                    .selected()
                    .and_then(|i| worktrees.get(i));
                
                f.render_widget(DetailsWidget::new(selected_worktree, context), chunks[2]);
            }
            AppState::Prompting { prompt_type, input, .. } => {
                let table = WorktreeListWidget::new(&[]);
                f.render_stateful_widget(table, chunks[1], &mut TableState::default());
                f.render_widget(DetailsWidget::new(None, context), chunks[2]);

                let area = centered_rect(60, 20, f.area());
                f.render_widget(Clear, area);
                
                let title = match prompt_type {
                    PromptType::AddIntent => " ADD NEW WORKTREE (Name) ",
                    PromptType::InitUrl => " INITIALIZE REPOSITORY (URL) ",
                };

                let input_widget = Paragraph::new(Span::styled(
                    format!(" > {}_", input),
                    Style::default().fg(RatatuiColor::Cyan).add_modifier(Modifier::BOLD)
                ))
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(Span::styled(title, Style::default().add_modifier(Modifier::BOLD))));
                
                f.render_widget(input_widget, area);
            }
            AppState::SelectingEditor { options, selected, .. } => {
                let table = WorktreeListWidget::new(&[]);
                f.render_stateful_widget(table, chunks[1], &mut TableState::default());
                f.render_widget(DetailsWidget::new(None, context), chunks[2]);

                let area = centered_rect(60, 40, f.area());
                f.render_widget(Clear, area);
                
                let items: Vec<Line> = options.iter().enumerate().map(|(i, opt)| {
                    if i == *selected {
                        Line::from(vec![
                            Span::styled(" > ", Style::default().fg(RatatuiColor::Cyan).add_modifier(Modifier::BOLD)),
                            Span::styled(&opt.name, Style::default().fg(RatatuiColor::Cyan).add_modifier(Modifier::BOLD)),
                            Span::styled(format!(" ({})", opt.command), Style::default().fg(RatatuiColor::DarkGray)),
                        ])
                    } else {
                        Line::from(vec![
                            Span::raw("   "),
                            Span::raw(&opt.name),
                            Span::styled(format!(" ({})", opt.command), Style::default().fg(RatatuiColor::DarkGray)),
                        ])
                    }
                }).collect();

                let p = Paragraph::new(items)
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .title(Span::styled(" SELECT PREFERRED EDITOR ", Style::default().add_modifier(Modifier::BOLD))))
                    .alignment(Alignment::Left);
                
                f.render_widget(p, area);
            }
            AppState::Syncing { branch } => {
                let table = WorktreeListWidget::new(&[]);
                f.render_stateful_widget(table, chunks[1], &mut TableState::default());
                f.render_widget(DetailsWidget::new(None, context), chunks[2]);

                let area = centered_rect(60, 20, f.area());
                f.render_widget(Clear, area);
                
                let p = Paragraph::new(vec![
                    Line::from(vec![Span::styled(" SYNCING CONFIGURATIONS ", Style::default().fg(RatatuiColor::Cyan).add_modifier(Modifier::BOLD))]),
                    Line::from(""),
                    Line::from(vec![Span::raw("Target: "), Span::styled(branch.as_str(), Style::default().fg(RatatuiColor::Magenta).add_modifier(Modifier::BOLD))]),
                ])
                .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded))
                .alignment(Alignment::Center);
                f.render_widget(p, area);
            }
            AppState::SyncComplete { branch } => {
                let table = WorktreeListWidget::new(&[]);
                f.render_stateful_widget(table, chunks[1], &mut TableState::default());
                f.render_widget(DetailsWidget::new(None, context), chunks[2]);

                let area = centered_rect(60, 20, f.area());
                f.render_widget(Clear, area);
                
                let p = Paragraph::new(vec![
                    Line::from(vec![Span::styled(" SYNC COMPLETE ", Style::default().fg(RatatuiColor::Green).add_modifier(Modifier::BOLD))]),
                    Line::from(""),
                    Line::from(vec![Span::raw("Successfully synced: "), Span::styled(branch.as_str(), Style::default().fg(RatatuiColor::Magenta).add_modifier(Modifier::BOLD))]),
                ])
                .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded))
                .alignment(Alignment::Center);
                f.render_widget(p, area);
            }
            AppState::OpeningEditor { branch, editor } => {
                let table = WorktreeListWidget::new(&[]);
                f.render_stateful_widget(table, chunks[1], &mut TableState::default());
                f.render_widget(DetailsWidget::new(None, context), chunks[2]);

                let area = centered_rect(60, 20, f.area());
                f.render_widget(Clear, area);
                
                let p = Paragraph::new(vec![
                    Line::from(vec![Span::styled(" OPENING IN EDITOR ", Style::default().fg(RatatuiColor::Cyan).add_modifier(Modifier::BOLD))]),
                    Line::from(""),
                    Line::from(vec![Span::raw("Branch: "), Span::styled(branch.as_str(), Style::default().fg(RatatuiColor::Magenta).add_modifier(Modifier::BOLD))]),
                    Line::from(vec![Span::raw("Editor: "), Span::styled(editor.as_str(), Style::default().fg(RatatuiColor::Yellow).add_modifier(Modifier::BOLD))]),
                ])
                .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded))
                .alignment(Alignment::Center);
                f.render_widget(p, area);
            }
            AppState::Welcome => {
                let welcome_text = vec![
                    Line::from(vec![Span::styled("NO BARE REPOSITORY DETECTED", Style::default().fg(RatatuiColor::Red).add_modifier(Modifier::BOLD))]),
                    Line::from(""),
                    Line::from(vec![Span::raw("Press "), Span::styled("'i'", Style::default().fg(RatatuiColor::Cyan).add_modifier(Modifier::BOLD)), Span::raw(" to initialize a new repository")]),
                    Line::from(vec![Span::raw("Press "), Span::styled("'q'", Style::default().fg(RatatuiColor::Red).add_modifier(Modifier::BOLD)), Span::raw(" to exit")]),
                ];
                let p = Paragraph::new(welcome_text)
                    .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded))
                    .alignment(Alignment::Center);
                f.render_widget(p, chunks[1]);
                f.render_widget(DetailsWidget::new(None, context), chunks[2]);
            }
            _ => {}
        }

        f.render_widget(FooterWidget, chunks[3]);
    }

    pub fn render(state: AppState) {
        match state {
            AppState::Initializing { project_name } => {
                println!(
                    "{} {} [{} {}]",
                    "🚀".blue(),
                    format!("INITIALIZING BARE REPOSITORY: {}", project_name)
                        .blue()
                        .bold(),
                    "STATUS:".dimmed(),
                    "PREPARING".purple()
                );
            }
            AppState::Initialized { project_name } => {
                println!(
                    "\n{} {}",
                    "✅".green(),
                    "BARE REPOSITORY ESTABLISHED".green().bold()
                );
                println!(
                    "   {} {}",
                    "├─ Location:".dimmed(),
                    format!("{}/.bare", project_name).white()
                );
                println!(
                    "   {} {}",
                    "└─ Action:  ".dimmed(),
                    format!("cd {} && wt setup", project_name).blue().bold()
                );
            }
            AppState::AddingWorktree { intent, branch } => {
                println!(
                    "{} {} [{} {}]",
                    "📁".purple(),
                    format!("ADDING WORKTREE: {} (branch: {})", intent, branch)
                        .purple()
                        .bold(),
                    "STATUS:".dimmed(),
                    "CREATING".blue()
                );
            }
            AppState::WorktreeAdded { intent } => {
                println!(
                    "   {} {} {}",
                    "┗━".dimmed(),
                    "SUCCESS:".green().bold(),
                    format!("Worktree active at ./{}", intent).white()
                );
            }
            AppState::RemovingWorktree { intent } => {
                println!(
                    "{} {} [{} {}]",
                    "🔥".red(),
                    format!("REMOVING WORKTREE: {}", intent).red().bold(),
                    "STATUS:".dimmed(),
                    "DELETING".purple()
                );
            }
            AppState::WorktreeRemoved => {
                println!(
                    "   {} {}",
                    "┗━".dimmed(),
                    "WORKTREE REMOVED".green().bold()
                );
            }
            AppState::Syncing { branch } => {
                println!(
                    "{} {} [{} {}]",
                    "🔄".cyan(),
                    format!("SYNCING CONFIGURATIONS: {}", branch).cyan().bold(),
                    "STATUS:".dimmed(),
                    "SYNCHRONIZING".yellow()
                );
            }
            AppState::SyncComplete { branch } => {
                println!(
                    "   {} {} {}",
                    "┗━".dimmed(),
                    "SUCCESS:".green().bold(),
                    format!("Synced configurations for {}", branch).white()
                );
            }
            AppState::SelectingEditor { branch, .. } => {
                println!(
                    "{} {} [{} {}]",
                    "🔍".cyan(),
                    format!("SELECTING EDITOR FOR: {}", branch).cyan().bold(),
                    "STATUS:".dimmed(),
                    "PENDING SELECTION".yellow()
                );
            }
            AppState::OpeningEditor { branch, editor } => {
                println!(
                    "{} {} [{} {}]",
                    "📝".yellow(),
                    format!("OPENING WORKTREE: {}", branch).yellow().bold(),
                    "EDITOR:".dimmed(),
                    editor.purple().bold()
                );
            }
            AppState::ListingWorktrees { worktrees, .. } => {
                // Fallback / Log view
                println!(
                    "{} {} [{} {}]",
                    "📋".blue(),
                    "ACTIVE WORKTREES".blue().bold(),
                    "TOTAL:".dimmed(),
                    worktrees.len().to_string().purple().bold()
                );
            }
            AppState::SettingUpDefaults => {
                println!(
                    "{} {}",
                    "⚡".purple(),
                    "SETTING UP DEFAULT WORKTREES".purple().bold()
                );
            }
            AppState::SetupComplete => {
                println!(
                    "\n{} {}",
                    "🚀".blue(),
                    "SETUP COMPLETE.".blue().bold()
                );
                println!(
                    "   {}",
                    "All default worktrees have been created."
                        .dimmed()
                );
            }
            AppState::Error(msg) => {
                eprintln!("\n{} {} {}", "❌".red(), "ERROR:".red().bold(), msg.red());
                eprintln!(
                    "   {} {}",
                    "└─".dimmed(),
                    "Check git state and permissions.".dimmed()
                );
            }
            AppState::Welcome | AppState::Prompting { .. } => {
                // These are handled by render_tui, no-op for CLI log view
            }
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
