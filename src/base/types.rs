// Result Types.

use std::ops::Deref;

use serde::{self, Serialize, Deserialize};
use geo::{Coord, Geometry, Rect};

use super::geo::get_timezones;

// Result types.

/// A shortened version of [`anyhow::Result<T>`].
pub type Res<T> = anyhow::Result<T>;
/// A shortened version of [`anyhow::Result<()>`](anyhow::Result).
pub type Void = anyhow::Result<()>;
/// A shortened version of [`anyhow::Error`].
pub type Err = anyhow::Error;

// Helper types.

pub type RoundLngLat = (i16, i16);
pub type LngLat = (f64, f64);

pub type TimezoneIds = [i16; 10];
pub type TimezoneRef = &'static Timezone;
pub type TimezoneRefs = Vec<TimezoneRef>;

// Geo Types.

/// A collection of [`Timezone`]s.
pub struct ConcreteTimezones(Vec<Timezone>);

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

/// A TZ version of [`geojson::Feature`].
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Timezone {
    pub id: usize,
    pub objectid: u64,
    pub friendly_name: Option<String>,

    pub description: String,
    pub dst_description: Option<String>,

    pub offset_str: String,

    pub zone_num: Option<i64>,
    pub zone_str: String,
    pub raw_offset: i64,

    pub bbox: Rect,
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
pub trait IntoTimezone {
    fn into_timezone(self) -> Res<TimezoneRef>;
}

impl IntoTimezone for u16 {
    fn into_timezone(self) -> Res<TimezoneRef> {
        Some(&self).map_timezone().ok_or_else(|| anyhow::Error::msg("Timezone not found."))
    }
}

/// Trait that allows converting a [`u16`] into a [`Timezone`] reference (from the global list).
pub trait MapIntoTimezone {
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
pub trait MapIntoTimezones {
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

            if let Some (tz) = tz {
                result.push(tz);
            }
        }

        Some(result)
    }
}
