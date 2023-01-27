use crate::{Player, ReplaySummary, ReplayEntry, SummaryStat, TinybirdGame};
use crate::replay::{Parsed, Metadata, Replay};
use crate::game::Game;
use crate::events::EventParser;
use crate::decoders::DecoderResult;

use std::collections::HashMap;

pub type RaceMappings<'a> = HashMap<&'a str, &'a str>;

pub struct ReplayParser<'a> {
  race_mappings: RaceMappings<'a>,
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

  pub fn parse_replay(&'a self, raw_replay: Replay, builds: &mut Vec<String>, units: &mut Vec<String>) -> Result<ReplaySummary, &'static str> {
    let replay = raw_replay.parsed;
    let tags = replay.tags.clone();

    let mut game = Game::new();
    let mut event_parser = EventParser::new(&replay, &mut game);

    for event in &replay.tracker_events {
      if let Err(e) = event_parser.parse(event) {
        println!("event parsing failed: {:?}\n", e);
        continue;
      }
    }

    let resources_collected: [(u16, u16); 2] = [
      (game.minerals_collected[0], game.gas_collected[0]),
      (game.minerals_collected[1], game.gas_collected[1]),
    ];
    let resources_lost: [(u16, u16); 2] = [
      (game.minerals_lost[1], game.gas_lost[1]),
      (game.minerals_lost[0], game.gas_lost[0]),
    ];

    let mut avg_collection_rate: [(u16, u16); 2] = [(0, 0), (0, 0)];
    for (index, player_collection_rate) in game.collection_rate.iter().enumerate() {
      let mut player_total_collection_rate: [u64; 2] = [0, 0];
      for (minerals, gas) in player_collection_rate {
        player_total_collection_rate[0] += *minerals as u64;
        player_total_collection_rate[1] += *gas as u64;
      }
      let num_collection_rate = player_collection_rate.len() as u64;
      avg_collection_rate[index] = (
        if num_collection_rate == 0 { 0 } else { (player_total_collection_rate[0] / num_collection_rate) as u16 },
        if num_collection_rate == 0 { 0 } else { (player_total_collection_rate[1] / num_collection_rate) as u16 },
      );
    }

    let mut avg_unspent_resources: [(u16, u16); 2] = [(0, 0), (0, 0)];
    for (index, player_unspent_resources) in game.unspent_resources.iter().enumerate() {
      let mut player_total_unspent_resources: [u64; 2] = [0, 0];
      for (minerals, gas) in player_unspent_resources {
        player_total_unspent_resources[0] += *minerals as u64;
        player_total_unspent_resources[1] += *gas as u64;
      }
      let num_unspent_resources = player_unspent_resources.len() as u64;
      avg_unspent_resources[index] = (
        if num_unspent_resources == 0 { 0 } else { (player_total_unspent_resources[0] / num_unspent_resources) as u16 },
        if num_unspent_resources == 0 { 0 } else { (player_total_unspent_resources[1] / num_unspent_resources) as u16 },
      );
    }

    let mut summary_stats = HashMap::new();
    for player_index in 0..2 {
      let player_summary_stats = HashMap::from([
        ("avg_collection_rate", SummaryStat::ResourceValues(avg_collection_rate[player_index])),
        ("resources_collected", SummaryStat::ResourceValues(resources_collected[player_index])),
        ("resources_lost", SummaryStat::ResourceValues(resources_lost[player_index])),
        ("avg_unspent_resources", SummaryStat::ResourceValues(avg_unspent_resources[player_index])),
        ("workers_produced", SummaryStat::Value(game.workers_active[player_index] as u16)),
        ("workers_lost", SummaryStat::Value(0)),
      ]);
      summary_stats.insert((player_index + 1) as u8, player_summary_stats);
    }

    // println!("player info {:?}", &replay.player_info);

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
    let mut map = String::new();
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
        .trim_end_matches(" LE")
        .to_string();
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

    let mut replay_build_mappings: [u16; 2] = [0, 0];
    let mut replay_builds: [Vec<String>; 2] = [vec![], vec![]];
    for (replay_build_index, build) in game.builds.iter_mut().enumerate() {
      build.sort_by(|a, b| a.1.cmp(&b.1));
      replay_builds[replay_build_index] = build
        .iter()
        .map(|(building, _)| building.clone())
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

    let mut replay_units_mappings: [u16; 2] = [0, 0];
    let mut replay_units: [Vec<String>; 2] = [vec![], vec![]];
    for (replay_unit_index, unit) in game.units.iter_mut().enumerate() {
      unit.sort_by(|a, b| a.1.cmp(&b.1));
      replay_units[replay_unit_index] = unit
        .iter()
        .map(|(unit, _)| unit.clone())
        .collect::<Vec<String>>();

      let joined_units = replay_units[replay_unit_index].join(",");
      match units.iter().position(|seen_units| &joined_units == seen_units) {
        Some(unit_index) => replay_units_mappings[replay_unit_index] = unit_index as u16,
        None => {
          units.push(joined_units);
          replay_units_mappings[replay_unit_index] = units.len() as u16 - 1;
        }
      }
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

    const GAS_BUILDINGS: [&str; 3] = [
      "Assimilator",
      "Refinery",
      "Extractor",
    ];

    let winner_build = replay_builds[(winner - 1) as usize]
      .iter()
      .filter(|building| !GAS_BUILDINGS.contains(&building.as_str()))
      .map(|building| building.clone())
      .collect::<Vec<String>>()
      .join(",");
    let loser_build = replay_builds[(loser - 1) as usize]
      .iter()
      .filter(|building| !GAS_BUILDINGS.contains(&building.as_str()))
      .map(|building| building.clone())
      .collect::<Vec<String>>()
      .join(",");

    let tinybird_game = TinybirdGame {
      content_hash: raw_replay.content_hash.clone(),
      winner_id: winner,
      winner_name: players[(winner - 1) as usize].name.clone(),
      winner_race: players[(winner - 1) as usize].race.clone(),
      winner_build,
      loser_id: loser,
      loser_name: players[(loser - 1) as usize].name.clone(),
      loser_race: players[(loser - 1) as usize].race.clone(),
      loser_build,
      matchup: serialized_matchup.join(""),
      player_names: serialized_players.join(""),
      players: serde_json::to_string(&players).unwrap(),
      builds: serde_json::to_string(&replay_builds).unwrap(),
      map: map.clone(),
      game_length,
      played_at,
      event: replay.tags.clone(),
    };

    let replay_summary: ReplaySummary = HashMap::from([
      ("players", ReplayEntry::Players(players)),
      ("builds", ReplayEntry::Builds(replay_builds)),
      ("build_mappings", ReplayEntry::BuildMappings(replay_build_mappings)),
      ("units", ReplayEntry::Units(replay_units)),
      ("units_mappings", ReplayEntry::UnitsMappings(replay_units_mappings)),
      ("winner", ReplayEntry::Winner(winner)),
      ("game_length", ReplayEntry::GameLength(game_length)),
      ("map", ReplayEntry::Map(map)),
      ("played_at", ReplayEntry::PlayedAt(played_at)),
      // ("summary_stats", ReplayEntry::SummaryStats(summary_stats)),
      // timeline
      ("metadata", ReplayEntry::Metadata(tags)),
      ("tinybird", ReplayEntry::Tinybird(tinybird_game)),
    ]);

    Ok(replay_summary)
  }
}
