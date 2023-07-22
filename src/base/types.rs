//! All of the types used in the library.

use std::ops::Deref;

use geo::{Coord, Geometry, Rect};
use serde::{self, Deserialize, Serialize};

use super::geo::get_timezones;

// Result types.

/// A shortened version of [`anyhow::Result<T>`].
pub(crate) type Res<T> = anyhow::Result<T>;
/// A shortened version of [`anyhow::Result<()>`](anyhow::Result).
pub type Void = anyhow::Result<()>;
/// A shortened version of [`anyhow::Error`].
//pub(crate) type Err = anyhow::Error;

// Helper types.

pub(crate) type RoundInt = i16;
pub(crate) type RoundLngLat = (RoundInt, RoundInt);
//pub(crate) type LngLat = (f64, f64);

/// A collection of `id`s into the global time zone static cache.
pub(crate) type TimezoneIds = [RoundInt; 10];
/// A [`Timezone`] static reference.
pub type TimezoneRef = &'static Timezone;
/// A collection of [`Timezone`] static references.
pub type TimezoneRefs = Vec<TimezoneRef>;

// Geo Types.

/// A concrete collection of [`Timezone`]s.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ConcreteTimezones(Vec<Timezone>);

impl Deref for ConcreteTimezones {
    type Target = Vec<Timezone>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<&geojson::FeatureCollection> for ConcreteTimezones {
    fn from(value: &geojson::FeatureCollection) -> ConcreteTimezones {
        ConcreteTimezones(value.features.iter().enumerate().map(Timezone::from).collect())
    }
}

impl IntoIterator for ConcreteTimezones {
    type IntoIter = std::vec::IntoIter<Timezone>;
    type Item = Timezone;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a ConcreteTimezones {
    type IntoIter = std::slice::Iter<'a, Timezone>;
    type Item = &'a Timezone;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

/// A representation of the [Natural Earth Data](https://www.naturalearthdata.com/)
/// [geojson](https://github.com/nvkelso/natural-earth-vector/blob/master/geojson/ne_10m_time_zones.geojson)
/// [`geojson::Feature`]s.
#[derive(Debug, Serialize, Deserialize)]
pub struct Timezone {
    /// The index of the [`Timezone`] in the global static cache.
    pub id: usize,
    /// The `objectid` of the [`Timezone`].
    pub objectid: u64,
    /// The `friendly_name` of the [`Timezone`] (e.g., `America/Los_Angeles`).
    pub friendly_name: Option<String>,

    /// The `description` of the [`Timezone`] (e.g., the countries affected).
    pub description: String,
    /// The `dst_description` of the [`Timezone`] (i.e., daylight savings time information).
    pub dst_description: Option<String>,

    /// The `offset_str` of the [`Timezone`] (e.g., `UTC-8:00`).
    pub offset_str: String,

    /// The `zone_num` of the [`Timezone`] (e.g., `-8`).
    pub zone_num: Option<i64>,
    /// The `zone_str` of the [`Timezone`] (e.g., `"-9.5"`).
    pub zone_str: String,
    /// The `raw_offset` of the [`Timezone`] (e.g., `-28800`).
    pub raw_offset: i64,

    /// The bounding box of the [`Timezone`].
    pub bbox: Rect,
    /// The geometry of the [`Timezone`].
    pub geometry: Geometry,
}

impl PartialEq for Timezone {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl From<(usize, &geojson::Feature)> for Timezone {
    fn from(value: (usize, &geojson::Feature)) -> Timezone {
        let id = value.0;
        let bbox = value.1.bbox.as_ref().unwrap();
        let properties = value.1.properties.as_ref().unwrap();
        let geometry = value.1.geometry.as_ref().unwrap();

        let objectid = properties.get("objectid").unwrap().as_u64().unwrap();
        let dst_places = properties.get("dst_places").unwrap().as_str().map(ToOwned::to_owned);
        let name = properties.get("name").unwrap().as_str().unwrap().to_owned();
        let places = properties.get("places").unwrap().as_str().unwrap().to_owned();

        let time_zone = properties.get("time_zone").unwrap().as_str().unwrap().to_owned();
        let tz_name1st = properties.get("tz_name1st").unwrap().as_str().map(ToOwned::to_owned);
        let zone = properties.get("zone").unwrap().as_i64();

        let bbox = Rect::new(Coord { x: bbox[0], y: bbox[1] }, Coord { x: bbox[2], y: bbox[3] });

        let geometry: Geometry = geometry.value.clone().try_into().unwrap();

        //let mut parsable_offset = time_zone.clone();
        //parsable_offset.remove_matches('+');
        let raw_offset = (name.parse::<f64>().unwrap() * 3600.0).round() as i64;

        Timezone {
            id,
            objectid,
            dst_description: dst_places,
            zone_str: name,
            description: places,
            offset_str: time_zone,
            friendly_name: tz_name1st,
            zone_num: zone,
            raw_offset,
            bbox,
            geometry,
        }
    }
}

/// Trait that allows converting a [`u16`] into a [`Timezone`] reference (from the global list).
pub(crate) trait IntoTimezone {
    fn into_timezone(self) -> Res<TimezoneRef>;
}

impl IntoTimezone for u16 {
    fn into_timezone(self) -> Res<TimezoneRef> {
        Some(&self).map_timezone().ok_or_else(|| anyhow::Error::msg("Timezone not found."))
    }
}

/// Trait that allows converting a [`u16`] into a [`Timezone`] reference (from the global list).
pub(crate) trait MapIntoTimezone {
    fn map_timezone(self) -> Option<TimezoneRef>;
}

impl MapIntoTimezone for Option<&u16> {
    fn map_timezone(self) -> Option<TimezoneRef> {
        let Some(value) = self else {
            return None;
        };

        let timezones = get_timezones();

        timezones.get(*value as usize)
    }
}

/// Trait that allows converting a [`u16`] into a [`Timezone`] reference (from the global list).
pub(crate) trait MapIntoTimezones {
    fn map_timezones(self) -> Option<TimezoneRefs>;
}

impl MapIntoTimezones for Option<&TimezoneIds> {
    fn map_timezones(self) -> Option<TimezoneRefs> {
        let Some(value) = self else {
            return None;
        };

        let timezones = get_timezones();

        let mut result = Vec::with_capacity(10);
        for id in value {
            if *id == -1 {
                continue;
            }

            let tz = timezones.get(*id as usize);

            if let Some(tz) = tz {
                result.push(tz);
            }
        }

        Some(result)
    }
}
