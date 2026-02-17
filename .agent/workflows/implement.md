# /implement - Feature Implementation Workflow
1. **Analyze**: Use `@ARCHITECTURE.md` to analyze current system state.
2. **Plan**: Generate an implementation plan with a list of all files to be modified.
3. **Verify Plan**: Stop and wait for user approval of the plan.
4. **Execute**: Modify code. Follow Rust standards in `ARCHITECTURE.md` (e.g., let-chains over nested if-lets).
5. **Quality Check**: Run the "CI Debugger" skill if any tests fail.
6. **Audit**: Run `cargo clippy --all-targets --all-features -- -D warnings`.
7. **Walkthrough**: Generate a post-implementation walkthrough video or screenshot.
