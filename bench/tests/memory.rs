#![allow(unsafe_op_in_unsafe_fn)]
#![allow(dead_code)]

use bench::*;

use ahash::{HashMap, HashMapExt};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cmp::Ord;
use std::collections::BTreeMap;
use std::hash::Hash;
use std::io::Write;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use sysinfo::Pid;

#[global_allocator]
static A: CountingAlloc = CountingAlloc;

pub struct CountingAlloc;

pub static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
        System.dealloc(ptr, layout)
    }
}

fn allocator_memory<T: FromStr>(name: &str) {
    const SIZE: usize = 48;
    const TRIALS: usize = 100;

    let mut memory: [f64; SIZE as usize + 1] = [0.0; SIZE as usize + 1];
    let now = Instant::now();

    for size in 0..=SIZE {
        let base = ALLOCATED.load(Ordering::SeqCst);
        let mut strings: Vec<T> = Vec::with_capacity(TRIALS);
        for _ in 0..TRIALS {
            strings.push(random_string(size, size));
        }
        let mem_used = ALLOCATED.load(Ordering::SeqCst) - base;
        memory[size] = mem_used as f64 / TRIALS as f64;
    }

    let mut file = std::fs::File::create(format!("{}.csv", name)).unwrap();
    for items in 0..=SIZE {
        let row = format!("{},{}\n", items, memory[items as usize]);
        file.write_all(row.as_bytes()).unwrap();
    }

    println!(
        "{} done in {} s.",
        name,
        now.elapsed().as_millis() as f64 / 1000.0
    );
}

/// Not run automatically.
/// Run with `cargo test test_allocator_memory --release -- --no-capture --include-ignored`
/// Then, `python memory.py`
#[test]
#[ignore]
fn test_allocator_memory() {
    allocator_memory::<String>("std");
    allocator_memory::<smol_str::SmolStr>("smol_str");
    allocator_memory::<compact_str::CompactString>("compact_str");
    allocator_memory::<smartstring::alias::String>("smartstring");
    allocator_memory::<smallstr::SmallString<[u8; 8]>>("smallstr");
    allocator_memory::<compact_string::CompactString>("compact_string");
    allocator_memory::<cold_string::ColdString>("cold-string");
}

fn hash_map_workload<T: FromStr + Hash + Eq>(min: usize, max: usize) {
    let mut strings: HashMap<T, T> = HashMap::with_capacity(TRIALS);
    for _ in 0..TRIALS {
        strings.insert(random_string(min, max), random_string(min, max));
    }
    let strings = std::hint::black_box(strings);
    std::mem::forget(strings);
}

fn vec_workload<T: FromStr + Hash + Eq>(min: usize, max: usize) {
    let mut strings: Vec<T> = Vec::with_capacity(TRIALS);
    for _ in 0..TRIALS {
        strings.push(random_string(min, max));
    }
    let strings = std::hint::black_box(strings);
    std::mem::forget(strings);
}

fn btree_workload<T: FromStr + Hash + Eq + Ord>(min: usize, max: usize) {
    let mut strings: BTreeMap<T, T> = BTreeMap::new();
    for _ in 0..TRIALS {
        strings.insert(random_string(min, max), random_string(min, max));
    }
    let strings = std::hint::black_box(strings);
    std::mem::forget(strings);
}

/// Demo data with potential poor "alignment waste"
#[derive(Eq, Hash, PartialEq)]
struct Data<T> {
    s: T,
    b: bool,
}

impl<T: FromStr> FromStr for Data<T> {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.parse().map_err(|_| ()).unwrap();
        Ok(Self { s, b: false })
    }
}

const SIZES: &[usize] = &[4, 8, 16, 32, 64];
const CELL_WIDTH: usize = 7;
const NAME_WIDTH: usize = 16;
const TRIALS: usize = 1_000_000;

fn system_memory(name: &str, workload: impl Fn(usize, usize)) {
    print!("{:<NAME_WIDTH$} ", name);

    for max in SIZES {
        let mut sys = sysinfo::System::new_all();
        let pid = Pid::from(std::process::id() as usize);

        sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), false);
        let proc = sys.process(pid).unwrap();
        let base_mem = proc.memory();
        let base_virt = proc.virtual_memory();

        workload(0, *max);

        sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), false);
        let proc = sys.process(pid).unwrap();
        let rss = (proc.memory() - base_mem) as f64 / (TRIALS as f64);
        let _vsz = (proc.virtual_memory() - base_virt) as f64 / (TRIALS as f64);
        print!(" | {:>CELL_WIDTH$.1}", rss);
    }
    print!("\n");
}

/// Not run automatically.
/// Run with `cargo test test_system_memory --release -- --no-capture --include-ignored`
/// Or specify min,max:
/// ```
/// cargo test test_system_memory --release -- --no-capture --include-ignored
/// ```
#[test]
#[rustfmt::skip]
#[ignore]
fn test_system_memory() {
    print!("{:<NAME_WIDTH$} ", "Crate");
    for &size in SIZES {
        print!(" | {:>CELL_WIDTH$}", format!("{}..={}", 0, size));
    }
    println!();

    print!("{: <NAME_WIDTH$}  |", ":---");
    for _ in SIZES {
        print!(" {: ^CELL_WIDTH$} |", ":---:");
    }
    println!();

    system_memory("cold-string", hash_map_workload::<cold_string::ColdString>);
    system_memory("compact_str", hash_map_workload::<compact_str::CompactString>);
    system_memory("compact_string", hash_map_workload::<compact_string::CompactString>);
    system_memory("smallstr", hash_map_workload::<smallstr::SmallString<[u8; 8]>>);
    system_memory("smartstring", hash_map_workload::<smartstring::alias::String>);
    system_memory("smol_str", hash_map_workload::<smol_str::SmolStr>);
    system_memory("std", hash_map_workload::<String>);
}
