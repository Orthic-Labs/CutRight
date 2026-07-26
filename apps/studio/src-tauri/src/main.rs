#[tauri::command]
fn read_project(path: String) -> Result<serde_json::Value, String> {
    let manifest = std::path::Path::new(&path).join("project.json");
    let bytes = std::fs::read(&manifest).map_err(|error| format!("{}: {error}", manifest.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", manifest.display()))
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![read_project])
        .run(tauri::generate_context!())
        .expect("CutRight Studio failed to start");
}
