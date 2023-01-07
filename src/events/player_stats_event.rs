use crate::replay::Event;
use crate::decoders::DecoderResult;
use crate::game::Game;
use crate::game_state::GameState;

pub struct PlayerStatsEvent;

impl PlayerStatsEvent {
  pub fn new(game: &mut Game, state: &mut GameState, event: &Event) -> Result<(), &'static str> {
    let mut player_id: u8 = 0;
    for (field, value) in &event.entries {
      match field.as_str() {
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

          game.minerals_collected[player_index] = event_minerals_collected as u16;
          game.minerals_lost[player_index] = event_minerals_lost as u16;

          game.gas_collected[player_index] = event_gas_collected as u16;
          game.gas_lost[player_index] = event_gas_lost as u16;

          game.collection_rate[player_index].push((event_minerals_collection_rate, event_gas_collection_rate));
          game.unspent_resources[player_index].push((event_minerals_unspent_resources, event_gas_unspent_resources));
        } else {
          panic!("didn't find struct {:?}",  value);
        },
        _other => continue,
      }
    }

    Ok(())
  }
}
