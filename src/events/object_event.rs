use crate::game::{Game, GameObject};
use crate::replay::Event;
use crate::decoders::DecoderResult;
use crate::game_state::GameState;

use std::collections::hash_map::Entry;

const UNITS: [&str; 54] = [
  // Protoss
  "Probe",
  "Zealot",
  "Stalker",
  "Sentry",
  "Adept",
  "HighTemplar",
  "DarkTemplar",
  "Archon",
  "Observer",
  "WarpPrism",
  "Immortal",
  "Colossus",
  "Disruptor",
  "Phoenix",
  "VoidRay",
  "Oracle",
  "Tempest",
  "Carrier",
  "Mothership",

  // Terran
  "SCV",
  "MULE",
  "Marine",
  "Reaper",
  "Marauder",
  "Ghost",
  "Hellion",
  "WidowMine",
  "Cyclone",
  "SiegeTank",

  // which does thor spawn as?
  "Thor",
  "ThorAP",

  "VikingFighter",
  "Medivac",
  "Liberator",
  "Raven",
  "Banshee",
  "Battlecruiser",

  // Zerg
  "Drone",
  "Overlord",
  "Queen",
  "Zergling",
  "Baneling",
  "Roach",
  "Ravager",
  "Overseer",
  "Hydralisk",
  "LurkerMP",
  "Mutalisk",
  "Corrupter",
  "SwarmHostMP",
  "Infestor",
  "Viper",
  "Ultralisk",
  "BroodLord",
];

// doesn't include supply structures, gas collectors and support structures
const BUILDINGS: [&str; 45] = [
  // Protoss
  "Nexus",
  "Gateway",
  "Forge",
  "CyberneticsCore",
  "PhotonCannon", // we'll see about this one
  "RoboticsFacility",
  "Stargate",
  "TwilightCouncil",
  "RoboticsBay",
  "FleetBeacon",
  "TemplarArchives",
  "DarkShrine",

  // Terran
  "CommandCenter",
  "OrbitalCommand",
  "PlanetaryFortress",
  "Barracks",
  "EngineeringBay",
  "GhostAcademy",
  "Factory",
  "Starport",
  "Armory",
  "FusionCore",
  "BarracksTechLab",
  "FactoryTechLab",
  "StarportTechLab",
  "BarracksReactor",
  "FactoryReactor",
  "StarportReactor",

  // Zerg
  "Hatchery",
  "SpawningPool",
  "EvolutionChamber",
  "RoachWarren",
  "BanelingNest",
  "Lair",
  "HydraliskDen",
  "LurkerDenMP",
  "Spire",
  "GreaterSpire",
  "NydusNetwork",
  "InfestationPit",
  "Hive",
  "UltraliskCavern",

  // gas collectors
  "Assimilator",
  "Refinery",
  "Extractor",
];

const ALLOWED_TRANSITIONS: [(&str, &str); 3] = [
  ("CommandCenter", "OrbitalCommand"),
  ("Hatchery", "Lair"),
  ("Lair", "Hive"),
];

const TRANSITION_BUILD_TIMES: [(&str, u16); 3] = [
  ("OrbitalCommand", 560),
  ("Lair", 1277),
  ("Hive", 1590),
];

pub struct ObjectEvent;

const MAX_BUILD_LENGTH: u8 = 15;

impl ObjectEvent {
  pub fn new(game: &mut Game, state: &mut GameState, event: &Event, event_name: &String) -> Result<(), &'static str> {
    let mut player_id: u8 = 0;
    let mut event_object_name = "";
    let mut event_object_type = "";
    let mut tag_index = 0;
    let mut tag_recycle = 0;
    let mut current_gameloop = 0;

    // println!("event entry values {:?}", event.entries);
    for (field, value) in &event.entries {
      match field.as_str() {
        "m_controlPlayerId" => player_id = if let DecoderResult::Value(v) = value {
          *v as u8
        } else {
          return Err("Player ID is not a value");
        },
        "m_unitTypeName" => if let DecoderResult::Blob(name) = value {
          if UNITS.contains(&name.as_str()) {
            event_object_name = name;
            event_object_type = "unit"
          }

          if BUILDINGS.contains(&name.as_str()) {
            event_object_name = name;
            event_object_type = "building";
          }
        },
        "m_unitTagIndex" => if let DecoderResult::Value(index) = value {
          tag_index = *index as u32;
        },
        "m_unitTagRecycle" => if let DecoderResult::Value(recycle) = value{
          tag_recycle = *recycle as u32;
        },
        "_gameloop" => if let DecoderResult::Value(gameloop) = value {
          current_gameloop = *gameloop as u16;
        },
        _other => continue,
      }
    }

    if event_name == "NNet.Replay.Tracker.SUnitDiedEvent" {
      if let Entry::Occupied(game_object) = game.objects.entry(tag_index) {
        game_object.remove_entry();
      }
    }

    if event_object_name == "" {
      return Err("Object name not found");
    }

    let tag = (tag_index << 18) + tag_recycle;
    let player_index = match game.buildings.get(&tag) {
      Some(building_player_id) => {
        building_player_id - 1
      },
      None => {
        game.buildings.insert(tag, player_id);
        player_id - 1
      },
    };

    if player_index > 1 {
      return Err("More than 2 players in replay");
    }

    // if game.builds[player_index as usize].len() < 10 && current_gameloop > 0 {
    //   game.builds[player_index as usize].push(building_name);
    // }

    state.gameloop = current_gameloop;
    let event_object_type_state = if event_object_type == "building" {
      &mut state.buildings
    } else {
      &mut state.units
    };
    event_object_type_state
      .entry(event_object_name.to_string())
      .and_modify(|count| *count += 1)
      .or_insert(1);

    // let mut game_object = game.fetch_or_create_object_by_id(event_object_name, event_object_type, tag_index, tag_recycle);

    let game_object = game.objects
      .entry(tag_index)
      .or_insert(GameObject {
        object_name: event_object_name.to_string(),
        object_type: event_object_type.to_string(),
        tag_index,
        tag_recycle,
      });
    let transition = (game_object.object_name.as_str(), event_object_name);
    let mut calculated_gameloop = current_gameloop;

    if event_name == "NNet.Replay.Tracker.SUnitTypeChangeEvent" && game_object.object_type == "building" {
      if ALLOWED_TRANSITIONS.contains(&transition) {
        game_object.object_name = event_object_name.to_string();

        let transition_object= TRANSITION_BUILD_TIMES
          .iter()
          .find(|(name, _)| *name == event_object_name);

        calculated_gameloop = match transition_object {
          Some((_, transition_gameloops)) => current_gameloop - transition_gameloops,
          None => current_gameloop,
        };
      } else {
        return Ok(());
      }
    }

    // 9408 = ~7min, 22.4 gameloops per sec
    if
      calculated_gameloop > 0 &&
      calculated_gameloop < 9408 &&
      game_object.object_type == "building" &&
      !(game_object.object_name.contains("Reactor") || game_object.object_name.contains("TechLab")) &&
      game.builds[player_index as usize].len() < MAX_BUILD_LENGTH as usize
    {
      game.builds[player_index as usize].push((game_object.object_name.to_string(), calculated_gameloop));
    }

    Ok(())
  }
}
