use std::{fs::DirEntry, path::{Path, PathBuf}, process::{Child, Command, Stdio}};

use rand::seq::SliceRandom;

const FLATPAK_APPLICATIONS_PATH: &str = ".var/app/com.valvesoftware.Steam/data/Steam";
const VANILLA_APPLICATIONS_PATH_NIX: &str = r#".local/share/Steam"#;
#[cfg(target_os = "windows")]
const VANILLA_APPLICATIONS_PATH_WINDOWS: &str = r#"C:\Program Files (x86)\Steam"#;

const MANIFEST_DIR: &str = "steamapps/";

fn generate_steam_rungame(id: &str) -> String {
    format!("steam://rungameid/{}", id)
}

fn is_proton(app_name: &str) -> bool {
    if app_name.starts_with("Proton") {
        let number = app_name.split(' ').collect::<Vec<&str>>();
        if number.len() > 1 {
            let number = number[1].parse::<f32>();
            if number.unwrap_or(0.0) != 0.0 {
                return true;
            }
        }
    }
    false
}

fn is_blacklisted(app_name: &str) -> bool {
    let steam_libs = [
        "Steamworks Common Redistributables",
        "SteamVR",
        "Proton Experimental",
    ];

    steam_libs.iter().any(|&b| b == app_name)
        || app_name.ends_with("Soundtrack") // This **should** deal with downloaded albums
        || is_proton(app_name)
        || app_name.starts_with("Steam Linux Runtime")
}

fn get_other_install_dirs(path: &Path) -> Vec<String> {
    let mut path = path.to_path_buf();
    path.push("libraryfolders.vdf");

    let contents = std::fs::read_to_string(path);
    let lines = contents.unwrap();

    let mut libs = Vec::new();

    // NOTE: I'm assuming that after the "TimeNextStatsReport" and "ContentStatsID" come the alternative paths
    let lines = lines.lines().skip(2).collect::<Vec<&str>>();
    for line in lines.iter().take(lines.len() - 1) {
        let path = line
        .split('\t')
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>();

        let tag = path[0].replace("\"", "");
        let path = path[1].trim().replace("\"", "");
        if tag.chars().all(|c| c.is_digit(10)) {
            libs.push(path.to_string());
        }
    }
    libs
}

fn get_games_from_manifest_in_path(path: &Path) -> Vec<(String, String)> {
    let dir = std::fs::read_dir(path).unwrap();

    let manifest_files = dir
        .map(|e| e.unwrap())
        .filter(|file| {
            let file_name = file.file_name();
            let file_name = file_name.to_str().unwrap();
            file_name.starts_with("appmanifest")
        })
        .collect::<Vec<DirEntry>>();

    let mut games = Vec::new();

    for file in manifest_files {
        let file_path = file.path();
        let contents = std::fs::read_to_string(file_path);
        let lines = contents.unwrap();
        let lines = lines.lines().skip(2).collect::<Vec<&str>>();

        let mut game = "".to_string();
        let mut id = "".to_string();

        for line in lines.iter().take(lines.len() - 1) {
            let line = line
                .split('\t')
                .filter(|s| !s.is_empty())
                .collect::<Vec<&str>>();
            if line[0].contains("name") {
                let app_name = line[1].replace("\"", "");

                game = app_name.clone();
            }

            if line[0].contains("appid") {
                let app_id = line[1].replace("\"", "");
                id = app_id.clone();
            }
        }
        if !is_blacklisted(&game) {
            games.push((game, id));
        }
    }

    games
}

#[derive(Debug, PartialEq)]
enum SteamKind {
    Vanilla,
    Flatpak,
    NotFound,
}

#[cfg(target_os = "linux")]
fn detect_steam() -> SteamKind {
    let has_steam_vanilla = which::which("steam").is_ok();
    let flatpak_list = Command::new("flatpak")
        .arg("list")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut flatpak_steam_output = Command::new("grep")
        .stdin(flatpak_list.stdout.unwrap())
        .arg("-c")
        .arg("Steam")
        .output()
        .unwrap()
        .stdout;
    flatpak_steam_output.remove(flatpak_steam_output.len() - 1);
    let has_flatpak_steam = String::from_utf8(flatpak_steam_output).unwrap();

    let has_flatpak_steam = has_flatpak_steam.parse::<u32>().unwrap() > 0;

    match (has_steam_vanilla, has_flatpak_steam) {
        (true, _) => SteamKind::Vanilla,
        (_, true) => SteamKind::Flatpak,
        _ => SteamKind::NotFound,
    }
}

#[cfg(target_os = "windows")]
fn detect_steam() -> SteamKind {
    let has_steam_vanilla = which::which(r#"C:\Program Files (x86)\Steam\steam.exe"#).is_ok();
    match (has_steam_vanilla) {
        true => SteamKind::Vanilla,
        _ => SteamKind::NotFound,
    }
}

#[cfg(target_os = "linux")]
fn run(steam_type: SteamKind, id: &str) -> std::io::Result<Child> {
    let child  = match steam_type {
        SteamKind::Flatpak => std::process::Command::new("flatpak")
            .args(&[
                "run",
                "com.valvesoftware.Steam",
                &generate_steam_rungame(id),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?,
        SteamKind::Vanilla => std::process::Command::new("steam")
            .arg(&generate_steam_rungame(id))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?,
        SteamKind::NotFound => panic!("Couldn't find steam!"),
    };
    Ok(child)
}

#[cfg(target_os = "windows")]
fn run(steam_type: SteamKind, id: &str) -> std::io::Result<Child> {
    let child = match steam_type {
        SteamKind::Vanilla => {
            std::process::Command::new(r#"C:\Program Files (x86)\Steam\steam.exe"#)
                .arg(&generate_steam_rungame(id))
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
        }
        _ => panic!("Couldn't find steam!"),
    };
    Some(child)
}

fn main() {
    let steam_type = detect_steam();

    if steam_type == SteamKind::NotFound {
        eprintln!("Couldn't find Steam.");
        return;
    }

    let mut path = {
        let mut home = dirs::home_dir().unwrap();
        match steam_type {
            SteamKind::Flatpak => home.push(FLATPAK_APPLICATIONS_PATH),
            #[cfg(target_os = "linux")]
            SteamKind::Vanilla => home.push(VANILLA_APPLICATIONS_PATH_NIX),
            #[cfg(target_os = "windows")]
            SteamKind::Vanilla => home.push(VANILLA_APPLICATIONS_PATH_WINDOWS),
            _ => {}
        }
        home
    };

    path.push(MANIFEST_DIR);

    let install_dirs = get_other_install_dirs(&path);

    let mut games = get_games_from_manifest_in_path(&path);

    for other_dir in install_dirs {
        let mut path = PathBuf::new();
        path.push(other_dir);
        path.push(MANIFEST_DIR);
        games.extend(get_games_from_manifest_in_path(&path));
    }

    let (game, id) = games.choose(&mut rand::thread_rng()).unwrap();

    println!("Randomly launching \"{}\"! Have fun!", game);

    let _ = run(steam_type, id).unwrap();
}