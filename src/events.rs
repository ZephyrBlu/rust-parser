
mod player_stats_event;
mod object_event;

use crate::game_state::GameState;
use crate::replay::{Event, Parsed};
use crate::game::Game;
use crate::decoders::DecoderResult;
use player_stats_event::PlayerStatsEvent;
use object_event::ObjectEvent;

pub struct EventParser<'a> {
  replay: &'a Parsed,
  game: &'a mut Game,
  state: GameState,
  timeline: Vec<String>,
}

impl<'a> EventParser<'a> {
  pub fn new(replay: &'a Parsed, game: &'a mut Game) -> EventParser<'a> {
    EventParser {
      replay,
      game,
      state: GameState::new(),
      timeline: vec![],
    }
  }

  pub fn parse(&mut self, event: &Event) -> Result<(), &'static str> {
    if let DecoderResult::Name(name) = &event.entries.last().unwrap().1 {
      match name.as_str() {
        "NNet.Replay.Tracker.SPlayerStatsEvent" => {
          PlayerStatsEvent::new(self.game, &mut self.state, event);
          // Ok(())
        },
        "NNet.Replay.Tracker.SUnitInitEvent" |
        "NNet.Replay.Tracker.SUnitBornEvent" |
        "NNet.Replay.Tracker.SUnitTypeChangeEvent" |
        "NNet.Replay.Tracker.SUnitDiedEvent" => {
          ObjectEvent::new(self.game, &mut self.state, event, name);
          // Ok(())
        },
        _other => () // Ok(()),
      }

      // 672 gameloops = ~30sec
      if self.state.gameloop != 0 && self.state.gameloop % 672 == 0 {
        let serialized_state = serde_json::to_string(&self.state).unwrap();
        self.timeline.push(serialized_state);
      }

      Ok(())
    } else {
      Err("Found event without name")
    }
  }
}
