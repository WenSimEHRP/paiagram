use crate::types::*;
use anyhow::Result;
use serde::Deserialize;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

/// hash string to ids
fn hash_id(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

#[derive(Deserialize)]
#[serde(try_from = "NetworkHelper")]
pub struct Network {
    pub stations: HashMap<StationID, Station>,
    pub trains: HashMap<TrainID, Train>,
    pub intervals: HashMap<IntervalID, Interval>,
}

#[derive(Deserialize)]
struct NetworkHelper {
    stations: HashMap<String, StationHelper>,
    trains: HashMap<String, TrainHelper>,
    intervals: Vec<((String, String), IntervalHelper)>,
}

impl TryFrom<NetworkHelper> for Network {
    type Error = anyhow::Error;
    fn try_from(helper: NetworkHelper) -> Result<Self, Self::Error> {
        let mut stations: HashMap<StationID, Station> =
            HashMap::with_capacity(helper.stations.len());
        let mut trains: HashMap<TrainID, Train> = HashMap::with_capacity(helper.trains.len());
        let mut intervals: HashMap<IntervalID, Interval> =
            HashMap::with_capacity(helper.intervals.len());
        for (station_name, station_helper) in helper.stations {
            let station_id = hash_id(&station_name);
            let station = Station {
                label_size: station_helper.label_size,
                // milestones: station_helper.milestones,
                // tracks: station_helper.tracks.unwrap_or(1),
                // name: station_name,
                intervals: HashSet::new(),
                trains: HashSet::new(),
            };
            stations.insert(station_id, station);
        }
        for ((from_station, to_station), interval_helper) in helper.intervals {
            let from_station_id = hash_id(&from_station);
            let to_station_id = hash_id(&to_station);
            let interval_id = (from_station_id, to_station_id);
            let new_interval = Interval {
                // name: interval_helper.name,
                length: interval_helper.length,
            };
            match interval_helper.bidirectional {
                Some(true) | None => {
                    if intervals.contains_key(&interval_id.reverse()) {
                        return Err(anyhow::anyhow!(
                            "Interval from '{}' to '{}' already exists",
                            to_station,
                            from_station
                        ));
                    }
                    intervals.insert(interval_id.reverse(), new_interval.clone());
                    if intervals.contains_key(&interval_id) {
                        return Err(anyhow::anyhow!(
                            "Interval from '{}' to '{}' already exists",
                            from_station,
                            to_station
                        ));
                    }
                    intervals.insert(interval_id, new_interval);
                }
                _ => {
                    if intervals.contains_key(&interval_id) {
                        return Err(anyhow::anyhow!(
                            "Interval from '{}' to '{}' already exists",
                            from_station,
                            to_station
                        ));
                    }
                    intervals.insert(interval_id, new_interval);
                }
            }
            if let Some(from_station_obj) = stations.get_mut(&from_station_id) {
                from_station_obj.intervals.insert(interval_id);
            }
            if let Some(to_station_obj) = stations.get_mut(&to_station_id) {
                to_station_obj.intervals.insert(interval_id);
            }
        }
        for (train_name, train_helper) in helper.trains {
            let train_id = hash_id(&train_name);
            let label_size = train_helper.label_size;
            let mut schedule = Vec::with_capacity(train_helper.schedule.len());
            // let mut schedule_index: MultiMap<StationID, usize> = MultiMap::new();
            for schedule_entry in train_helper.schedule.into_iter() {
                let station_id = hash_id(&schedule_entry.station);
                // schedule_index.insert(station_id, entry_idx);
                if let Some(station) = stations.get_mut(&station_id) {
                    station.trains.insert(train_id);
                }
                schedule.push(ScheduleEntry {
                    arrival: schedule_entry.arrival,
                    departure: schedule_entry.departure,
                    station: station_id,
                    // actions: schedule_entry.actions.unwrap_or_default(),
                });
            }
            trains.insert(
                train_id,
                Train {
                    name: train_name,
                    label_size,
                    schedule,
                    // schedule_index,
                },
            );
        }
        Ok(Network {
            stations,
            trains,
            intervals,
        })
    }
}

pub struct Station {
    // pub milestones: Option<HashMap<String, IntervalLength>>,
    // pub tracks: u16,
    // pub name: String,
    // those fields are completed afterwards
    pub intervals: HashSet<IntervalID>,
    pub trains: HashSet<TrainID>,
    pub label_size: (GraphLength, GraphLength),
}

#[derive(Deserialize)]
struct StationHelper {
    label_size: (GraphLength, GraphLength),
    // milestones: Option<HashMap<String, IntervalLength>>,
    // tracks: Option<u16>,
}

pub struct Train {
    pub name: String,
    pub label_size: (GraphLength, GraphLength),
    pub schedule: Vec<ScheduleEntry>,
    // pub schedule_index: MultiMap<StationID, usize>,
}

#[derive(Deserialize)]
struct TrainHelper {
    label_size: (GraphLength, GraphLength),
    schedule: Vec<ScheduleEntryHelper>,
}

pub struct ScheduleEntry {
    pub arrival: Time,
    pub departure: Time,
    pub station: StationID,
    // pub actions: HashSet<TrainAction>,
}

#[derive(Deserialize)]
struct ScheduleEntryHelper {
    arrival: Time,
    departure: Time,
    station: String,
    // actions: Option<HashSet<TrainAction>>,
}

#[derive(Clone)]
pub struct Interval {
    // pub name: Option<String>,
    pub length: IntervalLength,
}

#[derive(Deserialize)]
struct IntervalHelper {
    // name: Option<String>,
    length: IntervalLength,
    bidirectional: Option<bool>,
}

#[derive(Deserialize)]
#[serde(try_from = "NetworkConfigHelper")]
pub struct NetworkConfig {
    pub stations_to_draw: Vec<StationID>,
    pub start_time: Time,
    pub end_time: Time,
    pub unit_length: GraphLength,
    pub position_axis_scale_mode: ScaleMode,
    // pub time_axis_scale_mode: ScaleMode,
    pub position_axis_scale: f64,
    pub time_axis_scale: f64,
    pub label_angle: f64,
    pub line_stack_space: GraphLength,
}

#[derive(Deserialize)]
struct NetworkConfigHelper {
    stations_to_draw: Vec<String>,
    start_time: Time,
    end_time: Time,
    unit_length: GraphLength,
    position_axis_scale_mode: ScaleMode,
    // time_axis_scale_mode: ScaleMode,
    position_axis_scale: f64,
    time_axis_scale: f64,
    label_angle: f64,
    line_stack_space: GraphLength,
}

impl TryFrom<NetworkConfigHelper> for NetworkConfig {
    type Error = anyhow::Error;
    fn try_from(helper: NetworkConfigHelper) -> Result<Self, Self::Error> {
        if helper.stations_to_draw.is_empty() {
            return Err(anyhow::anyhow!(
                "You must specify at least one station to draw"
            ));
        }

        let stations_to_draw: Vec<StationID> = helper
            .stations_to_draw
            .iter()
            .map(|station_name| hash_id(station_name))
            .collect();

        for (window_idx, station_window) in helper.stations_to_draw.windows(3).enumerate() {
            let [_, current_station_name, next_station_name] = station_window else {
                continue;
            };
            let previous_station_id = stations_to_draw[window_idx];
            let current_station_id = stations_to_draw[window_idx + 1];
            let next_station_id = stations_to_draw[window_idx + 2];
            if current_station_id == next_station_id {
                return Err(anyhow::anyhow!(
                    "Two consecutive stations cannot be the same: '{}'",
                    current_station_name
                ));
            }
            if previous_station_id == next_station_id {
                return Err(anyhow::anyhow!(
                    "The station '{}' cannot be both the beginning of the previous interval and the end of the next one",
                    next_station_name
                ));
            }
        }

        if helper.start_time.seconds() > helper.end_time.seconds() {
            return Err(anyhow::anyhow!(
                "The beginning time cannot be after the end time"
            ));
        }

        Ok(NetworkConfig {
            stations_to_draw,
            start_time: helper.start_time,
            end_time: helper.end_time,
            unit_length: helper.unit_length,
            position_axis_scale_mode: helper.position_axis_scale_mode,
            line_stack_space: helper.line_stack_space,
            // time_axis_scale_mode: helper.time_axis_scale_mode,
            position_axis_scale: helper.position_axis_scale,
            time_axis_scale: helper.time_axis_scale,
            label_angle: helper.label_angle,
        })
    }
}
