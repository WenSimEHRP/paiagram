use crate::collision::*;
use crate::input::*;
use crate::types::*;
use anyhow::{Result, anyhow};
use multimap::MultiMap;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

enum LabelDirection {
    Up,
    Down,
    // Left,
    // Right,
}

enum LabelPosition {
    Beg(LabelDirection),
    End(LabelDirection),
}

#[derive(Serialize)]
struct OutputTrain {
    edges: Vec<OutputEdge>,
    name: String,
    // TODO colors
}

#[derive(Serialize)]
struct OutputEdge {
    edges: Vec<Node>,
    labels: Option<OutputLabel>,
}

#[derive(Serialize)]
struct OutputLabel {
    start: (Node, f64),
    end: (Node, f64),
}

#[derive(Serialize)]
pub struct Output {
    collision_manager: CollisionManager,
    trains: Vec<OutputTrain>,
    graph_intervals: Vec<GraphLength>,
    #[serde(skip)]
    stations_draw_info: Vec<(StationID, GraphLength)>,
    #[serde(skip)]
    station_indices: MultiMap<StationID, usize>,
    #[serde(skip)]
    config: NetworkConfig,
}

impl Output {
    pub fn new(config: NetworkConfig) -> Self {
        let collision_manager = CollisionManager::new(config.unit_length);

        Self {
            collision_manager,
            trains: Vec::new(),
            graph_intervals: Vec::new(),
            stations_draw_info: Vec::new(),
            station_indices: MultiMap::new(),
            config,
        }
    }

    pub fn populate(&mut self, network: Network) -> Result<()> {
        let train_ids_to_draw =
            self.make_station_draw_info(&network.stations, &network.intervals)?;

        let time_unit_length = self.config.unit_length * self.config.time_axis_scale;

        self.collision_manager.update_x_min(GraphLength::from(
            self.config.beg.to_graph_length(time_unit_length).value(),
        ));
        self.collision_manager.update_x_max(GraphLength::from(
            self.config.end.to_graph_length(time_unit_length).value(),
        ));
        self.collision_manager.update_y_min(GraphLength::from(
            self.stations_draw_info
                .first()
                .map_or(0.0, |(_, y)| y.value()),
        ));
        self.collision_manager.update_y_max(GraphLength::from(
            self.stations_draw_info
                .last()
                .map_or(0.0, |(_, y)| y.value()),
        ));

        self.trains = Vec::with_capacity(train_ids_to_draw.len());
        for train_id in train_ids_to_draw {
            let output_train = self.make_train(network.trains.get(&train_id).unwrap())?;
            self.trains.push(output_train);
        }

        Ok(())
    }

    fn make_station_draw_info(
        &mut self,
        stations: &HashMap<StationID, Station>,
        intervals: &HashMap<IntervalID, Interval>,
    ) -> Result<HashSet<TrainID>> {
        if self.config.stations_to_draw.is_empty() {
            return Err(anyhow!("No stations to draw"));
        }

        // check if all stations to draw exist
        for &station_id in &self.config.stations_to_draw {
            if !stations.contains_key(&station_id) {
                return Err(anyhow!("Station {} not found", station_id));
            }
        }

        let train_ids: HashSet<TrainID> = self
            .config
            .stations_to_draw
            .iter()
            .filter_map(|id| stations.get(id))
            .flat_map(|station| &station.trains)
            .copied()
            .collect();

        self.stations_draw_info = Vec::with_capacity(self.config.stations_to_draw.len());
        self.station_indices = MultiMap::with_capacity(self.config.stations_to_draw.len());
        self.graph_intervals =
            Vec::with_capacity(self.config.stations_to_draw.len().saturating_sub(1));
        let mut position: GraphLength = 0.0.into();

        let unit_length = self.config.unit_length * self.config.position_axis_scale;
        let label_start = self
            .config
            .beg
            .to_graph_length(self.config.unit_length * self.config.time_axis_scale);

        // process the first station
        let first_station = self.config.stations_to_draw[0];
        self.stations_draw_info.push((first_station, position));
        self.station_indices.insert(first_station, 0);
        // handle the first station label
        let (width, height) = stations.get(&first_station).unwrap().label_size;
        self.collision_manager.add_collision(vec![
            Node(label_start - width - 3.0.into(), position - height * 0.5),
            Node(label_start - 3.0.into(), position - height * 0.5),
            Node(label_start - 3.0.into(), position + height * 0.5),
            Node(label_start - width - 3.0.into(), position + height * 0.5),
        ])?;

        for (window_idx, window) in self.config.stations_to_draw.windows(2).enumerate() {
            let [start_station, end_station] = window else {
                continue;
            };
            if *start_station == *end_station {
                return Err(anyhow!("Consecutive stations cannot be the same"));
            }

            let interval_length = match (
                intervals
                    .get(&(*start_station, *end_station))
                    .map(|it| it.length),
                intervals
                    .get(&(*end_station, *start_station))
                    .map(|it| it.length),
            ) {
                (Some(len1), Some(len2)) => {
                    IntervalLength::new((len1.meters() + len2.meters()) / 2)
                        .to_graph_length(unit_length, self.config.position_axis_scale_mode)
                }
                (Some(len), None) | (None, Some(len)) => {
                    len.to_graph_length(unit_length, self.config.position_axis_scale_mode)
                }
                (None, None) => {
                    return Err(anyhow!(
                        "No interval found between stations {} and {}",
                        start_station,
                        end_station
                    ));
                }
            };

            self.graph_intervals.push(interval_length);
            position += interval_length;
            self.stations_draw_info.push((*end_station, position));
            self.station_indices.insert(*end_station, window_idx + 1);

            let (width, height) = stations.get(end_station).unwrap().label_size;
            // insert station label. nodes are in absolute coordinates
            self.collision_manager.add_collision(vec![
                Node(label_start - width - 3.0.into(), position - height * 0.5),
                Node(label_start - 3.0.into(), position - height * 0.5),
                Node(label_start - 3.0.into(), position + height * 0.5),
                Node(label_start - width - 3.0.into(), position + height * 0.5),
            ])?;
        }

        Ok(train_ids)
    }

    fn make_train(&mut self, train: &Train) -> Result<OutputTrain> {
        let schedule = &train.schedule;
        let mut output_edges: Vec<OutputEdge> = Vec::new();
        let mut local_edges: Vec<(Vec<Node>, usize)> = Vec::new();
        let unit_length = self.config.unit_length * self.config.time_axis_scale;

        for schedule_entry in schedule {
            let Some(graph_indices) = self.station_indices.get_vec(&schedule_entry.station) else {
                if local_edges.is_empty() {
                    continue;
                }
                // Convert local edges to OutputEdge and add to output_edges
                output_edges.extend(std::mem::take(&mut local_edges).into_iter().map(
                    |(edge_nodes, _)| OutputEdge {
                        edges: edge_nodes,
                        labels: None,
                    },
                ));
                continue;
            };
            let mut remaining_edges: Vec<(Vec<Node>, usize)> = Vec::new();
            for &graph_index in graph_indices {
                if let Some(edge_position) = local_edges
                    .iter()
                    .position(|(_, last_graph_index)| graph_index.abs_diff(*last_graph_index) == 1)
                {
                    let (mut matched_edge_nodes, _) = local_edges.remove(edge_position);
                    // add nodes to remaining
                    matched_edge_nodes.push(Node(
                        schedule_entry.arrival.to_graph_length(unit_length),
                        self.stations_draw_info[graph_index].1,
                    ));
                    if schedule_entry.arrival != schedule_entry.departure {
                        matched_edge_nodes.push(Node(
                            schedule_entry.departure.to_graph_length(unit_length),
                            self.stations_draw_info[graph_index].1,
                        ));
                    }
                    remaining_edges.push((matched_edge_nodes, graph_index));
                } else {
                    // start a new edge, if not found
                    let mut new_edge_nodes = vec![Node(
                        schedule_entry.arrival.to_graph_length(unit_length),
                        self.stations_draw_info[graph_index].1,
                    )];
                    if schedule_entry.arrival != schedule_entry.departure {
                        new_edge_nodes.push(Node(
                            schedule_entry.departure.to_graph_length(unit_length),
                            self.stations_draw_info[graph_index].1,
                        ));
                    }
                    remaining_edges.push((new_edge_nodes, graph_index));
                }
            }
            if !local_edges.is_empty() {
                // Convert local edges to OutputEdge and add to output_edges
                output_edges.extend(std::mem::take(&mut local_edges).into_iter().map(
                    |(edge_nodes, _)| OutputEdge {
                        edges: edge_nodes,
                        labels: None,
                    },
                ));
            }
            // update local_edges with remaining
            local_edges = remaining_edges;
        }
        // handle the remaining local edges
        output_edges.extend(local_edges.into_iter().map(|(edge_nodes, _)| OutputEdge {
            edges: edge_nodes,
            labels: None,
        }));

        // Filter out edges with less than 2 nodes before processing labels
        output_edges.retain(|output_edge| output_edge.edges.len() >= 2);

        // iterate over all edges and add collision nodes
        let (label_width, label_height) = train.label_size;
        for output_edge in &mut output_edges {
            let (start_label_node, end_label_node) =
                self.add_train_labels_to_edge(&mut output_edge.edges, label_width, label_height)?;
            output_edge.labels = Some(OutputLabel {
                start: start_label_node,
                end: end_label_node,
            })
        }

        Ok(OutputTrain {
            edges: output_edges,
            name: train.name.clone(),
        })
    }

    fn create_label_polygon(
        &self,
        anchor: Node,
        label_width: GraphLength,
        label_height: GraphLength,
        direction: &LabelPosition,
    ) -> (Vec<Node>, f64, f64) {
        match direction {
            LabelPosition::Beg(dir) => {
                let polygon = vec![
                    Node(anchor.0 - label_width, anchor.1 - label_height),
                    Node(anchor.0, anchor.1 - label_height),
                    anchor,
                    Node(anchor.0 - label_width, anchor.1),
                ];
                match dir {
                    // the up and downs are reversed for typst
                    LabelDirection::Up => (
                        rotate_polygon(polygon, anchor, -self.config.label_angle),
                        90.0f64.to_radians(),
                        -self.config.label_angle,
                    ),
                    _ => (
                        rotate_polygon(polygon, anchor, self.config.label_angle),
                        -90.0f64.to_radians(),
                        self.config.label_angle,
                    ),
                }
            }
            LabelPosition::End(dir) => {
                let polygon = vec![
                    Node(anchor.0, anchor.1 - label_height),
                    Node(anchor.0 + label_width, anchor.1 - label_height),
                    Node(anchor.0 + label_width, anchor.1),
                    anchor,
                ];
                match dir {
                    LabelDirection::Up => (
                        rotate_polygon(polygon, anchor, -self.config.label_angle),
                        -90.0f64.to_radians(),
                        -self.config.label_angle,
                    ),
                    _ => (
                        rotate_polygon(polygon, anchor, self.config.label_angle),
                        90.0f64.to_radians(),
                        self.config.label_angle,
                    ),
                }
            }
        }
    }

    fn add_train_labels_to_edge(
        &mut self,
        edge: &mut Vec<Node>,
        label_width: GraphLength,
        label_height: GraphLength,
    ) -> Result<((Node, f64), (Node, f64))> {
        let edge_start = *edge.first().unwrap();
        let edge_end = *edge.last().unwrap();

        let start_label_direction = if edge.len() > 2 {
            // check the first three nodes to determine general direction
            let (first, second, third) = (edge[0], edge[1], edge[2]);
            // check if the general trend is upwards
            // typst logic is reversed, so the directions are reversed
            if (second.1 > first.1) || (third.1 > second.1) {
                LabelPosition::Beg(LabelDirection::Down)
            } else {
                LabelPosition::Beg(LabelDirection::Up)
            }
        } else {
            let (first, last) = (edge[0], edge[1]);
            if first.1 < last.1 {
                LabelPosition::Beg(LabelDirection::Down)
            } else {
                LabelPosition::Beg(LabelDirection::Up)
            }
        };

        // Add label at the beginning of the edge
        let start_label_node = self.add_label_to_edge(
            edge,
            edge_start,
            label_width,
            label_height,
            &start_label_direction,
        )?;

        // Add label at the end of the edge (only if edge has more than one node)

        // Determine direction for end label (might be different from beginning)
        let end_label_direction = if edge.len() > 2 {
            // check the last three nodes to determine general direction
            let (last, second_last, third_last) = (
                edge[edge.len() - 1],
                edge[edge.len() - 2],
                edge[edge.len() - 3],
            );
            // check if the general trend is upwards
            if (last.1 < second_last.1) || (second_last.1 < third_last.1) {
                LabelPosition::End(LabelDirection::Up)
            } else {
                LabelPosition::End(LabelDirection::Down)
            }
        } else {
            let (first, last) = (edge[0], edge[1]);
            if last.1 < first.1 {
                LabelPosition::End(LabelDirection::Up)
            } else {
                LabelPosition::End(LabelDirection::Down)
            }
        };

        // Insert at the end
        let end_label_node = self.add_label_to_edge(
            edge,
            edge_end,
            label_width,
            label_height,
            &end_label_direction,
        )?;

        Ok((start_label_node, end_label_node))
    }

    fn add_label_to_edge(
        &mut self,
        edge: &mut Vec<Node>,
        anchor_point: Node,
        label_width: GraphLength,
        label_height: GraphLength,
        label_direction: &LabelPosition,
    ) -> Result<(Node, f64)> {
        let (polygon, movement_angle, label_angle) =
            self.create_label_polygon(anchor_point, label_width, label_height, label_direction);

        let (resolved_polygon, _) = self
            .collision_manager
            .resolve_collisions(polygon, movement_angle)?;

        // Insert label nodes based on direction
        match label_direction {
            LabelPosition::Beg(_) => {
                edge.insert(0, resolved_polygon[2]);
                edge.insert(0, resolved_polygon[3]);
            }
            LabelPosition::End(_) => {
                edge.push(resolved_polygon[3]);
                edge.push(resolved_polygon[2]);
            }
        }

        Ok((resolved_polygon[0], label_angle))
    }
}
