//! All of the geo-specific functions for NED TZ lookups.

use geo::{Coord, Geometry, Rect};
use serde::{Deserialize, Serialize};

use crate::base::types::Float;

use super::shared::IsTimezone;

// Statics.

/// The address of the GeoJSON file.
pub static GEOJSON_ADDRESS: &str = "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_10m_time_zones.geojson";
/// The name of the timezone bincode file.
pub static TIMEZONE_BINCODE_DESTINATION_NAME: &str = "ne_10m_time_zones.bincode";
/// The name of the cache bincode file.
pub static CACHE_BINCODE_DESTINATION_NAME: &str = "ne_time_zone_cache.bincode";

// Types.

/// A representation of the [Natural Earth Data](https://www.naturalearthdata.com/)
/// [geojson](https://github.com/nvkelso/natural-earth-vector/blob/master/geojson/ne_10m_time_zones.geojson)
/// [`geojson::Feature`]s.
#[derive(Debug, Serialize, Deserialize)]
pub struct NedTimezone {
    /// The index of the [`NedTimezone`] in the global static cache.
    /// 
    /// This is is not stable across builds or new data sets.  It is merely unique during a single build.
    pub id: usize,
    /// The `identifier` of the [`NedTimezone`] (e.g., `America/Los_Angeles`).
    ///
    /// Essentially, it is the IANA TZ identifier.
    pub identifier: Option<String>,

    /// The `description` of the [`NedTimezone`] (e.g., the countries affected).
    pub description: String,
    /// The `dst_description` of the [`NedTimezone`] (i.e., daylight savings time information).
    pub dst_description: Option<String>,

    /// The `offset` of the [`NedTimezone`] (e.g., `UTC-8:00`).
    pub offset: String,

    /// The `zone_num` of the [`NedTimezone`] (e.g., `-8.0`).
    pub zone: f32,
    /// The `raw_offset` of the [`NedTimezone`] (e.g., `-28800`).
    pub raw_offset: i32,

    /// The bounding box of the [`NedTimezone`].
    pub bbox: Rect<Float>,
    /// The geometry of the [`NedTimezone`].
    pub geometry: Geometry<Float>,
}

impl PartialEq for NedTimezone {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl From<(usize, geojson::Feature)> for NedTimezone {
    fn from(value: (usize, geojson::Feature)) -> NedTimezone {
        let id = value.0;
        let bbox = value.1.bbox.as_ref().unwrap();
        let properties = value.1.properties.as_ref().unwrap();
        let geometry = value.1.geometry.as_ref().unwrap();

        let dst_places = properties.get("dst_places").unwrap().as_str().map(ToOwned::to_owned);
        let places = properties.get("places").unwrap().as_str().unwrap().to_owned();

        let time_zone = properties.get("time_zone").unwrap().as_str().unwrap().to_owned();
        let tz_name1st = properties.get("tz_name1st").unwrap().as_str().map(ToOwned::to_owned);
        let zone = properties.get("zone").unwrap().as_f64().unwrap() as f32;

        let bbox = Rect::<Float>::new(Coord { x: bbox[0] as Float, y: bbox[1] as Float }, Coord { x: bbox[2] as Float, y: bbox[3] as Float });

        let geometry: Geometry<Float> = geometry.value.clone().try_into().unwrap();

        let raw_offset = (zone * 3600.0).round() as i32;

        NedTimezone {
            id,
            dst_description: dst_places,
            description: places,
            offset: time_zone,
            identifier: tz_name1st,
            zone,
            raw_offset,
            bbox,
            geometry,
        }
    }
}

impl IsTimezone for NedTimezone {
    fn id(&self) -> usize {
        self.id
    }

    fn identifier(&self) -> &str {
        self.identifier.as_deref().unwrap_or("")
    }

    fn geometry(&self) -> &Geometry<Float> {
        &self.geometry
    }
}