use crate::types::*;
use anyhow::{Result, anyhow};

pub fn intersection(n0: Node, n1: Node, v: GraphLength) -> Result<Node> {
    let (x0, y0) = (n0.0.value(), n0.1.value());
    let (x1, y1) = (n1.0.value(), n1.1.value());
    let vx = v.value();

    // If the segment is vertical
    if (x1 - x0).abs() < f64::EPSILON {
        if (vx - x0).abs() < f64::EPSILON {
            // Line is vertical and coincides with x = v — return one of the endpoints
            return Ok(n0);
        } else {
            return Err(anyhow!("Segment is vertical and does not intersect x = v"));
        }
    }

    // Compute slope and intercept of the line
    let slope = (y1 - y0) / (x1 - x0);
    let intercept = y0 - slope * x0;

    // y = slope * x + intercept
    let y = slope * vx + intercept;

    Ok(Node(vx.into(), y.into()))
}
