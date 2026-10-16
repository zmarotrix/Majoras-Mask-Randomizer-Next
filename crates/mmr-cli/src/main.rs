//! `mmr`: command-line interface for the Majora's Mask Randomizer.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use mmr_logic::validate::Severity;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

mod stats;

fn parse_style(style: &str) -> Result<mmr_randomizer::Style> {
    Ok(match style.to_ascii_lowercase().as_str() {
        "random" => mmr_randomizer::Style::Random,
        "bespoke" => mmr_randomizer::Style::Bespoke,
        "classic" => mmr_randomizer::Style::Classic,
        other => bail!("unknown placement style {other:?} (random, bespoke or classic)"),
    })
}

#[derive(Parser)]
#[command(name = "mmr", version, about = "Majora's Mask Randomizer Next")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Work with logic files.
    #[command(subcommand)]
    Logic(LogicCommand),
    /// Work with settings.
    #[command(subcommand)]
    Settings(SettingsCommand),
    /// Place items for a seed and print where the important ones landed.
    /// With --rom, also build the randomized ROM.
    Generate {
        /// Preset id, e.g. "modern" (see `mmr settings presets`).
        #[arg(long, default_value = "season-2-standard")]
        preset: String,
        /// Seed: a number or any text. Random if omitted.
        #[arg(long)]
        seed: Option<String>,
        /// Placement style: random, bespoke or classic.
        #[arg(long, default_value = "random")]
        style: String,
        /// Write the full result as JSON here.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// A clean Majora's Mask (USA) ROM: build the randomized ROM.
        #[arg(long)]
        rom: Option<PathBuf>,
        /// Where to write the randomized ROM (default: next to --rom).
        #[arg(long)]
        rom_out: Option<PathBuf>,
        /// Write the build plan (what the ROM builder was given) as JSON.
        #[arg(long)]
        plan_out: Option<PathBuf>,
        /// A folder of extra music packs (.mmrs) for random music.
        #[arg(long)]
        music: Option<PathBuf>,
        /// Also write a patch (.mmrn) here (needs --rom).
        #[arg(long)]
        patch_out: Option<PathBuf>,
        /// Also write the spoiler log here (needs --rom). A `.json` path
        /// gets the full spoiler data as JSON instead of text.
        #[arg(long)]
        spoiler_out: Option<PathBuf>,
        /// Also write the spoiler-free HTML tracker here (needs --rom).
        #[arg(long)]
        tracker_out: Option<PathBuf>,
        /// Change a setting: KEY=VALUE, the value as JSON (e.g.
        /// GossipHintStyle='"Competitive"' or EntranceMode='["Grottos"]').
        #[arg(long = "set", value_name = "KEY=VALUE")]
        set: Vec<String>,
        /// An item plando file (*ItemPlando.json); can be given more than once.
        #[arg(long)]
        plando: Vec<PathBuf>,
    },
    /// Turn a patch (.mmrn) and a clean ROM into the randomized ROM.
    ApplyPatch {
        /// A clean Majora's Mask (USA) ROM.
        #[arg(long)]
        rom: PathBuf,
        /// The patch.
        patch: PathBuf,
        /// Where to write the ROM (default: next to the patch).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Generate many seeds without a ROM and summarize them: success rate,
    /// retries, speed, playthrough length, and where important items land.
    Stats {
        /// Preset id, e.g. "modern".
        #[arg(long, default_value = "season-2-standard")]
        preset: String,
        /// How many seeds.
        #[arg(long, default_value_t = 200)]
        count: u64,
        /// First seed number.
        #[arg(long, default_value_t = 1)]
        from: u64,
        /// Placement style: random, bespoke or classic.
        #[arg(long, default_value = "random")]
        style: String,
        /// Change a setting: KEY=VALUE, the value as JSON.
        #[arg(long = "set", value_name = "KEY=VALUE")]
        set: Vec<String>,
        /// Also write every seed's playthrough and the totals as JSON here.
        #[arg(long)]
        json: Option<PathBuf>,
    },
    /// Write the manual (every setting, preset and trick) as one HTML page.
    Manual {
        #[arg(short, long, default_value = "MMR Next Manual.html")]
        output: PathBuf,
    },
    /// Write the data the UI loads at startup, for previewing the UI in a
    /// plain browser (see app/src/lib/backend.ts).
    UiData {
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Check a ROM, and optionally write a fixed (byteswapped) copy.
    Rom {
        rom: PathBuf,
        /// Write the ROM in big-endian (.z64) order to this path.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum LogicCommand {
    /// Convert a logic file between the MMR Next format and the older (v31) format.
    Convert {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        /// Write the older (v31) format instead of the MMR Next format.
        #[arg(long)]
        legacy: bool,
    },
    /// Convert a logic file (any format) to regions, and prove the result
    /// behaves exactly like the original.
    Regionize {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        /// Also write a readable text version here.
        #[arg(long)]
        text: Option<PathBuf>,
        /// Random scenarios for the equivalence check.
        #[arg(long, default_value_t = 3000)]
        scenarios: usize,
    },
    /// Report problems in a logic file and test that the vanilla game is beatable.
    Check {
        /// A logic file, or the name of a bundled logic (`casual`, `glitched`).
        logic: String,
        /// Enable every trick for the playthrough test.
        #[arg(long)]
        all_tricks: bool,
        /// Also list informational notes.
        #[arg(long)]
        verbose: bool,
    },
}

#[derive(Subcommand)]
enum SettingsCommand {
    /// List the bundled presets.
    Presets,
    /// Convert an older randomizer settings.json into a settings string.
    Import { file: PathBuf },
    /// Print the settings a settings string contains (only non-defaults).
    Decode { string: String },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Logic(LogicCommand::Convert { input, output, legacy }) => {
            let (file, _) = mmr_logic::load(&read(&input)?)?;
            let json = if legacy {
                let schema = mmr_settings::schema::schema();
                let enum_type = |k: &str| schema.get(k).and_then(|d| d.enum_type.clone());
                serde_json::to_string_pretty(&mmr_logic::legacy::to_legacy(&file, &enum_type))?
            } else {
                mmr_logic::to_json(&file)
            };
            std::fs::write(&output, json + "\n").with_context(|| format!("writing {}", output.display()))?;
            println!("Wrote {} entries to {}", file.entries.len(), output.display());
        }
        Command::Logic(LogicCommand::Check { logic, all_tricks, verbose }) => check_logic(&logic, all_tricks, verbose)?,
        Command::Logic(LogicCommand::Regionize { input, output, text, scenarios }) => {
            let (old, _) = mmr_logic::load(&read(&input)?)?;
            let (regions, report) = mmr_logic::regionize::to_regions_auto(&old);
            let new = regions.compile();
            let game = mmr_data::game();
            let items: Vec<String> = game.items.iter().filter(|i| i.name.is_some()).map(|i| i.id.clone()).collect();
            let tricks: Vec<String> = old.entries.iter().filter(|e| e.trick.is_some()).map(|e| e.id.clone()).collect();
            let presets: Vec<_> = mmr_settings::presets().iter().map(|p| mmr_settings::apply_preset(p, None)).collect();
            let cases = mmr_logic::equivalence::scenarios(scenarios, 0xC0FFEE, &items, &tricks, &presets);
            let pairs: Vec<(String, String)> =
                old.entries.iter().map(|e| (e.id.clone(), report.replaced.get(&e.id).cloned().unwrap_or_else(|| e.id.clone()))).collect();
            let result = mmr_logic::equivalence::compare_pairs(&old, &new, &pairs, &cases);
            println!(
                "{} regions ({} replace old place entries), {} checks and {} events placed in regions, {} checks name no place",
                report.regions,
                report.equivalent_regions,
                report.locations_in_regions,
                report.events_in_regions,
                report.unmapped_locations.len()
            );
            if !result.is_equivalent() {
                let ids: std::collections::BTreeSet<&str> = result.mismatches.iter().map(|m| m.id.as_str()).collect();
                bail!("the converted logic behaves differently for {} entries, e.g. {:?}. Nothing was written.", ids.len(), ids.iter().take(5).collect::<Vec<_>>());
            }
            println!("Equivalent to the original on {} scenarios x {} entries.", result.scenarios, result.compared_ids);
            std::fs::write(&output, serde_json::to_string_pretty(&regions)? + "\n")?;
            println!("Wrote {}", output.display());
            if let Some(t) = text {
                std::fs::write(&t, mmr_logic::region::text::regions(&regions.regions))?;
                println!("Wrote {}", t.display());
            }
        }
        Command::Settings(SettingsCommand::Presets) => {
            for p in mmr_settings::presets() {
                let values = mmr_settings::apply_preset(&p, None);
                println!("{:<24} {:>3} changed settings", p.name, mmr_settings::changed(&values).len());
            }
        }
        Command::Settings(SettingsCommand::Import { file }) => {
            let imported = mmr_settings::legacy::import_settings_file(&read(&file)?)?;
            for key in &imported.skipped {
                eprintln!("skipped (no longer exists): {key}");
            }
            for issue in mmr_settings::validate(&imported.values) {
                eprintln!("warning: {}: {}", issue.key, issue.message);
            }
            println!("{}", mmr_settings::encode_settings_string(&imported.values, true));
        }
        Command::Settings(SettingsCommand::Decode { string }) => {
            let values = mmr_settings::decode_settings_string(&string)?;
            println!("{}", serde_json::to_string_pretty(&mmr_settings::changed(&values))?);
        }
        Command::Manual { output } => {
            std::fs::write(&output, mmr_build::manual::html())?;
            println!("Wrote {}", output.display());
        }
        Command::Stats { preset, count, from, style, set, json } => {
            let p = mmr_settings::presets().into_iter().find(|p| p.id == preset).with_context(|| format!("no preset called {preset}"))?;
            let mut values = mmr_settings::apply_preset(&p, None);
            for kv in &set {
                let (k, v) = kv.split_once('=').with_context(|| format!("--set {kv}: expected KEY=VALUE"))?;
                let value = serde_json::from_str(v).unwrap_or_else(|_| serde_json::Value::String(v.to_string()));
                values.insert(k.to_string(), value);
            }
            let logic_id = if values.get("LogicMode").and_then(|v| v.as_str()) == Some("Glitched") { "glitched" } else { "casual" };
            let logic = mmr_logic::bundled(logic_id).expect("bundled logic");
            let style = parse_style(&style)?;
            let (text, data) = stats::stats(&logic, &values, style, from, count.max(1));
            println!("{} · {text}", p.name);
            if let Some(path) = json {
                std::fs::write(&path, serde_json::to_string_pretty(&data)?)?;
                println!("Wrote {}", path.display());
            }
        }
        Command::Generate { preset, seed, style, output, rom, rom_out, plan_out, music, patch_out, spoiler_out, tracker_out, set, plando } => {
            let p = mmr_settings::presets().into_iter().find(|p| p.id == preset).with_context(|| format!("no preset called {preset}"))?;
            let mut values = mmr_settings::apply_preset(&p, None);
            for kv in &set {
                let (k, v) = kv.split_once('=').with_context(|| format!("--set {kv}: expected KEY=VALUE"))?;
                let value = serde_json::from_str(v).unwrap_or_else(|_| serde_json::Value::String(v.to_string()));
                values.insert(k.to_string(), value);
            }
            let logic_id = if values.get("LogicMode").and_then(|v| v.as_str()) == Some("Glitched") { "glitched" } else { "casual" };
            let logic = mmr_logic::bundled(logic_id).expect("bundled logic");
            let seed = seed.map(|s| mmr_randomizer::Seed::parse(&s)).unwrap_or_else(mmr_randomizer::Seed::random);
            let style = parse_style(&style)?;
            let started = std::time::Instant::now();
            if let Some(rom) = rom {
                let mut bytes = std::fs::read(&rom).with_context(|| format!("reading {}", rom.display()))?;
                mmr_rom::prepare(&mut bytes)?;
                let mut options = mmr_build::Options::default();
                for file in &plando {
                    let text = std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
                    options.plando.extend(mmr_randomizer::plando::parse(&text).map_err(anyhow::Error::msg)?);
                }
                if let Some(dir) = &music {
                    options.music = mmr_build::music::read_folder(dir).with_context(|| format!("reading music from {}", dir.display()))?;
                }
                let built = mmr_build::seed::build_seed(&bytes, &values, &logic, seed, style, &options)?;
                let path = rom_out.unwrap_or_else(|| rom.with_file_name(format!("MMRN-{}.z64", built.outcome.seed)));
                std::fs::write(&path, &built.rom)?;
                println!("{} · seed {} · built in {} ms · wrote {}", p.name, built.outcome.seed, started.elapsed().as_millis(), path.display());
                if !built.hash_icons.is_empty() {
                    println!("Hash: {}", built.hash_icons.iter().map(|i| i.name).collect::<Vec<_>>().join(" · "));
                }
                if let Some(out) = patch_out {
                    let info = serde_json::json!({ "app": "MMR Next", "version": env!("CARGO_PKG_VERSION"), "seed": built.outcome.seed.to_string(), "hashIcons": built.hash_icons.iter().map(|i| i.name).collect::<Vec<_>>() });
                    std::fs::write(&out, mmr_rom::patch::create(&bytes, &built.rom, info.to_string().as_bytes()))?;
                    println!("Wrote {}", out.display());
                }
                if spoiler_out.is_some() || tracker_out.is_some() {
                    let s = mmr_build::spoiler::build(&built.outcome, &built.plan, &values, &logic, mmr_build::spoiler::BuildInfo { hash: &built.hash, icons: &built.hash_icons, music: &built.music, item_icons: &built.item_icons });
                    if let Some(out) = spoiler_out {
                        let json = out.extension().is_some_and(|e| e.eq_ignore_ascii_case("json"));
                        std::fs::write(&out, if json { serde_json::to_string_pretty(&s)? } else { mmr_build::spoiler::text(&s) })?;
                        println!("Wrote {}", out.display());
                    }
                    if let Some(out) = tracker_out {
                        std::fs::write(&out, mmr_build::spoiler::html(&s))?;
                        println!("Wrote {}", out.display());
                    }
                }
                return Ok(());
            }
            let out = mmr_randomizer::generate(&logic, &values, seed, style)?;
            if let Some(path) = &plan_out {
                let plan = mmr_build::seed::plan_for(&out, &values, &logic);
                std::fs::write(path, serde_json::to_string_pretty(&plan)?)?;
            }
            println!("{} · seed {} · {:?} · attempt {} · {} ms", p.name, out.seed, out.style, out.attempt, started.elapsed().as_millis());
            for (i, sphere) in out.spheres.iter().enumerate() {
                println!("Sphere {}:", i + 1);
                for pl in sphere {
                    println!("  {:<32} {}", pl.item_name, pl.location_name);
                }
            }
            if let Some(path) = output {
                std::fs::write(&path, serde_json::to_string_pretty(&out)?)?;
                println!("Wrote {}", path.display());
            }
        }
        Command::ApplyPatch { rom, patch, output } => {
            let mut clean = std::fs::read(&rom).with_context(|| format!("reading {}", rom.display()))?;
            mmr_rom::prepare(&mut clean)?;
            let data = std::fs::read(&patch).with_context(|| format!("reading {}", patch.display()))?;
            let (out, info) = mmr_rom::patch::apply(&clean, &data)?;
            let path = output.unwrap_or_else(|| patch.with_extension("z64"));
            std::fs::write(&path, out)?;
            println!("{}", String::from_utf8_lossy(info));
            println!("Wrote {}", path.display());
        }
        Command::UiData { output } => {
            let g = mmr_data::game();
            let data = serde_json::json!({
                "game": { "items": g.items, "regions": g.regions, "categories": g.categories },
                "schema": {
                    "settings": mmr_settings::schema::schema().settings,
                    "categories": mmr_settings::schema::CATEGORIES,
                    "defaults": mmr_settings::defaults(),
                    "layout": serde_json::from_str::<serde_json::Value>(mmr_settings::schema::LAYOUT_JSON)?,
                },
                "presets": mmr_settings::presets(),
                "logic": { "casual": mmr_logic::bundled("casual"), "glitched": mmr_logic::bundled("glitched") },
                "regions": { "casual": mmr_logic::bundled_regions("casual"), "glitched": mmr_logic::bundled_regions("glitched") },
            });
            std::fs::write(&output, serde_json::to_string(&data)?)?;
            println!("Wrote {}", output.display());
        }
        Command::Rom { rom, output } => {
            let mut bytes = std::fs::read(&rom).with_context(|| format!("reading {}", rom.display()))?;
            match mmr_rom::prepare(&mut bytes) {
                Ok(info) => {
                    println!("OK: {} ({}), byte order: {:?}", info.header.name, info.header.region(), info.original_order);
                    if let Some(out) = output {
                        std::fs::write(&out, &bytes)?;
                        println!("Wrote {}", out.display());
                    }
                }
                Err(e) => bail!("{e}"),
            }
        }
    }
    Ok(())
}

fn read(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
}

fn check_logic(logic: &str, all_tricks: bool, verbose: bool) -> Result<()> {
    let file = match mmr_logic::bundled(logic) {
        Some(f) => f,
        None => mmr_logic::load(&read(Path::new(logic))?)?.0,
    };
    let known: HashSet<String> = mmr_settings::schema::schema().settings.iter().map(|s| s.key.clone()).collect();
    let issues = mmr_logic::validate::validate(&file, Some(&known));
    let mut counts = [0; 3];
    for issue in &issues {
        counts[issue.severity as usize] += 1;
        if issue.severity == Severity::Info && !verbose {
            continue;
        }
        let entry = issue.entry.as_deref().unwrap_or("-");
        println!("{:?} [{}] {entry}: {}", issue.severity, issue.code, issue.message);
    }
    println!("{} errors, {} warnings, {} notes", counts[0], counts[1], counts[2]);

    let mut scenario = mmr_logic::eval::Scenario { settings: mmr_settings::defaults(), ..Default::default() };
    if all_tricks {
        scenario.tricks = file.entries.iter().filter(|e| e.trick.is_some()).map(|e| e.id.clone()).collect();
    }
    let playthrough = mmr_logic::eval::Evaluator::new(&file).vanilla_playthrough(&scenario);
    println!(
        "Vanilla playthrough: {} in {} spheres; {} locations unreachable",
        if playthrough.beatable { "beatable" } else { "NOT beatable" },
        playthrough.spheres.len(),
        playthrough.unreachable.len()
    );
    if verbose {
        for l in &playthrough.unreachable {
            println!("  unreachable: {l}");
        }
    }
    if counts[0] > 0 || !playthrough.beatable {
        std::process::exit(1);
    }
    Ok(())
}
