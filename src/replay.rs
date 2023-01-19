use crate::decoders::{DecoderResult, EventEntry};
use crate::mpq::MPQArchive;
use crate::protocol::Protocol;

use serde::Deserialize;

use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug)]
pub struct Event {
  pub entries: Vec<(String, DecoderResult)>,
}

impl Event {
  pub fn new(entries: Vec<(String, DecoderResult)>) -> Event {
    Event {
      entries
    }
  }
}

#[derive(Debug, Deserialize)]
pub struct PlayerMetadata<'a> {
  pub PlayerID: u8,
  pub APM: f32,
  pub Result: &'a str,
  pub SelectedRace: &'a str,
  pub AssignedRace: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct Metadata<'a> {
  pub Title: &'a str,
  // pub GameVersion: &'a str,
  // pub DataBuild: &'a str,
  // pub DataVersion: &'a str,
  // pub BaseBuild: &'a str,
  pub Duration: u16,
  // pub IsNotAvailable: bool,
  pub Players: Vec<PlayerMetadata<'a>>,
}

#[derive(Debug)]
pub struct Parsed {
  pub player_info: Vec<EventEntry>,
  pub tracker_events: Vec<Event>,
  pub metadata: String,
  pub tags: String,
}

pub struct Replay {
  pub file_path: String,
  pub content_hash: String,
  pub parsed: Parsed,
}

impl<'a> Replay {
  pub fn new(file_path: PathBuf, content_hash: String, tags: Vec<&'a str>) -> Replay {
    let path_str = file_path.to_str().unwrap();
    println!("parsing replay {:?}", path_str);

    let archive = MPQArchive::new(path_str);
    let protocol: Protocol = Protocol::new();
    let parsed = Replay::parse(archive, protocol, tags);

    Replay {
      file_path: path_str.to_string(),
      content_hash,
      parsed,
    }
  }

  fn parse (mut archive: MPQArchive, protocol: Protocol, tags: Vec<&'a str>) -> Parsed {
    let now = Instant::now();

    // let header_content = &self.archive
    //   .header
    //   .user_data_header
    //   .as_ref()
    //   .expect("No user data header")
    //   .content;
    // // println!("read header {:.2?}", now.elapsed());

    let contents = archive.read_file("replay.tracker.events").unwrap();
    // println!("read tracker events {:.2?}", now.elapsed());

    // let game_info = self.archive.read_file("replay.game.events").unwrap();
    // // println!("read game events {:.2?}", now.elapsed());

    // let init_data = self.archive.read_file("replay.initData").unwrap();
    // // println!("read details {:.2?}", now.elapsed());

    let raw_metadata = archive.read_file("replay.gamemetadata.json").unwrap();
    let metadata = String::from_utf8(raw_metadata.clone()).unwrap();
    // println!("read metadata {:.2?}", now.elapsed());

    let details = archive.read_file("replay.details").unwrap();
    let player_info = protocol.decode_replay_details(details);

    let tracker_events = protocol.decode_replay_tracker_events(contents);
    // println!("decoded replay tracker events {:.2?}", now.elapsed());

    // let game_events = self.protocol.decode_replay_game_events(game_info);
    // // println!("decoding replay game events {:.2?}", now.elapsed());

    println!("parsed in {:.2?}", now.elapsed());

    Parsed {
      player_info,
      tracker_events,
      metadata,
      tags: tags.join(","),
    }
  }    

  // // function that doesn't parse replay events for speed
  // // can return high level information about game like
  // // date, matchup, MMR, etc to decide whether to skip parsing
  // pub fn peek() {

  // }
}
