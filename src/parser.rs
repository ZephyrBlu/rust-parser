use serde::Serialize;

use crate::{Player, TinybirdGame, TinybirdTimelineEntry};
use crate::replay::{Metadata, Replay};
use crate::game::Game;
use crate::events::EventParser;
use crate::decoders::DecoderResult;

use std::collections::HashMap;

pub type RaceMappings<'a> = HashMap<&'a str, &'a str>;

pub struct ReplayParser<'a> {
  race_mappings: RaceMappings<'a>,
}

#[derive(Clone, Serialize)]
pub struct ReplaySummary {
  pub players: Vec<Player>,
  pub builds: [Vec<String>; 2],
  pub build_mappings: [u16; 2],
  // pub units: [Vec<String>; 2],
  // pub unit_mappings: [u16; 2],
  pub winner: u8,
  pub game_length: u16,
  pub map: String,
  pub played_at: u64,
  pub tags: String,
  pub tinybird: TinybirdGame,
  pub timeline: Vec<TinybirdTimelineEntry>,
}

pub struct TimelineContext {
  pub content_hash: String,
  pub players: Vec<Player>,
  pub workers_lost: [u16; 2],
  pub workers_killed: [u16; 2],
  pub winner_id: u8,
  pub map: String,
  pub event: String,
  pub matchup: String,
  pub game_length: u16,
  pub played_at: u64,
  pub game_version: String,
}

impl<'a> ReplayParser<'a> {
  pub fn new() -> ReplayParser<'a> {
    let race_mappings: RaceMappings = HashMap::from([
      ("저그", "Zerg"),
      ("异虫", "Zerg"),
      ("蟲族", "Zerg"),
      ("Zergi", "Zerg"),
      ("테란", "Terran"),
      ("人類", "Terran"),
      ("人类", "Terran"),
      ("Terraner", "Terran"),
      ("Терраны", "Terran"),
      ("프로토스", "Protoss"),
      ("神族", "Protoss"),
      ("Protosi", "Protoss"),
      ("星灵", "Protoss"),
      ("Протоссы", "Protoss"),
    ]); 

    ReplayParser {
      race_mappings,
    }
  }

  pub fn parse_replay(
    &'a self,
    raw_replay: Replay,
    builds: &mut Vec<String>,
    // units: &mut Vec<String>,
  ) -> Result<ReplaySummary, &'static str> {
    let replay = raw_replay.parsed;
    let tags = replay.tags.clone();

    let parsed_metadata: Metadata = serde_json::from_str(&replay.metadata).unwrap();

    let winner = match parsed_metadata.Players
      .iter()
      .find(|player| player.Result == "Win") {
        Some(player) => player.PlayerID,
        None => return Err("couldn't find winner"),
      };
    let game_length = parsed_metadata.Duration;

    let raw_map = &replay.player_info
      .iter()
      .find(|(field, _)| *field == "m_title")
      .unwrap().1;
    let mut map = "";
    if let DecoderResult::Blob(value) = raw_map {
      map = value
        .trim_start_matches("[M] ")
        .trim_start_matches("[SO] ")
        .trim_start_matches("[ESL] ")
        .trim_start_matches("[GSL] ")
        .trim_start_matches("[TLMC14] ")
        .trim_start_matches("[TLMC15] ")
        .trim_start_matches("[TLMC16] ")
        .trim_start_matches("[TLMC17] ")
        .trim_end_matches(" LE");
    }

    let raw_played_at = &replay.player_info
      .iter()
      .find(|(field, _)| *field == "m_timeUTC")
      .unwrap().1;
    let mut played_at = 0;
    if let DecoderResult::Value(value) = raw_played_at {
      // TODO: this truncation is not working properly
      played_at = value.clone() as u64;
    }
    // game records time in window epoch for some reason
    // https://en.wikipedia.org/wiki/Epoch_(computing)
    played_at = (played_at / 10000000) - 11644473600;

    let (_, player_list) = &replay.player_info
      .iter()
      .find(|(field, _)| *field == "m_playerList")
      .unwrap();

    let mut players = vec![];
    match player_list {
      DecoderResult::Array(values) => {
        // TODO: enumerated id is incorrect for P1 and P2 in games

        // don't support 1 player or 3+ player games
        if values.len() != 2 {
          return Err("Not 2 players in replay");
        }

        for (id, player) in values.iter().enumerate() {
          match player {
            DecoderResult::Struct(player_values) => {
              let raw_race = &player_values
                .iter()
                .find(|(field, _)| *field == "m_race")
                .unwrap().1;
              let mut race = String::new();
              if let DecoderResult::Blob(value) = raw_race {
                race = value.clone();
              }

              if let Some(value) = self.race_mappings.get(race.as_str()) {
                race = value.to_string();
              }

              let raw_name = &player_values
                .iter()
                .find(|(field, _)| *field == "m_name")
                .unwrap().1;
              let mut name = String::new();
              if let DecoderResult::Blob(value) = raw_name {
                name = match value.find(">") {
                  Some(clan_tag_index) => value[clan_tag_index + 1..].to_string(),
                  None => value.clone(),
                };
              }

              players.push(Player {
                id: (id + 1) as u8,
                race,
                name,
              });
            },
            _other => panic!("Found DecoderResult::{:?}", _other)
          }
        }
      },
      _other => panic!("Found DecoderResult::{:?}", _other)
    }

    let mut serialized_players = vec![];
    let mut serialized_matchup = vec![];
    for player in &players {
      serialized_players.push(player.name.clone());
      serialized_matchup.push(player.race.clone());
    }
    serialized_players.sort();
    serialized_matchup.sort();

    let loser: u8 = if winner == 1 {
      2
    } else {
      1
    };

    players.sort_by(|a, b| a.id.cmp(&b.id));

    let mut game = Game::new();
    let mut timeline: Vec<TinybirdTimelineEntry> = vec![];
    let context = TimelineContext {
      content_hash: raw_replay.content_hash.clone(),
      players: players.clone(),
      workers_lost: [0, 0],
      workers_killed: [0, 0],
      winner_id: winner,
      matchup: serialized_matchup.join(""),
      map: map.to_owned(),
      event: tags.clone(),
      game_length,
      played_at,
      game_version: parsed_metadata.GameVersion.to_string(),
    };
    let mut event_parser = EventParser::new(
      context,
      &mut game,
      &mut timeline,
    );

    for event in &replay.tracker_events {
      if let Err(e) = event_parser.parse(event) {
        println!("event parsing failed: {:?}\n", e);
        continue;
      }
    }

    let mut replay_build_mappings: [u16; 2] = [0, 0];
    let mut replay_builds: [Vec<String>; 2] = [vec![], vec![]];
    for (replay_build_index, build) in game.builds.iter_mut().enumerate() {
      if build.len() == 0 {
        return Err("build is length 0");
      }

      build.sort_by(|a, b| a.1.cmp(&b.1));
      replay_builds[replay_build_index] = build
        .iter()
        .map(|(building, _)| building.to_owned())
        .collect::<Vec<String>>();

      let joined_build = replay_builds[replay_build_index].join(",");
      match builds.iter().position(|seen_build| &joined_build == seen_build) {
        Some(build_index) => replay_build_mappings[replay_build_index] = build_index as u16,
        None => {
          builds.push(joined_build);
          replay_build_mappings[replay_build_index] = builds.len() as u16 - 1;
        }
      }
    }

    // let mut replay_units_mappings: [u16; 2] = [0, 0];
    // let mut replay_units: [Vec<String>; 2] = [vec![], vec![]];
    // for (replay_unit_index, unit) in game.units.iter_mut().enumerate() {
    //   unit.sort_by(|a, b| a.1.cmp(&b.1));
    //   replay_units[replay_unit_index] = unit
    //     .iter()
    //     .map(|(unit, _)| unit.to_owned())
    //     .collect::<Vec<String>>();

    //   let joined_units = replay_units[replay_unit_index].join(",");
    //   match units.iter().position(|seen_units| &joined_units == seen_units) {
    //     Some(unit_index) => replay_units_mappings[replay_unit_index] = unit_index as u16,
    //     None => {
    //       units.push(joined_units);
    //       replay_units_mappings[replay_unit_index] = units.len() as u16 - 1;
    //     }
    //   }
    // }

    const GAS_BUILDINGS: [&str; 3] = [
      "Assimilator",
      "Refinery",
      "Extractor",
    ];

    let winner_build = replay_builds[(winner - 1) as usize]
      .iter()
      .filter(|building| !GAS_BUILDINGS.contains(&building.as_str()))
      .map(|building| building.to_string())
      .collect::<Vec<String>>()
      .join(",");
    let loser_build = replay_builds[(loser - 1) as usize]
      .iter()
      .filter(|building| !GAS_BUILDINGS.contains(&building.as_str()))
      .map(|building| building.to_string())
      .collect::<Vec<String>>()
      .join(",");

    let tinybird_game = TinybirdGame {
      content_hash: raw_replay.content_hash.clone(),
      winner_id: winner,
      winner_name: players[(winner - 1) as usize].name.clone(),
      winner_race: players[(winner - 1) as usize].race.clone(),
      winner_build: winner_build.clone(),
      loser_id: loser,
      loser_name: players[(loser - 1) as usize].name.clone(),
      loser_race: players[(loser - 1) as usize].race.clone(),
      loser_build: loser_build.clone(),
      matchup: serialized_matchup.join(""),
      player_names: serialized_players.join(""),
      players: serde_json::to_string(&players).unwrap(),
      builds: serde_json::to_string(&replay_builds).unwrap(),
      map: map.to_owned(),
      game_length,
      played_at,
      event: replay.tags.clone(),
      game_version: parsed_metadata.GameVersion.to_string(),
    };

    let replay_summary: ReplaySummary = ReplaySummary {
      players,
      builds: replay_builds,
      build_mappings: replay_build_mappings,
      // units: replay_units,
      // unit_mappings: replay_units_mappings,
      winner,
      game_length,
      map: map.to_owned(),
      played_at,
      tags: tags.clone(),
      tinybird: tinybird_game,
      timeline,
    };

    Ok(replay_summary)
  }
}
