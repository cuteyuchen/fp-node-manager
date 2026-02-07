use std::fs::File;
use std::process::Command;
use std::env;
use tauri::AppHandle;

#[tauri::command]
pub async fn install_update(app: AppHandle, url: String) -> Result<(), String> {
    println!("Starting update download from: {}", url);
    
    // Use blocking task to avoid blocking the async runtime with file I/O and synchronous download
    // But since we added blocking feature to reqwest, we can use it inside spawn_blocking
    
    let result = tauri::async_runtime::spawn_blocking(move || {
        let mut response = reqwest::blocking::get(&url).map_err(|e| e.to_string())?;
        
        let mut temp_path = env::temp_dir();
        temp_path.push("frontend-manager-update.exe");
        
        println!("Downloading to: {:?}", temp_path);
        
        let mut dest = File::create(&temp_path).map_err(|e| e.to_string())?;
        response.copy_to(&mut dest).map_err(|e| e.to_string())?;
        
        Ok::<std::path::PathBuf, String>(temp_path)
    }).await.map_err(|e| e.to_string())??;

    println!("Download complete. Launching installer...");

    // Launch the installer
    // Using cmd /c start to ensure it runs independently
    #[cfg(target_os = "windows")]
    Command::new("cmd")
        .args(["/C", "start", "", result.to_str().unwrap()])
        .spawn()
        .map_err(|e| e.to_string())?;

    #[cfg(not(target_os = "windows"))]
    Command::new(result)
        .spawn()
        .map_err(|e| e.to_string())?;
        
    println!("Installer launched. Exiting app.");
    app.exit(0);
    
    Ok(())
}
