use std::{env, hint::black_box, process::Command, str};

use arcstr::ArcStr;
use cold_string::{ArcColdString, ArcColdString32};
use string_cache::DefaultAtom;
use sysinfo::{Pid, ProcessesToUpdate, System};

const LENGTHS: &[usize] = &[8, 128, 512];
const KINDS: &[&str] = &[
    "Arc<str>",
    "ArcStr",
    "DefaultAtom",
    "ArcColdString",
    "ArcColdString32",
];
const ALPHABET: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

fn write_unique_suffix(buf: &mut [u8], mut value: usize) {
    for byte in buf.iter_mut().rev().take(8) {
        *byte = ALPHABET[value % ALPHABET.len()];
        value /= ALPHABET.len();
    }
    assert_eq!(value, 0, "item count exceeds the unique suffix range");
}

fn rss(system: &mut System, pid: Pid) -> u64 {
    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), false);
    system.process(pid).unwrap().memory()
}

fn measure<T>(len: usize, count: usize) -> u64
where
    for<'a> T: From<&'a str>,
{
    let mut scratch = vec![b'x'; len];
    let pid = Pid::from(std::process::id() as usize);
    let mut system = System::new_all();
    let baseline = rss(&mut system, pid);

    let mut strings = Vec::<T>::with_capacity(count);
    for i in 0..count {
        write_unique_suffix(&mut scratch, i);
        let value = unsafe { str::from_utf8_unchecked(&scratch) };
        strings.push(T::from(value));
    }
    black_box(&strings);

    let used = rss(&mut system, pid).saturating_sub(baseline);
    std::mem::forget(strings);
    used
}

fn child(kind: &str, len: usize, count: usize) {
    let used = match kind {
        "Arc<str>" => measure::<std::sync::Arc<str>>(len, count),
        "ArcStr" => measure::<ArcStr>(len, count),
        "DefaultAtom" => measure::<DefaultAtom>(len, count),
        "ArcColdString" => measure::<ArcColdString>(len, count),
        "ArcColdString32" => measure::<ArcColdString32>(len, count),
        _ => panic!("unknown string kind: {kind}"),
    };
    println!("{used}");
}

fn sample(kind: &str, len: usize, count: usize) -> u64 {
    let output = Command::new(env::current_exe().unwrap())
        .args(["--child", kind, &len.to_string(), &count.to_string()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    str::from_utf8(&output.stdout)
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.get(1).map(String::as_str) == Some("--child") {
        child(&args[2], args[3].parse().unwrap(), args[4].parse().unwrap());
        return;
    }

    let count = env::var("COLD_STRING_MEMORY_COUNT")
        .ok()
        .map(|value| value.parse().unwrap())
        .unwrap_or(1_000_000);
    let trials = env::var("COLD_STRING_MEMORY_TRIALS")
        .ok()
        .map(|value| value.parse().unwrap())
        .unwrap_or(3);
    assert!(count > 0 && trials > 0);

    println!("RSS bytes per unique string in Vec ({count} strings, median of {trials} runs)");
    print!("| Type |");
    for len in LENGTHS {
        print!(" {len} bytes |");
    }
    println!();
    print!("|:--|");
    for _ in LENGTHS {
        print!("--:|");
    }
    println!();

    for kind in KINDS {
        print!("| {kind} |");
        for &len in LENGTHS {
            let mut samples: Vec<_> = (0..trials).map(|_| sample(kind, len, count)).collect();
            samples.sort_unstable();
            let bytes_per_string = samples[trials / 2] as f64 / count as f64;
            print!(" {bytes_per_string:.1} |");
        }
        println!();
    }
}
