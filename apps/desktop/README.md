# OCAF Agents Builder — Windows/macOS Code + NoCode

Tauri v2 native host of `apps/agents-builder-web`, using one existing shared Code/NoCode editor. No duplicate graph model; no shell/filesystem/updater privileges. The Local/Cloud switch in the editor does not constitute an OAuth deployment.

Requires Rust, OS-specific SDK and Tauri CLI on the **actual target OS**.

```sh
cd apps/desktop
cargo tauri build --manifest-path src-tauri/Cargo.toml
```

The installer is NOT certified until native Windows and macOS builds, signing, GUI E2E, hashes, SBOM and independent GATE-10 evidence are verified.
