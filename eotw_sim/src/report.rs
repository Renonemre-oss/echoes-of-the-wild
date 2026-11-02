//! Relatórios: CSV de populações e um mapa em texto.

use crate::sim::{DayStats, Simulation};

pub fn to_csv(stats: &[DayStats]) -> String {
    let mut out = String::from(
        "dia,coelhos,nascimentos,mortes_fome,mortes_idade,mortes_predacao,\
         raposas,nasc_raposas,mortes_fome_raposas,mortes_idade_raposas,migracoes,biomassa_media,\
         visitas_cabana_coelhos,visitas_cabana_raposas,horas_cabana_coelhos,horas_cabana_raposas\n",
    );
    for s in stats {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{:.3},{},{},{},{}\n",
            s.day,
            s.rabbits,
            s.births,
            s.deaths_hunger,
            s.deaths_age,
            s.deaths_predation,
            s.foxes,
            s.fox_births,
            s.fox_deaths_hunger,
            s.fox_deaths_age,
            s.migrations,
            s.mean_biomass,
            s.cabin_visits_rabbits,
            s.cabin_visits_foxes,
            s.cabin_hours_rabbits,
            s.cabin_hours_foxes
        ));
    }
    out
}

/// Desenha o mundo em texto. `F` = raposa, `R` = coelho (se estiverem na mesma
/// célula, ganha a raposa), `C` = cabana, `o` = toca vazia; os restantes símbolos mostram a vegetação:
/// espaço (quase nada), `.`, `:`, `*`, `#` (cheio).
pub fn ascii_map(sim: &Simulation) -> String {
    let w = &sim.world;
    let mut rabbits_per_cell = vec![0usize; w.width * w.height];
    for r in &sim.rabbits {
        rabbits_per_cell[r.y * w.width + r.x] += 1;
    }
    let mut foxes_per_cell = vec![0usize; w.width * w.height];
    for f in &sim.foxes {
        foxes_per_cell[f.y * w.width + f.x] += 1;
    }
    let mut out = String::new();
    for y in 0..w.height {
        for x in 0..w.width {
            let ch = if foxes_per_cell[y * w.width + x] > 0 {
                'F'
            } else if rabbits_per_cell[y * w.width + x] > 0 {
                'R'
            } else if sim.config().cabin.as_ref().is_some_and(|c| (c.x, c.y) == (x, y)) {
                'C'
            } else if w.is_burrow(x, y) {
                'o'
            } else {
                let ratio = w.biomass_at(x, y) / w.capacity();
                match ratio {
                    r if r < 0.10 => ' ',
                    r if r < 0.35 => '.',
                    r if r < 0.60 => ':',
                    r if r < 0.85 => '*',
                    _ => '#',
                }
            };
            out.push(ch);
        }
        out.push('\n');
    }
    out
}
