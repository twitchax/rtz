//! The main binary entrypoint.

use clap::{command, Parser, Subcommand};
use rtz_core::{
    base::types::Void,
    geo::{
        admin::osm::OsmAdmin,
        tz::{ned::NedTimezone, osm::OsmTimezone},
    },
};
use rtzlib::geo::shared::CanPerformGeoLookup;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Resolve a timezone from a lng,lat pair using the NED dataset.
    #[cfg(feature = "tz-ned")]
    ResolveNed {
        /// The lng,lat pair for which to lookup timezone information.
        lng_lat: String,
    },

    /// Resolve a timezone from a lng,lat pair using the OSM dataset.
    #[cfg(feature = "tz-osm")]
    ResolveOsm {
        /// The lng,lat pair for which to lookup timezone information.
        lng_lat: String,
    },

    /// Resolve a timezone from a lng,lat pair using the OSM dataset.
    DumpGeojson {
        /// The prefix to use on the file names.
        prefix: String,
    },

    /// Serve the timezone API.
    #[cfg(feature = "web")]
    Serve {
        /// The server configuration path.
        #[arg(short, long, default_value = "dummy_5439258095")]
        config_path: String,

        /// The address on which to serve the API.
        #[arg(short, long)]
        bind_address: Option<String>,

        /// The port on which to serve the API.
        #[arg(short, long)]
        port: Option<u16>,

        /// Whether or not to log.
        #[arg(short, long)]
        should_log: Option<bool>,
    },
}

fn main() -> Void {
    let args = Args::parse();

    start(args)?;

    Ok(())
}

fn start(args: Args) -> Void {
    match args.command {
        #[cfg(feature = "tz-ned")]
        Some(Command::ResolveNed { lng_lat }) => {
            use rtz_core::base::types::Float;

            let Some((lng, lat)) = lng_lat.split_once(',') else {
                return Err(anyhow::Error::msg("Invalid lng,lat pair."));
            };

            let (lng, lat) = (lng.parse::<Float>()?, lat.parse::<Float>()?);
            let tzs = NedTimezone::lookup(lng, lat);

            for tz in tzs {
                println!();
                println!("Identifier:      {}", tz.identifier.as_deref().unwrap_or(""));
                println!("UTC Offset:      {}", tz.offset);
                println!("Offset Seconds:  {}", tz.raw_offset);
                println!("Description:     {}", tz.description);
                println!("DST Description: {}", tz.dst_description.as_deref().unwrap_or(""));
                println!();
            }
        }
        #[cfg(feature = "tz-osm")]
        Some(Command::ResolveOsm { lng_lat }) => {
            use rtz_core::base::types::Float;

            let Some((lng, lat)) = lng_lat.split_once(',') else {
                return Err(anyhow::Error::msg("Invalid lng,lat pair."));
            };

            let (lng, lat) = (lng.parse::<Float>()?, lat.parse::<Float>()?);
            let tzs = OsmTimezone::lookup(lng, lat);

            for tz in tzs {
                println!();
                println!("Identifier:      {}", tz.identifier);
                println!();
            }
        }
        #[cfg(feature = "web")]
        Some(Command::Serve {
            config_path,
            bind_address,
            port,
            should_log,
        }) => {
            rtzlib::server_start(config_path, bind_address, port, should_log)?;
        }
        Some(Command::DumpGeojson { prefix }) => {
            #[cfg(feature = "tz-ned")]
            {
                let json = NedTimezone::memory_data_to_geojson();

                std::fs::write(format!("{}-tz-ned.geojson", prefix), json)?;
            }

            #[cfg(feature = "tz-osm")]
            {
                let json = OsmTimezone::memory_data_to_geojson();

                std::fs::write(format!("{}-tz-osm.geojson", prefix), json)?;
            }

            #[cfg(feature = "admin-osm")]
            {
                let json = OsmAdmin::memory_data_to_geojson();

                std::fs::write(format!("{}-admin-osm.geojson", prefix), json)?;
            }
        }
        #[allow(unreachable_patterns)]
        Some(_) | None => {
            return Err(anyhow::Error::msg("No command specified."));
        }
    }

    #[allow(unreachable_code)]
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "tz-ned")]
    fn can_resolve_ned() {
        start(Args {
            command: Some(Command::ResolveNed { lng_lat: "-87.62,41.88".to_string() }),
        })
        .unwrap();
    }

    #[test]
    #[cfg(feature = "tz-osm")]
    fn can_resolve_osm() {
        start(Args {
            command: Some(Command::ResolveOsm { lng_lat: "-87.62,41.88".to_string() }),
        })
        .unwrap();
    }
}
