//! Shared functionality for geo operations.

// Statics.

use geo::{Geometry, SimplifyVw};

use crate::base::types::Float;

#[cfg(not(feature = "extrasimplified"))]
const SIMPLIFICATION_EPSILON: Float = 0.0001;
#[cfg(feature = "extrasimplified")]
const SIMPLIFICATION_EPSILON: Float = 0.01;

/// Simplifies a [`Geometry`] using the [Visvalingam-Whyatt algorithm](https://bost.ocks.org/mike/simplify/).
/// 
/// For geometries that cannot be simplified, the original geometry is returned.
pub fn simplify_geometry(geometry: Geometry<Float>) -> Geometry<Float> {
    #[cfg(not(feature = "unsimplified"))]
    let geometry = match geometry {
        Geometry::Polygon(polygon) => {
            let simplified = polygon.simplify_vw(&SIMPLIFICATION_EPSILON);
            Geometry::Polygon(simplified)
        }
        Geometry::MultiPolygon(multi_polygon) => {
            let simplified = multi_polygon.simplify_vw(&SIMPLIFICATION_EPSILON);
            Geometry::MultiPolygon(simplified)
        }
        Geometry::LineString(line_string) => {
            let simplified = line_string.simplify_vw(&SIMPLIFICATION_EPSILON);
            Geometry::LineString(simplified)
        }
        Geometry::MultiLineString(multi_line_string) => {
            let simplified = multi_line_string.simplify_vw(&SIMPLIFICATION_EPSILON);
            Geometry::MultiLineString(simplified)
        }
        g => g,
    };

    geometry
}