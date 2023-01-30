use crate::TinybirdTimelineEntry;
use crate::parser::TimelineContext;
use crate::replay::Event;
use crate::decoders::DecoderResult;
use crate::game::Game;

pub struct PlayerStatsEvent;

impl PlayerStatsEvent {
  pub fn new(
    context: &TimelineContext,
    game: &mut Game,
    timeline: &mut Vec<TinybirdTimelineEntry>,
    event: &Event,
  ) -> Result<(), &'static str> {
    let mut player_id: u8 = 0;
    let mut gameloop: u16 = 0;
    let mut timeline_entry: TinybirdTimelineEntry = Default::default();

    for (field, value) in &event.entries {
      match field.as_str() {
        "_gameloop" => gameloop = if let DecoderResult::Value(v) = value {
          *v as u16
        } else {
          return Err("No gameloop present");
        },
        "m_playerId" => player_id = if let DecoderResult::Value(v) = value {
          *v as u8
        } else {
          return Err("Player ID is not a value");
        },
        "m_stats" => if let DecoderResult::Struct(entries) = value {
          let player_index = (player_id - 1) as usize;

          let mut event_minerals_collected: i64 = 0;
          let mut event_minerals_lost: i64 = 0;

          let mut event_gas_collected: i64 = 0;
          let mut event_gas_lost: i64 = 0;

          let mut event_minerals_collection_rate: u16 = 0;
          let mut event_gas_collection_rate: u16 = 0;

          let mut event_minerals_unspent_resources: u16 = 0;
          let mut event_gas_unspent_resources: u16 = 0;

          let mut event_minerals_army_value: u16 = 0;
          let mut event_gas_army_value: u16 = 0;

          // don't support more than 2 players
          if player_index > 1 {
            return Err("More than 1 player in replay");
          }

          for (key, value) in entries {
            match key.as_str() {
              "m_scoreValueWorkersActiveCount" => if let DecoderResult::Value(workers) = value {
                game.workers_active[player_index] = *workers as u8;
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
              // "m_scoreValueMineralsUsedInProgressArmy" |
              // "m_scoreValueMineralsUsedInProgressEconomy" |
              // "m_scoreValueMineralsUsedInProgressTechnology" |
              // "m_scoreValueMineralsUsedCurrentArmy" |
              // "m_scoreValueMineralsUsedCurrentEconomy" |
              // "m_scoreValueMineralsUsedCurrentTechnology" => if let DecoderResult::Value(minerals) = value {
              //   event_minerals_collected += minerals;
              // },
              // "m_scoreValueVespeneUsedInProgressArmy" |
              // "m_scoreValueVespeneUsedInProgressEconomy" |
              // "m_scoreValueVespeneUsedInProgressTechnology" |
              // "m_scoreValueVespeneUsedCurrentArmy" |
              // "m_scoreValueVespeneUsedCurrentEconomy" |
              // "m_scoreValueVespeneUsedCurrentTechnology" => if let DecoderResult::Value(gas) = value {
              //   event_gas_collected += gas;
              // },
              "m_scoreValueMineralsUsedInProgressArmy" |
              "m_scoreValueMineralsUsedCurrentArmy" => if let DecoderResult::Value(minerals) = value {
                event_minerals_army_value = *minerals as u16;
              },
              "m_scoreValueVespeneUsedInProgressArmy" |
              "m_scoreValueVespeneUsedCurrentArmy" => if let DecoderResult::Value(gas) = value {
                event_gas_army_value = *gas as u16;
              },
              _other => continue,
            }
          }

          game.minerals_collected[player_index] = event_minerals_collected as u16;
          game.minerals_lost[player_index] = event_minerals_lost as u16;

          game.gas_collected[player_index] = event_gas_collected as u16;
          game.gas_lost[player_index] = event_gas_lost as u16;

          game.collection_rate[player_index].push((event_minerals_collection_rate, event_gas_collection_rate));
          game.unspent_resources[player_index].push((event_minerals_unspent_resources, event_gas_unspent_resources));

          let win = if context.winner_id == player_id {
            1
          } else {
            0
          };

          let timeline_state: TinybirdTimelineEntry = TinybirdTimelineEntry {
            content_hash: context.content_hash.clone(),
            win,
            player_name: context.players[player_index].name.clone(),
            player_race: context.players[player_index].race.clone(),
            // player_build: context.players[player_index].build,
            player_collection_rate: event_minerals_collection_rate + event_gas_collection_rate,
            player_army_value: event_minerals_army_value + event_gas_army_value,
            player_workers_active: game.workers_active[player_index] as u16,
            // player_workers_lost: (),
            // player_workers_killed: (),
            matchup: context.matchup.clone(),
            map: context.map.clone(),
            event: context.event.clone(),
            game_length: context.game_length,
            played_at: context.played_at,
            ..Default::default()
          };

          timeline_entry = timeline_state;
        } else {
          panic!("didn't find struct {:?}",  value);
        },
        _other => continue,
      }
    }

    // event might be encountered before gameloop
    timeline_entry.gameloop = gameloop;

    if let Some(previous_timeline_entry) = timeline.last_mut() {
      if previous_timeline_entry.gameloop == gameloop {
        timeline_entry.opponent_name = previous_timeline_entry.player_name.clone();
        timeline_entry.opponent_race = previous_timeline_entry.player_race.clone();
        timeline_entry.opponent_collection_rate = previous_timeline_entry.player_collection_rate.clone();
        timeline_entry.opponent_army_value = previous_timeline_entry.player_army_value.clone();
        timeline_entry.opponent_workers_active = previous_timeline_entry.player_workers_active.clone();

        previous_timeline_entry.opponent_name = timeline_entry.player_name.clone();
        previous_timeline_entry.opponent_race = timeline_entry.player_race.clone();
        previous_timeline_entry.opponent_collection_rate = timeline_entry.player_collection_rate.clone();
        previous_timeline_entry.opponent_army_value = timeline_entry.player_army_value.clone();
        previous_timeline_entry.opponent_workers_active = timeline_entry.player_workers_active.clone();
      }
    }

    timeline.push(timeline_entry);

    Ok(())
  }
}
