use crate::models::*;
use std::io::{self, BufRead, Write};

pub struct SqbsParser<R: BufRead> {
    reader: R,
    lines: Vec<String>,
    pos: usize,
}

impl<R: BufRead> SqbsParser<R> {
    pub fn new(reader: R) -> Self {
        SqbsParser { reader, lines: Vec::new(), pos: 0 }
    }

    fn load_lines(&mut self) -> io::Result<()> {
        let mut collected = Vec::new();
        loop {
            let mut line = String::new();
            let n = self.reader.read_line(&mut line)?;
            if n == 0 { break; }
            collected.push(line.trim_end_matches(['\r', '\n']).to_string());
        }
        self.lines = collected;
        Ok(())
    }

    fn next_line(&mut self) -> io::Result<String> {
        if self.pos >= self.lines.len() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "EOF"));
        }
        let s = self.lines[self.pos].clone();
        self.pos += 1;
        Ok(s)
    }

    fn next_int(&mut self) -> io::Result<i64> {
        let line = self.next_line()?;
        line.trim()
            .parse::<i64>()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
    }

    pub fn parse(mut self) -> io::Result<Tournament> {
        self.load_lines()?;
        let mut tournament = Tournament::default();

        let team_count_raw = self.next_int()?;
        if !(0..=100).contains(&team_count_raw) {
            return Err(io::Error::new(io::ErrorKind::InvalidData,
                format!("invalid team count: {}", team_count_raw)));
        }
        let team_count = team_count_raw as usize;
        for _ in 0..team_count {
            let player_count_raw = self.next_int()?;
            if player_count_raw < 0 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "negative player count"));
            }
            let player_count = player_count_raw as usize;
            let team_name = self.next_line()?;
            let mut players = Vec::with_capacity(player_count);
            for _ in 0..player_count {
                players.push(Player { name: self.next_line()? });
            }
            tournament.teams.push(Team {
                name: team_name,
                players,
                division: None,
                exhibition: false,
            });
        }

        let game_count_raw = self.next_int()?;
        if game_count_raw < 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData,
                format!("invalid game count: {}", game_count_raw)));
        }
        let game_count = game_count_raw as usize;
        let mut min_round: u32 = u32::MAX;
        let mut max_round: u32 = 0;
        for _ in 0..game_count {
            let game = self.parse_game()?;
            if game.round < min_round { min_round = game.round; }
            if game.round > max_round { max_round = game.round; }
            tournament.games.push(game);
        }
        if min_round == u32::MAX { min_round = 1; max_round = 0; }

        // Initialize packet slots for every round seen
        if max_round >= min_round {
            for r in min_round..=max_round {
                tournament.packets.entry(r).or_insert_with(|| String::from("-"));
            }
        }

        // Settings
        tournament.track_bonuses = self.next_int()? != 0;
        tournament.scoring.auto_track = self.next_int()? as u8;

        let track_power_neg_raw = self.next_int()?;
        let read_exhibition_info = (track_power_neg_raw & 2) != 0;
        tournament.scoring.track_power_neg = (track_power_neg_raw & 1) != 0;

        tournament.scoring.track_light_round = self.next_int()? != 0;
        tournament.scoring.track_tuh = self.next_int()? != 0;

        let tossup_sort_raw = self.next_int()?;
        let read_packets_info = (tossup_sort_raw & 2) != 0;
        tournament.scoring.sort_by_ppg = (tossup_sort_raw & 1) != 0;

        tournament.warn_flags = self.next_int()? as u8;

        // Per-report enable booleans
        tournament.reports.include_rounds        = self.next_int()? != 0;
        tournament.reports.include_standings     = self.next_int()? != 0;
        tournament.reports.include_individuals   = self.next_int()? != 0;
        tournament.reports.include_games         = self.next_int()? != 0;
        tournament.reports.include_team_detail   = self.next_int()? != 0;
        tournament.reports.include_player_detail = self.next_int()? != 0;
        tournament.reports.include_stat_key      = self.next_int()? != 0;
        tournament.reports.use_style_sheet       = self.next_int()? != 0;

        tournament.uses_divisions = self.next_int()? != 0;
        tournament.sort_method = self.next_int()? as u8;

        tournament.name = self.next_line()?;
        let _host_name = self.next_line()?;
        let _user_name = self.next_line()?;
        let _directory_name = self.next_line()?;
        tournament.reports.base_name = self.next_line()?;

        let paths_value = self.next_int()?;
        tournament.reports.british_style = (paths_value & 2) != 0;

        tournament.reports.rounds        = self.next_line()?;
        tournament.reports.standings     = self.next_line()?;
        tournament.reports.individuals   = self.next_line()?;
        tournament.reports.games         = self.next_line()?;
        tournament.reports.team_detail   = self.next_line()?;
        tournament.reports.player_detail = self.next_line()?;
        tournament.reports.stat_key      = self.next_line()?;
        tournament.reports.style         = self.next_line()?;

        let num_divisions = self.next_int()?;
        if num_divisions > 0 {
            for _ in 0..num_divisions {
                tournament.divisions.push(self.next_line()?);
            }
            let _ = self.next_int()?; // team count
            for i in 0..team_count {
                let div_idx = self.next_int()?;
                if div_idx >= 0 && (div_idx as usize) < tournament.divisions.len() {
                    tournament.teams[i].division =
                        Some(tournament.divisions[div_idx as usize].clone());
                }
            }
        } else {
            let _ = self.next_int()?;
            for _ in 0..team_count { let _ = self.next_int()?; }
        }

        // Question values (0 = disabled)
        for i in 0..4 {
            let v = self.next_int()? as i32;
            if v != 0 {
                tournament.scoring.q_values[i] = v;
                tournament.scoring.q_enabled[i] = true;
            } else {
                tournament.scoring.q_enabled[i] = false;
            }
        }

        // Optional packets
        if read_packets_info {
            if let Ok(n) = self.next_int() {
                if n >= 0 {
                    tournament.packets.clear();
                    for round_offset in 0..n {
                        let name = self.next_line().unwrap_or_else(|_| String::from("-"));
                        if let Some(round) = min_round.checked_add(round_offset as u32) {
                            tournament.packets.insert(round, name);
                        }
                    }
                }
            }
        }

        // Optional exhibition
        if read_exhibition_info {
            if let Ok(n) = self.next_int() {
                if n >= 0 {
                    for i in 0..(n as usize) {
                        if let Ok(v) = self.next_int() {
                            if i < tournament.teams.len() {
                                tournament.teams[i].exhibition = v != 0;
                            }
                        }
                    }
                }
            }
        }

        Ok(tournament)
    }

    fn parse_game(&mut self) -> io::Result<Game> {
        let game_index = self.next_line()?;
        let team_a_idx = self.next_int()? as usize;
        let team_b_idx = self.next_int()? as usize;
        let a_score_str = self.next_line()?;
        let b_score_str = self.next_line()?;
        let tossups_heard = self.next_int()? as u32;
        let round = self.next_int()? as u32;

        let a_bonus_encoded   = self.next_int()?;
        let a_bonus_pts_enc   = self.next_int()?;
        let b_bonus_encoded   = self.next_int()?;
        let b_bonus_pts_enc   = self.next_int()?;

        const BB: i64 = 10000;
        let a_bonus_heard  = (a_bonus_encoded % BB) as i32;
        let a_bb_heard     = (a_bonus_encoded / BB) as i32;
        let a_bonus_points = (a_bonus_pts_enc % BB) as i32;
        let a_bb_points    = (a_bonus_pts_enc / BB) as i32;
        let b_bonus_heard  = (b_bonus_encoded % BB) as i32;
        let b_bb_heard     = (b_bonus_encoded / BB) as i32;
        let b_bonus_points = (b_bonus_pts_enc % BB) as i32;
        let b_bb_points    = (b_bonus_pts_enc / BB) as i32;

        let overtime_raw = self.next_int()?;
        let a_ot   = self.next_int()? as i32;
        let b_ot   = self.next_int()? as i32;
        let forfeit_raw = self.next_int()?;
        let a_lgt  = self.next_int()? as i32;
        let b_lgt  = self.next_int()? as i32;

        let a_total: i32 = a_score_str.trim().parse().unwrap_or(0);
        let b_total: i32 = b_score_str.trim().parse().unwrap_or(0);

        let mut a_ps: Vec<Option<PlayerScore>> = Vec::new();
        let mut b_ps: Vec<Option<PlayerScore>> = Vec::new();
        for _ in 0..8 {
            a_ps.push(self.parse_player_record()?);
            b_ps.push(self.parse_player_record()?);
        }

        Ok(Game {
            game_index,
            round,
            tossups_heard,
            overtime: overtime_raw != 0,
            forfeit: forfeit_raw != 0,
            team_a: TeamScore {
                team_index: team_a_idx,
                total_points: a_total,
                bonus_heard: a_bonus_heard, bonus_points: a_bonus_points,
                bb_heard: a_bb_heard, bb_points: a_bb_points,
                ot_gets: a_ot, lightning_points: a_lgt,
                player_scores: a_ps,
            },
            team_b: TeamScore {
                team_index: team_b_idx,
                total_points: b_total,
                bonus_heard: b_bonus_heard, bonus_points: b_bonus_points,
                bb_heard: b_bb_heard, bb_points: b_bb_points,
                ot_gets: b_ot, lightning_points: b_lgt,
                player_scores: b_ps,
            },
        })
    }

    fn parse_player_record(&mut self) -> io::Result<Option<PlayerScore>> {
        let idx_raw = self.next_int()?;
        if idx_raw < 0 {
            for _ in 0..6 { let _ = self.next_line()?; }
            return Ok(None);
        }
        let gp: f32 = self.next_line()?.trim().parse().unwrap_or(0.0);
        let q0 = self.next_int()? as i32;
        let q1 = self.next_int()? as i32;
        let q2 = self.next_int()? as i32;
        let q3 = self.next_int()? as i32;
        let pts = self.next_int()? as i32;
        Ok(Some(PlayerScore { player_index: idx_raw as usize, gp, q: [q0, q1, q2, q3], points: pts }))
    }
}

pub fn write_sqbs<W: Write>(w: &mut W, t: &Tournament) -> io::Result<()> {
    let team_count = t.teams.len();

    writeln!(w, "{}", team_count)?;
    for team in &t.teams {
        writeln!(w, "{}", team.players.len())?;
        writeln!(w, "{}", team.name)?;
        for p in &team.players {
            writeln!(w, "{}", p.name)?;
        }
    }

    writeln!(w, "{}", t.games.len())?;
    for game in &t.games {
        writeln!(w, "{}", game.game_index)?;
        writeln!(w, "{}", game.team_a.team_index)?;
        writeln!(w, "{}", game.team_b.team_index)?;
        writeln!(w, "{}", game.team_a.total_points)?;
        writeln!(w, "{}", game.team_b.total_points)?;
        writeln!(w, "{}", game.tossups_heard)?;
        writeln!(w, "{}", game.round)?;

        const BB: i64 = 10000;
        writeln!(w, "{}", (game.team_a.bonus_heard as i64) + (game.team_a.bb_heard as i64) * BB)?;
        writeln!(w, "{}", (game.team_a.bonus_points as i64) + (game.team_a.bb_points as i64) * BB)?;
        writeln!(w, "{}", (game.team_b.bonus_heard as i64) + (game.team_b.bb_heard as i64) * BB)?;
        writeln!(w, "{}", (game.team_b.bonus_points as i64) + (game.team_b.bb_points as i64) * BB)?;

        writeln!(w, "{}", if game.overtime { 1 } else { 0 })?;
        writeln!(w, "{}", game.team_a.ot_gets)?;
        writeln!(w, "{}", game.team_b.ot_gets)?;
        writeln!(w, "{}", if game.forfeit { 1 } else { 0 })?;
        writeln!(w, "{}", game.team_a.lightning_points)?;
        writeln!(w, "{}", game.team_b.lightning_points)?;

        for slot in 0..8 {
            write_player_record(w, game.team_a.player_scores.get(slot).and_then(|x| x.as_ref()))?;
            write_player_record(w, game.team_b.player_scores.get(slot).and_then(|x| x.as_ref()))?;
        }
    }

    writeln!(w, "{}", if t.track_bonuses { 1 } else { 0 })?;
    writeln!(w, "{}", t.scoring.auto_track)?;
    // Bit 1 always set (signals exhibition section follows); bit 0 = track_power_neg
    writeln!(w, "{}", if t.scoring.track_power_neg { 3i64 } else { 2 })?;
    writeln!(w, "{}", if t.scoring.track_light_round { 1 } else { 0 })?;
    writeln!(w, "{}", if t.scoring.track_tuh { 1 } else { 0 })?;
    // Bit 1 always set (signals packet section follows); bit 0 = sort_by_ppg
    writeln!(w, "{}", if t.scoring.sort_by_ppg { 3i64 } else { 2 })?;
    writeln!(w, "{}", t.warn_flags)?;

    writeln!(w, "{}", if t.reports.include_rounds        { 1 } else { 0 })?;
    writeln!(w, "{}", if t.reports.include_standings     { 1 } else { 0 })?;
    writeln!(w, "{}", if t.reports.include_individuals   { 1 } else { 0 })?;
    writeln!(w, "{}", if t.reports.include_games         { 1 } else { 0 })?;
    writeln!(w, "{}", if t.reports.include_team_detail   { 1 } else { 0 })?;
    writeln!(w, "{}", if t.reports.include_player_detail { 1 } else { 0 })?;
    writeln!(w, "{}", if t.reports.include_stat_key      { 1 } else { 0 })?;
    writeln!(w, "{}", if t.reports.use_style_sheet       { 1 } else { 0 })?;

    writeln!(w, "{}", if t.uses_divisions { 1 } else { 0 })?;
    writeln!(w, "{}", t.sort_method)?;

    writeln!(w, "{}", t.name)?;
    writeln!(w)?; writeln!(w)?; writeln!(w)?; // host, user, directory
    writeln!(w, "{}", t.reports.base_name)?;
    writeln!(w, "{}", if t.reports.british_style { 2i64 } else { 0 })?;

    writeln!(w, "{}", t.reports.rounds)?;
    writeln!(w, "{}", t.reports.standings)?;
    writeln!(w, "{}", t.reports.individuals)?;
    writeln!(w, "{}", t.reports.games)?;
    writeln!(w, "{}", t.reports.team_detail)?;
    writeln!(w, "{}", t.reports.player_detail)?;
    writeln!(w, "{}", t.reports.stat_key)?;
    writeln!(w, "{}", t.reports.style)?;

    if t.uses_divisions && !t.divisions.is_empty() {
        writeln!(w, "{}", t.divisions.len())?;
        for div in &t.divisions { writeln!(w, "{}", div)?; }
        writeln!(w, "{}", team_count)?;
        for team in &t.teams {
            let idx = match &team.division {
                Some(name) => t.divisions.iter().position(|d| d == name).map(|i| i as i64).unwrap_or(-1),
                None => -1,
            };
            writeln!(w, "{}", idx)?;
        }
    } else {
        writeln!(w, "0")?;
        writeln!(w, "0")?;
        for _ in 0..team_count { writeln!(w, "0")?; }
    }

    for i in 0..4 { writeln!(w, "{}", t.effective_q_value(i))?; }

    // Packets: always write count; write names only when count > 0
    match (t.min_round(), t.max_round()) {
        (Some(min_r), Some(max_r)) if max_r >= min_r && !t.packets.is_empty() => {
            let count = max_r - min_r + 1;
            writeln!(w, "{}", count)?;
            for r in min_r..=max_r {
                writeln!(w, "{}", t.packets.get(&r).map(|s| s.as_str()).unwrap_or(" "))?;
            }
        }
        _ => { writeln!(w, "0")?; }
    }

    // Exhibition: always write team count + per-team flags
    writeln!(w, "{}", team_count)?;
    for team in &t.teams {
        writeln!(w, "{}", if team.exhibition { 1 } else { 0 })?;
    }

    Ok(())
}

fn write_player_record<W: Write>(w: &mut W, ps: Option<&PlayerScore>) -> io::Result<()> {
    match ps {
        None => {
            writeln!(w, "-1")?;
            for _ in 0..6 { writeln!(w, "0")?; }
        }
        Some(p) => {
            writeln!(w, "{}", p.player_index)?;
            writeln!(w, "{:.2}", p.gp)?;
            writeln!(w, "{}", p.q[0])?;
            writeln!(w, "{}", p.q[1])?;
            writeln!(w, "{}", p.q[2])?;
            writeln!(w, "{}", p.q[3])?;
            writeln!(w, "{}", p.points)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, Cursor};

    fn round_trip(t: &Tournament) -> Tournament {
        let mut buf = Vec::new();
        write_sqbs(&mut buf, t).expect("write failed");
        let reader = BufReader::new(Cursor::new(buf));
        SqbsParser::new(reader).parse().expect("parse failed")
    }

    fn minimal() -> Tournament {
        let mut t = Tournament::default();
        t.name = "My Tournament".to_string();
        t.reports.base_name = "mytest".to_string();
        t.teams = vec![
            Team { name: "Alpha".to_string(), players: vec![Player { name: "Alice".to_string() }], division: None, exhibition: false },
            Team { name: "Beta".to_string(),  players: vec![Player { name: "Bob".to_string() }],  division: None, exhibition: false },
        ];
        t
    }

    fn with_game() -> Tournament {
        let mut t = minimal();
        let mut ps_a: Vec<Option<PlayerScore>> = vec![
            Some(PlayerScore { player_index: 0, gp: 1.0, q: [1, 0, 2, 0], points: 40 }),
        ];
        let mut ps_b: Vec<Option<PlayerScore>> = vec![
            Some(PlayerScore { player_index: 0, gp: 0.5, q: [0, 0, 1, 1], points: 5 }),
        ];
        while ps_a.len() < 8 { ps_a.push(None); }
        while ps_b.len() < 8 { ps_b.push(None); }
        t.games.push(Game {
            game_index: "1".to_string(), round: 3, tossups_heard: 20,
            overtime: true, forfeit: false,
            team_a: TeamScore { team_index: 0, total_points: 100, bonus_heard: 5, bonus_points: 30, bb_heard: 2, bb_points: 15, ot_gets: 1, lightning_points: 0, player_scores: ps_a },
            team_b: TeamScore { team_index: 1, total_points: 55,  bonus_heard: 3, bonus_points: 20, bb_heard: 0, bb_points: 0,  ot_gets: 0, lightning_points: 0, player_scores: ps_b },
        });
        t
    }

    #[test]
    fn round_trip_team_and_player_names() {
        let t2 = round_trip(&minimal());
        assert_eq!(t2.teams[0].name, "Alpha");
        assert_eq!(t2.teams[1].players[0].name, "Bob");
    }

    #[test]
    fn round_trip_tournament_name_and_base_name() {
        let t2 = round_trip(&minimal());
        assert_eq!(t2.name, "My Tournament");
        assert_eq!(t2.reports.base_name, "mytest");
    }

    #[test]
    fn round_trip_per_report_enables() {
        let mut t = minimal();
        t.reports.include_rounds = false;
        t.reports.include_stat_key = false;
        t.reports.british_style = true;
        let t2 = round_trip(&t);
        assert!(!t2.reports.include_rounds);
        assert!(!t2.reports.include_stat_key);
        assert!(t2.reports.british_style);
        assert!(t2.reports.include_standings);
    }

    #[test]
    fn round_trip_game_scalars() {
        let t2 = round_trip(&with_game());
        let g = &t2.games[0];
        assert_eq!(g.round, 3);
        assert!(g.overtime);
        assert_eq!(g.team_a.total_points, 100);
    }

    #[test]
    fn round_trip_player_scores() {
        let t2 = round_trip(&with_game());
        let ps = t2.games[0].team_a.player_scores[0].as_ref().unwrap();
        assert_eq!(ps.q, [1, 0, 2, 0]);
        assert!(t2.games[0].team_a.player_scores[1].is_none());
    }

    #[test]
    fn round_trip_bounceback_encoding() {
        let t2 = round_trip(&with_game());
        assert_eq!(t2.games[0].team_a.bb_heard, 2);
        assert_eq!(t2.games[0].team_a.bb_points, 15);
    }

    #[test]
    fn round_trip_scoring_settings() {
        let mut t = minimal();
        t.scoring.q_values = [20, 15, 10, -5];
        t.scoring.q_enabled = [true, true, true, true];
        t.scoring.auto_track = 3;
        let t2 = round_trip(&t);
        assert_eq!(t2.scoring.q_values, [20, 15, 10, -5]);
        assert_eq!(t2.scoring.q_enabled, [true, true, true, true]);
        assert!(t2.bouncebacks_enabled());
    }

    #[test]
    fn round_trip_disabled_q_value() {
        let mut t = minimal();
        t.scoring.q_values = [20, 15, 10, -5];
        t.scoring.q_enabled = [true, false, true, true];
        let t2 = round_trip(&t);
        assert!(t2.scoring.q_enabled[0]);
        assert!(!t2.scoring.q_enabled[1]);
    }

    #[test]
    fn round_trip_sort_method_and_warn_flags() {
        let mut t = minimal();
        t.sort_method = 3;
        t.warn_flags = 0b0111_1110;
        let t2 = round_trip(&t);
        assert_eq!(t2.sort_method, 3);
        assert_eq!(t2.warn_flags, 0b0111_1110);
    }

    #[test]
    fn round_trip_packets() {
        let mut t = with_game(); // round 3
        t.packets.insert(3, "Packet Gamma".to_string());
        let t2 = round_trip(&t);
        assert_eq!(t2.packets.get(&3).map(|s| s.as_str()), Some("Packet Gamma"));
    }

    #[test]
    fn round_trip_divisions() {
        let mut t = minimal();
        t.uses_divisions = true;
        t.divisions = vec!["East".to_string(), "West".to_string()];
        t.teams[0].division = Some("East".to_string());
        t.teams[1].division = Some("West".to_string());
        let t2 = round_trip(&t);
        assert_eq!(t2.teams[0].division.as_deref(), Some("East"));
        assert_eq!(t2.teams[1].division.as_deref(), Some("West"));
    }

    #[test]
    fn round_trip_exhibition() {
        let mut t = minimal();
        t.teams[1].exhibition = true;
        let t2 = round_trip(&t);
        assert!(!t2.teams[0].exhibition);
        assert!(t2.teams[1].exhibition);
    }

    #[test]
    fn round_trip_sort_by_ppg() {
        let mut t = minimal();
        t.scoring.sort_by_ppg = true;
        let t2 = round_trip(&t);
        assert!(t2.scoring.sort_by_ppg);

        let mut t3 = minimal();
        t3.scoring.sort_by_ppg = false;
        let t4 = round_trip(&t3);
        assert!(!t4.scoring.sort_by_ppg);
    }

    #[test]
    fn empty_tournament_round_trips() {
        let t2 = round_trip(&Tournament::default());
        assert_eq!(t2.teams.len(), 0);
        assert_eq!(t2.games.len(), 0);
    }
}
