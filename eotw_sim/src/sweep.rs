//! Varreduras de parâmetros: corre a mesma variação da configuração com muitas
//! seeds e resume o que aconteceu.
//!
//! Serve para saber não só *se* o equilíbrio funciona com os valores por
//! omissão, mas *quão longe* estão esses valores do ponto onde deixa de
//! funcionar. Um valor estável mas mesmo ao lado de um colapso é frágil.
//! Usado por `eotw_cli --sweep` e pelo teste `margens_de_equilibrio`.

use crate::sim::{Config, Simulation};

/// O resumo de uma variação, somado sobre todas as seeds.
#[derive(Clone, Debug)]
pub struct Outcome {
    pub label: String,
    pub seeds: usize,
    /// Ativações da migração de segurança (coelhos + raposas), incluindo o aquecimento.
    pub migrations: usize,
    /// Seeds em que os coelhos ou as raposas desapareceram (mesmo que tenham voltado
    /// por migração).
    pub collapsed_seeds: usize,
    /// O mínimo de coelhos visto depois do aquecimento, em todas as seeds.
    pub min_rabbits: usize,
    pub mean_rabbits: f32,
    pub max_foxes: usize,
    pub mean_foxes: f32,
}

impl Outcome {
    /// Estável: nunca foi precisa a migração e os coelhos nunca chegaram perto
    /// de zero depois de o jogador chegar.
    pub fn is_stable(&self) -> bool {
        self.migrations == 0 && self.min_rabbits >= MIN_RABBITS
    }

    /// Estável com o limite de migrações do GDD 18.6 ("se necessitar dela,
    /// o número de ativações fica abaixo de um limite definido"): uma por
    /// cada `MIGRATION_LIMIT_DAYS` dias de região. Com ≈7 raposas por região e
    /// uma ninhada por ano, uma região fechada pode perder as raposas por
    /// acaso; numa paisagem, as vizinhas repõem-nas.
    pub fn is_stable_within_limit(&self, days: u64) -> bool {
        let limit = (self.seeds as u64 * days) / MIGRATION_LIMIT_DAYS;
        self.migrations as u64 <= limit && self.min_rabbits >= MIN_RABBITS
    }
}

/// O limite de migrações de segurança: uma por cada tantos dias de região.
pub const MIGRATION_LIMIT_DAYS: u64 = 1000;

/// Abaixo disto, um número de coelhos já é uma quase extinção.
pub const MIN_RABBITS: usize = 20;

/// Corre `seeds` seeds (1, 2, ...) com a configuração por omissão alterada por
/// `change`. Cada seed faz o aquecimento e depois `days` dias.
pub fn run(label: &str, seeds: u64, days: u64, change: impl Fn(&mut Config)) -> Outcome {
    let mut out = Outcome {
        label: label.to_string(),
        seeds: seeds as usize,
        migrations: 0,
        collapsed_seeds: 0,
        min_rabbits: usize::MAX,
        mean_rabbits: 0.0,
        max_foxes: 0,
        mean_foxes: 0.0,
    };
    let mut counted_days = 0;
    for seed in 1..=seeds {
        let mut config = Config {
            seed,
            ..Config::default()
        };
        change(&mut config);
        let warmup = config.warmup_days as usize;
        let foxes_from = config.foxes_arrive_day as usize;
        let mut sim = Simulation::warmed_up(config);
        sim.run_days(days);

        out.migrations += sim.migrations_rabbits + sim.migrations_foxes;
        let after_arrival = &sim.stats[foxes_from.min(sim.stats.len())..];
        if after_arrival.iter().any(|d| d.rabbits == 0 || d.foxes == 0) {
            out.collapsed_seeds += 1;
        }
        for d in &sim.stats[warmup..] {
            out.min_rabbits = out.min_rabbits.min(d.rabbits);
            out.max_foxes = out.max_foxes.max(d.foxes);
            out.mean_rabbits += d.rabbits as f32;
            out.mean_foxes += d.foxes as f32;
            counted_days += 1;
        }
    }
    out.mean_rabbits /= counted_days as f32;
    out.mean_foxes /= counted_days as f32;
    out
}

/// Uma variação com nome, para a lista de varreduras.
pub type Variation = (String, Box<dyn Fn(&mut Config)>);

/// As varreduras de referência: cada parâmetro sensível, de um lado ao outro
/// do valor por omissão.
pub fn standard() -> Vec<Variation> {
    let mut list: Vec<Variation> = Vec::new();
    for h in [40.0, 45.0, 50.0, 55.0, 60.0, 62.5, 65.0, 67.5, 70.0, 75.0] {
        list.push((format!("hunt_below {h}"), Box::new(move |c| c.fox.hunt_below = h)));
    }
    for n in [1, 2, 3, 4, 6, 8] {
        list.push((format!("raposas iniciais {n}"), Box::new(move |c| c.initial_foxes = n)));
    }
    for d in [0, 10, 20, 30] {
        list.push((format!("raposas chegam no dia {d}"), Box::new(move |c| c.foxes_arrive_day = d)));
    }
    for b in [0, 10, 20, 30, 40] {
        list.push((format!("tocas {b}"), Box::new(move |c| c.burrows = b)));
    }
    for size in [12, 16, 20, 24, 32, 48] {
        list.push((format!("mapa {size}x{size}"), Box::new(move |c| sized_map(c, size))));
    }
    list
}

/// Um mapa quadrado com coelhos e tocas na mesma densidade do mapa de 24x24.
pub fn sized_map(c: &mut Config, size: usize) {
    let area = size * size;
    let base = c.width * c.height;
    c.initial_rabbits = (c.initial_rabbits * area / base).max(2);
    c.burrows = c.burrows * area / base;
    c.width = size;
    c.height = size;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fox::FoxParams;
    use crate::sim::CabinParams;
    use crate::sim::MIN_REGION_SIZE;

    /// Os valores por omissão não podem estar à beira de um colapso: cada
    /// parâmetro sensível tem de aguentar uma margem para os dois lados.
    /// Se este teste falhar, o equilíbrio ficou frágil, mesmo que
    /// `equilibrio_predador_presa` ainda passe. Usa `eotw_cli --sweep` para
    /// ver onde estão agora os limiares.
    #[test]
    fn margens_de_equilibrio() {
        let hunt = FoxParams::default().hunt_below;
        let foxes = Config::default().initial_foxes;
        let burrows = Config::default().burrows;
        let margens: Vec<Variation> = vec![
            ("hunt_below - 10".into(), Box::new(move |c| c.fox.hunt_below = hunt - 10.0)),
            ("hunt_below + 10".into(), Box::new(move |c| c.fox.hunt_below = hunt + 10.0)),
            ("1 raposa".into(), Box::new(|c| c.initial_foxes = 1)),
            ("3 raposas".into(), Box::new(|c| c.initial_foxes = 3)),
            ("o triplo".into(), Box::new(|c| c.initial_foxes = 30)),
            ("o triplo das raposas".into(), Box::new(move |c| c.initial_foxes = foxes * 3)),
            ("um terço das tocas".into(), Box::new(move |c| c.burrows = burrows / 3)),
            ("tamanho mínimo de região".into(), Box::new(|c| sized_map(c, MIN_REGION_SIZE))),
        ];
        let mut falhas = Vec::new();
        for (nome, change) in margens {
            let r = run(&nome, 12, 100, change);
            if !r.is_stable_within_limit(100) {
                falhas.push(format!("{nome}: {} migrações, mínimo de {} coelhos", r.migrations, r.min_rabbits));
            }
        }
        assert!(falhas.is_empty(), "margens que falham: {falhas:?}");
    }

    /// Muita comida junto à cabana não pode esterilizar as raposas. Já
    /// aconteceu de duas maneiras: as raposas acampadas comiam só uma dentada de
    /// cada vez e nunca chegavam à energia para ter crias; e, todas juntas, o
    /// travão de densidade via sempre rivais perto. Com 200 restos por dia
    /// ficavam 2 raposas e era precisa migração.
    #[test]
    fn muita_comida_nao_esteriliza_as_raposas() {
        for food in [50.0, 200.0] {
            let r = run("cabana com muita comida", 10, 300, move |c| {
                c.cabin = Some(CabinParams::centered(c.width, c.height, food));
            });
            assert!(r.mean_foxes >= 4.0, "{food} por dia: só {:.1} raposas em média", r.mean_foxes);
            assert!(r.is_stable_within_limit(300), "{food} por dia: {} migrações", r.migrations);
        }
    }

    /// O lado do tamanho de mapa que se sabe que falha: se um dia deixar de
    /// falhar, o tamanho mínimo de região pode descer.
    #[test]
    fn mapa_demasiado_pequeno_colapsa() {
        // Num mapa pequeno a simulação é rápida, por isso podem ser mais seeds
        // e mais dias: o colapso é raro e precisa de tempo para aparecer.
        // Com a noite, as raposas caçam menos horas por dia e o colapso
        // passou a precisar de mais tempo: aos 240 dias já não aparecia.
        let r = run("mapa 12x12", 40, 600, |c| sized_map(c, 12));
        assert!(!r.is_stable(), "o mapa de 12x12 ficou estável: rever MIN_REGION_SIZE");
    }
}

