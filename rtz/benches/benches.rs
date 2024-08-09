//! Benchmarks for the the Admin OSM features.

use std::hint::black_box;
use criterion::{criterion_group, criterion_main, Criterion};

use rtz_core::base::types::Float;
use rtzlib::{CanPerformGeoLookup, OsmAdmin, NedTimezone, OsmTimezone};

// Admin OSM features.

#[allow(dead_code)]
fn admin_osm_bench_full_lookup_sweep(c: &mut Criterion) {
    let xs = (-179..180).step_by(10);
    let ys = (-89..90).step_by(10);

    c.bench_function("admin_osm_bench_full_lookup_sweep", |b| {
        b.iter(|| {
            for x in xs.clone() {
                for y in ys.clone() {
                    black_box(OsmAdmin::lookup_slow(x as Float, y as Float));
                }
            }
        });
    });
}

fn admin_osm_bench_lookup_assisted_sweep(c: &mut Criterion) {
    let xs = (-179..180).step_by(10);
    let ys = (-89..90).step_by(10);

    c.bench_function("admin_osm_bench_lookup_assisted_sweep", |b| {
        b.iter(|| {
            for x in xs.clone() {
                for y in ys.clone() {
                    black_box(OsmAdmin::lookup(x as Float, y as Float));
                }
            }
        });
    });
}

// TODO: Discover the actual worst case location.
fn admin_osm_bench_worst_case_full_lookup_single(c: &mut Criterion) {
    let x = -86.5;
    let y = 38.5;

    c.bench_function("admin_osm_bench_worst_case_full_lookup_single", |b| {   
        b.iter(|| {
            black_box(OsmAdmin::lookup_slow(x as Float, y as Float));
        });
    });
}

// TODO: Discover the actual worst case location.
fn admin_osm_bench_worst_case_lookup_assisted_single(c: &mut Criterion) {
    let x = -86.5;
    let y = 38.5;

    c.bench_function("admin_osm_bench_worst_case_lookup_assisted_single", |b| {   
        b.iter(|| {
            black_box(OsmAdmin::lookup(x as Float, y as Float));
        });
    });
}

fn admin_osm_bench_cities(c: &mut Criterion) {
    c.bench_function("admin_osm_bench_cities", |b| {
        b.iter(|| {
            let city = cities_json::get_random_cities();
            black_box(OsmAdmin::lookup(city.lng as f32, city.lat as f32));
        });
    });
}

// TZ NED Features

fn tz_ned_bench_lookup_assisted_sweep(c: &mut Criterion) {
    let xs = (-179..180).step_by(10);
    let ys = (-89..90).step_by(10);

    c.bench_function("tz_ned_bench_lookup_assisted_sweep", |b| {
        b.iter(|| {
            for x in xs.clone() {
                for y in ys.clone() {
                    black_box(NedTimezone::lookup(x as Float, y as Float));
                }
            }
        });
    });
}

fn tz_ned_bench_worst_case_full_lookup_single(c: &mut Criterion) {
    let x = -177;
    let y = -15;

    c.bench_function("tz_ned_bench_worst_case_full_lookup_single", |b| {
        b.iter(|| {
            black_box(NedTimezone::lookup_slow(x as Float, y as Float));
        });
    });
}

fn tz_ned_bench_worst_case_lookup_assisted_single(c: &mut Criterion) {
    let x = -67.5;
    let y = -66.5;

    c.bench_function("tz_ned_bench_worst_case_lookup_assisted_single", |b| {
        b.iter(|| {
            black_box(NedTimezone::lookup(x as Float, y as Float));
        });
    });
}

fn tz_ned_bench_cities(c: &mut Criterion) {
    c.bench_function("tz_ned_bench_cities", |b| {
        b.iter(|| {
            let city = cities_json::get_random_cities();
            black_box(NedTimezone::lookup(city.lng as f32, city.lat as f32));
        });
    });
}

// TZ OSM Features

fn tz_osm_bench_lookup_assisted_sweep(c: &mut Criterion) {
    let xs = (-179..180).step_by(10);
    let ys = (-89..90).step_by(10);

    c.bench_function("tz_osm_bench_lookup_assisted_sweep", |b| {
        b.iter(|| {
            for x in xs.clone() {
                for y in ys.clone() {
                    black_box(OsmTimezone::lookup(x as Float, y as Float));
                }
            }
        })
    });
}

fn tz_osm_bench_worst_case_full_lookup_single(c: &mut Criterion) {
    let x = -86.5;
    let y = 38.5;

    c.bench_function("tz_osm_bench_worst_case_full_lookup_single", |b| {
        b.iter(|| {
            black_box(OsmTimezone::lookup_slow(x as Float, y as Float));
        });
    });
}

fn tz_osm_bench_worst_case_lookup_assisted_single(c: &mut Criterion) {
    let x = -86.5;
    let y = 38.5;

    c.bench_function("tz_osm_bench_worst_case_lookup_assisted_single", |b| {
        b.iter(|| {
            black_box(OsmTimezone::lookup(x, y));
        });
    });
}

fn tz_osm_bench_cities(c: &mut Criterion) {
    c.bench_function("tz_osm_bench_cities", |b| {
        b.iter(|| {
            let city = cities_json::get_random_cities();
            black_box(OsmTimezone::lookup(city.lng as f32, city.lat as f32));
        });
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(100);
    targets = admin_osm_bench_lookup_assisted_sweep, admin_osm_bench_worst_case_full_lookup_single, admin_osm_bench_worst_case_lookup_assisted_single, admin_osm_bench_cities,
              tz_ned_bench_lookup_assisted_sweep, tz_ned_bench_worst_case_full_lookup_single, tz_ned_bench_worst_case_lookup_assisted_single, tz_ned_bench_cities,
              tz_osm_bench_lookup_assisted_sweep, tz_osm_bench_worst_case_full_lookup_single, tz_osm_bench_worst_case_lookup_assisted_single, tz_osm_bench_cities
);
criterion_main!(benches);