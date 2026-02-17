#[derive(Clone, Debug)]
pub enum Intent {
    Initialize {
        url: Option<String>,
        name: Option<String>,
        warp: bool,
    },
    AddWorktree {
        intent: String,
        branch: Option<String>,
    },
    RemoveWorktree {
        intent: String,
        force: bool,
    },
    ListWorktrees,
    SetupDefaults,
    RunCommand {
        intent: String,
        branch: Option<String>,
        command: Vec<String>,
    },
    SyncConfigurations {
        intent: Option<String>,
    },
    #[allow(dead_code)]
    Pull {
        intent: Option<String>,
    },
    Push {
        intent: Option<String>,
    },
    Config {
        key: Option<String>,
        show: bool,
    },
    CleanWorktrees {
        dry_run: bool,
        artifacts: bool,
    },
    SwitchWorktree {
        name: String,
        copy: bool,
    },
    Convert {
        name: Option<String>,
        branch: Option<String>,
    },
    CheckoutWorktree {
        intent: String,
        branch: String,
    },
    Completions {
        shell: clap_complete::Shell,
    },
    Open,
    Migrate {
        force: bool,
        dry_run: bool,
    },
    Rebase {
        upstream: Option<String>,
    },
    ViewStashes {
        path: String,
        branch: String,
    },
    ApplyStash {
        path: String,
        index: usize,
    },
    PopStash {
        path: String,
        index: usize,
    },
    DropStash {
        path: String,
        index: usize,
    },
    StashSave {
        path: String,
        message: Option<String>,
    },
    ChangeMode(crate::app::model::AppMode),
}
