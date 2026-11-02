//! Corre a simulação do Echoes of the Wild em modo texto (sem gráficos).
//!
//! Exemplos:
//!   cargo run -p eotw_cli
//!   cargo run -p eotw_cli -- --seed 7 --days 200
//!   cargo run -p eotw_cli -- --seed 7 --days 200 --csv relatorio.csv
//!   cargo run -p eotw_cli -- --cabin
//!   cargo run --release -p eotw_cli -- --sweep
//!   cargo run --release -p eotw_cli -- --bench

use eotw_sim::{report, sweep, CabinParams, Config, Simulation, TICKS_PER_DAY};
use std::time::Instant;

fn print_help() {
    println!("eotw_cli: simulação do Echoes of the Wild em modo texto");
    println!();
    println!("Opções:");
    println!("  --seed N     seed do mundo (por omissão 42)");
    println!("  --days N     dias a simular depois do aquecimento (por omissão 120)");
    println!("  --csv FICH   guardar o relatório diário em CSV");
    println!("  --cabin      teste da cabana: tempo na zona com e sem comida");
    println!("  --sweep      varrer os parâmetros sensíveis com várias seeds");
    println!("  --bench      medir quanto tempo demora um tick, por tamanho de mapa");
    println!("  --seeds N    quantas seeds usar em --sweep e --cabin (por omissão 40 e 20)");
    println!("  --help       mostrar esta ajuda");
}

enum Mode {
    Run,
    Cabin,
    Sweep,
    Bench,
}

fn main() {
    let mut seed: u64 = 42;
    let mut days: u64 = 120;
    let mut seeds: Option<u64> = None;
    let mut csv_path: Option<String> = None;
    let mut mode = Mode::Run;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--seed" => {
                i += 1;
                seed = args.get(i).and_then(|v| v.parse().ok()).unwrap_or(seed);
            }
            "--days" => {
                i += 1;
                days = args.get(i).and_then(|v| v.parse().ok()).unwrap_or(days);
            }
            "--seeds" => {
                i += 1;
                seeds = args.get(i).and_then(|v| v.parse().ok());
            }
            "--csv" => {
                i += 1;
                csv_path = args.get(i).cloned();
            }
            "--cabin" => mode = Mode::Cabin,
            "--sweep" => mode = Mode::Sweep,
            "--bench" => mode = Mode::Bench,
            "--help" | "-h" => {
                print_help();
                return;
            }
            other => {
                eprintln!("Argumento desconhecido: {other}");
                print_help();
                std::process::exit(2);
            }
        }
        i += 1;
    }

    match mode {
        Mode::Run => run(seed, days, csv_path),
        Mode::Cabin => cabin(seeds.unwrap_or(20), days),
        Mode::Sweep => run_sweep(seeds.unwrap_or(40), days),
        Mode::Bench => bench(),
    }
}

fn run(seed: u64, days: u64, csv_path: Option<String>) {
    let mut sim = Simulation::warmed_up(Config {
        seed,
        ..Config::default()
    });
    let arrival = sim.day();
    println!(
        "Echoes of the Wild | seed {} | o jogador chega no dia {} | {} coelhos e {} raposas",
        seed,
        arrival,
        sim.rabbits.len(),
        sim.foxes.len()
    );
    println!();
    println!("            ---------- coelhos ----------   --- raposas ---");
    println!("dia  coelhos  nasc  fome  idade  caçados   raposas  nasc  mortes  biomassa");

    sim.run_days(days);

    let total = sim.stats.len();
    for (n, s) in sim.stats.iter().enumerate() {
        if s.day < arrival {
            continue;
        }
        if s.day % 10 == 0 || s.day == arrival || n + 1 == total {
            println!(
                "{:>3}  {:>7}  {:>4}  {:>4}  {:>5}  {:>7}   {:>7}  {:>4}  {:>6}  {:>8.2}",
                s.day,
                s.rabbits,
                s.births,
                s.deaths_hunger,
                s.deaths_age,
                s.deaths_predation,
                s.foxes,
                s.fox_births,
                s.fox_deaths_hunger + s.fox_deaths_age,
                s.mean_biomass
            );
        }
    }

    println!();
    if sim.events.is_empty() {
        println!("Sem acontecimentos importantes.");
    } else {
        println!("Acontecimentos (incluindo o aquecimento):");
        for e in &sim.events {
            println!("  {e}");
        }
    }

    println!();
    println!(
        "Mapa no dia {} (F = raposa; R = coelho; o = toca; ' .:*#' = vegetação, do vazio ao cheio):",
        sim.day()
    );
    print!("{}", report::ascii_map(&sim));

    println!();
    println!(
        "Migrações de segurança: {} coelhos, {} raposas",
        sim.migrations_rabbits, sim.migrations_foxes
    );

    if let Some(path) = csv_path {
        match std::fs::write(&path, report::to_csv(&sim.stats)) {
            Ok(()) => println!("\nRelatório guardado em {path}"),
            Err(e) => eprintln!("\nNão consegui guardar o CSV em {path}: {e}"),
        }
    }
}

/// Teste da cabana (GDD 18.6): as mesmas seeds com e sem comida, e o que
/// acontece na zona da cabana depois de o jogador chegar.
fn cabin(seeds: u64, days: u64) {
    println!("Teste da cabana | {seeds} seeds | {days} dias depois do aquecimento");
    println!();
    println!("seed   horas de coelhos (sem → com)   horas de raposas (sem → com)   cadeias");
    // [sem comida, com comida] × [entradas coelhos, entradas raposas, horas coelhos, horas raposas]
    let mut total = [[0usize; 4]; 2];
    let mut chains = 0;
    let mut chain_seeds = 0;
    // A primeira história com uma cadeia, para mostrar no fim.
    let mut story: Option<(u64, Vec<String>)> = None;
    for seed in 1..=seeds {
        let mut seed_chains = 0;
        let mut this = [[0usize; 4]; 2];
        for (k, food) in [0.0, 10.0].into_iter().enumerate() {
            let config = Config::default();
            let mut sim = Simulation::warmed_up(Config {
                seed,
                cabin: Some(CabinParams::centered(config.width, config.height, food)),
                ..config
            });
            let from = sim.stats.len();
            let lines_before = sim.cabin_log.lines.len();
            let chains_before = sim.cabin_log.chains;
            sim.run_days(days);
            if food > 0.0 {
                seed_chains = sim.cabin_log.chains - chains_before;
                if seed_chains > 0 && story.is_none() {
                    story = Some((seed, sim.cabin_log.lines[lines_before..].to_vec()));
                }
            }
            for s in &sim.stats[from..] {
                this[k][0] += s.cabin_visits_rabbits;
                this[k][1] += s.cabin_visits_foxes;
                this[k][2] += s.cabin_hours_rabbits;
                this[k][3] += s.cabin_hours_foxes;
            }
            for i in 0..4 {
                total[k][i] += this[k][i];
            }
        }
        chains += seed_chains;
        if seed_chains > 0 {
            chain_seeds += 1;
        }
        println!(
            "{seed:>4}   {:>12} → {:<12}   {:>10} → {:<10}   {seed_chains:>7}",
            this[0][2], this[1][2], this[0][3], this[1][3]
        );
    }
    let ratio = |i: usize| total[1][i] as f32 / total[0][i].max(1) as f32;
    println!();
    println!(
        "Horas na zona:  coelhos {} → {} (×{:.2}), raposas {} → {} (×{:.2})",
        total[0][2], total[1][2], ratio(2), total[0][3], total[1][3], ratio(3)
    );
    println!(
        "Entradas:       coelhos {} → {} (×{:.2}), raposas {} → {} (×{:.2})",
        total[0][0], total[1][0], ratio(0), total[0][1], total[1][1], ratio(1)
    );
    println!(
        "Cadeias coelho → raposa → caça (com comida): {chains}, em {chain_seeds} de {seeds} seeds"
    );

    if let Some((seed, lines)) = story {
        // Mostra o que aconteceu até à primeira cadeia, e um pouco depois.
        let end = lines
            .iter()
            .position(|l| l.contains("cadeia"))
            .map_or(lines.len(), |i| (i + 4).min(lines.len()));
        let start = end.saturating_sub(16);
        println!();
        println!("Registo da cabana (seed {seed}, com comida):");
        for l in &lines[start..end] {
            println!("  {l}");
        }
    }
}

fn run_sweep(seeds: u64, days: u64) {
    println!(
        "Varredura | {seeds} seeds | aquecimento + {days} dias | estável = 0 migrações e nunca menos de {} coelhos",
        sweep::MIN_RABBITS
    );
    println!();
    println!(
        "{:<28} {:>9} {:>10} {:>8} {:>8} {:>8} {:>8}  ",
        "variação", "migrações", "colapsos", "R mín", "R média", "F máx", "F média"
    );
    for (label, change) in sweep::standard() {
        let r = sweep::run(&label, seeds, days, change);
        println!(
            "{:<28} {:>9} {:>10} {:>8} {:>8.0} {:>8} {:>8.1}  {}",
            r.label,
            r.migrations,
            r.collapsed_seeds,
            r.min_rabbits,
            r.mean_rabbits,
            r.max_foxes,
            r.mean_foxes,
            if r.is_stable() { "estável" } else { "INSTÁVEL" }
        );
    }
}

/// Mede o tempo de um tick em mapas de vários tamanhos, com coelhos e tocas na
/// mesma densidade do mapa de 24x24. Corre depois do aquecimento, para medir o
/// mundo cheio e não o arranque. Só faz sentido em `--release`.
fn bench() {
    if cfg!(debug_assertions) {
        println!("Aviso: isto é uma compilação de debug; usa --release para números a sério.\n");
    }
    println!("Tempo por tick (1 hora de jogo), média de 10 dias depois do aquecimento");
    println!();
    println!("{:>8} {:>9} {:>9} {:>8} {:>12} {:>14}", "mapa", "células", "coelhos", "raposas", "µs por tick", "dias por seg.");
    for size in [24, 48, 96, 192] {
        let mut config = Config::default();
        sweep::sized_map(&mut config, size);
        config.initial_foxes = (config.initial_foxes * size * size / (24 * 24)).max(1);
        let mut sim = Simulation::warmed_up(config);
        let ticks = 10 * TICKS_PER_DAY;
        let start = Instant::now();
        sim.run_days(10);
        let per_tick = start.elapsed().as_secs_f64() / ticks as f64;
        println!(
            "{:>8} {:>9} {:>9} {:>8} {:>12.0} {:>14.0}",
            format!("{size}x{size}"),
            size * size,
            sim.rabbits.len(),
            sim.foxes.len(),
            per_tick * 1e6,
            1.0 / (per_tick * TICKS_PER_DAY as f64)
        );
    }
}
