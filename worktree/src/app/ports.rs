use crate::app::model::AppState;
use crate::domain::repository::Worktree;
use miette::Result;

/// Abstract interface for the UI, allowing us to swap Real UI for a Test Spy.
pub trait ViewPort: Send + Sync + 'static {
    fn render(&self, state: AppState);
    fn render_json<T: serde::Serialize>(&self, data: &T) -> Result<()>;
    fn render_banner(&self);
    fn render_listing_table(&self, worktrees: &[Worktree]);
    fn render_feedback_prompt(&self);
}

/// The production implementation that calls your static View
#[derive(Clone, Default)]
pub struct RatatuiView;

impl ViewPort for RatatuiView {
    fn render(&self, state: AppState) {
        crate::app::view::View::render(state);
    }

    fn render_json<T: serde::Serialize>(&self, data: &T) -> Result<()> {
        crate::app::view::View::render_json(data).map_err(|e| miette::miette!(e))
    }

    fn render_banner(&self) {
        crate::app::view::View::render_banner();
    }

    fn render_listing_table(&self, worktrees: &[Worktree]) {
        crate::app::view::View::render_listing_table(worktrees);
    }

    fn render_feedback_prompt(&self) {
        crate::app::view::View::render_feedback_prompt();
    }
}
