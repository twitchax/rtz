// Features.

#![feature(async_closure)]
#![feature(test)]
#![feature(string_remove_matches)]

// Modules.

extern crate test;
pub mod base;

// Imports.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use base::types::Void;
use geo::{Contains, Coord, Intersects};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};

use crate::base::geo::{generate_100km_cache, get_timezones, get_timezone};

#[tokio::main]
async fn main() -> Void {
    println!("Hello, world!");
    
    for _ in (0..10) {
        let x = rand::random::<f64>() * 360.0 - 180.0;
        let y = rand::random::<f64>() * 180.0 - 90.0;
        
        let cache_assisted = get_timezone(x, y).unwrap();

        println!("{},{} : {:?}, {}", y, x, cache_assisted.raw_offset, cache_assisted.offset_str);
    }

    //println!("{} GB", std::mem::size_of_val(&a) as f64 / 1000000000.0);

    println!("{}", std::mem::size_of::<u16>());

    Ok(())
}

#[cfg(test)]
mod tests {
    use test::{black_box, Bencher};

    use crate::base::geo::{get_timezone, get_timezone_via_full_lookup};

    #[bench]
    fn bench_full_lookup_sweep(b: &mut Bencher) {
        // Optionally include some setup.
        let xs = (-179..180).step_by(10);
        let ys = (-89..90).step_by(10);

        b.iter(|| {
            for x in xs.clone() {
                for y in ys.clone() {
                    black_box(get_timezone_via_full_lookup(x as f64, y as f64));
                }
            }
        });
    }

    #[bench]
    fn bench_cache_assisted_sweep(b: &mut Bencher) {
        // Optionally include some setup.
        let xs = (-179..180).step_by(10);
        let ys = (-89..90).step_by(10);

        b.iter(|| {
            for x in xs.clone() {
                for y in ys.clone() {
                    black_box(get_timezone(x as f64, y as f64));
                }
            }
        });
    }

    #[bench]
    fn bench_worst_case_full_lookup_single(b: &mut Bencher) {
        // Optionally include some setup.
        let x = -177;
        let y = -15;

        b.iter(|| {
            black_box(get_timezone_via_full_lookup(x as f64, y as f64));
        });
    }

    #[bench]
    fn bench_worst_case_cache_assisted_single(b: &mut Bencher) {
        // Optionally include some setup.
        let x = -67.5;
        let y = -66.5;

        b.iter(|| {
            black_box(get_timezone(x, y));
        });
    }
}
