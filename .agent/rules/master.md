# Antigravity Master Rules
@AGENTS.md
@ARCHITECTURE.md

## 2026 Operational Protocol
- **Strict Planning**: You MUST use `PLANNING` mode for any change involving more than 3 files.
- **Pattern Matching**: Prefer `.is_some()`/.`is_none()` over `if let Some(_)`.
- **Audit Requirement**: Every file modification MUST be followed by `cargo check` and `cargo fmt`.
- **Security Check**: For sensitive files (like API keys), ensure `mode(0o600)` is used.
