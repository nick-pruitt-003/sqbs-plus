mod models;
mod sqbs_format;
mod reports;

use models::Tournament;
use sqbs_format::{SqbsParser, write_sqbs};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::sync::Mutex;
use tauri::State;

struct AppState {
    tournament: Mutex<Tournament>,
    file_path: Mutex<Option<String>>,
}

#[tauri::command]
fn get_tournament(state: State<AppState>) -> Tournament {
    state.tournament.lock().unwrap().clone()
}

#[tauri::command]
fn new_tournament(state: State<AppState>) -> Tournament {
    let t = Tournament::default();
    *state.tournament.lock().unwrap() = t.clone();
    *state.file_path.lock().unwrap() = None;
    t
}

#[tauri::command]
fn open_file(path: String, state: State<AppState>) -> Result<Tournament, String> {
    let file = File::open(&path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let parser = SqbsParser::new(reader);
    let mut tournament = parser.parse().map_err(|e| e.to_string())?;
    if tournament.reports.base_name.is_empty() {
        if let Some(stem) = Path::new(&path).file_stem().and_then(|s| s.to_str()) {
            tournament.reports.base_name = stem.to_string();
        }
    }
    *state.tournament.lock().unwrap() = tournament.clone();
    *state.file_path.lock().unwrap() = Some(path);
    Ok(tournament)
}

#[tauri::command]
fn save_file(path: String, state: State<AppState>) -> Result<Tournament, String> {
    let mut tournament = state.tournament.lock().unwrap().clone();
    if tournament.reports.base_name.is_empty() {
        if let Some(stem) = Path::new(&path).file_stem().and_then(|s| s.to_str()) {
            tournament.reports.base_name = stem.to_string();
        }
    }
    let file = File::create(&path).map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(file);
    write_sqbs(&mut writer, &tournament).map_err(|e| e.to_string())?;
    *state.tournament.lock().unwrap() = tournament.clone();
    *state.file_path.lock().unwrap() = Some(path);
    Ok(tournament)
}

#[tauri::command]
fn update_tournament(tournament: Tournament, state: State<AppState>) -> Tournament {
    let t = tournament.clone();
    *state.tournament.lock().unwrap() = tournament;
    t
}

#[tauri::command]
fn get_file_path(state: State<AppState>) -> Option<String> {
    state.file_path.lock().unwrap().clone()
}

#[tauri::command]
fn generate_reports(dir: String, state: State<AppState>) -> Result<Vec<String>, String> {
    let tournament = state.tournament.lock().unwrap().clone();
    reports::generate_all_reports(&tournament, &dir).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            tournament: Mutex::new(Tournament::default()),
            file_path: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_tournament,
            new_tournament,
            open_file,
            save_file,
            update_tournament,
            get_file_path,
            generate_reports,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
