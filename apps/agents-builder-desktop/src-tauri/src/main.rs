#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
mod native_cloud;
#[cfg(target_os = "linux")]
mod native_local;

fn main() {
    let builder = tauri::Builder::default();
    #[cfg(target_os = "linux")]
    let builder = builder
        .manage(native_cloud::NativeCloud::default())
        .invoke_handler(tauri::generate_handler![
            native_cloud::native_cloud_request,
            native_local::native_local_compile,
            native_local::native_export_project
        ]);
    builder
        .run(tauri::generate_context!())
        .expect("OCAF Agents Builder desktop runtime failed");
}
