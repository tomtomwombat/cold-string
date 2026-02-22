#![allow(unsafe_op_in_unsafe_fn)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::fmt::Debug;
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

fn random_string<T: FromStr>(min: usize, max: usize) -> T
where
    <T as FromStr>::Err: Debug,
{
    let len = fastrand::usize(min..=max);
    let mut scratch = [0u8; 128];
    for i in 0..len {
        scratch[i] = fastrand::alphanumeric() as u8;
    }
    let s = unsafe { std::str::from_utf8_unchecked(&scratch[..len]) };
    s.parse().unwrap()
}

fn allocator_memory<T: FromStr>(name: &str)
where
    <T as FromStr>::Err: Debug,
{
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

fn system_memory<T: FromStr>(name: &str, min: usize, max: usize)
where
    <T as FromStr>::Err: Debug,
{
    let mut sys = sysinfo::System::new_all();
    let pid = Pid::from(std::process::id() as usize);

    const TRIALS: usize = 10_000_000;

    sys.refresh_all();
    let proc = sys.process(pid).unwrap();
    let base_mem = proc.memory();
    let base_virt = proc.virtual_memory();

    let mut strings: Vec<T> = Vec::with_capacity(TRIALS);
    for _ in 0..TRIALS {
        strings.push(random_string(min, max));
    }

    sys.refresh_all();
    let proc = sys.process(pid).unwrap();
    let rss = (proc.memory() - base_mem) as f64 / (TRIALS as f64);
    let vsz = (proc.virtual_memory() - base_virt) as f64 / (TRIALS as f64);
    let n_w = 18;
    let r_w = 12;
    let v_w = 12;
    println!("{:<n_w$} | {:>r_w$.1} | {:>v_w$.1}", name, rss, vsz);
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
    allocator_memory::<compact_string::CompactString>("compact_string");
    allocator_memory::<cold_string::ColdString>("cold-string");
}

/// Not run automatically.
/// Run with `cargo test test_system_memory --release -- --no-capture --include-ignored`
/// Or specify min,max:
/// ```
/// $env:STR_MIN=0; $env:STR_MAX=64; cargo test test_system_memory --release -- --no-capture --include-ignored
/// ```
#[test]
#[ignore]
fn test_system_memory() {
    let min: usize = std::env::var("STR_MIN")
        .map(|v| v.parse().unwrap_or(0))
        .unwrap_or(0);
    let max: usize = std::env::var("STR_MAX")
        .map(|v| v.parse().unwrap_or(16))
        .unwrap_or(16);

    let n_w = 18;
    let r_w = 12;
    let v_w = 12;
    let title = format!("Crate, len {}..={}", min, max);
    println!(
        "{:<n_w$} | {:>r_w$} | {:>v_w$}",
        title, "RSS (B)", "Virtual (B)"
    );
    println!("{:-<n_w$}-|-{:-<r_w$}-|-{:-<v_w$}", "", "", "");

    system_memory::<String>("std", min, max);
    system_memory::<smol_str::SmolStr>("smol_str", min, max);
    system_memory::<compact_str::CompactString>("compact_str", min, max);
    system_memory::<compact_string::CompactString>("compact_string", min, max);
    system_memory::<cold_string::ColdString>("cold-string", min, max);
}
