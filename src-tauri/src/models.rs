use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default, specta::Type)]
pub struct Player {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, specta::Type)]
pub struct Team {
    pub name: String,
    pub players: Vec<Player>,
    pub division: Option<String>,
    pub exhibition: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, specta::Type)]
pub struct PlayerScore {
    pub player_index: usize,
    pub gp: f32,
    pub q: [i32; 4],
    pub points: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, specta::Type)]
pub struct TeamScore {
    pub team_index: usize,
    pub total_points: i32,
    pub bonus_heard: i32,
    pub bonus_points: i32,
    pub bb_heard: i32,
    pub bb_points: i32,
    pub ot_gets: i32,
    pub lightning_points: i32,
    pub player_scores: Vec<Option<PlayerScore>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct Game {
    pub game_index: String,
    pub round: u32,
    pub team_a: TeamScore,
    pub team_b: TeamScore,
    pub tossups_heard: u32,
    pub overtime: bool,
    pub forfeit: bool,
}

impl Default for Game {
    fn default() -> Self {
        Game {
            game_index: String::from("1"),
            round: 1,
            team_a: TeamScore::default(),
            team_b: TeamScore::default(),
            tossups_heard: 20,
            overtime: false,
            forfeit: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ScoringSettings {
    /// Point values for each tossup category [power, `alt_power`, normal, neg]
    pub q_values: [i32; 4],
    /// Whether each category is actively tracked (false = disabled, write 0 to file)
    pub q_enabled: [bool; 4],
    pub track_power_neg: bool,
    pub track_light_round: bool,
    pub track_tuh: bool,
    pub auto_track: u8,
    /// Sort individual standings by Pts/TUH instead of total points
    pub sort_by_ppg: bool,
}

impl Default for ScoringSettings {
    fn default() -> Self {
        ScoringSettings {
            q_values: [20, 15, 10, -5],
            q_enabled: [true, false, true, true],
            track_power_neg: true,
            track_light_round: false,
            track_tuh: true,
            auto_track: 1, // Automatic
            sort_by_ppg: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ReportSettings {
    pub base_name: String,
    // filenames
    pub rounds: String,
    pub standings: String,
    pub individuals: String,
    pub games: String,
    pub team_detail: String,
    pub player_detail: String,
    pub stat_key: String,
    pub style: String,
    // per-report enable flags
    pub include_rounds: bool,
    pub include_standings: bool,
    pub include_individuals: bool,
    pub include_games: bool,
    pub include_team_detail: bool,
    pub include_player_detail: bool,
    pub include_stat_key: bool,
    pub use_style_sheet: bool,
    pub british_style: bool,
}

impl Default for ReportSettings {
    fn default() -> Self {
        ReportSettings {
            base_name: String::new(),
            rounds: String::from("_rounds.html"),
            standings: String::from("_standings.html"),
            individuals: String::from("_individuals.html"),
            games: String::from("_games.html"),
            team_detail: String::from("_teamdetail.html"),
            player_detail: String::from("_playerdetail.html"),
            stat_key: String::from("_statkey.html"),
            style: String::new(),
            include_rounds: true,
            include_standings: true,
            include_individuals: true,
            include_games: true,
            include_team_detail: true,
            include_player_detail: true,
            include_stat_key: true,
            use_style_sheet: false,
            british_style: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct Tournament {
    pub name: String,
    pub teams: Vec<Team>,
    pub games: Vec<Game>,
    pub scoring: ScoringSettings,
    pub reports: ReportSettings,
    pub uses_divisions: bool,
    pub divisions: Vec<String>,
    pub track_bonuses: bool,
    pub max_players_per_team: u32,
    /// Standings sort method: 1=Record/PPG, 2=Record/H2H/PPG, 3=Record/Strength,
    /// 4=Record/PPTH, 5=Record/H2H/PPTH
    pub sort_method: u8,
    /// Warnings bitmask: bit7=W1(same team 2x), bit6=W2(same player 2x),
    /// bit5=W3(score inconsistency), bit4=W4(GP>max), bit3=W5(PPB out of range),
    /// bit2=W6(bonus pts but heard=0), bit1=W7(negative TUH)
    pub warn_flags: u8,
    /// Packet names keyed by round number
    pub packets: BTreeMap<u32, String>,
}

impl Default for Tournament {
    fn default() -> Self {
        Tournament {
            name: String::new(),
            teams: Vec::new(),
            games: Vec::new(),
            scoring: ScoringSettings::default(),
            reports: ReportSettings::default(),
            uses_divisions: false,
            divisions: Vec::new(),
            track_bonuses: true,
            max_players_per_team: 6,
            sort_method: 1, // Record / PPG
            warn_flags: 0b0111_1110, // W2-W7 on, W1 off
            packets: BTreeMap::new(),
        }
    }
}

impl Tournament {
    pub fn bouncebacks_enabled(&self) -> bool {
        self.scoring.auto_track >= 3
    }

    /// Effective `q_value` for file writing: 0 if disabled, actual value if enabled
    pub fn effective_q_value(&self, i: usize) -> i32 {
        if i >= 4 { return 0; }
        if self.scoring.q_enabled[i] { self.scoring.q_values[i] } else { 0 }
    }

    pub fn min_round(&self) -> Option<u32> {
        self.games.iter().map(|g| g.round).min()
    }

    pub fn max_round(&self) -> Option<u32> {
        self.games.iter().map(|g| g.round).max()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bouncebacks_disabled_by_default() {
        assert!(!Tournament::default().bouncebacks_enabled());
    }

    #[test]
    fn bouncebacks_enabled_at_threshold_3() {
        let mut t = Tournament::default();
        t.scoring.auto_track = 2;
        assert!(!t.bouncebacks_enabled());
        t.scoring.auto_track = 3;
        assert!(t.bouncebacks_enabled());
        t.scoring.auto_track = 4;
        assert!(t.bouncebacks_enabled());
    }

    #[test]
    fn default_q_values() {
        let s = ScoringSettings::default();
        assert_eq!(s.q_values, [20, 15, 10, -5]);
    }

    #[test]
    fn q_enabled_controls_effective_value() {
        let mut t = Tournament::default();
        t.scoring.q_enabled[1] = false;
        assert_eq!(t.effective_q_value(0), 20);
        assert_eq!(t.effective_q_value(1), 0);
    }

    #[test]
    fn default_report_suffixes_non_empty() {
        let r = ReportSettings::default();
        assert!(!r.standings.is_empty());
        assert!(!r.individuals.is_empty());
        assert!(!r.games.is_empty());
        assert!(!r.team_detail.is_empty());
        assert!(!r.player_detail.is_empty());
    }

    #[test]
    fn default_warn_flags() {
        let t = Tournament::default();
        assert_eq!(t.warn_flags, 0b0111_1110);
    }
}
