use crate::models::{ReportSettings, Tournament, Game, TeamScore};
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::io;
use std::path::Path;

fn safe_div(a: f64, b: f64) -> f64 {
    if b == 0.0 { 0.0 } else { a / b }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
}

pub(crate) struct Nav {
    standings: String,
    individuals: String,
    games: String,
    team_detail: String,
    player_detail: String,
    rounds: String,
    stat_key: String,
    style: String,
}

impl Nav {
    fn new(base: &str, r: &ReportSettings) -> Self {
        Self {
            standings: format!("{}{}", base, r.standings),
            individuals: format!("{}{}", base, r.individuals),
            games: format!("{}{}", base, r.games),
            team_detail: format!("{}{}", base, r.team_detail),
            player_detail: format!("{}{}", base, r.player_detail),
            rounds: format!("{}{}", base, r.rounds),
            stat_key: format!("{}{}", base, r.stat_key),
            style: if r.style.is_empty() { String::new() } else { format!("{}{}", base, r.style) },
        }
    }

    fn bar(&self) -> String {
        format!(
            "<table border=0 width=100%>\n<tr>\n  \
<td><A HREF={s}>Standings</A></td>\n  \
<td><A HREF={i}>Individuals</A></td>\n  \
<td><A HREF={g}>Scoreboard</A></td>\n  \
<td><A HREF={td}>Team Detail</A></td>\n  \
<td><A HREF={pd}>Individual Detail</A></td>\n  \
<td><A HREF={r}>Round Report</A></td>\n  \
<td><A HREF={sk}>Stat Key</A></td>\n\
</tr>\n</table>\n",
            s = self.standings, i = self.individuals, g = self.games,
            td = self.team_detail, pd = self.player_detail, r = self.rounds, sk = self.stat_key
        )
    }

    fn style_link(&self) -> String {
        if self.style.is_empty() {
            String::new()
        } else {
            format!("<LINK rel=\"stylesheet\" href=\"{}\">\n", self.style)
        }
    }
}

fn html_page(title: &str, nav: &Nav, body: &str) -> String {
    format!(
        "<HTML>\n<HEAD>\n<TITLE>{}</TITLE>\n{}\n</HEAD>\n<BODY>\n{}{}</BODY>\n</HTML>\n",
        title,
        nav.style_link(),
        nav.bar(),
        body
    )
}

// ── Aggregated team stats ──────────────────────────────────────────────────

struct TeamAgg {
    team_index: usize,
    wins: i32,
    losses: i32,
    ties: i32,
    pf: i32,
    pa: i32,
    games: i32,
    q: [i32; 4],
    tuh: i32,
    bh: i32,
    bp: i32,
    bbh: i32,
    bbp: i32,
}

fn aggregate_teams(t: &Tournament) -> Vec<TeamAgg> {
    let n = t.teams.len();
    let mut agg: Vec<TeamAgg> = (0..n)
        .map(|i| TeamAgg {
            team_index: i,
            wins: 0, losses: 0, ties: 0,
            pf: 0, pa: 0, games: 0,
            q: [0; 4], tuh: 0,
            bh: 0, bp: 0, bbh: 0, bbp: 0,
        })
        .collect();

    for game in &t.games {
        if game.forfeit { continue; }
        let (ai, bi) = (game.team_a.team_index, game.team_b.team_index);
        if ai >= n || bi >= n { continue; }
        let (a_pts, b_pts) = (game.team_a.total_points, game.team_b.total_points);

        for (ti, ts, opp) in [
            (ai, &game.team_a, b_pts),
            (bi, &game.team_b, a_pts),
        ] {
            agg[ti].games += 1;
            agg[ti].pf += ts.total_points;
            agg[ti].pa += opp;
            agg[ti].tuh += game.tossups_heard as i32;
            agg[ti].bh += ts.bonus_heard;
            agg[ti].bp += ts.bonus_points;
            agg[ti].bbh += ts.bb_heard;
            agg[ti].bbp += ts.bb_points;
            for ps in ts.player_scores.iter().flatten() {
                for j in 0..4 { agg[ti].q[j] += ps.q[j]; }
            }
        }
        if a_pts > b_pts { agg[ai].wins += 1; agg[bi].losses += 1; }
        else if b_pts > a_pts { agg[bi].wins += 1; agg[ai].losses += 1; }
        else { agg[ai].ties += 1; agg[bi].ties += 1; }
    }
    agg
}

// ── Aggregated player stats ────────────────────────────────────────────────

struct PlayerGameEntry {
    opp_team_index: usize,
    round: u32,
    gp: f64,
    q: [i32; 4],
    tuh: i32,
    pts: i32,
}

struct PlayerAgg {
    team_index: usize,
    player_index: usize,
    gp: f64,
    q: [i32; 4],
    tuh: i32,
    pts: i32,
    games: Vec<PlayerGameEntry>,
}

fn aggregate_players(t: &Tournament) -> Vec<PlayerAgg> {
    let n = t.teams.len();
    let mut map: HashMap<(usize, usize), PlayerAgg> = HashMap::new();

    for game in &t.games {
        if game.forfeit { continue; }
        let ai = game.team_a.team_index;
        let bi = game.team_b.team_index;
        if ai >= n || bi >= n { continue; }

        for (ts, opp_idx) in [(&game.team_a, bi), (&game.team_b, ai)] {
            for ps in ts.player_scores.iter().flatten() {
                let tuh = (ps.gp * game.tossups_heard as f32).round() as i32;
                let entry = PlayerGameEntry {
                    opp_team_index: opp_idx,
                    round: game.round,
                    gp: f64::from(ps.gp),
                    q: ps.q,
                    tuh,
                    pts: ps.points,
                };
                let agg = map
                    .entry((ts.team_index, ps.player_index))
                    .or_insert(PlayerAgg {
                        team_index: ts.team_index,
                        player_index: ps.player_index,
                        gp: 0.0, q: [0; 4], tuh: 0, pts: 0, games: Vec::new(),
                    });
                agg.gp += f64::from(ps.gp);
                for j in 0..4 { agg.q[j] += ps.q[j]; }
                agg.tuh += tuh;
                agg.pts += ps.points;
                agg.games.push(entry);
            }
        }
    }

    let mut result: Vec<PlayerAgg> = map.into_values().collect();
    result.sort_by(|a, b| a.team_index.cmp(&b.team_index).then(a.player_index.cmp(&b.player_index)));
    result
}

// ── Q-value column headers ─────────────────────────────────────────────────

fn q_headers_right(qv: &[i32; 4], enabled: &[bool; 4]) -> String {
    qv.iter().zip(enabled).filter(|(_, &e)| e).map(|(v, _)| format!("  <td ALIGN=RIGHT><B>{v}</B></td>\n")).collect()
}

fn q_cells_right(q: &[i32; 4], enabled: &[bool; 4]) -> String {
    q.iter().zip(enabled).filter(|(_, &e)| e).map(|(v, _)| format!("  <td ALIGN=RIGHT>{v}</td>\n")).collect()
}

fn q_cells_bold(q: &[i32; 4], enabled: &[bool; 4]) -> String {
    q.iter().zip(enabled).filter(|(_, &e)| e).map(|(v, _)| format!("  <td ALIGN=RIGHT><B>{v}</B></td>\n")).collect()
}

fn q_label(qv: &[i32; 4], enabled: &[bool; 4]) -> String {
    qv.iter().zip(enabled).filter(|(_, &e)| e).map(|(v, _)| v.to_string()).collect::<Vec<_>>().join(" ")
}

fn q_score_inline(q: &[i32; 4], enabled: &[bool; 4]) -> String {
    q.iter().zip(enabled).filter(|(_, &e)| e).map(|(v, _)| v.to_string()).collect::<Vec<_>>().join(" ")
}

fn ratio_inf(numerator: i32, denominator: i32) -> String {
    if denominator == 0 { "inf".to_string() } else { format!("{:.2}", f64::from(numerator) / f64::from(denominator)) }
}

fn neg_slot(qv: &[i32; 4], enabled: &[bool; 4]) -> Option<usize> {
    qv.iter().zip(enabled.iter()).enumerate()
        .find(|(_, (v, &e))| e && **v < 0)
        .map(|(i, _)| i)
}

fn pn_str(q: &[i32; 4], qv: &[i32; 4], enabled: &[bool; 4]) -> String {
    match neg_slot(qv, enabled) {
        Some(ni) => ratio_inf(q[0], q[ni]),
        None => "inf".to_string(),
    }
}

fn gn_str(q: &[i32; 4], qv: &[i32; 4], enabled: &[bool; 4]) -> String {
    let gets: i32 = q.iter().zip(qv.iter()).zip(enabled.iter())
        .filter(|((_, v), &e)| e && **v > 0)
        .map(|((cnt, _), _)| *cnt)
        .sum();
    match neg_slot(qv, enabled) {
        Some(ni) => ratio_inf(gets, q[ni]),
        None => "inf".to_string(),
    }
}

fn packet_label(t: &Tournament, round: u32) -> String {
    match t.packets.get(&round) {
        Some(name) if name != "-" => name.clone(),
        _ => round.to_string(),
    }
}

// ── Standings ──────────────────────────────────────────────────────────────

pub fn standings_html(t: &Tournament, nav: &Nav) -> String {
    let agg = aggregate_teams(t);
    let mut sorted: Vec<&TeamAgg> = agg.iter().filter(|a| a.games > 0).collect();
    sorted.sort_by(|a, b| {
        let a_pct = safe_div(f64::from(a.wins) + f64::from(a.ties) * 0.5, f64::from(a.games));
        let b_pct = safe_div(f64::from(b.wins) + f64::from(b.ties) * 0.5, f64::from(b.games));
        b_pct.partial_cmp(&a_pct).unwrap()
            .then(safe_div(f64::from(b.pf), f64::from(b.games))
                .partial_cmp(&safe_div(f64::from(a.pf), f64::from(a.games))).unwrap())
    });

    let bb = t.bouncebacks_enabled();
    let qv = &t.scoring.q_values;
    let qe = &t.scoring.q_enabled;

    let mut body = String::new();
    body.push_str(&format!("<H1>{} Team Standings </H1><P>\n", t.name));
    body.push_str("<table border=1 width=100%>\n");
    body.push_str("<tr>\n");
    body.push_str("  <td ALIGN=LEFT><B>Rank</B></td>\n");
    body.push_str("  <td ALIGN=LEFT><B>Team</B></td>\n");
    body.push_str("  <td ALIGN=RIGHT><B>W</B></td>\n");
    body.push_str("  <td ALIGN=RIGHT><B>L</B></td>\n");
    body.push_str("  <td ALIGN=RIGHT><B>T</B></td>\n");
    body.push_str("  <td ALIGN=RIGHT><B>Pct</B></td>\n");
    body.push_str("  <td ALIGN=RIGHT><B>PPG</B></td>\n");
    body.push_str("  <td ALIGN=RIGHT><B>PAPG</B></td>\n");
    body.push_str("  <td ALIGN=RIGHT><B>Mrg</B></td>\n");
    body.push_str(&q_headers_right(qv, qe));
    body.push_str("  <td ALIGN=RIGHT><B>TUH</B></td>\n");
    body.push_str("  <td ALIGN=RIGHT><B>P/TU</B></td>\n");
    body.push_str("  <td ALIGN=RIGHT><B>P/N</B></td>\n");
    body.push_str("  <td ALIGN=RIGHT><B>G/N</B></td>\n");
    if t.track_bonuses {
        body.push_str("  <td ALIGN=RIGHT><B>BHrd</B></td>\n");
        body.push_str("  <td ALIGN=RIGHT><B>BPts</B></td>\n");
        body.push_str("  <td ALIGN=RIGHT><B>P/B</B></td>\n");
        if bb {
            body.push_str("  <td ALIGN=RIGHT><B>BBHrd</B></td>\n");
            body.push_str("  <td ALIGN=RIGHT><B>BBPts</B></td>\n");
            body.push_str("  <td ALIGN=RIGHT><B>P/BB</B></td>\n");
        }
    }
    body.push_str("</tr>\n");

    for (rank, a) in sorted.iter().enumerate() {
        let team_name = escape_html(t.teams.get(a.team_index).map_or("?", |t| t.name.as_str()));
        let pct = safe_div(f64::from(a.wins) + f64::from(a.ties) * 0.5, f64::from(a.games));
        let ppg = safe_div(f64::from(a.pf), f64::from(a.games));
        let papg = safe_div(f64::from(a.pa), f64::from(a.games));
        let mrg = safe_div(f64::from(a.pf - a.pa), f64::from(a.games));
        let ppth = safe_div(f64::from(a.pf), f64::from(a.tuh));
        let pn = pn_str(&a.q, qv, qe);
        let gn = gn_str(&a.q, qv, qe);

        body.push_str("<tr>\n");
        body.push_str(&format!(
            "  <td ALIGN=LEFT>{}</td>  <td ALIGN=LEFT>\n<A HREF={}#t{}>{}</A></td>\n",
            rank + 1, nav.team_detail, a.team_index, team_name
        ));
        body.push_str(&format!("  <td ALIGN=RIGHT>{}</td>\n", a.wins));
        body.push_str(&format!("  <td ALIGN=RIGHT>{}</td>\n", a.losses));
        body.push_str(&format!("  <td ALIGN=RIGHT>{}</td>\n", a.ties));
        body.push_str(&format!("  <td ALIGN=RIGHT>{pct:.3}</td>\n"));
        body.push_str(&format!("  <td ALIGN=RIGHT>{ppg:.1}</td>\n"));
        body.push_str(&format!("  <td ALIGN=RIGHT>{papg:.1}</td>\n"));
        body.push_str(&format!("  <td ALIGN=RIGHT>{mrg:.1}</td>\n"));
        body.push_str(&q_cells_right(&a.q, qe));
        body.push_str(&format!("  <td ALIGN=RIGHT>{}</td>\n", a.tuh));
        body.push_str(&format!("  <td ALIGN=RIGHT>{ppth:.2}</td>\n"));
        body.push_str(&format!("  <td ALIGN=RIGHT>{pn}</td>\n"));
        body.push_str(&format!("  <td ALIGN=RIGHT>{gn}</td>\n"));
        if t.track_bonuses {
            body.push_str(&format!("  <td ALIGN=RIGHT>{}</td>\n", a.bh));
            body.push_str(&format!("  <td ALIGN=RIGHT>{}</td>\n", a.bp));
            body.push_str(&format!("  <td ALIGN=RIGHT>{:.2}</td>\n", safe_div(f64::from(a.bp), f64::from(a.bh))));
            if bb {
                body.push_str(&format!("  <td ALIGN=RIGHT>{}</td>\n", a.bbh));
                body.push_str(&format!("  <td ALIGN=RIGHT>{}</td>\n", a.bbp));
                body.push_str(&format!("  <td ALIGN=RIGHT>{:.2}</td>\n", safe_div(f64::from(a.bbp), f64::from(a.bbh))));
            }
        }
        body.push_str("</tr>\n");
    }
    body.push_str("</table>\n");

    html_page(&format!("{} Team Standings ", t.name), nav, &body)
}

// ── Individuals ────────────────────────────────────────────────────────────

pub fn individuals_html(t: &Tournament, nav: &Nav) -> String {
    let players = aggregate_players(t);
    let mut sorted: Vec<&PlayerAgg> = players.iter().collect();
    sorted.sort_by(|a, b| {
        let a_ppg = safe_div(f64::from(a.pts), a.gp);
        let b_ppg = safe_div(f64::from(b.pts), b.gp);
        b_ppg.partial_cmp(&a_ppg).unwrap()
            .then(b.pts.cmp(&a.pts))
            .then(a.team_index.cmp(&b.team_index))
            .then(a.player_index.cmp(&b.player_index))
    });

    let qv = &t.scoring.q_values;
    let qe = &t.scoring.q_enabled;
    let mut body = String::new();
    body.push_str(&format!("<H1>{} Individual Statistics </H1><P>\n", t.name));
    body.push_str("<table border=1 width=100%>\n<tr>\n");
    body.push_str("  <td ALIGN=LEFT><B>Rank</B></td>\n");
    body.push_str("  <td ALIGN=LEFT><B>Player</B></td>\n");
    body.push_str("  <td ALIGN=LEFT><B>Team</B></td>\n");
    body.push_str("  <td ALIGN=RIGHT><B>GP</B></td>\n");
    body.push_str(&q_headers_right(qv, qe));
    body.push_str("  <td ALIGN=RIGHT><B>TUH</B></td>\n");
    body.push_str("  <td ALIGN=RIGHT><B>P/TU</B></td>\n");
    body.push_str("  <td ALIGN=RIGHT><B>P/N</B></td>\n");
    body.push_str("  <td ALIGN=RIGHT><B>G/N</B></td>\n");
    body.push_str("  <td ALIGN=RIGHT><B>Pts</B></td>\n");
    body.push_str("  <td ALIGN=RIGHT><B>PPG</B></td>\n");
    body.push_str("</tr>\n");

    for (rank, p) in sorted.iter().enumerate() {
        let team_name = t.teams.get(p.team_index).map_or("?", |t| t.name.as_str());
        let player_name = escape_html(t.teams.get(p.team_index)
            .and_then(|tm| tm.players.get(p.player_index))
            .map_or("?", |pl| pl.name.as_str()));
        let ppg = safe_div(f64::from(p.pts), p.gp);
        let ptu = safe_div(f64::from(p.pts), f64::from(p.tuh));
        let pn = pn_str(&p.q, qv, qe);
        let gn = gn_str(&p.q, qv, qe);
        let anchor = format!("{}#p{}_{}", nav.player_detail, p.player_index + 1, p.team_index);

        body.push_str("<tr>\n");
        body.push_str(&format!("  <td ALIGN=LEFT>{}</td>\n", rank + 1));
        body.push_str(&format!("  <td ALIGN=LEFT><A HREF={anchor}>{player_name}</A></td>\n"));
        body.push_str(&format!("  <td ALIGN=LEFT>{}</td>\n", escape_html(team_name)));
        body.push_str(&format!("  <td ALIGN=RIGHT>{:.2}</td>\n", p.gp));
        body.push_str(&q_cells_right(&p.q, qe));
        body.push_str(&format!("  <td ALIGN=RIGHT>{}</td>\n", p.tuh));
        body.push_str(&format!("  <td ALIGN=RIGHT>{ptu:.2}</td>\n"));
        body.push_str(&format!("  <td ALIGN=RIGHT>{pn}</td>\n"));
        body.push_str(&format!("  <td ALIGN=RIGHT>{gn}</td>\n"));
        body.push_str(&format!("  <td ALIGN=RIGHT>{}</td>\n", p.pts));
        body.push_str(&format!("  <td ALIGN=RIGHT>{ppg:.2}</td>\n"));
        body.push_str("</tr>\n");
    }
    body.push_str("</table>\n");
    html_page(&format!("{} Individual Statistics ", t.name), nav, &body)
}

// ── Scoreboard (games) ─────────────────────────────────────────────────────

pub fn games_html(t: &Tournament, nav: &Nav) -> String {
    let bb = t.bouncebacks_enabled();
    let qe = &t.scoring.q_enabled;
    let mut rounds: Vec<u32> = t.games.iter().map(|g| g.round).collect();
    rounds.sort_unstable();
    rounds.dedup();

    let mut body = String::new();
    body.push_str(&format!("<H1>{} Scoreboard </H1><P>\n", t.name));

    for (idx, round) in rounds.iter().enumerate() {
        if idx > 0 { body.push_str("<hr>"); }
        let pkt = packet_label(t, *round);
        if pkt == round.to_string() {
            body.push_str(&format!("<FONT SIZE=+1 COLOR=red>Round {round}</FONT><P>\n"));
        } else {
            body.push_str(&format!("<FONT SIZE=+1 COLOR=red>Round {round} (Packet: {pkt})</FONT><P>\n"));
        }
        let round_games: Vec<&Game> = t.games.iter().filter(|g| g.round == *round).collect();

        for game in &round_games {
            let a_name = escape_html(t.teams.get(game.team_a.team_index).map_or("?", |t| t.name.as_str()));
            let b_name = escape_html(t.teams.get(game.team_b.team_index).map_or("?", |t| t.name.as_str()));
            let (winner_name, winner_pts, loser_name, loser_pts) =
                if game.team_a.total_points >= game.team_b.total_points {
                    (a_name.as_str(), game.team_a.total_points, b_name.as_str(), game.team_b.total_points)
                } else {
                    (b_name.as_str(), game.team_b.total_points, a_name.as_str(), game.team_a.total_points)
                };
            body.push_str(&format!("<FONT SIZE=+1>{winner_name} {winner_pts}, {loser_name} {loser_pts}</FONT><br>\n"));
            body.push_str("<FONT SIZE=-1>\n");

            for (ts, side_name) in [(&game.team_a, a_name.as_str()), (&game.team_b, b_name.as_str())] {
                let mut player_parts: Vec<String> = Vec::new();
                for ps in ts.player_scores.iter().flatten() {
                    let pname = escape_html(t.teams.get(ts.team_index)
                        .and_then(|tm| tm.players.get(ps.player_index))
                        .map_or("?", |p| p.name.as_str()));
                    player_parts.push(format!("{} {} {}", pname, q_score_inline(&ps.q, qe), ps.points));
                }
                body.push_str(&format!("{}: {}<br>\n", side_name, player_parts.join(", ")));
            }

            if t.track_bonuses {
                let mut bonus_parts: Vec<String> = Vec::new();
                for (ts, side_name) in [(&game.team_a, a_name.as_str()), (&game.team_b, b_name.as_str())] {
                    bonus_parts.push(format!("{} {} {} {:.2}", side_name, ts.bonus_heard, ts.bonus_points,
                        safe_div(f64::from(ts.bonus_points), f64::from(ts.bonus_heard))));
                }
                body.push_str(&format!("Bonuses: {}<br>\n", bonus_parts.join(", ")));

                if bb {
                    let mut bb_parts: Vec<String> = Vec::new();
                    for (ts, side_name) in [(&game.team_a, a_name.as_str()), (&game.team_b, b_name.as_str())] {
                        bb_parts.push(format!("{} {} {} {:.2}", side_name, ts.bb_heard, ts.bb_points,
                            safe_div(f64::from(ts.bb_points), f64::from(ts.bb_heard))));
                    }
                    body.push_str(&format!("Bonus Bouncebacks: {}<br>\n", bb_parts.join(", ")));
                }
            }
            body.push_str("<P></FONT>\n");
        }
    }

    html_page(&format!("{} Scoreboard ", t.name), nav, &body)
}

// ── Round Report ───────────────────────────────────────────────────────────

pub fn rounds_html(t: &Tournament, nav: &Nav) -> String {
    let mut round_nums: Vec<u32> = t.games.iter().map(|g| g.round).collect();
    round_nums.sort_unstable();
    round_nums.dedup();
    let bb = t.bouncebacks_enabled();

    let mut body = String::new();
    body.push_str(&format!("<H1>{} Round Report </H1><P>\n", t.name));
    body.push_str("<table border=1 width=100%>\n<tr>\n");
    body.push_str("  <td><B>Round</B></td>\n");
    body.push_str("  <td><B>PPG/Team</B></td>\n");
    body.push_str("  <td><B>TUPts/TUH</B></td>\n");
    if t.track_bonuses {
        body.push_str("  <td><B>BPts/BHrd</B></td>\n");
        if bb { body.push_str("  <td><B>BBPts/BBHrd</B></td>\n"); }
    }
    body.push_str("</tr>\n");

    for round in &round_nums {
        let games: Vec<&Game> = t.games.iter().filter(|g| g.round == *round && !g.forfeit).collect();
        if games.is_empty() { continue; }

        let n_games = games.len() as f64;
        let mut total_pts: i64 = 0;
        let mut total_tuh: i64 = 0;
        let mut total_player_pts: i64 = 0;
        let mut total_bh: i64 = 0;
        let mut total_bp: i64 = 0;
        let mut total_bbh: i64 = 0;
        let mut total_bbp: i64 = 0;

        for game in &games {
            total_pts += i64::from(game.team_a.total_points + game.team_b.total_points);
            total_tuh += i64::from(game.tossups_heard);
            for ts in [&game.team_a, &game.team_b] {
                for ps in ts.player_scores.iter().flatten() {
                    total_player_pts += i64::from(ps.points);
                }
                total_bh += i64::from(ts.bonus_heard);
                total_bp += i64::from(ts.bonus_points);
                total_bbh += i64::from(ts.bb_heard);
                total_bbp += i64::from(ts.bb_points);
            }
        }

        let ppg_team = safe_div(total_pts as f64, n_games * 2.0);
        let tu_pts_tuh = safe_div(total_player_pts as f64, total_tuh as f64);
        let bp_bh = safe_div(total_bp as f64, total_bh as f64);
        let bbp_bbh = safe_div(total_bbp as f64, total_bbh as f64);

        let pkt = packet_label(t, *round);
        body.push_str(&format!("  <td>{pkt}</td>\n"));
        body.push_str(&format!("  <td>{ppg_team:.2}</td>\n"));
        body.push_str(&format!("  <td>{tu_pts_tuh:.2}</td>\n"));
        if t.track_bonuses {
            body.push_str(&format!("  <td>{bp_bh:.2}</td>\n"));
            if bb { body.push_str(&format!("  <td>{bbp_bbh:.2}</td>\n")); }
        }
        body.push_str("</tr>\n");
    }
    body.push_str("</table>\n");

    html_page(&format!("{} Round Report ", t.name), nav, &body)
}

// ── Team Detail ────────────────────────────────────────────────────────────

pub fn team_detail_html(t: &Tournament, nav: &Nav) -> String {
    let bb = t.bouncebacks_enabled();
    let qv = &t.scoring.q_values;
    let qe = &t.scoring.q_enabled;
    let players = aggregate_players(t);
    let n = t.teams.len();

    let mut body = String::new();
    body.push_str(&format!("<H1>{} Team Details </H1><P>\n", t.name));

    for ti in 0..n {
        let team_name = escape_html(t.teams[ti].name.as_str());
        let team_name = team_name.as_str();
        body.push_str(&format!("<P><P><H2><A NAME=t{ti}>{team_name}</A></H2><P>\n"));

        // Game-by-game table
        body.push_str("<table border=1 width=100%>\n<tr>\n");
        body.push_str("<td ALIGN=LEFT><B>Opponent</B></td>\n");
        body.push_str("<td ALIGN=RIGHT><B>Result</B></td>\n");
        body.push_str("<td ALIGN=RIGHT><B>PF</B></td>\n");
        body.push_str("<td ALIGN=RIGHT><B>PA</B></td>\n");
        body.push_str(&q_headers_right(qv, qe));
        body.push_str("  <td ALIGN=RIGHT><B>TUH</B></td>\n");
        body.push_str("  <td ALIGN=RIGHT><B>PPTH</B></td>\n");
        body.push_str("  <td ALIGN=RIGHT><B>P/N</B></td>\n");
        body.push_str("  <td ALIGN=RIGHT><B>G/N</B></td>\n");
        if t.track_bonuses {
            body.push_str("  <td ALIGN=RIGHT><B>BHrd</B></td>\n");
            body.push_str("  <td ALIGN=RIGHT><B>BPts</B></td>\n");
            body.push_str("  <td ALIGN=RIGHT><B>P/B</B></td>\n");
            if bb {
                body.push_str("  <td ALIGN=RIGHT><B>BBHrd</B></td>\n");
                body.push_str("  <td ALIGN=RIGHT><B>BBPts</B></td>\n");
                body.push_str("  <td ALIGN=RIGHT><B>P/BB</B></td>\n");
            }
        }
        body.push_str("  <td ALIGN=LEFT><B>Packet</B></td>\n");
        body.push_str("</tr>\n");

        // Per-game rows + accumulate totals
        let mut tot_pf = 0i32; let mut tot_pa = 0i32;
        let mut tot_q = [0i32; 4]; let mut tot_tuh = 0i32;
        let mut tot_bh = 0i32; let mut tot_bp = 0i32;
        let mut tot_bbh = 0i32; let mut tot_bbp = 0i32;

        let team_games: Vec<(&Game, &TeamScore, &TeamScore)> = t.games.iter()
            .filter(|g| !g.forfeit && (g.team_a.team_index == ti || g.team_b.team_index == ti))
            .map(|g| {
                if g.team_a.team_index == ti { (g, &g.team_a, &g.team_b) }
                else { (g, &g.team_b, &g.team_a) }
            })
            .collect();

        for (game, my, opp) in &team_games {
            let opp_name = escape_html(t.teams.get(opp.team_index).map_or("?", |t| t.name.as_str()));
            let result = if my.total_points > opp.total_points { "W" }
                         else if my.total_points < opp.total_points { "L" } else { "T" };
            let tuh = game.tossups_heard as i32;
            let mut q = [0i32; 4];
            for ps in my.player_scores.iter().flatten() {
                for (a, b) in q.iter_mut().zip(ps.q.iter()) { *a += b; }
            }
            let ppth = safe_div(f64::from(my.total_points), f64::from(tuh));
            let pn = pn_str(&q, qv, qe);
            let gn = gn_str(&q, qv, qe);
            let pkt = packet_label(t, game.round);

            tot_pf += my.total_points; tot_pa += opp.total_points;
            for j in 0..4 { tot_q[j] += q[j]; }
            tot_tuh += tuh;
            tot_bh += my.bonus_heard; tot_bp += my.bonus_points;
            tot_bbh += my.bb_heard; tot_bbp += my.bb_points;

            body.push_str("<tr>\n");
            body.push_str(&format!("  <td ALIGN=LEFT>{opp_name}</td>\n  <td ALIGN=RIGHT>{result}</td>\n"));
            body.push_str(&format!("  <td ALIGN=RIGHT>{}</td>\n  <td ALIGN=RIGHT>{}</td>\n", my.total_points, opp.total_points));
            body.push_str(&q_cells_right(&q, qe));
            body.push_str(&format!("  <td ALIGN=RIGHT>{tuh}</td>\n"));
            body.push_str(&format!("  <td ALIGN=RIGHT>{ppth:.2}</td>\n"));
            body.push_str(&format!("  <td ALIGN=RIGHT>{pn}</td>\n"));
            body.push_str(&format!("  <td ALIGN=RIGHT>{gn}</td>\n"));
            if t.track_bonuses {
                body.push_str(&format!("  <td ALIGN=RIGHT>{}</td>\n", my.bonus_heard));
                body.push_str(&format!("  <td ALIGN=RIGHT>{}</td>\n", my.bonus_points));
                body.push_str(&format!("  <td ALIGN=RIGHT>{:.2}</td>\n", safe_div(f64::from(my.bonus_points), f64::from(my.bonus_heard))));
                if bb {
                    body.push_str(&format!("  <td ALIGN=RIGHT>{}</td>\n", my.bb_heard));
                    body.push_str(&format!("  <td ALIGN=RIGHT>{}</td>\n", my.bb_points));
                    body.push_str(&format!("  <td ALIGN=RIGHT>{:.2}</td>\n", safe_div(f64::from(my.bb_points), f64::from(my.bb_heard))));
                }
            }
            body.push_str(&format!("  <td ALIGN=LEFT>{pkt}</td>\n"));
            body.push_str("</tr>\n");
        }

        // Total row
        let tot_ppth = safe_div(f64::from(tot_pf), f64::from(tot_tuh));
        let tot_pn = pn_str(&tot_q, qv, qe);
        let tot_gn = gn_str(&tot_q, qv, qe);
        body.push_str("<tr>\n");
        body.push_str("  <td ALIGN=LEFT><B>Total</B></td>\n  <td></td>\n");
        body.push_str(&format!("  <td ALIGN=RIGHT><B>{tot_pf}</B></td>\n  <td ALIGN=RIGHT><B>{tot_pa}</B>\n"));
        body.push_str(&q_cells_bold(&tot_q, qe));
        body.push_str(&format!("  <td ALIGN=RIGHT><B>{tot_tuh}</B></td>\n"));
        body.push_str(&format!("  <td ALIGN=RIGHT><B>{tot_ppth:.2}</B></td>\n"));
        body.push_str(&format!("  <td ALIGN=RIGHT><B>{tot_pn}</B></td>\n"));
        body.push_str(&format!("  <td ALIGN=RIGHT><B>{tot_gn}</B></td>\n"));
        if t.track_bonuses {
            body.push_str(&format!("  <td ALIGN=RIGHT><B>{tot_bh}</B></td>\n"));
            body.push_str(&format!("  <td ALIGN=RIGHT><B>{tot_bp}</B></td>\n"));
            body.push_str(&format!("  <td ALIGN=RIGHT><B>{:.2}</B></td>\n", safe_div(f64::from(tot_bp), f64::from(tot_bh))));
            if bb {
                body.push_str(&format!("  <td ALIGN=RIGHT><B>{tot_bbh}</B></td>\n"));
                body.push_str(&format!("  <td ALIGN=RIGHT><B>{tot_bbp}</B></td>\n"));
                body.push_str(&format!("  <td ALIGN=RIGHT><B>{:.2}</B></td>\n", safe_div(f64::from(tot_bbp), f64::from(tot_bbh))));
            }
        }
        body.push_str("  <td ALIGN=LEFT> </td>\n");
        body.push_str("</tr>\n</table><P>\n");

        // Player table for this team
        body.push_str("<table border=1 width=100%>\n<tr>\n");
        body.push_str("  <td ALIGN=LEFT><B>Player</B></td>\n");
        body.push_str("  <td ALIGN=LEFT><B>Team</B></td>\n");
        body.push_str("  <td ALIGN=RIGHT><B>GP</B></td>\n");
        body.push_str(&q_headers_right(qv, qe));
        body.push_str("  <td ALIGN=RIGHT><B>TUH</B></td>\n");
        body.push_str("  <td ALIGN=RIGHT><B>P/TU</B></td>\n");
        body.push_str("  <td ALIGN=RIGHT><B>P/N</B></td>\n");
        body.push_str("  <td ALIGN=RIGHT><B>G/N</B></td>\n");
        body.push_str("  <td ALIGN=RIGHT><B>Pts</B></td>\n");
        body.push_str("  <td ALIGN=RIGHT><B>PPG</B></td>\n");
        body.push_str("</tr>\n");

        let team_players: Vec<&PlayerAgg> = players.iter().filter(|p| p.team_index == ti).collect();
        for p in &team_players {
            let pname = escape_html(t.teams[ti].players.get(p.player_index).map_or("?", |pl| pl.name.as_str()));
            let ptu = safe_div(f64::from(p.pts), f64::from(p.tuh));
            let pn = pn_str(&p.q, qv, qe);
            let gn = gn_str(&p.q, qv, qe);
            let ppg = safe_div(f64::from(p.pts), p.gp);
            let anchor = format!("{}#p{}_{}", nav.player_detail, p.player_index + 1, p.team_index);
            body.push_str("<tr>\n");
            body.push_str(&format!("  <td ALIGN=LEFT><A HREF={anchor}>{pname}</A></td>\n"));
            body.push_str(&format!("  <td ALIGN=LEFT>{}</td>\n", escape_html(team_name)));
            body.push_str(&format!("  <td ALIGN=RIGHT>{:.1}</td>\n", p.gp));
            body.push_str(&q_cells_right(&p.q, qe));
            body.push_str(&format!("  <td ALIGN=RIGHT>{}</td>\n", p.tuh));
            body.push_str(&format!("  <td ALIGN=RIGHT>{ptu:.2}</td>\n"));
            body.push_str(&format!("  <td ALIGN=RIGHT>{pn}</td>\n"));
            body.push_str(&format!("  <td ALIGN=RIGHT>{gn}</td>\n"));
            body.push_str(&format!("  <td ALIGN=RIGHT>{}</td>\n", p.pts));
            body.push_str(&format!("  <td ALIGN=RIGHT>{ppg:.2}</td>\n"));
            body.push_str("</tr>\n");
        }
        body.push_str("</table>\n");
    }

    html_page(&format!("{} Team Details ", t.name), nav, &body)
}

// ── Player Detail ──────────────────────────────────────────────────────────

pub fn player_detail_html(t: &Tournament, nav: &Nav) -> String {
    let players = aggregate_players(t);
    let qv = &t.scoring.q_values;
    let qe = &t.scoring.q_enabled;

    let mut body = String::new();
    body.push_str(&format!("<H1>{} Individual Detail </H1><P>\n", t.name));

    for p in &players {
        let team_name = escape_html(t.teams.get(p.team_index).map_or("?", |t| t.name.as_str()));
        let pname = escape_html(t.teams.get(p.team_index)
            .and_then(|tm| tm.players.get(p.player_index))
            .map_or("?", |pl| pl.name.as_str()));

        body.push_str(&format!("<P><P><H2><A NAME=p{}_{}>{}</A>, {}</H2><P>\n",
            p.player_index + 1, p.team_index, pname, team_name));
        body.push_str("<table border=1 width=100%>\n<tr>\n");
        body.push_str("<td ALIGN=LEFT><B>Opponent</B></td>\n");
        body.push_str("  <td ALIGN=LEFT><B>Packet</B></td>\n");
        body.push_str("  <td ALIGN=RIGHT><B>GP</B></td>\n");
        body.push_str(&q_headers_right(qv, qe));
        body.push_str("  <td ALIGN=RIGHT><B>TUH</B></td>\n");
        body.push_str("  <td ALIGN=RIGHT><B>P/TU</B></td>\n");
        body.push_str("  <td ALIGN=RIGHT><B>P/N</B></td>\n");
        body.push_str("  <td ALIGN=RIGHT><B>G/N</B></td>\n");
        body.push_str("  <td ALIGN=RIGHT><B>Pts</B></td>\n");
        body.push_str("</tr>\n");

        for ge in &p.games {
            let opp_name = t.teams.get(ge.opp_team_index).map_or("?", |t| t.name.as_str());
            let pkt = packet_label(t, ge.round);
            let ptu = safe_div(f64::from(ge.pts), f64::from(ge.tuh));
            let pn = pn_str(&ge.q, qv, qe);
            let gn = gn_str(&ge.q, qv, qe);
            body.push_str(&format!("  <td ALIGN=LEFT>{opp_name}</td>\n"));
            body.push_str(&format!("  <td ALIGN=LEFT>{pkt}</td>\n"));
            body.push_str(&format!("  <td ALIGN=RIGHT>{:.2}</td>\n", ge.gp));
            body.push_str(&q_cells_right(&ge.q, qe));
            body.push_str(&format!("  <td ALIGN=RIGHT>{}</td>\n", ge.tuh));
            body.push_str(&format!("  <td ALIGN=RIGHT>{ptu:.2}</td>\n"));
            body.push_str(&format!("  <td ALIGN=RIGHT>{pn}</td>\n"));
            body.push_str(&format!("  <td ALIGN=RIGHT>{gn}</td>\n"));
            body.push_str(&format!("  <td ALIGN=RIGHT>{}</td>\n", ge.pts));
            body.push_str("</tr>\n");
        }

        // Totals row
        let ptu = safe_div(f64::from(p.pts), f64::from(p.tuh));
        let pn = pn_str(&p.q, qv, qe);
        let gn = gn_str(&p.q, qv, qe);
        body.push_str("<tr>\n");
        body.push_str("  <td ALIGN=LEFT><B>Total</B></td>\n");
        body.push_str("  <td ALIGN=LEFT><B> </B></td>\n");
        body.push_str(&format!("  <td ALIGN=RIGHT><B>{:.2}</B></td>\n", p.gp));
        body.push_str(&q_cells_bold(&p.q, qe));
        body.push_str(&format!("  <td ALIGN=RIGHT><B>{}</B></td>\n", p.tuh));
        body.push_str(&format!("  <td ALIGN=RIGHT><B>{ptu:.2}</B></td>\n"));
        body.push_str(&format!("  <td ALIGN=RIGHT><B>{pn}</B></td>\n"));
        body.push_str(&format!("  <td ALIGN=RIGHT><B>{gn}</B></td>\n"));
        body.push_str(&format!("  <td ALIGN=RIGHT><B>{}</B></td>\n", p.pts));
        body.push_str("</tr>\n");
        body.push_str("</table>\n");
    }

    html_page(&format!("{} Individual Detail ", t.name), nav, &body)
}

// ── Stat Key ───────────────────────────────────────────────────────────────

pub fn stat_key_html(t: &Tournament, nav: &Nav) -> String {
    let ql = q_label(&t.scoring.q_values, &t.scoring.q_enabled);
    let bb = t.bouncebacks_enabled();

    let mut body = String::new();
    body.push_str(&format!("<H1>{} Stat Key </H1><P>\n", t.name));

    // Team Standings section
    body.push_str("<H2><A NAME=TeamStandings>Team Standings</A></H2><br>\n");
    body.push_str("<table border=1 width=100%>\n");
    for (abbr, desc) in &[
        ("W, L, T", "Number of games won (W), lost (L), or tied (T)"),
        ("Pct", "Fraction of games won"),
        ("PPG", "Average number of points scored by the team in a game"),
        ("PAPG", "Average number of points scored against the team in a game"),
        ("Mrg", "Team's average margin of victory (if positive) or defeat (if negative)"),
    ] {
        body.push_str(&format!("<tr>\n<td>{abbr}</td>\n<td>{desc}</td>\n</tr>\n"));
    }
    body.push_str(&format!("<tr>\n<td>{ql} </td>\n<td>Number of toss-ups answered for the corresponding point value (negative point values are for incorrect interrupts)</td>\n</tr>\n"));
    for (abbr, desc) in &[
        ("TUH", "Total number of toss-ups heard by the team"),
        ("PPTH", "Average number of points scored by the team per toss-up heard"),
        ("P/N", "Ratio of powers to negs"),
        ("G/N", "Ratio of gets (correctly answered questions) to negs"),
    ] {
        body.push_str(&format!("<tr>\n<td>{abbr}</td>\n<td>{desc}</td>\n</tr>\n"));
    }
    if t.track_bonuses {
        for (abbr, desc) in &[
            ("BHrd", "Total number of bonus questions heard by the team"),
            ("BPts", "Total number of points scored by the team on bonus questions"),
            ("P/B", "Average number of points scored by the team on bonus questions"),
        ] {
            body.push_str(&format!("<tr>\n<td>{abbr}</td>\n<td>{desc}</td>\n</tr>\n"));
        }
        if bb {
            for (abbr, desc) in &[
                ("BBHrd", "Total number of bonus bouncebacks heard by the team"),
                ("BBPts", "Total number of points scored by the team on bonus bouncebacks"),
                ("P/BB", "Average number of points scored by the team on bonus bouncebacks"),
            ] {
                body.push_str(&format!("<tr>\n<td>{abbr}</td>\n<td>{desc}</td>\n</tr>\n"));
            }
        }
    }
    body.push_str("</table><P>\n");

    // Individual Statistics section
    body.push_str("<H2><A NAME=IndividualStandings>Individual Statistics</A></H2><br>\n");
    body.push_str("<table border=1 width=100%>\n");
    body.push_str("<tr>\n<td>GP</td>\n<td>Number of games in which the player participated</td>\n</tr>\n");
    body.push_str(&format!("<tr>\n<td>{ql} </td>\n<td>Number of toss-ups answered for the corresponding point value (negative point values are for incorrect interrupts)</td>\n</tr>\n"));
    for (abbr, desc) in &[
        ("TUH", "Total number of toss-ups heard by the player"),
        ("P/TU", "Average number of points scored by the player per toss-up heard"),
        ("P/N", "Ratio of powers to negs"),
        ("G/N", "Ratio of gets (correctly answered questions) to negs"),
        ("Pts", "Total number of points scored by the player"),
        ("PPG", "Average number of points scored by the player per game"),
    ] {
        body.push_str(&format!("<tr>\n<td>{abbr}</td>\n<td>{desc}</td>\n</tr>\n"));
    }
    body.push_str("</table><P>\n");

    // Scoreboard section
    body.push_str("<H2><A NAME=Scoreboard>Scoreboard</A></H2><br>\n");
    body.push_str("For each game, the score is listed in large bold print on the first line.  Then each individual who played in the game is listed, by team, along with the number of each type of question answered (in the order that they appear in the other reports (usually decreasing order, such as 15 10 -5).  The last number after each name is the individual's total points for the game.\n");
    if t.track_bonuses {
        body.push_str("The next line of the boxscore gives, for each team, the total number of bonuses heard, the total number of points scored on bonuses, and the average number of points scored per bonus heard.");
        if bb {
            body.push_str("The next line of the boxscore gives, for each team, the total number of bonus bouncebacks heard, the total number of points scored on bonus bouncebacks, and the average number of points scored per bonus bounceback heard.");
        }
    }
    body.push_str("<P><P>\n");

    // Team Detail section
    body.push_str("<H2><A NAME=TeamDetail>Team Detail</A></H2><br>\n");
    body.push_str("<table border=1 width=100%>\n");
    for (abbr, desc) in &[
        ("Result", "Whether the team won (W), lost (L), or tied (T) the game"),
        ("PF", "Total number of points scored by the team"),
        ("PA", "Total number of points scored by the opponent"),
    ] {
        let _ = write!(body, "<tr>\n<td>{abbr}</td>\n<td>{desc}</td>\n</tr>\n");
    }
    let _ = write!(body, "<tr>\n<td>{ql} </td>\n<td>Number of toss-ups answered for the corresponding point value (negative point values are for incorrect interrupts)</td>\n</tr>\n");
    for (abbr, desc) in &[
        ("TUH", "Total number of toss-ups heard"),
        ("PPTH", "Average number of points scored by the team per toss-up heard"),
        ("P/N", "Ratio of powers to negs"),
        ("G/N", "Ratio of gets (correctly answered questions) to negs"),
    ] {
        let _ = write!(body, "<tr>\n<td>{abbr}</td>\n<td>{desc}</td>\n</tr>\n");
    }
    if t.track_bonuses {
        for (abbr, desc) in &[
            ("BHrd", "Total number of bonus questions heard by the team"),
            ("BPts", "Total number of points scored by the team on bonus questions"),
            ("P/B", "Average number of points scored by the team on bonus questions"),
        ] {
            let _ = write!(body, "<tr>\n<td>{abbr}</td>\n<td>{desc}</td>\n</tr>\n");
        }
        if bb {
            for (abbr, desc) in &[
                ("BBHrd", "Total number of bonus bouncebacks heard by the team"),
                ("BBPts", "Total number of points scored by the team on bonus bouncebacks"),
                ("P/BB", "Average number of points scored by the team on bonus bouncebacks"),
            ] {
                let _ = write!(body, "<tr>\n<td>{abbr}</td>\n<td>{desc}</td>\n</tr>\n");
            }
        }
    }
    body.push_str("</table><P>\n");

    // Individual Detail section
    body.push_str("<H2><A NAME=IndividualDetail>Individual Detail</A></H2><br>\n");
    body.push_str("<table border=1 width=100%>\n");
    body.push_str("<tr>\n<td>GP</td>\n<td>Number of games in which the player participated</td>\n</tr>\n");
    let _ = write!(body, "<tr>\n<td>{ql} </td>\n<td>Number of toss-ups answered for the corresponding point value (negative point values are for incorrect interrupts)</td>\n</tr>\n");
    for (abbr, desc) in &[
        ("TUH", "Total number of toss-ups heard by the player"),
        ("P/TU", "Average number of points scored by the player per toss-up heard"),
        ("P/N", "Ratio of powers to negs"),
        ("G/N", "Ratio of gets (correctly answered questions) to negs"),
        ("Pts", "Total number of points scored by the player"),
        ("PPG", "Average number of points scored by the player per game"),
    ] {
        let _ = write!(body, "<tr>\n<td>{abbr}</td>\n<td>{desc}</td>\n</tr>\n");
    }
    body.push_str("</table><P>\n");

    // Round Report section
    body.push_str("<H2><A NAME=RoundReport>Round Report</A></H2><br>\n");
    body.push_str("<table border=1 width=100%>\n");
    body.push_str("<tr>\n<td>PPG/Team</td>\n<td>Average number of points scored per team per game</td>\n</tr>\n");
    body.push_str("<tr>\n<td>TUPts/TUH.</td>\n<td>Average number of points scored on toss-up questions per toss-up heard</td>\n</tr>\n");
    if t.track_bonuses {
        body.push_str("<tr>\n<td>BPts/BH</td>\n<td>Average number of points scored on bonus questions per bonus heard</td>\n</tr>\n");
        if bb {
            body.push_str("<tr>\n<td>BBPts/BBH</td>\n<td>Average number of points scored on bonus bouncebacks per bonus bounceback heard</td>\n</tr>\n");
        }
    }
    body.push_str("</table><P>\n");
    body.push_str("For information about this statistics program, visit the <A HREF=http://www.stanford.edu/~csewell/sqbs/index.htm>SQBS homepage</A>.");

    html_page(&format!("{} Stat Key ", t.name), nav, &body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Game, Player, PlayerScore, Team, TeamScore, Tournament};

    // ── Helpers ────────────────────────────────────────────────────────────

    fn make_tournament() -> Tournament {
        // Alpha 125, Beta 45 — Round 1, TUH 20
        // q_values = [20, 15, 10, -5]
        // Alice (Alpha): q=[1,0,2,0] → pts = 20+20 = 40
        // Bob   (Alpha): q=[0,0,3,1] → pts = 30-5  = 25   TU=65 + bonus=60 = 125
        // Carol (Beta):  q=[0,0,2,0] → pts = 20
        // Dan   (Beta):  q=[0,0,1,1] → pts = 10-5  =  5   TU=25 + bonus=20 = 45
        let mut t = Tournament::default();
        t.name = "Test Tournament".to_string();
        t.scoring.q_values = [20, 15, 10, -5];
        t.track_bonuses = true;
        t.reports.base_name = "test".to_string();

        t.teams = vec![
            Team {
                name: "Alpha".to_string(),
                players: vec![
                    Player { name: "Alice".to_string() },
                    Player { name: "Bob".to_string() },
                ],
                division: None,
                exhibition: false,
            },
            Team {
                name: "Beta".to_string(),
                players: vec![
                    Player { name: "Carol".to_string() },
                    Player { name: "Dan".to_string() },
                ],
                division: None,
                exhibition: false,
            },
        ];

        let mut ps_a: Vec<Option<PlayerScore>> = vec![
            Some(PlayerScore { player_index: 0, gp: 1.0, q: [1, 0, 2, 0], points: 40 }),
            Some(PlayerScore { player_index: 1, gp: 1.0, q: [0, 0, 3, 1], points: 25 }),
        ];
        let mut ps_b: Vec<Option<PlayerScore>> = vec![
            Some(PlayerScore { player_index: 0, gp: 1.0, q: [0, 0, 2, 0], points: 20 }),
            Some(PlayerScore { player_index: 1, gp: 1.0, q: [0, 0, 1, 1], points:  5 }),
        ];
        while ps_a.len() < 8 { ps_a.push(None); }
        while ps_b.len() < 8 { ps_b.push(None); }

        t.games.push(Game {
            game_index: "1".to_string(),
            round: 1,
            tossups_heard: 20,
            overtime: false,
            forfeit: false,
            team_a: TeamScore {
                team_index: 0, total_points: 125,
                bonus_heard: 8, bonus_points: 60,
                bb_heard: 0, bb_points: 0,
                ot_gets: 0, lightning_points: 0,
                player_scores: ps_a,
            },
            team_b: TeamScore {
                team_index: 1, total_points: 45,
                bonus_heard: 3, bonus_points: 20,
                bb_heard: 0, bb_points: 0,
                ot_gets: 0, lightning_points: 0,
                player_scores: ps_b,
            },
        });
        t
    }

    fn make_nav() -> Nav {
        Nav::new("test", &ReportSettings::default())
    }

    // ── safe_div ───────────────────────────────────────────────────────────

    #[test]
    fn safe_div_normal() {
        assert!((safe_div(10.0, 4.0) - 2.5).abs() < 1e-9);
    }

    #[test]
    fn safe_div_zero_denominator() {
        assert_eq!(safe_div(42.0, 0.0), 0.0);
    }

    #[test]
    fn safe_div_both_zero() {
        assert_eq!(safe_div(0.0, 0.0), 0.0);
    }

    // ── aggregate_teams ────────────────────────────────────────────────────

    #[test]
    fn aggregate_wins_and_losses() {
        let agg = aggregate_teams(&make_tournament());
        assert_eq!(agg[0].wins, 1);
        assert_eq!(agg[0].losses, 0);
        assert_eq!(agg[1].wins, 0);
        assert_eq!(agg[1].losses, 1);
    }

    #[test]
    fn aggregate_points_for_and_against() {
        let agg = aggregate_teams(&make_tournament());
        assert_eq!(agg[0].pf, 125);
        assert_eq!(agg[0].pa, 45);
        assert_eq!(agg[1].pf, 45);
        assert_eq!(agg[1].pa, 125);
    }

    #[test]
    fn aggregate_q_values_summed() {
        let agg = aggregate_teams(&make_tournament());
        // Alpha: Alice q=[1,0,2,0] + Bob q=[0,0,3,1] = [1,0,5,1]
        assert_eq!(agg[0].q, [1, 0, 5, 1]);
        // Beta: Carol q=[0,0,2,0] + Dan q=[0,0,1,1] = [0,0,3,1]
        assert_eq!(agg[1].q, [0, 0, 3, 1]);
    }

    #[test]
    fn forfeit_excluded_from_aggregation() {
        let mut t = make_tournament();
        t.games[0].forfeit = true;
        let agg = aggregate_teams(&t);
        assert!(agg.iter().all(|a| a.games == 0 && a.wins == 0));
    }

    // ── standings_html ─────────────────────────────────────────────────────

    #[test]
    fn standings_contains_team_names() {
        let html = standings_html(&make_tournament(), &make_nav());
        assert!(html.contains("Alpha"));
        assert!(html.contains("Beta"));
    }

    #[test]
    fn standings_winner_appears_first() {
        let html = standings_html(&make_tournament(), &make_nav());
        let alpha_pos = html.find("Alpha").unwrap();
        let beta_pos  = html.find("Beta").unwrap();
        assert!(alpha_pos < beta_pos, "Alpha (1-0) should rank above Beta (0-1)");
    }

    #[test]
    fn standings_valid_html_structure() {
        let html = standings_html(&make_tournament(), &make_nav());
        assert!(html.starts_with("<HTML>"));
        assert!(html.contains("Team Standings"));
        assert!(html.ends_with("</HTML>\n"));
    }

    // ── individuals_html ───────────────────────────────────────────────────

    #[test]
    fn individuals_contains_all_player_names() {
        let html = individuals_html(&make_tournament(), &make_nav());
        assert!(html.contains("Alice"));
        assert!(html.contains("Bob"));
        assert!(html.contains("Carol"));
        assert!(html.contains("Dan"));
    }

    #[test]
    fn individuals_alice_ranks_first() {
        let html = individuals_html(&make_tournament(), &make_nav());
        // Alice 40 pts, Bob 25 pts — Alice should appear before Bob
        let alice_pos = html.find("Alice").unwrap();
        let bob_pos   = html.find("Bob").unwrap();
        assert!(alice_pos < bob_pos);
    }

    // ── team_detail_html ───────────────────────────────────────────────────

    #[test]
    fn team_detail_gn_formula() {
        // Alpha total: q0=1, q1=0, q2=5, q3=1
        // G/N = (q0+q2)/q3 = (1+5)/1 = 6.00 (both per-game and total)
        // P/N = q0/q3 = 1/1 = 1.00
        let html = team_detail_html(&make_tournament(), &make_nav());
        assert!(html.contains("6.00"), "G/N should be (q0+q2)/q3 = 6.00");
        assert!(html.contains("1.00"), "P/N should be q0/q3 = 1.00");
    }

    #[test]
    fn team_detail_has_team_anchors() {
        let html = team_detail_html(&make_tournament(), &make_nav());
        assert!(html.contains("NAME=t0"), "expected anchor t0 for Alpha");
        assert!(html.contains("NAME=t1"), "expected anchor t1 for Beta");
    }

    // ── player_detail_html ─────────────────────────────────────────────────

    #[test]
    fn player_anchor_format() {
        // anchor = NAME=p{player_index+1}_{team_index}
        // Alice: player_index=0, team_index=0 → p1_0
        // Carol: player_index=0, team_index=1 → p1_1
        let html = player_detail_html(&make_tournament(), &make_nav());
        assert!(html.contains("NAME=p1_0"), "expected anchor p1_0 for Alice");
        assert!(html.contains("NAME=p2_0"), "expected anchor p2_0 for Bob");
        assert!(html.contains("NAME=p1_1"), "expected anchor p1_1 for Carol");
    }

    // ── generate_all_reports ───────────────────────────────────────────────

    #[test]
    fn generate_all_reports_writes_seven_files() {
        let t = make_tournament();
        let dir = std::env::temp_dir().to_string_lossy().to_string();
        let written = generate_all_reports(&t, &dir).expect("generate failed");
        assert_eq!(written.len(), 7);
        for path in &written {
            assert!(
                std::path::Path::new(path).exists(),
                "missing report file: {}",
                path
            );
            std::fs::remove_file(path).ok();
        }
    }
}

// ── Path traversal guard ───────────────────────────────────────────────────

fn validate_report_name(name: &str) -> io::Result<()> {
    let p = Path::new(name);
    if p.is_absolute() || p.components().count() != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("report filename '{name}' must be a plain filename with no path separators"),
        ));
    }
    Ok(())
}

// ── Main entry point ───────────────────────────────────────────────────────

pub fn generate_all_reports(t: &Tournament, dir: &str) -> io::Result<Vec<String>> {
    let base = &t.reports.base_name;
    let nav = Nav::new(base, &t.reports);

    // Validate all filenames before generating any content
    for name in &[
        &nav.standings, &nav.individuals, &nav.games,
        &nav.rounds, &nav.team_detail, &nav.player_detail, &nav.stat_key,
    ] {
        validate_report_name(name)?;
    }

    let files: Vec<(String, String)> = vec![
        (Path::new(dir).join(&nav.standings).to_string_lossy().to_string(),  standings_html(t, &nav)),
        (Path::new(dir).join(&nav.individuals).to_string_lossy().to_string(), individuals_html(t, &nav)),
        (Path::new(dir).join(&nav.games).to_string_lossy().to_string(), games_html(t, &nav)),
        (Path::new(dir).join(&nav.rounds).to_string_lossy().to_string(), rounds_html(t, &nav)),
        (Path::new(dir).join(&nav.team_detail).to_string_lossy().to_string(), team_detail_html(t, &nav)),
        (Path::new(dir).join(&nav.player_detail).to_string_lossy().to_string(), player_detail_html(t, &nav)),
        (Path::new(dir).join(&nav.stat_key).to_string_lossy().to_string(), stat_key_html(t, &nav)),
    ];

    let mut written = Vec::new();
    for (path, content) in files {
        let tmp_path = format!("{path}.tmp");
        fs::write(&tmp_path, &content)?;
        fs::rename(&tmp_path, &path)?;
        written.push(path);
    }
    Ok(written)
}
