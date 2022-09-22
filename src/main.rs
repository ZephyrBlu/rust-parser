mod decoders;
mod protocol;
mod mpq;
mod replay;

use crate::replay::Replay;
use crate::decoders::DecoderResult;

use serde::Serialize;
use std::collections::HashMap;
use std::fs::read_dir;
use std::fs::File;
use std::io::Result;

use std::path::Path;
// use bzip2_rs::ParallelDecoderReader;
// use bzip2_rs::RayonThreadPool;

fn visit_dirs(replays: &mut Vec<Replay>, dir: &Path) -> Result<()> {
  if dir.is_dir() {
    for entry in read_dir(dir)? {
      let entry = entry?;
      let path = entry.path();
      // let filename = entry.file_name();
      if path.is_dir() {
        visit_dirs(replays, &entry.path())?;
      }

      match path.extension() {
        Some(extension) => {
          if extension == "SC2Replay" {
            replays.push(Replay::new(path));
          }
        },
        None => continue,
      }
    }
  }
  Ok(())
}

use std::time::Instant;

// #[global_allocator]
// static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Serialize)]
#[serde(untagged)]
enum SummaryStat {
  ResourceValues((u16, u16)),
  Value(u16),
}

#[derive(Serialize)]
struct Player {
  id: u8,
  name: String,
  race: String,
}

#[derive(Serialize)]
#[serde(untagged)]
enum ReplayEntry<'a> {
  Players(Vec<Player>),
  Winner(u8),
  GameLength(u16),
  // Map(&'a DecoderResult<'a>), // &str
  // PlayedAt(&'a DecoderResult<'a>), // u32
  Map(String),
  PlayedAt(u32),
  SummaryStats(HashMap<u8, HashMap<&'a str, SummaryStat>>),
  Metadata(&'a str),
}
type ReplaySummary<'a> = HashMap<&'a str, ReplayEntry<'a>>;

#[derive(Serialize)]
struct SerializedReplays<'a> {
  #[serde(borrow)]
  replays: Vec<ReplaySummary<'a>>,
}

fn main() {
  let now = Instant::now();

  let replay_dir = Path::new("/Users/lukeholroyd/Desktop/replays/structured/");
  let mut replays: Vec<Replay> = vec![];
  visit_dirs(&mut replays, replay_dir).unwrap();

  let num_replays = replays.len();
  println!("visited {:?} files in {:.2?}", num_replays, now.elapsed());

  let replay_summaries: Vec<ReplaySummary> = vec![];
  let mut result = SerializedReplays {
    replays: replay_summaries,
  };

  'replay: for mut replay in replays {
    let parsed = replay.parse();

    // println!("player_info {:?}", parsed.player_info);

    let mut player_id: u8 = 0;
    let mut workers_active: [u8; 2] = [0, 0];

    let mut minerals_collected: [u16; 2] = [0, 0];
    let mut minerals_lost: [u16; 2] = [0, 0];

    let mut gas_collected: [u16; 2] = [0, 0];
    let mut gas_lost: [u16; 2] = [0, 0];

    let mut collection_rate: Vec<Vec<(u16, u16)>> = vec![vec![], vec![]];
    let mut unspent_resources: Vec<Vec<(u16, u16)>> = vec![vec![], vec![]];

    for event in &parsed.tracker_events {
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
            panic!("Player ID is not a value {:?}", value);
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

            // only support 2 player games
            if player_index > 1 {
              continue 'replay;
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

    println!("current workers active for player 1 {:?}", workers_active[0]);
    println!("current workers active for player 2 {:?}", workers_active[1]);

    let resources_collected: [(u16, u16); 2] = [
      (minerals_collected[0], gas_collected[0]),
      (minerals_collected[1], gas_collected[1]),
    ];
    let resources_lost: [(u16, u16); 2] = [
      (minerals_lost[0], gas_lost[0]),
      (minerals_lost[1], gas_lost[1]),
    ];

    println!("resources collected player 1 {:?} / {:?}", minerals_collected[0], gas_collected[0]);
    println!("resources collected player 2 {:?} / {:?}", minerals_collected[1], gas_collected[1]);

    println!("resources lost player 1 {:?} / {:?}", minerals_lost[0], gas_lost[0]);
    println!("resources lost player 2 {:?} / {:?}", minerals_lost[1], gas_lost[1]);

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
    println!("avg collection rate player 1 {:?} / {:?}", avg_collection_rate[0].0, avg_collection_rate[0].1);
    println!("avg collection rate player 2 {:?} / {:?}", avg_collection_rate[1].0, avg_collection_rate[1].1);

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
    println!("avg unspent resources rate player 1 {:?} / {:?}", avg_unspent_resources[0].0, avg_unspent_resources[0].1);
    println!("avg unspent resources rate player 2 {:?} / {:?}", avg_unspent_resources[1].0, avg_unspent_resources[1].1);

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

    let parsed_metadata: replay::Metadata = serde_json::from_str(&parsed.metadata).unwrap();

    let (_, player_list) = &parsed.player_info
      .iter()
      .find(|(field, _)| *field == "m_playerList")
      .unwrap();

    let mut players = vec![];
    match player_list {
      DecoderResult::Array(values) => {
        // TODO: enumerated id is incorrect for P1 and P2 in games
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

    let winner = parsed_metadata.Players
      .iter()
      .find(|player| player.Result == "Win")
      .unwrap().PlayerID;
    let game_length = parsed_metadata.Duration;

    let raw_map = &parsed.player_info
      .iter()
      .find(|(field, _)| *field == "m_title")
      .unwrap().1;
    let mut map = String::new();
    if let DecoderResult::Blob(value) = raw_map {
      map = value.clone();
    }

    let raw_played_at = &parsed.player_info
      .iter()
      .find(|(field, _)| *field == "m_timeUTC")
      .unwrap().1;
    let mut played_at = 0;
    if let DecoderResult::Value(value) = raw_played_at {
      // TODO: this truncation is not working properly
      played_at = value.clone() as u32;
    }

    // let map = "CHANGE MAP NAME";
    // let played_at = 0;

    let replay_summary: ReplaySummary = HashMap::from([
      ("players", ReplayEntry::Players(players)),
      ("winner", ReplayEntry::Winner(winner)),
      ("game_length", ReplayEntry::GameLength(game_length)),
      ("map", ReplayEntry::Map(map)),
      ("played_at", ReplayEntry::PlayedAt(played_at)),
      ("summary_stats", ReplayEntry::SummaryStats(summary_stats)),
      ("metadata", ReplayEntry::Metadata("")),
    ]);

    result.replays.push(replay_summary);
  }

  println!("{:?} replays parsed in {:.2?}, {:?} per replay", num_replays, now.elapsed(), now.elapsed() / num_replays as u32);

  let output = File::create("../sc2.gg/src/assets/replays.json").unwrap();
  serde_json::to_writer(&output, &result);

  println!("replays serialized in {:?}", now.elapsed());
}
