use crate::game::{Game, GameObject};
use crate::parser::TimelineContext;
use crate::replay::Event;
use crate::decoders::DecoderResult;

use std::collections::hash_map::Entry;
use std::collections::HashSet;
use std::time::Instant;

const UNITS: [&str; 47] = [
  // Protoss
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
  "Marine",
  "Reaper",
  "Marauder",
  "Ghost",
  "Hellion",
  "WidowMine",
  "Cyclone",
  "SiegeTank",
  "Thor",
  "VikingFighter",
  "Medivac",
  "Liberator",
  "Raven",
  "Banshee",
  "Battlecruiser",

  // Zerg
  "Queen",
  "Zergling",
  "Baneling",
  "Roach",
  "Ravager",
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

const ALLOWED_TRANSITIONS: [(&str, &str); 9] = [
  // buildings
  ("CommandCenter", "OrbitalCommand"),
  ("CommandCenter", "PlanetaryFortress"),
  ("Hatchery", "Lair"),
  ("Lair", "Hive"),
  ("Spire", "GreaterSpire"),

  // units
  ("Zergling", "Baneling"),
  ("Roach", "Ravager"),
  ("Hydralisk", "LurkerMP"),
  ("Corruptor", "BroodLord"),
];

const TRANSITION_BUILD_TIMES: [(&str, u16); 9] = [
  // buildings
  ("OrbitalCommand", 560),
  ("PlanetaryFortress", 807),
  ("Lair", 1277),
  ("Hive", 1590),
  ("GreaterSpire", 1591),

  // units
  ("Zergling", 314),
  ("Roach", 269),
  ("Hydralisk", 404),
  ("Corruptor", 538),
];

const WORKERS: [&str; 3] = [
  "SCV",
  "Probe",
  "Drone",
];

pub struct ObjectEvent;

#[derive(Debug, PartialEq)]
pub enum ObjectType {
  Building,
  Unit,
}

const MAX_BUILD_LENGTH: u8 = 15;
const MAX_UNIT_BUILD_LENGTH: u8 = 30;
const MAX_UNIT_TYPES: u8 = 10;

impl ObjectEvent {
  pub fn new(
    names: &mut Vec<String>,
    context: &mut TimelineContext,
    game: &mut Game,
    event: &Event,
    event_name: &String,
  ) -> Result<(), &'static str> {
    let now = Instant::now();
    let mut player_id: u8 = 0;
    let mut event_object_name = "";
    let mut event_object_type = ObjectType::Building;
    let mut tag_index = 0;
    let mut tag_recycle = 0;
    let mut current_gameloop = 0;

    for (field, value) in &event.entries {
      match field.as_str() {
        "m_controlPlayerId" => player_id = if let DecoderResult::Value(v) = value {
          *v as u8
        } else {
          return Err("Player ID is not a value");
        },
        "m_unitTypeName" => if let DecoderResult::Blob(name) = value {
          if BUILDINGS.contains(&name.as_str()) {
            event_object_name = name;
            // event_object_type = "building";
          }

          // if UNITS.contains(&name.as_str()) {
          //   event_object_name = name;
          //   event_object_type = "unit";
          // }
        },
        // "m_killerUnitTagIndex" => if let DecoderResult::Blob
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
      match game.objects.binary_search_by(|obj| obj.tag_index.cmp(&tag_index)) {
        Ok(idx) => {
          game.objects.remove(idx);
          ()
        },
        Err(_) => (),
      }
    }

    if event_object_name == "" {
      return Err("Object name not found");
    }

    // if !game.objects.contains_key(&tag_index) {
    // if let None = game.objects.iter().find(|obj| obj.tag_index == tag_index) {
    let mut game_object = match game.objects.binary_search_by(|obj| obj.tag_index.cmp(&tag_index)) {
      Ok(idx) => &mut game.objects[idx],
      Err(idx) => {
        let tag_id = (tag_index << 18) + tag_recycle;

        let mut object_name_idx: i16 = -1;
        for (idx, name) in names.iter().enumerate() {
          if *name == event_object_name {
            object_name_idx = idx as i16;
            break;
          }
        }

        if object_name_idx == -1 {
          names.push(event_object_name.to_string());
          object_name_idx = names.len() as i16 - 1;
        }

        let new_object = GameObject {
          object_name_idx: object_name_idx as usize,
          object_type: event_object_type,
          tag_id,
          tag_index,
          tag_recycle,
          player_id,
        };

        game.objects.insert(idx, new_object);
        &mut game.objects[idx]
      },
    };

    let player_index = game_object.player_id - 1;
    let mut game_object_name = &names[game_object.object_name_idx];

    if player_index > 1 {
      return Err("More than 2 players in replay");
    }

    let transition = (game_object_name.as_str(), event_object_name);
    let mut calculated_gameloop = current_gameloop;

    if event_name == "NNet.Replay.Tracker.SUnitTypeChangeEvent" {
      if ALLOWED_TRANSITIONS.contains(&transition) {
        let mut new_object_name_idx: i16 = -1;
        for (idx, name) in names.iter().enumerate() {
          if *name == event_object_name {
            new_object_name_idx = idx as i16;
            break;
          }
        }

        if new_object_name_idx == -1 {
          names.push(event_object_name.to_string());
          new_object_name_idx = names.len() as i16 - 1;
        }
        game_object.object_name_idx = new_object_name_idx as usize;
        game_object_name = &names[game_object.object_name_idx];

        let transition_object= TRANSITION_BUILD_TIMES
          .iter()
          .find(|(name, _)| *name == game_object_name);

        calculated_gameloop = match transition_object {
          Some((_, transition_gameloops)) => current_gameloop - transition_gameloops,
          None => current_gameloop,
        };
      } else {
        return Ok(());
      }
    }

    if
      event_name == "NNet.Replay.Tracker.SUnitDiedEvent" &&
      game_object.object_type == ObjectType::Unit &&
      WORKERS.contains(&game_object_name.as_str())
      // and obj killed by something, drones can die morphing
    {
      let opponent_index: usize = if player_index == 1 {
        0
      } else {
        1
      };
      context.workers_lost[player_index as usize] += 1;
      context.workers_killed[opponent_index] += 1;
    }

    // 9408 = ~7min, 22.4 gameloops per sec
    if
      calculated_gameloop > 0 &&
      calculated_gameloop < 9408
    {
      // I think the partial eq here is super inefficient
      if game_object.object_type == ObjectType::Building &&
        !(game_object_name.contains("Reactor") || game_object_name.contains("TechLab")) &&
        game.builds[player_index as usize].len() < MAX_BUILD_LENGTH as usize
      {
        game.builds[player_index as usize].push((game_object_name.to_owned(), calculated_gameloop));
      }

      // if game_object.object_type == "unit" &&
      //   game.units[player_index as usize].len() < MAX_UNIT_BUILD_LENGTH as usize
      // {
      //   game.units[player_index as usize].push((game_object.object_name.to_string(), calculated_gameloop));
      // }
    }

    Ok(())
  }
}
