
mod player_stats_event;
mod object_event;

use crate::TinybirdTimelineEntry;
use crate::game_state::GameState;
use crate::parser::TimelineContext;
use crate::replay::Event;
use crate::game::Game;
use crate::decoders::DecoderResult;
use player_stats_event::PlayerStatsEvent;
use object_event::ObjectEvent;

pub struct EventParser {
  context: TimelineContext,
  pub game: Game,
  state: GameState,
  pub timeline: Vec<TinybirdTimelineEntry>,
}

impl EventParser {
  pub fn new() -> EventParser {
    let game = Game::new();
    let timeline: Vec<TinybirdTimelineEntry> = vec![];

    EventParser {
      context: Default::default(),
      game,
      state: GameState::new(),
      timeline,
    }
  }

  pub fn reset(&mut self, new_context: TimelineContext) {
    self.context = new_context;
    self.game.reset();
    self.state.reset();
    self.timeline.clear();
  }

  pub fn parse(&mut self, event: &Event) -> Result<(), &'static str> {
    if let DecoderResult::Name(name) = &event.entries.last().unwrap().1 {
      match name.as_str() {
        "NNet.Replay.Tracker.SPlayerStatsEvent" => {
          PlayerStatsEvent::new(
            &self.context,
            &mut self.game,
            &mut self.timeline,
            event,
          );
        },
        "NNet.Replay.Tracker.SUnitInitEvent" |
        "NNet.Replay.Tracker.SUnitBornEvent" |
        "NNet.Replay.Tracker.SUnitTypeChangeEvent" |
        "NNet.Replay.Tracker.SUnitDiedEvent" => {
          ObjectEvent::new(
            &mut self.context,
            &mut self.game,
            &mut self.state,
            event,
            name,
          );
        },
        _other => (),
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
