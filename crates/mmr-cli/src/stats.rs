//! `mmr stats`: generate many seeds (no ROM needed) and summarize where
//! things land, to compare settings and placement styles.

use mmr_randomizer::{generate, Outcome, Seed, Style};
use mmr_settings::Values;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::time::Instant;

struct Run {
    seed: u64,
    millis: f64,
    result: Result<Outcome, String>,
    /// Locations whose item is needed to win.
    required: Vec<String>,
}

/// Generate seeds `first..first + count` on every core.
fn run_all(logic: &mmr_logic::LogicFile, values: &Values, style: Style, first: u64, count: u64) -> Vec<Run> {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(count.max(1) as usize);
    let next = std::sync::atomic::AtomicU64::new(first);
    let mut runs: Vec<Run> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads)
            .map(|_| {
                s.spawn(|| {
                    let mut out = Vec::new();
                    loop {
                        let seed = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if seed >= first + count {
                            break;
                        }
                        let t = Instant::now();
                        let result = generate(logic, values, Seed(seed), style).map_err(|e| e.to_string());
                        let millis = t.elapsed().as_secs_f64() * 1000.0;
                        let required = result.as_ref().map_or_else(|_| Vec::new(), |o| mmr_randomizer::importance::importance(logic, values, o, &[]).required.into_iter().collect());
                        out.push(Run { seed, millis, result, required });
                    }
                    out
                })
            })
            .collect();
        handles.into_iter().flat_map(|h| h.join().expect("stats thread")).collect()
    });
    runs.sort_by_key(|r| r.seed);
    runs
}

fn percent(n: usize, of: usize) -> f64 {
    if of == 0 {
        0.0
    } else {
        n as f64 * 100.0 / of as f64
    }
}

fn region_name(location: &str) -> String {
    mmr_build::spoiler::place(location)
}

/// The report, as text, plus the same numbers as JSON.
pub fn stats(logic: &mmr_logic::LogicFile, values: &Values, style: Style, first: u64, count: u64) -> (String, serde_json::Value) {
    let started = Instant::now();
    let runs = run_all(logic, values, style, first, count);
    let wall = started.elapsed().as_secs_f64();
    let ok: Vec<(&Run, &Outcome)> = runs.iter().filter_map(|r| r.result.as_ref().ok().map(|o| (r, o))).collect();
    let mut failures: BTreeMap<&str, Vec<u64>> = BTreeMap::new();
    for r in &runs {
        if let Err(e) = &r.result {
            failures.entry(e.as_str()).or_default().push(r.seed);
        }
    }

    let n = ok.len();
    let avg = |f: &dyn Fn(&Run, &Outcome) -> f64| if n == 0 { 0.0 } else { ok.iter().map(|(r, o)| f(r, o)).sum::<f64>() / n as f64 };
    let max = |f: &dyn Fn(&Run, &Outcome) -> f64| ok.iter().map(|(r, o)| f(r, o)).fold(0.0, f64::max);
    let min = |f: &dyn Fn(&Run, &Outcome) -> f64| ok.iter().map(|(r, o)| f(r, o)).fold(f64::INFINITY, f64::min);
    let spheres = |_: &Run, o: &Outcome| o.spheres.len() as f64;
    let attempts = |_: &Run, o: &Outcome| o.attempt as f64;
    let important = |_: &Run, o: &Outcome| o.spheres.iter().map(Vec::len).sum::<usize>() as f64;
    let required = |r: &Run, _: &Outcome| r.required.len() as f64;
    let millis = |r: &Run, _: &Outcome| r.millis;

    // Per important item: the sphere each copy is found in, where, and in
    // how many seeds.
    let mut items: BTreeMap<String, (Vec<usize>, BTreeMap<String, usize>, usize)> = BTreeMap::new();
    // Per region: in how many seeds it held something needed to win.
    let mut regions: BTreeMap<String, usize> = BTreeMap::new();
    for (r, o) in &ok {
        let mut seen: Vec<String> = Vec::new();
        let mut had: Vec<&str> = Vec::new();
        for (k, sphere) in o.spheres.iter().enumerate() {
            for p in sphere.iter().filter(|p| r.required.contains(&p.location)) {
                let region = region_name(&p.location);
                let e = items.entry(p.item_name.clone()).or_default();
                e.0.push(k + 1);
                if !had.contains(&p.item_name.as_str()) {
                    had.push(&p.item_name);
                    e.2 += 1;
                }
                *e.1.entry(region.clone()).or_default() += 1;
                if !seen.contains(&region) {
                    seen.push(region);
                }
            }
        }
        for r in seen {
            *regions.entry(r).or_default() += 1;
        }
    }

    let mut out = String::new();
    let _ = writeln!(out, "Seeds {first}..{} · {style:?} placement · {} threads, {wall:.1}s", first + count - 1, std::thread::available_parallelism().map_or(4, |n| n.get()));
    let _ = writeln!(out, "Succeeded          {n}/{} ({:.1}%)", runs.len(), percent(n, runs.len()));
    for (e, seeds) in &failures {
        let shown: Vec<String> = seeds.iter().take(8).map(u64::to_string).collect();
        let _ = writeln!(out, "  failed ×{:<5} {e} (seeds {}{})", seeds.len(), shown.join(", "), if seeds.len() > 8 { ", ..." } else { "" });
    }
    if n > 0 {
        let first_try = ok.iter().filter(|(_, o)| o.attempt == 1).count();
        let _ = writeln!(out, "First try          {:.1}% (attempts avg {:.2}, max {})", percent(first_try, n), avg(&attempts), max(&attempts));
        let _ = writeln!(out, "Time per seed      avg {:.0} ms, max {:.0} ms", avg(&millis), max(&millis));
        let _ = writeln!(out, "Playthrough steps  avg {:.1} (min {}, max {})", avg(&spheres), min(&spheres), max(&spheres));
        let _ = writeln!(out, "Important items    avg {:.1}", avg(&important));
        let _ = writeln!(out, "Needed to win      avg {:.1} (min {}, max {})", avg(&required), min(&required), max(&required));

        let _ = writeln!(out, "\nNeeded to win: how often, how late (playthrough step), and where");
        let mut rows: Vec<_> = items.iter().collect();
        rows.sort_by(|a, b| b.1 .2.cmp(&a.1 .2).then(a.0.cmp(b.0)));
        for (item, (steps, wh, seeds)) in rows {
            let mean = steps.iter().sum::<usize>() as f64 / steps.len() as f64;
            let mut top: Vec<_> = wh.iter().collect();
            top.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            let top: Vec<String> = top.iter().take(3).map(|(r, c)| format!("{r} {:.0}%", percent(**c, steps.len()))).collect();
            let _ = writeln!(out, "  {:<28} {:>5.1}% of seeds · step {:>4.1} · {}", item, percent(*seeds, n), mean, top.join(", "));
        }

        let _ = writeln!(out, "\nRegions on the Way of the Hero (holding something needed to win)");
        let mut rows: Vec<_> = regions.iter().collect();
        rows.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        for (region, c) in rows {
            let bar = "█".repeat((percent(*c, n) / 4.0).round() as usize);
            let _ = writeln!(out, "  {:<34} {:>5.1}% {bar}", region, percent(*c, n));
        }
    }

    let json = serde_json::json!({
        "first": first,
        "count": count,
        "style": format!("{style:?}"),
        "succeeded": n,
        "failures": failures.iter().map(|(e, s)| serde_json::json!({ "error": e, "seeds": s })).collect::<Vec<_>>(),
        "seeds": ok.iter().map(|(r, o)| serde_json::json!({
            "seed": r.seed,
            "attempt": o.attempt,
            "required": r.required,
            "millis": r.millis,
            "spheres": o.spheres.iter().map(|s| s.iter().map(|p| serde_json::json!([p.item, p.location])).collect::<Vec<_>>()).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    });
    (out, json)
}
