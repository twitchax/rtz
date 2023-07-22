//! The main binary entrypoint.

use clap::{command, Parser, Subcommand};
use rtzlib::{generate_bincodes, get_timezone, Void};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Resolve a timezone from a lng,lat pair.
    Resolve {
        /// The lng,lat pair for which to lookup timezone information.
        lng_lat: String,
    },

    /// Generate the bincoded timezone and cache files.
    Generate {
        /// The path to the timezone geojson file.
        geojson_input: String,

        /// The path to the destination timezone bincode file.
        #[arg(short, long, default_value = "assets/ne_10m_time_zones.bincode")]
        timezone_bincode_destination: String,

        /// The path to the destination cache bincode file.
        #[arg(short, long, default_value = "assets/100km_cache.bincode")]
        cache_bincode_destination: String,
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
        Some(Command::Resolve { lng_lat }) => {
            let Some((lng, lat)) = lng_lat.split_once(',') else {
                return Err(anyhow::Error::msg("Invalid lng,lat pair."));
            };

            let (lng, lat) = (lng.parse::<f64>()?, lat.parse::<f64>()?);

            let tz = get_timezone(lng, lat).ok_or_else(|| anyhow::Error::msg("Failed to resolve timezone."))?;

            println!();
            println!("Friendly Name:   {}", tz.friendly_name.as_deref().unwrap_or(""));
            println!("UTC Offset:      {}", tz.offset_str);
            println!("Offset Seconds:  {}", tz.raw_offset);
            println!("Description:     {}", tz.description);
            println!("DST Description: {}", tz.dst_description.as_deref().unwrap_or(""));
            println!();
        }
        Some(Command::Generate {
            geojson_input,
            timezone_bincode_destination,
            cache_bincode_destination,
        }) => {
            generate_bincodes(geojson_input, timezone_bincode_destination, cache_bincode_destination);
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
        None => {
            return Err(anyhow::Error::msg("No command specified."));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn can_resolve() {
        start(Args {
            command: Some(Command::Resolve { lng_lat: "-87.62,41.88".to_string() }),
        })
        .unwrap();
    }

    #[test]
    fn can_generate_bincodes() {
        let geojson_input = "test/ne_10m_time_zones.test.geojson";
        let timezone_bincode_destination = "test/ne_10m_time_zones.test.bincode";
        let cache_bincode_destination = "test/100km_cache.test.bincode";

        start(Args {
            command: Some(Command::Generate {
                geojson_input: geojson_input.to_string(),
                timezone_bincode_destination: timezone_bincode_destination.to_string(),
                cache_bincode_destination: cache_bincode_destination.to_string(),
            }),
        })
        .unwrap();

        assert!(Path::new(timezone_bincode_destination).exists());
        assert!(Path::new(cache_bincode_destination).exists());

        std::fs::remove_file(timezone_bincode_destination).unwrap();
        std::fs::remove_file(cache_bincode_destination).unwrap();
    }
}
