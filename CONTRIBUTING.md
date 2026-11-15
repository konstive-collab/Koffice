# Contributing to Koffice

Thanks for helping build an open, AI-native office suite.

1. Search open issues before starting major work.
2. Keep all document logic in Rust's engine layer. UI and future AI adapters must call the same operations.
3. Add regression tests for bug fixes, especially Unicode, invalid input, file integrity, and revision conflicts.
4. Run `npm test`, `npm run build`, and `cargo test --manifest-path src-tauri/Cargo.toml` before proposing changes.
5. Keep changes focused, document new behavior, and describe reproducible test steps in pull requests.
6. Never submit secrets, tokens, user documents, or model weights.

Code of conduct: Treat contributors with respect, discuss ideas constructively, and do not harass other participants.