// Tauri command handlers receive State<T> and String by value — that is required
// by the #[tauri::command] macro's dependency-injection and JSON deserialization
// machinery. Clippy's needless_pass_by_value suggestion does not apply here.
#![allow(clippy::needless_pass_by_value)]

mod models;
mod sqbs_format;
mod reports;

use models::Tournament;
use sqbs_format::{SqbsParser, write_sqbs};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;
use std::sync::Mutex;
use tauri::State;

struct AppState {
    tournament: Mutex<Tournament>,
    file_path: Mutex<Option<String>>,
}

#[tauri::command]
fn get_tournament(state: State<AppState>) -> Result<Tournament, String> {
    state.tournament.lock()
        .map(|g| g.clone())
        .map_err(|e| format!("state corrupted: {e}"))
}

#[tauri::command]
fn new_tournament(state: State<AppState>) -> Result<Tournament, String> {
    let t = Tournament::default();
    *state.tournament.lock().map_err(|e| format!("state corrupted: {e}"))? = t.clone();
    *state.file_path.lock().map_err(|e| format!("state corrupted: {e}"))? = None;
    Ok(t)
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
    // Update base_name under the lock if needed, then clone for I/O
    let tournament = {
        let mut t = state.tournament.lock().map_err(|e| format!("state corrupted: {e}"))?;
        if t.reports.base_name.is_empty() {
            if let Some(stem) = Path::new(&path).file_stem().and_then(|s| s.to_str()) {
                t.reports.base_name = stem.to_string();
            }
        }
        t.clone()
    };
    // Write file without holding the mutex
    let file = File::create(&path).map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(file);
    write_sqbs(&mut writer, &tournament).map_err(|e| e.to_string())?;
    writer.flush().map_err(|e: std::io::Error| e.to_string())?;
    // Only update file_path — the in-memory tournament is already current
    *state.file_path.lock().map_err(|e| format!("state corrupted: {e}"))? = Some(path);
    Ok(tournament)
}

#[tauri::command]
fn update_tournament(tournament: Tournament, state: State<AppState>) -> Tournament {
    *state.tournament.lock().unwrap() = tournament.clone();
    tournament
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
#[allow(clippy::missing_panics_doc)]
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

#[cfg(test)]
mod parity_tests {
    use super::sqbs_format::SqbsParser;
    use super::reports;
    use std::fs;
    use std::io::BufReader;
    use std::path::PathBuf;

    const QZX: &str = r"C:\Users\Nick\Downloads\Highland Games 2023 Rounds 1-11 [Prelims bracketing].qzx";
    const REF_DIR: &str = r"C:\Users\Nick\Downloads\SQBSTournamentKit-main\Tests\SQBSTournamentKitTests\Resources\Reference\HighlandGames";

    #[test]
    #[ignore = "requires Windows-only file paths; run manually on the PC with cargo test -- --ignored"]
    fn reports_match_reference() {
        let file = fs::File::open(QZX).expect("cannot open qzx file");
        let mut tournament = SqbsParser::new(BufReader::new(file)).parse().expect("parse failed");

        // Mirror open_file: derive base_name from file stem when not set in the file
        if tournament.reports.base_name.is_empty() {
            if let Some(stem) = std::path::Path::new(QZX).file_stem().and_then(|s| s.to_str()) {
                tournament.reports.base_name = stem.to_string();
            }
        }

        let tmp_dir = std::env::temp_dir().join("sqbs_parity");
        fs::create_dir_all(&tmp_dir).unwrap();
        let written = reports::generate_all_reports(&tournament, tmp_dir.to_str().unwrap())
            .expect("generate_all_reports failed");

        println!("Generated {} files:", written.len());
        for p in &written { println!("  {p}"); }

        let mut all_match = true;
        for path in &written {
            let filename = PathBuf::from(path).file_name().unwrap().to_str().unwrap().to_string();
            let ref_path = PathBuf::from(REF_DIR).join(&filename);
            let generated = fs::read_to_string(path).expect("cannot read generated file");

            match fs::read_to_string(&ref_path) {
                Ok(reference) => {
                    if generated == reference {
                        println!("PASS {filename}");
                    } else {
                        all_match = false;
                        println!("FAIL {filename}");
                        let gen_lines: Vec<_> = generated.lines().collect();
                        let ref_lines: Vec<_> = reference.lines().collect();
                        for (i, (g, r)) in gen_lines.iter().zip(ref_lines.iter()).enumerate() {
                            if g != r {
                                println!("  first diff line {}: generated={g:?} reference={r:?}", i + 1);
                                break;
                            }
                        }
                        println!("  line counts: generated={} reference={}", gen_lines.len(), ref_lines.len());
                    }
                }
                Err(_) => println!("SKIP {filename} (no reference at {})", ref_path.display()),
            }
        }

        assert!(all_match, "one or more report files did not match reference — see output above");
    }
}
