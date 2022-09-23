mod decoders;
mod protocol;
mod mpq;
mod replay;
mod index;

use crate::replay::Replay;
use crate::decoders::DecoderResult;
use crate::index::Index;

use serde::Serialize;
use std::collections::HashMap;
use std::fs::read_dir;
use std::fs::File;
use std::io::Result;

use std::path::Path;
// use bzip2_rs::ParallelDecoderReader;
// use bzip2_rs::RayonThreadPool;

fn visit_dirs(replays: &mut Vec<Replay>, dir: &Path) -> Result<()> {
  const TOURNAMENTS: [&str; 6] = [
    "ASUS ROG",
    "DreamHack Masters",
    "HomeStory Cup",
    "IEM Katowice",
    "StayAtHome Story Cup",
    "TSL",
  ];
  const VALID_TAGS: [&str; 10] = [
    "FINAL",
    "SEMIFINAL",
    "QUARTERFINAL",
    "PLAYOFF",
    "GROUP",
    "RO32",
    "RO16",
    "RO8",
    "RO4",
    "RO2",
  ];
  let TAG_MAPPINGS: HashMap<&str, &str> = HashMap::from([
    ("RO2", "Final"),
    ("RO4", "Semifinal"),
    ("RO8", "Quarterfinal"),
    ("FINAL", "Final"),
    ("SEMIFINAL", "Semifinal"),
    ("QUARTERFINAL", "Quarterfinal"),
    ("PLAYOFF", "Playoff"),
    ("GROUP", "Group"),
  ]);

  if dir.is_dir() {
    for entry in read_dir(dir)? {
      let entry = entry?;
      let path = entry.path();
      // let filename = entry.file_name();
      if path.is_dir() && !path.to_str().unwrap().contains("PiG") {
        visit_dirs(replays, &entry.path())?;
      }

      match path.extension() {
        Some(extension) => {
          if extension == "SC2Replay" {
            let current_path = path.to_str().unwrap();
            let mut tags = vec![];

            for tag in TOURNAMENTS {
              if current_path.contains(tag) {
                tags.push(tag);
              }
            }

            for tag in VALID_TAGS {
              if current_path.to_uppercase().contains(tag) {
                let mut mapped_tag = tag;
                if TAG_MAPPINGS.contains_key(mapped_tag) {
                  mapped_tag = match TAG_MAPPINGS.get(tag) {
                    Some(value) => value,
                    None => tag,
                  }
                }
                tags.push(mapped_tag);
              }
            }

            replays.push(Replay::new(path, tags));
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
  Id(u32),
  Players(Vec<Player>),
  Winner(u8),
  GameLength(u16),
  // Map(&'a DecoderResult<'a>), // &str
  // PlayedAt(&'a DecoderResult<'a>), // u32
  Map(String),
  PlayedAt(u64),
  SummaryStats(HashMap<u8, HashMap<&'a str, SummaryStat>>),
  Metadata(String),
}
type ReplaySummary<'a> = HashMap<&'a str, ReplayEntry<'a>>;

#[derive(Serialize)]
struct SerializedReplays<'a> {
  #[serde(borrow)]
  replays: Vec<ReplaySummary<'a>>,
}

// TODO: filter duplicate replays with content hash
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

  let mut race_index = Index::new();
  let mut player_index = Index::new();
  let mut metadata_index = Index::new();
  let mut map_index = Index::new();

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

  let mut replay_id = 0;
  'replay: for mut replay in replays {
    let parsed = replay.parse(replay_id as u32);

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

            // don't support more than 2 players
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

    // println!("player info {:?}", &parsed.player_info);

    let parsed_metadata: replay::Metadata = serde_json::from_str(&parsed.metadata).unwrap();

    let winner = match parsed_metadata.Players
    .iter()
    .find(|player| player.Result == "Win") {
      Some(player) => player.PlayerID,
      None => continue 'replay,
    };
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
      played_at = value.clone() as u64;
    }
    // game records time in window epoch for some reason
    // https://en.wikipedia.org/wiki/Epoch_(computing)
    played_at = (played_at / 10000000) - 11644473600;

    let (_, player_list) = &parsed.player_info
      .iter()
      .find(|(field, _)| *field == "m_playerList")
      .unwrap();

    let mut players = vec![];
    match player_list {
      DecoderResult::Array(values) => {
        // TODO: enumerated id is incorrect for P1 and P2 in games

        // don't support 1 player or 3+ player games
        if values.len() != 2 {
          continue 'replay;
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

              race_index.add(race.clone(), replay_id as u32);
              player_index.add(name.clone(), replay_id as u32);
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

    map_index.add(map.clone(), replay_id);
    for t in parsed.tags.split(", ") {
      metadata_index.add(t.to_string(), replay_id);
    }

    let replay_summary: ReplaySummary = HashMap::from([
      ("id", ReplayEntry::Id(parsed.id)),
      ("players", ReplayEntry::Players(players)),
      ("winner", ReplayEntry::Winner(winner)),
      ("game_length", ReplayEntry::GameLength(game_length)),
      ("map", ReplayEntry::Map(map)),
      ("played_at", ReplayEntry::PlayedAt(played_at)),
      ("summary_stats", ReplayEntry::SummaryStats(summary_stats)),
      ("metadata", ReplayEntry::Metadata(parsed.tags.clone())),
    ]);

    result.replays.push(replay_summary);
    replay_id += 1;
  }

  println!("{:?} replays parsed in {:.2?}, {:?} per replay", num_replays, now.elapsed(), now.elapsed() / num_replays as u32);

  let replay_output = File::create("../sc2.gg/src/assets/replays.json").unwrap();
  serde_json::to_writer(&replay_output, &result);

  let indexes = HashMap::from([
    ("race", race_index),
    ("player", player_index),
    ("metadata", metadata_index),
    ("map", map_index),
  ]);

  let index_output = File::create("../sc2.gg/src/assets/indexes.json").unwrap();
  serde_json::to_writer(&index_output, &indexes);

  println!("replays serialized in {:?}", now.elapsed());
}
