use crate::input::*;
use crate::types::*;
use anyhow::{Result, anyhow};
use multimap::MultiMap;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Serialize)]
struct CollisionManager {
    #[serde(skip)]
    indices: HashMap<(u32, u32), Vec<usize>>,
    collisions: Vec<Vec<Node>>,
    #[serde(skip)]
    unit_size: GraphLength,
    x_min: GraphLength,
    x_max: GraphLength,
    y_min: GraphLength,
    y_max: GraphLength,
}

impl CollisionManager {
    fn new(unit_size: GraphLength) -> Self {
        Self {
            indices: HashMap::new(),
            collisions: Vec::new(),
            unit_size,
            x_min: GraphLength::from(f64::INFINITY),
            x_max: GraphLength::from(f64::NEG_INFINITY),
            y_min: GraphLength::from(f64::INFINITY),
            y_max: GraphLength::from(f64::NEG_INFINITY),
        }
    }
    fn update_bounds(&mut self, bounds: (f64, f64, f64, f64)) {
        let (x_min, x_max, y_min, y_max) = bounds;

        // 更新全局边界
        self.x_min = GraphLength::from(self.x_min.value().min(x_min));
        self.x_max = GraphLength::from(self.x_max.value().max(x_max));
        self.y_min = GraphLength::from(self.y_min.value().min(y_min));
        self.y_max = GraphLength::from(self.y_max.value().max(y_max));
    }
    fn add(&mut self, nodes: &[Node]) {
        if nodes.is_empty() {
            return;
        }

        // 使用迭代器一次性计算边界
        let bounds = nodes.iter().fold(
            (
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
            ),
            |(x_min, x_max, y_min, y_max), node| {
                let (x, y) = (node.0.value(), node.1.value());
                (x_min.min(x), x_max.max(x), y_min.min(y), y_max.max(y))
            },
        );

        self.update_bounds(bounds);

        // 计算网格索引
        let unit_value = self.unit_size.value();
        let indices = (
            (bounds.0 / unit_value).floor() as u32,
            (bounds.1 / unit_value).ceil() as u32,
            (bounds.2 / unit_value).floor() as u32,
            (bounds.3 / unit_value).ceil() as u32,
        );

        let collision_index = self.collisions.len();

        // 批量更新网格索引
        for x in indices.0..=indices.1 {
            for y in indices.2..=indices.3 {
                self.indices
                    .entry((x, y))
                    .or_insert_with(Vec::new)
                    .push(collision_index);
            }
        }

        self.collisions.push(nodes.to_vec());
    }
}

#[derive(Serialize)]
struct OutputTrain {
    edges: Vec<Vec<Node>>,
    // TODO colors
}

#[derive(Serialize)]
pub struct Output {
    collision: CollisionManager,
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
    ) -> Result<(
        Vec<(StationID, GraphLength)>,
        MultiMap<StationID, usize>,
        HashSet<TrainID>,
        Vec<GraphLength>, // 新增：相对间隔
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

            // 存储相对间隔
            graph_intervals.push(interval_length);

            // 更新绝对位置
            position += interval_length;
            station_draw_info.push((*end, position));
            station_indices.insert(*end, window_idx + 1);
        }

        Ok((station_draw_info, station_indices, trains, graph_intervals))
    }

    pub fn new(network: &Network, config: &NetworkConfig) -> Result<Self> {
        let (stations_draw_info, station_indices, trains_draw_info, graph_intervals) =
            Self::make_station_draw_info(
                &config.stations_to_draw,
                &network.stations,
                &network.intervals,
                config.position_axis_scale_mode,
                config.unit_length * config.position_axis_scale,
            )?;
        let mut collision = CollisionManager::new(config.unit_length * 2.0);
        collision.update_bounds((
            config
                .beg
                .to_graph_length(
                    config.unit_length * config.time_axis_scale,
                    config.time_axis_scale_mode,
                )
                .value(),
            config
                .end
                .to_graph_length(
                    config.unit_length * config.time_axis_scale,
                    config.time_axis_scale_mode,
                )
                .value(),
            stations_draw_info.first().map_or(0.0, |(_, y)| y.value()),
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
                )
                .unwrap(),
            )
        }

        Ok(Self {
            collision,
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
        Ok(OutputTrain { edges })
    }
}
