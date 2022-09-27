
mod player_stats;
mod object_event;

use crate::replay::{Event, Parsed};
use crate::game::Game;
use crate::decoders::DecoderResult;
use player_stats::PlayerStatsEvent;
use object_event::ObjectEvent;

pub struct EventParser<'a> {
  replay: &'a Parsed,
  game: &'a mut Game,
}

impl<'a> EventParser<'a> {
  pub fn new(replay: &'a Parsed, game: &'a mut Game) -> EventParser<'a> {
    EventParser {
      replay,
      game,
    }
  }

  pub fn parse(&mut self, event: &Event) -> Result<(), &'static str> {
    if let DecoderResult::Name(name) = &event.entries.last().unwrap().1 {
      match name.as_str() {
        "NNet.Replay.Tracker.SPlayerStatsEvent" => {
          PlayerStatsEvent::new(self.game, event);
          Ok(())
        },
        "NNet.Replay.Tracker.SUnitInitEvent" |
        "NNet.Replay.Tracker.SUnitBornEvent" |
        "NNet.Replay.Tracker.SUnitTypeChangeEvent" => {
          ObjectEvent::new(self.game, event);
          Ok(())
        },
        _other => Ok(()),
      }
    } else {
      Err("Found event without name")
    }
  }
}