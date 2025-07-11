use crate::coillision::*;
use crate::input::*;
use crate::types::*;
use anyhow::{Result, anyhow};
use multimap::MultiMap;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Serialize)]
struct OutputTrain {
    edges: Vec<Vec<Node>>,
    // TODO colors
}

#[derive(Serialize)]
pub struct Output {
    collision_manager: CollisionManager,
    trains: Vec<OutputTrain>,
    graph_intervals: Vec<GraphLength>,
}

impl Output {
    fn make_station_draw_info(
        stations_to_draw: &[u64],
        stations: &HashMap<StationID, Station>,
        intervals: &HashMap<IntervalID, Interval>,
        scale_mode: ScaleMode,
        unit_length: GraphLength,
        label_beg: GraphLength,
        collision_manager: &mut CollisionManager,
    ) -> Result<(
        Vec<(StationID, GraphLength)>,
        MultiMap<StationID, usize>,
        HashSet<TrainID>,
        Vec<GraphLength>,
    )> {
        if stations_to_draw.is_empty() {
            return Err(anyhow!("No stations to draw"));
        }

        // check if all stations to draw exist
        for &station_id in stations_to_draw {
            if !stations.contains_key(&station_id) {
                return Err(anyhow!("Station {} not found", station_id));
            }
        }

        let trains: HashSet<TrainID> = stations_to_draw
            .iter()
            .filter_map(|id| stations.get(id))
            .flat_map(|station| &station.trains)
            .copied()
            .collect();

        let mut station_draw_info = Vec::with_capacity(stations_to_draw.len());
        let mut station_indices = MultiMap::with_capacity(stations_to_draw.len());
        let mut graph_intervals = Vec::with_capacity(stations_to_draw.len().saturating_sub(1));
        let mut position: GraphLength = 0.0.into();

        // process the first station
        let beg = stations_to_draw[0];
        station_draw_info.push((beg, position));
        station_indices.insert(beg, 0);
        // handle the first station label
        let (width, height) = stations.get(&beg).unwrap().label_size;
        collision_manager.add_collision(vec![
            Node(label_beg - width - 3.0.into(), position - height * 0.5),
            Node(label_beg - 3.0.into(), position - height * 0.5),
            Node(label_beg - 3.0.into(), position + height * 0.5),
            Node(label_beg - width - 3.0.into(), position + height * 0.5),
        ]);

        for (window_idx, win) in stations_to_draw.windows(2).enumerate() {
            let [beg, end] = win else {
                continue;
            };
            if *beg == *end {
                return Err(anyhow!("Consecutive stations cannot be the same"));
            }

            let interval_length = match (
                intervals.get(&(*beg, *end)).map(|it| it.length),
                intervals.get(&(*end, *beg)).map(|it| it.length),
            ) {
                (Some(len1), Some(len2)) => {
                    IntervalLength::new((len1.meters() + len2.meters()) / 2)
                        .to_graph_length(unit_length, scale_mode)
                }
                (Some(len), None) | (None, Some(len)) => {
                    len.to_graph_length(unit_length, scale_mode)
                }
                (None, None) => {
                    return Err(anyhow!(
                        "No interval found between stations {} and {}",
                        beg,
                        end
                    ));
                }
            };

            graph_intervals.push(interval_length);
            position += interval_length;
            station_draw_info.push((*end, position));
            station_indices.insert(*end, window_idx + 1);

            let (width, height) = stations.get(end).unwrap().label_size;
            // insert station label. nodes are in absolute coordinates
            collision_manager.add_collision(vec![
                Node(label_beg - width - 3.0.into(), position - height * 0.5),
                Node(label_beg - 3.0.into(), position - height * 0.5),
                Node(label_beg - 3.0.into(), position + height * 0.5),
                Node(label_beg - width - 3.0.into(), position + height * 0.5),
            ]);
        }

        Ok((station_draw_info, station_indices, trains, graph_intervals))
    }

    pub fn new(network: Network, config: &NetworkConfig) -> Result<Self> {
        let mut collision_manager = CollisionManager::new(config.unit_length);
        let (stations_draw_info, station_indices, trains_draw_info, graph_intervals) =
            Self::make_station_draw_info(
                &config.stations_to_draw,
                &network.stations,
                &network.intervals,
                config.position_axis_scale_mode,
                config.unit_length * config.position_axis_scale,
                config.beg.to_graph_length(
                    config.unit_length * config.time_axis_scale,
                    config.time_axis_scale_mode,
                ),
                &mut collision_manager,
            )?;

        collision_manager.update_x_min(GraphLength::from(
            config
                .beg
                .to_graph_length(
                    config.unit_length * config.time_axis_scale,
                    config.time_axis_scale_mode,
                )
                .value(),
        ));
        collision_manager.update_x_max(GraphLength::from(
            config
                .end
                .to_graph_length(
                    config.unit_length * config.time_axis_scale,
                    config.time_axis_scale_mode,
                )
                .value(),
        ));
        collision_manager.update_y_min(GraphLength::from(
            stations_draw_info.first().map_or(0.0, |(_, y)| y.value()),
        ));
        collision_manager.update_y_max(GraphLength::from(
            stations_draw_info.last().map_or(0.0, |(_, y)| y.value()),
        ));
        let mut trains: Vec<OutputTrain> = Vec::with_capacity(trains_draw_info.len());

        for train in trains_draw_info {
            trains.push(
                Self::make_train(
                    &stations_draw_info,
                    &station_indices,
                    network.trains.get(&train).unwrap(),
                    config.unit_length * config.time_axis_scale,
                    config.time_axis_scale_mode,
                    &mut collision_manager,
                )
                .unwrap(),
            )
        }

        Ok(Self {
            collision_manager,
            trains,
            graph_intervals,
        })
    }
    fn make_train(
        stations_draw_info: &[(StationID, GraphLength)],
        station_indices: &MultiMap<StationID, usize>,
        train: &Train,
        unit_length: GraphLength,
        scale_mode: ScaleMode,
        collision_manager: &mut CollisionManager,
    ) -> Result<OutputTrain> {
        let schedule = &train.schedule;
        let mut edges: Vec<Vec<Node>> = Vec::new();
        let mut local_edges: Vec<(Vec<Node>, usize)> = Vec::new();
        for entry in schedule {
            let Some(graph_idxs) = station_indices.get_vec(&entry.station) else {
                if local_edges.is_empty() {
                    continue;
                }
                edges.extend(
                    std::mem::take(&mut local_edges)
                        .into_iter()
                        .map(|(nodes, _)| nodes),
                );
                continue;
            };
            let mut remaining: Vec<(Vec<Node>, usize)> = Vec::new();
            for graph_idx in graph_idxs {
                if let Some(pos) = local_edges
                    .iter()
                    .position(|(_, last_graph_idx)| graph_idx.abs_diff(*last_graph_idx) == 1)
                {
                    let (mut matched_edge, _) = local_edges.remove(pos);
                    // add nodes to remaining
                    matched_edge.push(Node(
                        entry.arrival.to_graph_length(unit_length, scale_mode),
                        stations_draw_info[*graph_idx].1,
                    ));
                    if entry.arrival != entry.departure {
                        matched_edge.push(Node(
                            entry.departure.to_graph_length(unit_length, scale_mode),
                            stations_draw_info[*graph_idx].1,
                        ));
                    }
                    remaining.push((matched_edge, *graph_idx));
                } else {
                    // start a new edge, if not found
                    let mut new_edge = vec![Node(
                        entry.arrival.to_graph_length(unit_length, scale_mode),
                        stations_draw_info[*graph_idx].1,
                    )];
                    if entry.arrival != entry.departure {
                        new_edge.push(Node(
                            entry.departure.to_graph_length(unit_length, scale_mode),
                            stations_draw_info[*graph_idx].1,
                        ));
                    }
                    remaining.push((new_edge, *graph_idx));
                }
            }
            if !local_edges.is_empty() {
                edges.extend(
                    std::mem::take(&mut local_edges)
                        .into_iter()
                        .map(|(nodes, _)| nodes),
                );
            }
            // update local_edges with remaining
            local_edges = remaining;
        }
        // handle the remaining local edges
        edges.extend(local_edges.into_iter().map(|(nodes, _)| nodes));

        // iterate over all edges and add collision nodes
        let (label_width, label_height) = train.label_size;
        for edge in &edges {
            // precondition: an edge will have at least one node
            let first_node = edge.first().unwrap();
            let last_node = edge.last().unwrap();
            collision_manager.resolve_collisions(
                &rotate(
                    vec![
                        Node(first_node.0 - label_width, first_node.1 - label_height),
                        Node(first_node.0, first_node.1 - label_height),
                        first_node.clone(),
                        Node(first_node.0 - label_width, first_node.1),
                    ],
                    first_node.clone(),
                    20.0f64.to_radians(),
                ),
                90.0f64.to_radians(),
            )?;
        }

        Ok(OutputTrain { edges })
    }
}
