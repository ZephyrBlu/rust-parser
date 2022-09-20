use crate::{mpq::MPQArchive, protocol::Protocol, decoders::DecoderResult};

use std::{str::from_utf8, path::PathBuf};
use std::time::Instant;

pub struct Parsed<'a> {
  pub tracker_events: Vec<DecoderResult<'a>>,
}

pub struct Replay<'a> {
  pub file_path: String,
  pub archive: MPQArchive,
  pub protocol: Protocol,
  pub parsed: Option<Parsed<'a>>,
}

impl<'a> Replay<'a> {
  pub fn new(file_path: PathBuf) -> Replay<'a> {
    let path_str = file_path.to_str().unwrap();
    Replay {
      file_path: path_str.to_string(),
      archive: MPQArchive::new(path_str),
      protocol: Protocol::new(),
      parsed: None,
    }
  }

  pub fn parse (&'a mut self) {
    println!("parsing replay {:?}", self.file_path);

    let now = Instant::now();

    let header_content = &self.archive
      .header
      .user_data_header
      .as_ref()
      .expect("No user data header")
      .content;
    // println!("read header {:.2?}", now.elapsed());
  
    let contents = self.archive.read_file("replay.tracker.events").unwrap();
    // println!("read tracker events {:.2?}", now.elapsed());
  
    let game_info = self.archive.read_file("replay.game.events").unwrap();
    // println!("read game events {:.2?}", now.elapsed());
  
    let init_data = self.archive.read_file("replay.initData").unwrap();
    // println!("read details {:.2?}", now.elapsed());
  
    let raw_metadata = self.archive.read_file("replay.gamemetadata.json").unwrap();
    let metadata = from_utf8(&raw_metadata).unwrap();
    // println!("read metadata {:.2?}", now.elapsed());
  
    let details = self.archive.read_file("replay.details").unwrap();
  
    let tracker_events = self.protocol.decode_replay_tracker_events(contents);
    // println!("decoded replay tracker events {:.2?}", now.elapsed());
  
    let game_events = self.protocol.decode_replay_game_events(game_info);
    // println!("decoding replay game events {:.2?}", now.elapsed());

    self.parsed = Some(Parsed{ tracker_events });

    println!("parsed in {:.2?}", now.elapsed());
  }    
}
