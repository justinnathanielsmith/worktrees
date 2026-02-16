use crate::app::model::{AppState, EditorConfig, PromptType};
use crate::domain::repository::{ProjectRepository, Worktree};
use crate::ui::theme::CyberTheme;
use crate::ui::widgets::{
    details::DetailsWidget, footer::FooterWidget, header::HeaderWidget,
    worktree_list::WorktreeListWidget,
};
use anyhow::Result;
use comfy_table::Table;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use owo_colors::OwoColorize;
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color as RatatuiColor, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, TableState},
};
use std::io;
use std::process::Command;
use std::time::Duration;

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
            (6, 182, 212),  // Cyan
            (34, 158, 234), // Sky-Blue
            (59, 130, 246), // Blue
            (99, 102, 241), // Indigo
            (139, 92, 246), // Violet
            (168, 85, 247), // Purple
        ];

        for (i, line) in lines.iter().enumerate() {
            let (r, g, b) = colors.get(i).unwrap_or(&(168, 85, 247));
            println!("{}", line.truecolor(*r, *g, *b).bold());
        }

        println!(
            "                    {}",
            "HI-RES WORKTREE INFRASTRUCTURE"
                .truecolor(6, 182, 212)
                .italic()
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
        println!("{}", "Thank you for using the Worktree Manager.".bold());
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
            if let AppState::ListingWorktrees {
                refresh_needed: true,
                ..
            } = state
                && let Ok(worktrees) = repo.list_worktrees()
            {
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
                && let Event::Key(key) = event::read()?
            {
                let mut new_state = None;
                let current_state_clone = state.clone();
                match state {
                    AppState::ListingWorktrees {
                        worktrees,
                        table_state,
                        ..
                    } => {
                        new_state = Self::handle_listing_events(
                            key.code,
                            repo,
                            terminal,
                            worktrees,
                            table_state,
                            &current_state_clone,
                        )?;
                    }
                    AppState::ViewingStatus {
                        path,
                        branch,
                        status,
                        prev_state,
                        ..
                    } => {
                        new_state = Self::handle_status_events(
                            key.code,
                            repo,
                            path,
                            branch,
                            status,
                            prev_state,
                            &current_state_clone,
                        )?;
                    }
                    AppState::ViewingHistory {
                        commits,
                        selected_index,
                        prev_state,
                        ..
                    } => {
                        new_state = Self::handle_history_events(
                            key.code,
                            commits,
                            selected_index,
                            prev_state,
                        )?;
                    }
                    AppState::SwitchingBranch {
                        path,
                        branches,
                        selected_index,
                        prev_state,
                    } => {
                        new_state = Self::handle_branch_events(
                            key.code,
                            repo,
                            path,
                            branches,
                            selected_index,
                            prev_state,
                        )?;
                    }
                    AppState::Committing {
                        path,
                        branch,
                        selected_index,
                        prev_state,
                    } => {
                        new_state = Self::handle_committing_events(
                            key.code,
                            repo,
                            path,
                            branch,
                            selected_index,
                            prev_state,
                            &current_state_clone,
                        )?;
                    }
                    AppState::SelectingEditor {
                        branch,
                        options,
                        selected,
                        prev_state,
                    } => {
                        new_state = Self::handle_editor_events(
                            key.code, repo, terminal, branch, options, selected, prev_state,
                        )?;
                    }
                    AppState::Syncing { prev_state, .. }
                    | AppState::SyncComplete { prev_state, .. }
                    | AppState::OpeningEditor { prev_state, .. } => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => {
                            new_state = Some(*prev_state.clone());
                        }
                        _ => {}
                    },
                    AppState::Prompting {
                        prompt_type,
                        input,
                        prev_state,
                    } => {
                        new_state = Self::handle_prompt_events(
                            key.code,
                            repo,
                            prompt_type,
                            input,
                            prev_state,
                        )?;
                    }
                    AppState::Welcome => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        KeyCode::Char('i') => {
                            new_state = Some(AppState::Prompting {
                                prompt_type: PromptType::InitUrl,
                                input: String::new(),
                                prev_state: Box::new(AppState::Welcome),
                            });
                        }
                        _ => {}
                    },
                    _ => {
                        if let KeyCode::Char('q') | KeyCode::Esc = key.code {
                            return Ok(());
                        }
                    }
                }
                if let Some(ns) = new_state {
                    if matches!(ns, AppState::Exiting) {
                        return Ok(());
                    }
                    *state = ns;
                    state.request_refresh();
                }
            }
        }
    }

    fn move_selection(state: &mut TableState, len: usize, delta: isize) {
        if len == 0 {
            return;
        }
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

    fn handle_listing_events<R: ProjectRepository>(
        key_code: KeyCode,
        repo: &R,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        worktrees: &[Worktree],
        table_state: &mut TableState,
        current_state: &AppState,
    ) -> Result<Option<AppState>> {
        let normalized_code = match key_code {
            KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
            _ => key_code,
        };

        match normalized_code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(Some(AppState::Exiting)),
            KeyCode::Down | KeyCode::Char('j') => {
                Self::move_selection(table_state, worktrees.len(), 1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                Self::move_selection(table_state, worktrees.len(), -1);
            }
            KeyCode::Char('a') => {
                return Ok(Some(AppState::Prompting {
                    prompt_type: PromptType::AddIntent,
                    input: String::new(),
                    prev_state: Box::new(current_state.clone()),
                }));
            }
            KeyCode::Char('d') | KeyCode::Char('x') => {
                if let Some(i) = table_state.selected()
                    && let Some(wt) = worktrees.get(i).filter(|wt| !wt.is_bare)
                {
                    let _ = repo.remove_worktree(&wt.branch, false);
                    return Ok(Some(current_state.clone())); // Refresh signal
                }
            }
            KeyCode::Char('s') => {
                if let Some(i) = table_state.selected()
                    && let Some(wt) = worktrees.get(i).filter(|wt| !wt.is_bare)
                {
                    let branch = wt.branch.clone();
                    let path = wt.path.clone();
                    let prev = Box::new(current_state.clone());
                    let syncing_state = AppState::Syncing {
                        branch: branch.clone(),
                        prev_state: prev.clone(),
                    };
                    terminal.draw(|f| Self::draw(f, repo, &mut syncing_state.clone()))?;
                    let _ = repo.sync_configs(&path);
                    let complete_state = AppState::SyncComplete {
                        branch: branch.clone(),
                        prev_state: prev,
                    };
                    terminal.draw(|f| Self::draw(f, repo, &mut complete_state.clone()))?;
                    std::thread::sleep(Duration::from_millis(800));
                    return Ok(Some(complete_state.prev_state_boxed().clone()));
                }
            }
            KeyCode::Char('o') => {
                if let Some(i) = table_state.selected()
                    && let Some(wt) = worktrees.get(i).filter(|wt| !wt.is_bare)
                {
                    let branch = wt.branch.clone();
                    let path = wt.path.clone();
                    let prev = Box::new(current_state.clone());

                    if let Ok(Some(editor)) = repo.get_preferred_editor() {
                        let mut opening_state = AppState::OpeningEditor {
                            branch: branch.clone(),
                            editor: editor.clone(),
                            prev_state: prev.clone(),
                        };
                        terminal.draw(|f| Self::draw(f, repo, &mut opening_state))?;
                        let _ = Command::new(&editor).arg(&path).spawn();
                        std::thread::sleep(Duration::from_millis(800));
                        return Ok(Some(*prev));
                    } else {
                        let options = vec![
                            EditorConfig {
                                name: "VS Code".into(),
                                command: "code".into(),
                            },
                            EditorConfig {
                                name: "Cursor".into(),
                                command: "cursor".into(),
                            },
                            EditorConfig {
                                name: "Zed".into(),
                                command: "zed".into(),
                            },
                            EditorConfig {
                                name: "Android Studio".into(),
                                command: "studio".into(),
                            },
                            EditorConfig {
                                name: "IntelliJ IDEA".into(),
                                command: "idea".into(),
                            },
                            EditorConfig {
                                name: "Vim".into(),
                                command: "vim".into(),
                            },
                            EditorConfig {
                                name: "Neovim".into(),
                                command: "nvim".into(),
                            },
                            EditorConfig {
                                name: "Antigravity".into(),
                                command: "antigravity".into(),
                            },
                        ];
                        return Ok(Some(AppState::SelectingEditor {
                            branch,
                            options,
                            selected: 0,
                            prev_state: prev,
                        }));
                    }
                }
            }
            KeyCode::Char('u') => {
                let _ = repo.add_worktree("main", "main");
                let _ = repo.add_worktree("dev", "dev");
                return Ok(Some(current_state.clone()));
            }
            KeyCode::Char('g') => {
                if let Some(i) = table_state.selected()
                    && let Some(wt) = worktrees.get(i).filter(|wt| !wt.is_bare)
                    && let Ok(status) = repo.get_status(&wt.path)
                {
                    return Ok(Some(AppState::ViewingStatus {
                        path: wt.path.clone(),
                        branch: wt.branch.clone(),
                        status: crate::app::model::StatusViewState {
                            staged: status.staged,
                            unstaged: status.unstaged,
                            untracked: status.untracked,
                            selected_index: 0,
                        },
                        prev_state: Box::new(current_state.clone()),
                    }));
                }
            }
            KeyCode::Char('l') => {
                if let Some(i) = table_state.selected()
                    && let Some(wt) = worktrees.get(i).filter(|wt| !wt.is_bare)
                    && let Ok(commits) = repo.get_history(&wt.path, 50)
                {
                    return Ok(Some(AppState::ViewingHistory {
                        branch: wt.branch.clone(),
                        commits,
                        selected_index: 0,
                        prev_state: Box::new(current_state.clone()),
                    }));
                }
            }
            KeyCode::Char('b') => {
                if let Some(i) = table_state.selected()
                    && let Some(wt) = worktrees.get(i).filter(|wt| !wt.is_bare)
                    && let Ok(branches) = repo.list_branches()
                {
                    return Ok(Some(AppState::SwitchingBranch {
                        path: wt.path.clone(),
                        branches,
                        selected_index: 0,
                        prev_state: Box::new(current_state.clone()),
                    }));
                }
            }
            KeyCode::Char('f') => {
                if let Some(i) = table_state.selected()
                    && let Some(wt) = worktrees.get(i).filter(|wt| !wt.is_bare)
                {
                    let _ = repo.fetch(&wt.path);
                    return Ok(Some(current_state.clone()));
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn handle_status_events<R: ProjectRepository>(
        key_code: KeyCode,
        repo: &R,
        path: &str,
        branch: &str,
        status: &mut crate::app::model::StatusViewState,
        prev_state: &AppState,
        current_state: &AppState,
    ) -> Result<Option<AppState>> {
        let normalized_code = match key_code {
            KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
            _ => key_code,
        };

        match normalized_code {
            KeyCode::Esc | KeyCode::Char('q') => {
                return Ok(Some(prev_state.clone()));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let total = status.total();
                if total > 0 {
                    status.selected_index = (status.selected_index + 1) % total;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let total = status.total();
                if total > 0 {
                    status.selected_index = (status.selected_index + total - 1) % total;
                }
            }
            KeyCode::Char(' ') => {
                let idx = status.selected_index;
                if idx < status.staged.len() {
                    let _ = repo.unstage_file(path, &status.staged[idx]);
                } else if idx < status.staged.len() + status.unstaged.len() {
                    let _ = repo.stage_file(path, &status.unstaged[idx - status.staged.len()]);
                } else if idx < status.total() {
                    let _ = repo.stage_file(
                        path,
                        &status.untracked[idx - status.staged.len() - status.unstaged.len()],
                    );
                }
                if let Ok(new_status) = repo.get_status(path) {
                    status.staged = new_status.staged;
                    status.unstaged = new_status.unstaged;
                    status.untracked = new_status.untracked;
                    let new_total = status.total();
                    if new_total > 0 && status.selected_index >= new_total {
                        status.selected_index = new_total - 1;
                    }
                }
            }
            KeyCode::Char('c') => {
                return Ok(Some(AppState::Committing {
                    path: path.to_string(),
                    branch: branch.to_string(),
                    selected_index: 0,
                    prev_state: Box::new(current_state.clone()),
                }));
            }
            KeyCode::Char('a') => {
                let _ = repo.stage_all(path);
                if let Ok(new_status) = repo.get_status(path) {
                    status.staged = new_status.staged;
                    status.unstaged = new_status.unstaged;
                    status.untracked = new_status.untracked;
                    let new_total = status.total();
                    if new_total > 0 && status.selected_index >= new_total {
                        status.selected_index = new_total - 1;
                    }
                }
            }
            KeyCode::Char('u') => {
                let _ = repo.unstage_all(path);
                if let Ok(new_status) = repo.get_status(path) {
                    status.staged = new_status.staged;
                    status.unstaged = new_status.unstaged;
                    status.untracked = new_status.untracked;
                    let new_total = status.total();
                    if new_total > 0 && status.selected_index >= new_total {
                        status.selected_index = new_total - 1;
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn handle_committing_events<R: ProjectRepository>(
        key_code: KeyCode,
        repo: &R,
        path: &str,
        branch: &str,
        selected_index: &mut usize,
        prev_state: &AppState,
        current_state: &AppState,
    ) -> Result<Option<AppState>> {
        let options = ["Manual Commit", "AI Commit"];
        let normalized_code = match key_code {
            KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
            _ => key_code,
        };

        match normalized_code {
            KeyCode::Esc | KeyCode::Char('q') => {
                return Ok(Some(prev_state.clone()));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                *selected_index = (*selected_index + 1) % options.len();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                *selected_index = (*selected_index + options.len() - 1) % options.len();
            }
            KeyCode::Enter => {
                match *selected_index {
                    0 => {
                        // Manual
                        return Ok(Some(AppState::Prompting {
                            prompt_type: PromptType::CommitMessage,
                            input: String::new(),
                            prev_state: Box::new(current_state.clone()),
                        }));
                    }
                    1 => {
                        // AI
                        if let Ok(diff) = repo.get_diff(path)
                            && let Ok(msg) = repo.generate_commit_message(&diff, branch)
                        {
                            return Ok(Some(AppState::Prompting {
                                prompt_type: PromptType::CommitMessage,
                                input: msg,
                                prev_state: Box::new(current_state.clone()),
                            }));
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn handle_history_events(
        key_code: KeyCode,
        commits: &[crate::domain::repository::GitCommit],
        selected_index: &mut usize,
        prev_state: &AppState,
    ) -> Result<Option<AppState>> {
        let normalized_code = match key_code {
            KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
            _ => key_code,
        };

        match normalized_code {
            KeyCode::Esc | KeyCode::Char('q') => {
                return Ok(Some(prev_state.clone()));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !commits.is_empty() {
                    *selected_index = (*selected_index + 1) % commits.len();
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if !commits.is_empty() {
                    *selected_index = (*selected_index + commits.len() - 1) % commits.len();
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn handle_branch_events<R: ProjectRepository>(
        key_code: KeyCode,
        repo: &R,
        path: &str,
        branches: &[String],
        selected_index: &mut usize,
        prev_state: &AppState,
    ) -> Result<Option<AppState>> {
        let normalized_code = match key_code {
            KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
            _ => key_code,
        };

        match normalized_code {
            KeyCode::Esc | KeyCode::Char('q') => {
                return Ok(Some(prev_state.clone()));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !branches.is_empty() {
                    *selected_index = (*selected_index + 1) % branches.len();
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if !branches.is_empty() {
                    *selected_index = (*selected_index + branches.len() - 1) % branches.len();
                }
            }
            KeyCode::Enter => {
                if let Some(branch) = branches.get(*selected_index) {
                    let _ = repo.switch_branch(path, branch);
                }
                return Ok(Some(prev_state.clone()));
            }
            _ => {}
        }
        Ok(None)
    }

    fn handle_editor_events<R: ProjectRepository>(
        key_code: KeyCode,
        repo: &R,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        branch: &str,
        options: &[EditorConfig],
        selected: &mut usize,
        prev_state: &AppState,
    ) -> Result<Option<AppState>> {
        let normalized_code = match key_code {
            KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
            _ => key_code,
        };

        match normalized_code {
            KeyCode::Up | KeyCode::Char('k') => {
                if *selected > 0 {
                    *selected -= 1;
                } else {
                    *selected = options.len() - 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if *selected < options.len() - 1 {
                    *selected += 1;
                } else {
                    *selected = 0;
                }
            }
            KeyCode::Enter => {
                let editor = options[*selected].command.clone();
                let _ = repo.set_preferred_editor(&editor);
                let path = if let AppState::ListingWorktrees {
                    worktrees,
                    table_state,
                    ..
                } = prev_state
                {
                    table_state
                        .selected()
                        .and_then(|i| worktrees.get(i))
                        .map(|wt| wt.path.clone())
                } else {
                    None
                };
                if let Some(p) = path {
                    let mut opening_state = AppState::OpeningEditor {
                        branch: branch.to_string(),
                        editor: editor.clone(),
                        prev_state: Box::new(prev_state.clone()),
                    };
                    terminal.draw(|f| Self::draw(f, repo, &mut opening_state))?;
                    let _ = Command::new(&editor).arg(&p).spawn();
                    std::thread::sleep(Duration::from_millis(800));
                    return Ok(Some(prev_state.clone()));
                }
            }
            KeyCode::Esc => {
                return Ok(Some(prev_state.clone()));
            }
            _ => {}
        }
        Ok(None)
    }

    fn handle_prompt_events<R: ProjectRepository>(
        key_code: KeyCode,
        repo: &R,
        prompt_type: &PromptType,
        input: &mut String,
        prev_state: &AppState,
    ) -> Result<Option<AppState>> {
        match key_code {
            KeyCode::Enter => {
                let val = input.trim().to_string();
                if !val.is_empty() {
                    match prompt_type {
                        PromptType::AddIntent => {
                            let _ = repo.add_worktree(&val, &val);
                            return Ok(Some(AppState::ListingWorktrees {
                                worktrees: Vec::new(),
                                table_state: TableState::default(),
                                refresh_needed: true,
                            }));
                        }
                        PromptType::InitUrl => {
                            let _ = repo.init_bare_repo(&val, "project");
                            return Ok(Some(AppState::ListingWorktrees {
                                worktrees: Vec::new(),
                                table_state: TableState::default(),
                                refresh_needed: true,
                            }));
                        }
                        PromptType::CommitMessage => {
                            if let AppState::ViewingStatus { path, .. } = prev_state {
                                let _ = repo.commit(path, &val);
                                if let Ok(status) = repo.get_status(path) {
                                    let mut new_state = prev_state.clone();
                                    if let AppState::ViewingStatus { status: s, .. } =
                                        &mut new_state
                                    {
                                        s.staged = status.staged;
                                        s.unstaged = status.unstaged;
                                        s.untracked = status.untracked;
                                    }
                                    return Ok(Some(new_state));
                                }
                            }
                            return Ok(Some(prev_state.clone()));
                        }
                    }
                } else {
                    return Ok(Some(prev_state.clone()));
                }
            }
            KeyCode::Esc => {
                return Ok(Some(prev_state.clone()));
            }
            KeyCode::Char(c) => input.push(c),
            KeyCode::Backspace => {
                input.pop();
            }
            _ => {}
        }
        Ok(None)
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

                let selected_worktree = table_state.selected().and_then(|i| worktrees.get(i));

                f.render_widget(DetailsWidget::new(selected_worktree, context), chunks[2]);
            }
            AppState::ViewingStatus {
                branch,
                status,
                prev_state,
                ..
            } => {
                let theme = CyberTheme::default();

                // Create a side-panel layout by combining main and details areas
                let body_area = chunks[1].union(chunks[2]);
                let body_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
                    .split(body_area);

                // Render background list from prev_state for context
                if let AppState::ListingWorktrees {
                    worktrees,
                    table_state,
                    ..
                } = &**prev_state
                {
                    let mut ts = table_state.clone();
                    f.render_stateful_widget(
                        WorktreeListWidget::new(worktrees),
                        body_chunks[0],
                        &mut ts,
                    );
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
                let footer_area =
                    Rect::new(area.x + 2, area.y + area.height - 1, area.width - 4, 1);
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
            AppState::ViewingHistory {
                branch,
                commits,
                selected_index,
                ..
            } => {
                let theme = CyberTheme::default();
                let area = centered_rect(85, 80, f.area());
                f.render_widget(Clear, area);

                let outer_block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.secondary))
                    .title(Span::styled(
                        format!(" 󰊚 COMMIT LOG: {} ", branch),
                        Style::default()
                            .fg(theme.secondary)
                            .add_modifier(Modifier::BOLD),
                    ));

                let inner_area = outer_block.inner(area);
                f.render_widget(outer_block, area);

                let mut items = Vec::new();
                for (i, commit) in commits.iter().enumerate() {
                    let is_selected = i == *selected_index;
                    let row_style = if is_selected {
                        Style::default()
                            .bg(theme.selection_bg)
                            .fg(theme.text)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.text)
                    };

                    let prefix = if is_selected { " ▶ " } else { "   " };

                    items.push(Line::from(vec![
                        Span::styled(prefix, row_style.fg(theme.primary)),
                        Span::styled(&commit.hash, row_style.fg(theme.warning)),
                        Span::raw(" "),
                        Span::styled(format!(" {:<10} ", commit.date), row_style.fg(theme.subtle)),
                        Span::styled(
                            format!(" {:<15} ", commit.author),
                            row_style.fg(theme.accent),
                        ),
                        Span::styled(&commit.message, row_style),
                    ]));
                }

                let p = Paragraph::new(items).alignment(Alignment::Left);
                f.render_widget(p, inner_area);

                // Footer
                let footer_area =
                    Rect::new(area.x + 2, area.y + area.height - 1, area.width - 4, 1);
                let help_text = Paragraph::new(Line::from(vec![
                    Span::styled(
                        " [UP/DOWN]",
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" NAVIGATE  "),
                    Span::styled(
                        " [ESC/Q]",
                        Style::default()
                            .fg(theme.subtle)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" BACK "),
                ]))
                .alignment(Alignment::Center);
                f.render_widget(help_text, footer_area);
            }
            AppState::SwitchingBranch {
                branches,
                selected_index,
                ..
            } => {
                let theme = CyberTheme::default();
                let area = centered_rect(50, 60, f.area());
                f.render_widget(Clear, area);

                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.primary))
                    .title(Span::styled(
                        "  SWITCH BRANCH ",
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ));

                let inner_area = block.inner(area);
                f.render_widget(block, area);

                let mut items = Vec::new();
                for (i, branch) in branches.iter().enumerate() {
                    let is_selected = i == *selected_index;
                    let style = if is_selected {
                        Style::default()
                            .bg(theme.selection_bg)
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.text)
                    };

                    let prefix = if is_selected { " ▶ " } else { "   " };
                    items.push(Line::from(vec![
                        Span::styled(prefix, style.fg(theme.primary)),
                        Span::styled(branch, style),
                    ]));
                }

                f.render_widget(Paragraph::new(items), inner_area);
            }
            AppState::Committing { selected_index, .. } => {
                let theme = CyberTheme::default();
                let area = centered_rect(40, 30, f.area());
                f.render_widget(Clear, area);

                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.primary))
                    .title(Span::styled(
                        " 󰊚 COMMIT MENU ",
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ));

                let inner_area = block.inner(area);
                f.render_widget(block, area);

                let options = ["󰊚  MANUAL COMMIT", "󰚚  AI COMMIT (GEMINI)"];
                let mut items = Vec::new();
                for (i, opt) in options.iter().enumerate() {
                    let is_selected = i == *selected_index;
                    let style = if is_selected {
                        Style::default()
                            .bg(theme.selection_bg)
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.text)
                    };

                    let prefix = if is_selected { " ▶ " } else { "   " };
                    items.push(Line::from(vec![
                        Span::styled(prefix, style.fg(theme.primary)),
                        Span::styled(*opt, style),
                    ]));
                }

                f.render_widget(Paragraph::new(items), inner_area);
            }
            AppState::Prompting {
                prompt_type, input, ..
            } => {
                let theme = CyberTheme::default();
                let table = WorktreeListWidget::new(&[]);
                f.render_stateful_widget(table, chunks[1], &mut TableState::default());
                f.render_widget(DetailsWidget::new(None, context), chunks[2]);

                let area = centered_rect(60, 20, f.area());
                f.render_widget(Clear, area);

                let (title, icon) = match prompt_type {
                    PromptType::AddIntent => (" ADD NEW WORKTREE ", "󰙅 "),
                    PromptType::InitUrl => (" INITIALIZE REPOSITORY ", "󰚚 "),
                    PromptType::CommitMessage => (" COMMIT MESSAGE ", "󰊚 "),
                };

                let input_widget = Paragraph::new(Line::from(vec![
                    Span::styled(
                        format!(" {} > ", icon),
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(input.as_str(), Style::default().fg(theme.text)),
                    Span::styled(
                        "_",
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::SLOW_BLINK),
                    ),
                ]))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(theme.accent))
                        .title(Span::styled(
                            title,
                            Style::default()
                                .fg(theme.accent)
                                .add_modifier(Modifier::BOLD),
                        )),
                );

                f.render_widget(input_widget, area);
            }
            AppState::SelectingEditor {
                options, selected, ..
            } => {
                let table = WorktreeListWidget::new(&[]);
                f.render_stateful_widget(table, chunks[1], &mut TableState::default());
                f.render_widget(DetailsWidget::new(None, context), chunks[2]);

                let area = centered_rect(60, 40, f.area());
                f.render_widget(Clear, area);

                let items: Vec<Line> = options
                    .iter()
                    .enumerate()
                    .map(|(i, opt)| {
                        if i == *selected {
                            Line::from(vec![
                                Span::styled(
                                    " > ",
                                    Style::default()
                                        .fg(RatatuiColor::Cyan)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(
                                    &opt.name,
                                    Style::default()
                                        .fg(RatatuiColor::Cyan)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(
                                    format!(" ({})", opt.command),
                                    Style::default().fg(RatatuiColor::DarkGray),
                                ),
                            ])
                        } else {
                            Line::from(vec![
                                Span::raw("   "),
                                Span::raw(&opt.name),
                                Span::styled(
                                    format!(" ({})", opt.command),
                                    Style::default().fg(RatatuiColor::DarkGray),
                                ),
                            ])
                        }
                    })
                    .collect();

                let p = Paragraph::new(items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .title(Span::styled(
                                " SELECT PREFERRED EDITOR ",
                                Style::default().add_modifier(Modifier::BOLD),
                            )),
                    )
                    .alignment(Alignment::Left);

                f.render_widget(p, area);
            }
            AppState::Syncing { branch, .. } => {
                let table = WorktreeListWidget::new(&[]);
                f.render_stateful_widget(table, chunks[1], &mut TableState::default());
                f.render_widget(DetailsWidget::new(None, context), chunks[2]);

                let area = centered_rect(60, 20, f.area());
                f.render_widget(Clear, area);

                let p = Paragraph::new(vec![
                    Line::from(vec![Span::styled(
                        " SYNCING CONFIGURATIONS ",
                        Style::default()
                            .fg(RatatuiColor::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(""),
                    Line::from(vec![
                        Span::raw("Target: "),
                        Span::styled(
                            branch.as_str(),
                            Style::default()
                                .fg(RatatuiColor::Magenta)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]),
                ])
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded),
                )
                .alignment(Alignment::Center);
                f.render_widget(p, area);
            }
            AppState::SyncComplete { branch, .. } => {
                let table = WorktreeListWidget::new(&[]);
                f.render_stateful_widget(table, chunks[1], &mut TableState::default());
                f.render_widget(DetailsWidget::new(None, context), chunks[2]);

                let area = centered_rect(60, 20, f.area());
                f.render_widget(Clear, area);

                let p = Paragraph::new(vec![
                    Line::from(vec![Span::styled(
                        " SYNC COMPLETE ",
                        Style::default()
                            .fg(RatatuiColor::Green)
                            .add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(""),
                    Line::from(vec![
                        Span::raw("Successfully synced: "),
                        Span::styled(
                            branch.as_str(),
                            Style::default()
                                .fg(RatatuiColor::Magenta)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]),
                ])
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded),
                )
                .alignment(Alignment::Center);
                f.render_widget(p, area);
            }
            AppState::OpeningEditor { branch, editor, .. } => {
                let table = WorktreeListWidget::new(&[]);
                f.render_stateful_widget(table, chunks[1], &mut TableState::default());
                f.render_widget(DetailsWidget::new(None, context), chunks[2]);

                let area = centered_rect(60, 20, f.area());
                f.render_widget(Clear, area);

                let p = Paragraph::new(vec![
                    Line::from(vec![Span::styled(
                        " OPENING IN EDITOR ",
                        Style::default()
                            .fg(RatatuiColor::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(""),
                    Line::from(vec![
                        Span::raw("Branch: "),
                        Span::styled(
                            branch.as_str(),
                            Style::default()
                                .fg(RatatuiColor::Magenta)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(vec![
                        Span::raw("Editor: "),
                        Span::styled(
                            editor.as_str(),
                            Style::default()
                                .fg(RatatuiColor::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]),
                ])
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded),
                )
                .alignment(Alignment::Center);
                f.render_widget(p, area);
            }
            AppState::Welcome => {
                let welcome_text = vec![
                    Line::from(vec![Span::styled(
                        "NO BARE REPOSITORY DETECTED",
                        Style::default()
                            .fg(RatatuiColor::Red)
                            .add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(""),
                    Line::from(vec![
                        Span::raw("Press "),
                        Span::styled(
                            "'i'",
                            Style::default()
                                .fg(RatatuiColor::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" to initialize a new repository"),
                    ]),
                    Line::from(vec![
                        Span::raw("Press "),
                        Span::styled(
                            "'q'",
                            Style::default()
                                .fg(RatatuiColor::Red)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" to exit"),
                    ]),
                ];
                let p = Paragraph::new(welcome_text)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded),
                    )
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
                println!("   {} {}", "┗━".dimmed(), "WORKTREE REMOVED".green().bold());
            }
            AppState::Syncing { branch, .. } => {
                println!(
                    "{} {} [{} {}]",
                    "🔄".cyan(),
                    format!("SYNCING CONFIGURATIONS: {}", branch).cyan().bold(),
                    "STATUS:".dimmed(),
                    "SYNCHRONIZING".yellow()
                );
            }
            AppState::SyncComplete { branch, .. } => {
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
            AppState::OpeningEditor { branch, editor, .. } => {
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
                println!("\n{} {}", "🚀".blue(), "SETUP COMPLETE.".blue().bold());
                println!("   {}", "All default worktrees have been created.".dimmed());
            }
            AppState::Error(msg) => {
                eprintln!("\n{} {} {}", "❌".red(), "ERROR:".red().bold(), msg.red());
                eprintln!(
                    "   {} {}",
                    "└─".dimmed(),
                    "Check git state and permissions.".dimmed()
                );
            }
            AppState::Welcome
            | AppState::Prompting { .. }
            | AppState::ViewingStatus { .. }
            | AppState::ViewingHistory { .. }
            | AppState::SwitchingBranch { .. }
            | AppState::Committing { .. }
            | AppState::Exiting => {
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
