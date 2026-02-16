#[derive(Clone, Debug)]
pub enum Intent {
    Initialize {
        url: Option<String>,
        name: Option<String>,
    },
    AddWorktree {
        intent: String,
        branch: Option<String>,
    },
    RemoveWorktree {
        intent: String,
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
    },
    SwitchWorktree {
        name: String,
    },
}
