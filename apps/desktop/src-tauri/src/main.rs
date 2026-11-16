#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Source-only native Windows/macOS Code & NoCode application.
// No privileged commands, shell or filesystem access exposed to web code.
fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("OCAF Agents Builder startup failed");
}
