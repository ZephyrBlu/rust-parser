
mod player_stats_event;
mod object_event;

use crate::TinybirdTimelineEntry;
use crate::game_state::GameState;
use crate::parser::TimelineContext;
use crate::replay::{Event, Parsed};
use crate::game::Game;
use crate::decoders::DecoderResult;
use player_stats_event::PlayerStatsEvent;
use object_event::ObjectEvent;

pub struct EventParser<'a> {
  context: TimelineContext,
  replay: &'a Parsed,
  game: &'a mut Game,
  state: GameState,
  timeline: &'a mut Vec<TinybirdTimelineEntry>,
}

impl<'a> EventParser<'a> {
  pub fn new(context: TimelineContext, replay: &'a Parsed, game: &'a mut Game, timeline: &'a mut Vec<TinybirdTimelineEntry>) -> EventParser<'a> {
    EventParser {
      context,
      replay,
      game,
      state: GameState::new(),
      timeline,
    }
  }

  pub fn parse(&mut self, event: &Event) -> Result<(), &'static str> {
    if let DecoderResult::Name(name) = &event.entries.last().unwrap().1 {
      match name.as_str() {
        "NNet.Replay.Tracker.SPlayerStatsEvent" => {
          PlayerStatsEvent::new(&self.context, self.game, self.timeline, event);
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

      // // 672 gameloops = ~30sec
      // if self.state.gameloop % 672 == 0 {
      //   let serialized_state = serde_json::to_string(&self.state).unwrap();
      //   self.timeline.push(serialized_state);
      // }

      Ok(())
    } else {
      Err("Found event without name")
    }
  }
}
