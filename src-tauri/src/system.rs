use std::path::PathBuf;
use std::process::Command;

#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;

#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::io::Write;
#[cfg(target_os = "linux")]
use std::os::unix::fs::PermissionsExt;

#[derive(serde::Serialize)]
pub struct PlatformInfo {
    os: String,
    arch: String,
}

#[derive(serde::Serialize, Clone)]
pub struct TerminalInfo {
    id: String,
    name: String,
    available: bool,
}

#[tauri::command]
pub fn get_platform_info() -> PlatformInfo {
    PlatformInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
    }
}

fn get_exe_path() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|e| e.to_string())
}

//************* 终端检测功能 *************

fn check_command_exists(cmd: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        Command::new("where")
            .arg(cmd)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("which")
            .arg(cmd)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

#[cfg(target_os = "windows")]
fn check_windows_terminal() -> bool {
    let appdata = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let wt_path = format!(r"{}\Microsoft\WindowsApps\wt.exe", appdata);
    std::path::Path::new(&wt_path).exists() || check_command_exists("wt")
}

#[cfg(target_os = "windows")]
fn check_git_bash() -> bool {
    let program_files = std::env::var("ProgramFiles").unwrap_or_default();
    let program_files_x86 = std::env::var("ProgramFiles(x86)").unwrap_or_default();
    
    let git_bash_paths = vec![
        format!(r"{}\Git\bin\bash.exe", program_files),
        format!(r"{}\Git\bin\bash.exe", program_files_x86),
        r"C:\Program Files\Git\bin\bash.exe".to_string(),
        r"C:\Program Files (x86)\Git\bin\bash.exe".to_string(),
    ];
    
    git_bash_paths.iter().any(|p| std::path::Path::new(p).exists()) || check_command_exists("bash")
}

#[cfg(target_os = "windows")]
fn check_cmder() -> bool {
    let cmder_paths = vec![
        r"C:\cmder\Cmder.exe",
        r"C:\tools\cmder\Cmder.exe",
    ];
    cmder_paths.iter().any(|p| std::path::Path::new(p).exists())
}

#[cfg(target_os = "macos")]
fn check_terminal_app() -> bool {
    std::path::Path::new("/System/Applications/Utilities/Terminal.app").exists()
}

#[cfg(target_os = "macos")]
fn check_iterm2() -> bool {
    std::path::Path::new("/Applications/iTerm.app").exists()
}

#[tauri::command]
pub fn detect_available_terminals() -> Vec<TerminalInfo> {
    let mut terminals: Vec<TerminalInfo> = Vec::new();
    
    #[cfg(target_os = "windows")]
    {
        terminals.push(TerminalInfo {
            id: "cmd".to_string(),
            name: "Command Prompt (cmd.exe)".to_string(),
            available: true,
        });
        
        terminals.push(TerminalInfo {
            id: "powershell".to_string(),
            name: "PowerShell".to_string(),
            available: check_command_exists("powershell"),
        });
        
        terminals.push(TerminalInfo {
            id: "pwsh".to_string(),
            name: "PowerShell Core (pwsh)".to_string(),
            available: check_command_exists("pwsh"),
        });
        
        terminals.push(TerminalInfo {
            id: "git-bash".to_string(),
            name: "Git Bash".to_string(),
            available: check_git_bash(),
        });
        
        terminals.push(TerminalInfo {
            id: "windows-terminal".to_string(),
            name: "Windows Terminal".to_string(),
            available: check_windows_terminal(),
        });
        
        terminals.push(TerminalInfo {
            id: "cmder".to_string(),
            name: "Cmder".to_string(),
            available: check_cmder(),
        });
    }
    
    #[cfg(target_os = "macos")]
    {
        terminals.push(TerminalInfo {
            id: "terminal".to_string(),
            name: "Terminal.app".to_string(),
            available: check_terminal_app(),
        });
        
        terminals.push(TerminalInfo {
            id: "iterm2".to_string(),
            name: "iTerm2".to_string(),
            available: check_iterm2(),
        });
        
        terminals.push(TerminalInfo {
            id: "zsh".to_string(),
            name: "Zsh".to_string(),
            available: check_command_exists("zsh"),
        });
        
        terminals.push(TerminalInfo {
            id: "bash".to_string(),
            name: "Bash".to_string(),
            available: check_command_exists("bash"),
        });
    }
    
    #[cfg(target_os = "linux")]
    {
        terminals.push(TerminalInfo {
            id: "bash".to_string(),
            name: "Bash".to_string(),
            available: check_command_exists("bash"),
        });
        
        terminals.push(TerminalInfo {
            id: "zsh".to_string(),
            name: "Zsh".to_string(),
            available: check_command_exists("zsh"),
        });
        
        terminals.push(TerminalInfo {
            id: "gnome-terminal".to_string(),
            name: "GNOME Terminal".to_string(),
            available: check_command_exists("gnome-terminal"),
        });
        
        terminals.push(TerminalInfo {
            id: "konsole".to_string(),
            name: "Konsole (KDE)".to_string(),
            available: check_command_exists("konsole"),
        });
        
        terminals.push(TerminalInfo {
            id: "xfce4-terminal".to_string(),
            name: "XFCE Terminal".to_string(),
            available: check_command_exists("xfce4-terminal"),
        });
        
        terminals.push(TerminalInfo {
            id: "alacritty".to_string(),
            name: "Alacritty".to_string(),
            available: check_command_exists("alacritty"),
        });
        
        terminals.push(TerminalInfo {
            id: "kitty".to_string(),
            name: "Kitty".to_string(),
            available: check_command_exists("kitty"),
        });
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        terminals.push(TerminalInfo {
            id: "bash".to_string(),
            name: "Bash".to_string(),
            available: check_command_exists("bash"),
        });
    }
    
    terminals
}

//************* 右键菜单功能 *************

#[cfg(target_os = "windows")]
#[tauri::command]
pub fn set_context_menu(enable: bool, locale: String) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let exe_path = get_exe_path()?;
    let exe_str = exe_path.to_str().ok_or("Invalid path")?;
    
    let menu_text = if locale == "zh" {
        "在前端项目 & Node 管理器中打开"
    } else {
        "Open in Project & Node Manager"
    };
    
    let keys = vec![
        r"Software\Classes\Directory\shell\fp-node-manager",
        r"Software\Classes\Directory\Background\shell\fp-node-manager"
    ];
    
    for key_path in keys {
        if enable {
            let (key, _) = hkcu.create_subkey(key_path).map_err(|e| e.to_string())?;
            key.set_value("", &menu_text).map_err(|e| e.to_string())?;
            key.set_value("Icon", &exe_str).map_err(|e| e.to_string())?;
            let (cmd_key, _) = key.create_subkey("command").map_err(|e| e.to_string())?;
            let cmd_str = format!("\"{}\" \"%V\"", exe_str);
            cmd_key.set_value("", &cmd_str).map_err(|e| e.to_string())?;
        } else {
            let _ = hkcu.delete_subkey_all(key_path);
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
#[tauri::command]
pub fn check_context_menu() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key_path = r"Software\Classes\Directory\shell\fp-node-manager";
    hkcu.open_subkey(key_path).is_ok()
}

#[cfg(target_os = "linux")]
#[tauri::command]
pub fn set_context_menu(enable: bool, locale: String) -> Result<(), String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set")?;
    let applications_dir = std::path::Path::new(&home).join(".local/share/applications");
    let desktop_file_path = applications_dir.join("fp-node-manager-context.desktop");

    let menu_text = if locale == "zh" {
        "在前端项目 & Node 管理器中打开"
    } else {
        "Open in Project & Node Manager"
    };

    if enable {
        if !applications_dir.exists() {
             fs::create_dir_all(&applications_dir).map_err(|e| e.to_string())?;
        }

        let exe_path = get_exe_path()?;
        let exe_str = exe_path.to_str().ok_or("Invalid path")?;
        
        // Basic .desktop file for "Open With" support
        // MimeType=inode/directory registers it for folders
        let content = format!(r#"[Desktop Entry]
Type=Application
Name={}
Exec="{}" "%f"
Icon=folder-open
NoDisplay=true
MimeType=inode/directory;
"#, menu_text, exe_str);

        let mut file = fs::File::create(&desktop_file_path).map_err(|e| e.to_string())?;
        file.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
        
        // Make executable
        let mut perms = fs::metadata(&desktop_file_path).map_err(|e| e.to_string())?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&desktop_file_path, perms).map_err(|e| e.to_string())?;
        
        // Try to update desktop database (optional)
        std::process::Command::new("update-desktop-database")
            .arg(&applications_dir)
            .output()
            .ok();
            
    } else {
        if desktop_file_path.exists() {
            fs::remove_file(&desktop_file_path).map_err(|e| e.to_string())?;
            
             std::process::Command::new("update-desktop-database")
            .arg(&applications_dir)
            .output()
            .ok();
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
#[tauri::command]
pub fn check_context_menu() -> bool {
     let home = match std::env::var("HOME") {
        Ok(h) => h,
        Err(_) => return false,
    };
    let path = std::path::Path::new(&home).join(".local/share/applications/fp-node-manager-context.desktop");
    path.exists()
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
#[tauri::command]
pub fn set_context_menu(_enable: bool, _locale: String) -> Result<(), String> {
    Err("Not supported on this platform yet. Please use 'Open With' system configuration.".to_string())
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
#[tauri::command]
pub fn check_context_menu() -> bool {
    false
}

#[tauri::command]
pub fn is_context_menu_supported() -> bool {
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        true
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        false
    }
}
