//! The shared functionality for the timezone lookup module.

// Traits.

use rtz_core::geo::tz::shared::{ConcreteVec, RoundLngLat, TimezoneIds};
use std::collections::HashMap;

/// Trait that abstracts away getting the in-memory timezones / cache.
pub(crate) trait HasCachedData
where
    Self: Sized,
{
    fn get_timezones() -> &'static ConcreteVec<Self>;
    fn get_cache() -> &'static HashMap<RoundLngLat, TimezoneIds>;
}

/// Trait that allows converting a [`u16`] into a [`Timezone`] reference (from the global list).
pub(crate) trait MapIntoTimezone<T> {
    fn map_timezone(self) -> Option<&'static T>;
}

impl<T> MapIntoTimezone<T> for Option<&u16>
where
    T: HasCachedData,
{
    fn map_timezone(self) -> Option<&'static T> {
        let Some(value) = self else {
            return None;
        };

        let timezones = T::get_timezones();

        timezones.get(*value as usize)
    }
}

/// Trait that allows converting a [`u16`] into a [`Timezone`] reference (from the global list).
pub(crate) trait MapIntoTimezones<T> {
    fn map_timezones(self) -> Option<Vec<&'static T>>;
}

impl<T> MapIntoTimezones<T> for Option<&TimezoneIds>
where
    T: HasCachedData,
{
    fn map_timezones(self) -> Option<Vec<&'static T>> {
        let Some(value) = self else {
            return None;
        };

        let timezones = T::get_timezones();

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
