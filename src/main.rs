mod decoders;
mod protocol;
mod mpq;
mod replay;
mod index;
mod event_parser;
mod utils;

use crate::replay::Replay;
use crate::index::Index;
use crate::utils::visit_dirs;
use crate::event_parser::EventParser;

use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::Path;
// use bzip2_rs::ParallelDecoderReader;
// use bzip2_rs::RayonThreadPool;

use std::time::Instant;

// #[global_allocator]
// static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Serialize)]
#[serde(untagged)]
pub enum SummaryStat {
  ResourceValues((u16, u16)),
  Value(u16),
}

#[derive(Serialize)]
pub struct Player {
  id: u8,
  name: String,
  race: String,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum ReplayEntry<'a> {
  Id(u32),
  ContentHash(String),
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

fn main() {
  let now = Instant::now();

  let replay_dir = Path::new("/Users/lukeholroyd/Desktop/replays/structured/");
  let mut replays: Vec<Replay> = vec![];
  let mut seen_replays: HashSet<String> = HashSet::new();
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

  let mut replay_id = 0;
  for mut replay in replays {
    let content_hash = replay.content_hash.clone();
    // don't include replays we've seen before
    if seen_replays.contains(&content_hash) {
      continue;
    }

    let parsed = replay.parse();

    let mut replay_summary = match EventParser::parse(parsed) {
      Ok(summary) => summary,
      Err(e) => {
        println!("replay parsing failed: {:?}\n", e);
        continue;
      },
    };

    replay_summary.insert("id", ReplayEntry::Id(replay_id));
    replay_summary.insert("content_hash", ReplayEntry::ContentHash(content_hash.clone()));

    if let ReplayEntry::Map(map) = replay_summary.get("map").unwrap() {
      map_index.add(map.clone(), replay_id);
    }

    for t in parsed.tags.split(", ") {
      metadata_index.add(t.to_string(), replay_id);
    }

    if let ReplayEntry::Players(players) = replay_summary.get("players").unwrap() {
      for player in players {
        race_index.add(player.race.clone(), replay_id as u32);
        player_index.add(player.name.clone(), replay_id as u32);
      }
    }

    result.replays.push(replay_summary);
    seen_replays.insert(content_hash);
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
