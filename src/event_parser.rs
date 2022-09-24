use crate::{ReplaySummary, ReplayEntry, SummaryStat, Player};
use crate::replay;
use crate::replay::Replay;
use crate::replay::Parsed;
use crate::decoders::DecoderResult;

use std::collections::HashMap;

pub struct EventParser<'a> {
  replay: Replay<'a>,
}

impl<'a> EventParser<'a> {
  pub fn parse(replay: &Parsed) -> Result<ReplaySummary<'a>, &'a str> {
    let RACE_MAPPING = HashMap::from([
      ("저그", "Zerg"),
      ("异虫", "Zerg"),
      ("蟲族", "Zerg"),
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

    let mut player_id: u8 = 0;
    let mut workers_active: [u8; 2] = [0, 0];

    let mut minerals_collected: [u16; 2] = [0, 0];
    let mut minerals_lost: [u16; 2] = [0, 0];

    let mut gas_collected: [u16; 2] = [0, 0];
    let mut gas_lost: [u16; 2] = [0, 0];

    let mut collection_rate: Vec<Vec<(u16, u16)>> = vec![vec![], vec![]];
    let mut unspent_resources: Vec<Vec<(u16, u16)>> = vec![vec![], vec![]];

    for event in &replay.tracker_events {
      if let DecoderResult::Name(name) = event.entries.last().unwrap().1 {
        if *name != "NNet.Replay.Tracker.SPlayerStatsEvent" {
          continue;
        }
      };

      // println!("event entries {:?}", event.entries);

      for (field, value) in &event.entries {
        // println!("event entry values {:?} {:?}", field, value);
        match *field {
          "m_playerId" => player_id = if let DecoderResult::Value(v) = value {
            *v as u8
          } else {
            return Err("Player ID is not a value");
          },
          "m_stats" => if let DecoderResult::Struct(entries) = value {
            // println!("stats values {:?}", event.entries);

            let player_index = (player_id - 1) as usize;

            let mut event_minerals_collected: i64 = 0;
            let mut event_minerals_lost: i64 = 0;

            let mut event_gas_collected: i64 = 0;
            let mut event_gas_lost: i64 = 0;

            let mut event_minerals_collection_rate: u16 = 0;
            let mut event_gas_collection_rate: u16 = 0;

            let mut event_minerals_unspent_resources: u16 = 0;
            let mut event_gas_unspent_resources: u16 = 0;

            // don't support more than 2 players
            if player_index > 1 {
              return Err("More than 1 player in replay");
            }

            for (key, value) in entries {
              match *key {
                "m_scoreValueWorkersActiveCount" => if let DecoderResult::Value(workers) = value {
                  workers_active[player_index] = *workers as u8;
                },
                "m_scoreValueMineralsCollectionRate" => if let DecoderResult::Value(minerals) = value {
                  event_minerals_collection_rate = *minerals as u16;
                },
                "m_scoreValueVespeneCollectionRate" => if let DecoderResult::Value(gas) = value {
                  event_gas_collection_rate = *gas as u16;
                },
                "m_scoreValueMineralsCurrent" => if let DecoderResult::Value(minerals) = value {
                  event_minerals_unspent_resources = *minerals as u16;
                  event_minerals_collected += minerals;
                },
                "m_scoreValueVespeneCurrent" => if let DecoderResult::Value(gas) = value {
                  event_gas_unspent_resources = *gas as u16;
                  event_gas_collected += gas;
                },
                "m_scoreValueMineralsLostArmy" |
                "m_scoreValueMineralsLostEconomy" |
                "m_scoreValueMineralsLostTechnology" => if let DecoderResult::Value(minerals) = value {
                  event_minerals_lost += minerals.abs();
                  event_minerals_collected += minerals;
                }
                "m_scoreValueVespeneLostArmy" |
                "m_scoreValueVespeneLostEconomy" |
                "m_scoreValueVespeneLostTechnology" => if let DecoderResult::Value(gas) = value {
                  event_gas_lost += gas.abs();
                  event_gas_collected += gas;
                }
                "m_scoreValueMineralsUsedInProgressArmy" |
                "m_scoreValueMineralsUsedInProgressEconomy" |
                "m_scoreValueMineralsUsedInProgressTechnology" |
                "m_scoreValueMineralsUsedCurrentArmy" |
                "m_scoreValueMineralsUsedCurrentEconomy" |
                "m_scoreValueMineralsUsedCurrentTechnology" => if let DecoderResult::Value(minerals) = value {
                  event_minerals_collected += minerals;
                },
                "m_scoreValueVespeneUsedInProgressArmy" |
                "m_scoreValueVespeneUsedInProgressEconomy" |
                "m_scoreValueVespeneUsedInProgressTechnology" |
                "m_scoreValueVespeneUsedCurrentArmy" |
                "m_scoreValueVespeneUsedCurrentEconomy" |
                "m_scoreValueVespeneUsedCurrentTechnology" => if let DecoderResult::Value(gas) = value {
                  event_gas_collected += gas;
                },
                _other => continue,
              }
            }

            minerals_collected[player_index] = event_minerals_collected as u16;
            minerals_lost[player_index] = event_minerals_lost as u16;

            gas_collected[player_index] = event_gas_collected as u16;
            gas_lost[player_index] = event_gas_lost as u16;

            collection_rate[player_index].push((event_minerals_collection_rate, event_gas_collection_rate));
            unspent_resources[player_index].push((event_minerals_unspent_resources, event_gas_unspent_resources));
          } else {
            panic!("didn't find struct {:?}",  value);
          },
          _other => continue,
        }
      }
    }

    let resources_collected: [(u16, u16); 2] = [
      (minerals_collected[0], gas_collected[0]),
      (minerals_collected[1], gas_collected[1]),
    ];
    let resources_lost: [(u16, u16); 2] = [
      (minerals_lost[0], gas_lost[0]),
      (minerals_lost[1], gas_lost[1]),
    ];

    let mut avg_collection_rate: [(u16, u16); 2] = [(0, 0), (0, 0)];
    for (index, player_collection_rate) in collection_rate.iter().enumerate() {
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
    for (index, player_unspent_resources) in unspent_resources.iter().enumerate() {
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
        ("workers_produced", SummaryStat::Value(workers_active[player_index] as u16)),
        ("workers_lost", SummaryStat::Value(0)),
      ]);
      summary_stats.insert((player_index + 1) as u8, player_summary_stats);
    }

    // println!("player info {:?}", &parsed.player_info);

    let parsed_metadata: replay::Metadata = serde_json::from_str(&replay.metadata).unwrap();

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
      map = value.clone();
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

    let (_, player_list) = replay.player_info
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

              let raw_name = &player_values
                .iter()
                .find(|(field, _)| *field == "m_name")
                .unwrap().1;
              let mut name = String::new();
              if let DecoderResult::Blob(value) = raw_name {
                name = value.clone();
              }

              if let Some(value) = RACE_MAPPING.get(name.as_str()) {
                name = value.to_string();
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

    let replay_summary: ReplaySummary = HashMap::from([
      ("players", ReplayEntry::Players(players)),
      ("winner", ReplayEntry::Winner(winner)),
      ("game_length", ReplayEntry::GameLength(game_length)),
      ("map", ReplayEntry::Map(map)),
      ("played_at", ReplayEntry::PlayedAt(played_at)),
      ("summary_stats", ReplayEntry::SummaryStats(summary_stats)),
      ("metadata", ReplayEntry::Metadata(replay.tags.clone())),
    ]);

    Ok(replay_summary)
  }
}