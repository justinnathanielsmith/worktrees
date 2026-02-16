#[derive(Clone, Debug)]
pub enum Intent {
    Initialize {
        url: String,
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
}
