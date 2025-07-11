use derive_more::{Add, AddAssign, Sub};
use serde::{Deserialize, Serialize};
use std::ops;

pub type StationID = u64;
pub type TrainID = u64;
pub type IntervalID = (StationID, StationID);

pub trait IntervalIDExt {
    fn reverse(&self) -> Self;
}

impl IntervalIDExt for IntervalID {
    fn reverse(&self) -> Self {
        (self.1, self.0)
    }
}

/// Time representation in seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Add, Sub, Deserialize)]
pub struct Time(u32);

impl Time {
    pub fn new(seconds: u32) -> Self {
        Time(seconds)
    }
    pub fn seconds(&self) -> u32 {
        self.0
    }
    pub fn second(&self) -> u32 {
        self.0 % 60
    }
    pub fn minutes(&self) -> u32 {
        self.0 / 60
    }
    pub fn minute(&self) -> u32 {
        (self.0 / 60) % 60
    }
    pub fn hours(&self) -> u32 {
        self.0 / 3600
    }
    pub fn hour(&self) -> u32 {
        (self.0 / 3600) % 24
    }
    pub fn to_graph_length(&self, unit_length: GraphLength, scale_mode: ScaleMode) -> GraphLength {
        let hours = self.0 as f64 / 3600.0;
        unit_length * hours
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Add, Sub, Deserialize)]
pub struct IntervalLength(u32);

impl IntervalLength {
    pub fn new(meters: u32) -> Self {
        IntervalLength(meters)
    }
    pub fn meters(&self) -> u32 {
        self.0
    }
    pub fn kilometers(&self) -> f64 {
        self.0 as f64 / 1000.0
    }
    pub fn to_graph_length(&self, unit_length: GraphLength, scale_mode: ScaleMode) -> GraphLength {
        let length = match scale_mode {
            ScaleMode::Linear => self.kilometers(),
            ScaleMode::Logarithmic => self.kilometers().ln().max(1.0),
            ScaleMode::Uniform => 1.0,
        };
        unit_length * length
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Add, Sub, Deserialize, Serialize, AddAssign)]
pub struct GraphLength(f64);

impl GraphLength {
    pub fn value(&self) -> f64 {
        self.0
    }
}

impl From<GraphLength> for f64 {
    fn from(value: GraphLength) -> Self {
        value.0
    }
}

impl From<f64> for GraphLength {
    fn from(value: f64) -> Self {
        GraphLength(value)
    }
}

impl ops::Mul<GraphLength> for f64 {
    type Output = GraphLength;

    fn mul(self, rhs: GraphLength) -> Self::Output {
        GraphLength(self * rhs.0)
    }
}

impl ops::Mul<f64> for GraphLength {
    type Output = GraphLength;

    fn mul(self, rhs: f64) -> Self::Output {
        GraphLength(self.0 * rhs)
    }
}

impl ops::Div<GraphLength> for GraphLength {
    type Output = f64;

    fn div(self, rhs: GraphLength) -> Self::Output {
        self.0 / rhs.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ScaleMode {
    Linear,
    Logarithmic,
    Uniform,
}

#[derive(Debug, Serialize, Clone, Copy, Deserialize)]
pub struct Node(pub GraphLength, pub GraphLength);

impl Node {
    /// enters another node, outputs the slope
    pub fn slope(&self, other: &Node) -> f64 {
        if self.0 == other.0 {
            return 0.0; // vertical line
        }
        (other.1 - self.1) / (other.0 - self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Hash)]
pub enum TrainAction {
    Compose,
    Decompose,
    Outbound,
    Inbound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Direction {
    Bidirectional,
    Forward,
    Backward,
}
