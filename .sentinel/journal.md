# Sentinel's Journal

## Critical Learnings

* **Insecure File Creation Defaults:** `std::fs::write` creates files with default umask permissions (often 644/664), which is insecure for sensitive data like API keys. Use `std::fs::OpenOptions` with `.mode(0o600)` (on Unix) to restrict access.
