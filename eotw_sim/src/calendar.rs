//! O calendário do ano (GDD 6.3).
//!
//! O ano do jogo é comprimido: cada mês dura poucos dias de jogo. A escala
//! sai das idades que o jogo já usa. O coelho amadurece aos 10 dias de jogo
//! e o coelho-bravo do sul de Espanha cria pela primeira vez aos 3–4 meses.
//! A raposa amadurece aos 30 dias e a raposa pode acasalar aos 10 meses
//! (Newsome & Coman 1989). As duas dão cerca de 3 dias de jogo por mês, por
//! isso um ano tem 36 dias.
//!
//! Para cada mês o calendário diz a estação, se os coelhos criam, em que fase
//! está a raposa e com que probabilidade uma raposa jovem dispersa. Os
//! números vêm de `research_notes/Calendário do coelho e da raposa/`. Por
//! agora ninguém os usa: os coelhos e as raposas continuam a comportar-se
//! igual todo o ano, e o calendário só prepara as regras que vão depender
//! dele (reprodução por época, dispersão, grupos).

use serde::{Deserialize, Serialize};
use crate::sim::TICKS_PER_DAY;

/// Os nomes dos meses, a começar em janeiro.
pub const MONTHS: [&str; 12] = [
    "janeiro", "fevereiro", "março", "abril", "maio", "junho", "julho", "agosto", "setembro",
    "outubro", "novembro", "dezembro",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Season {
    Primavera,
    Verao,
    Outono,
    Inverno,
}

impl Season {
    pub fn name(self) -> &'static str {
        match self {
            Season::Primavera => "primavera",
            Season::Verao => "verão",
            Season::Outono => "outono",
            Season::Inverno => "inverno",
        }
    }
}

/// Quanto criam os coelhos num mês.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Breeding {
    /// Nenhuma fêmea prenhe.
    Nao,
    /// Poucas fêmeas, no início ou no fim da época.
    Pouca,
    Sim,
    /// O pico da época.
    Pico,
}

/// A fase do ano da raposa.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoxPhase {
    /// Cio e acasalamento (o cio de cada fêmea dura 1–6 dias).
    Acasalamento,
    /// Gestação, 52–53 dias.
    Gestacao,
    /// Partos e crias dentro da toca (olhos abertos aos 12–14 dias).
    CriasNaToca,
    /// As crias saem da toca e deixam de mamar.
    CriasFora,
    /// As crias vivem com a família e aprendem a caçar.
    Familia,
    /// Os jovens deixam o território. O acasalamento seguinte acontece ainda
    /// nesta fase, por isso a fase de acasalamento sobrepõe-se a ela.
    Dispersao,
}

impl FoxPhase {
    pub fn name(self) -> &'static str {
        match self {
            FoxPhase::Acasalamento => "acasalamento",
            FoxPhase::Gestacao => "gestação",
            FoxPhase::CriasNaToca => "crias na toca",
            FoxPhase::CriasFora => "crias fora da toca",
            FoxPhase::Familia => "família",
            FoxPhase::Dispersao => "dispersão",
        }
    }
}

/// O que acontece em cada mês num tipo de clima. Os meses são os do
/// hemisfério norte; um clima do sul tem `hemisphere_shift` de 6.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Climate {
    pub name: String,
    /// Meses a somar para passar do calendário do norte ao local: 0 no norte,
    /// 6 no sul (onde a época de reprodução cai em agosto–março).
    pub hemisphere_shift: usize,
    pub rabbit_breeding: [Breeding; 12],
    pub fox_phase: [FoxPhase; 12],
    /// Probabilidade de uma raposa jovem macho dispersar durante o mês.
    pub fox_dispersal_male: [f32; 12],
    pub fox_dispersal_female: [f32; 12],
}

use Breeding::{Nao as N, Pico as P, Pouca as Q, Sim as S};
use FoxPhase::{
    Acasalamento as Ac, CriasFora as Cf, CriasNaToca as Ct, Dispersao as Di, Familia as Fa,
    Gestacao as Ge,
};

/// Probabilidade mensal de dispersar das raposas jovens de Bristol, de
/// janeiro a dezembro (Smith & Harris 1991, na tabela 6.1 de Devenish-Nelson
/// 2012). É zero de abril a setembro. No total dispersam cerca de 44% dos
/// machos e 23% das fêmeas. Em Doñana a dispersão também vai de outubro a
/// janeiro (Travaini et al. 1993), por isso a tabela serve também para a
/// Ibéria até haver números ibéricos.
const BRISTOL_MALE: [f32; 12] =
    [0.159, 0.102, 0.057, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.068, 0.102, 0.182];
const BRISTOL_FEMALE: [f32; 12] =
    [0.045, 0.045, 0.030, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.030, 0.030, 0.136];

impl Climate {
    /// Mediterrânico ibérico, num ano normal. Os coelhos criam de novembro a
    /// junho, com pico em março e abril (Gonçalves 2002 em Pancas; Delibes &
    /// Calderón e Soriguer no sudoeste). Num ano seco só de janeiro a março;
    /// num ano húmido desde outubro. As raposas de Doñana têm partos do fim
    /// de fevereiro a março e as crias saem da toca em maio. O cio fica em
    /// janeiro: no nordeste de Espanha a ovulação média é a 5 de fevereiro
    /// (Cavallini & Santini 1995), e partos no fim de fevereiro pedem
    /// acasalamentos no início de janeiro.
    pub fn iberia() -> Self {
        Self {
            name: "mediterrânico ibérico".into(),
            hemisphere_shift: 0,
            //                jan fev mar abr mai jun jul ago set out nov dez
            rabbit_breeding: [S, S, P, P, S, S, N, N, N, Q, S, S],
            fox_phase: [Ac, Ge, Ct, Ct, Cf, Cf, Fa, Fa, Fa, Di, Di, Di],
            fox_dispersal_male: BRISTOL_MALE,
            fox_dispersal_female: BRISTOL_FEMALE,
        }
    }

    /// Temperado oceânico (Cambridgeshire, Países Baixos). Nenhuma coelha
    /// prenhe de outubro a dezembro, pico de abril a junho (Myhill & Boyd
    /// 1987; Drees 1988). As raposas inglesas ovulam em média a 22 de
    /// janeiro e parem em março (Cavallini & Santini 1995).
    pub fn oceanic() -> Self {
        Self {
            name: "temperado oceânico".into(),
            hemisphere_shift: 0,
            rabbit_breeding: [Q, S, S, P, P, P, S, S, Q, N, N, N],
            fox_phase: [Ac, Ge, Ct, Ct, Cf, Cf, Fa, Fa, Fa, Di, Di, Di],
            fox_dispersal_male: BRISTOL_MALE,
            fox_dispersal_female: BRISTOL_FEMALE,
        }
    }

    /// Temperado continental (Bayreuth, Suécia). Os coelhos só criam de
    /// março a setembro, e um inverno frio atrasa o início (Rödel 2022;
    /// Andersson 1980). As raposas do centro da Suécia ovulam em fevereiro
    /// ou março, mais tarde quanto mais a norte (Cavallini & Santini 1995).
    pub fn continental() -> Self {
        Self {
            name: "temperado continental".into(),
            hemisphere_shift: 0,
            rabbit_breeding: [N, N, S, P, P, S, S, S, Q, N, N, N],
            fox_phase: [Di, Ac, Ge, Ct, Ct, Cf, Cf, Fa, Fa, Di, Di, Di],
            fox_dispersal_male: BRISTOL_MALE,
            fox_dispersal_female: BRISTOL_FEMALE,
        }
    }

    /// Temperado do hemisfério sul (Nova Zelândia, sudeste da Austrália).
    /// Em Wairarapa mais de 90% das coelhas estão prenhes de setembro a
    /// novembro (Gibb, White & Ward 1985); em Canterbury criam de agosto a
    /// março (Bell 1977). As raposas do Território da Capital Australiana
    /// ovulam em média a 15 de julho (Cavallini & Santini 1995). As tabelas
    /// estão em meses do norte e `hemisphere_shift` passa-as para o sul. A
    /// dispersão australiana no outono, aos 7–9 meses, só tem dados não
    /// publicados (Coman, em Newsome & Coman 1989), por isso a tabela de
    /// Bristol é deslocada seis meses.
    pub fn southern() -> Self {
        Self {
            name: "temperado do sul".into(),
            hemisphere_shift: 6,
            // Em meses do norte: o índice 2 (março) é setembro local.
            rabbit_breeding: [N, Q, P, P, P, S, S, S, Q, N, N, N],
            ..Self::oceanic()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CalendarParams {
    /// Dias de jogo em cada mês. Ver o comentário do módulo.
    pub days_per_month: u64,
    /// O mês em que a simulação começa (0 é janeiro). Com 60 dias de
    /// aquecimento (20 meses), começar em julho põe o jogador a chegar em
    /// março, no pico da época dos coelhos.
    pub start_month: u64,
    pub climate: Climate,
}

impl Default for CalendarParams {
    fn default() -> Self {
        Self {
            days_per_month: 3,
            start_month: 6,
            climate: Climate::iberia(),
        }
    }
}

/// Uma data do calendário.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Date {
    /// O ano, a começar em 1.
    pub year: u64,
    /// O mês local, 0 a 11 (0 é janeiro).
    pub month: usize,
    /// O dia do mês, a começar em 1.
    pub day: u64,
}

impl Date {
    pub fn month_name(&self) -> &'static str {
        MONTHS[self.month]
    }
}

impl CalendarParams {
    pub fn ticks_per_month(&self) -> u64 {
        self.days_per_month * TICKS_PER_DAY
    }

    pub fn days_per_year(&self) -> u64 {
        12 * self.days_per_month
    }

    /// A data no tick `tick`.
    pub fn date(&self, tick: u64) -> Date {
        let day = tick / TICKS_PER_DAY;
        let months = day / self.days_per_month + self.start_month;
        Date {
            year: months / 12 + 1,
            month: (months % 12) as usize,
            day: day % self.days_per_month + 1,
        }
    }

    /// O mês do calendário do norte que corresponde ao mês local.
    fn northern(&self, month: usize) -> usize {
        (month + self.climate.hemisphere_shift) % 12
    }

    /// A estação meteorológica: inverno de dezembro a fevereiro no norte, e
    /// de junho a agosto no sul.
    pub fn season(&self, month: usize) -> Season {
        match self.northern(month) {
            2..=4 => Season::Primavera,
            5..=7 => Season::Verao,
            8..=10 => Season::Outono,
            _ => Season::Inverno,
        }
    }

    pub fn rabbit_breeding(&self, month: usize) -> Breeding {
        self.climate.rabbit_breeding[self.northern(month)]
    }

    pub fn fox_phase(&self, month: usize) -> FoxPhase {
        self.climate.fox_phase[self.northern(month)]
    }

    /// A probabilidade de uma raposa jovem dispersar num tick deste mês, para
    /// que a probabilidade no mês todo seja a da tabela:
    /// `1 − (1 − p)^(1 / ticks_do_mês)`.
    pub fn fox_dispersal_per_tick(&self, month: usize, male: bool) -> f32 {
        let m = self.northern(month);
        let p = if male {
            self.climate.fox_dispersal_male[m]
        } else {
            self.climate.fox_dispersal_female[m]
        };
        1.0 - (1.0 - p).powf(1.0 / self.ticks_per_month() as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick_of(day: u64) -> u64 {
        day * TICKS_PER_DAY
    }

    #[test]
    fn o_ano_tem_36_dias_e_comeca_em_julho() {
        let c = CalendarParams::default();
        assert_eq!(c.days_per_year(), 36);
        let d = c.date(0);
        assert_eq!((d.year, d.month_name(), d.day), (1, "julho", 1));
        let d = c.date(tick_of(3) - 1);
        assert_eq!((d.month_name(), d.day), ("julho", 3), "a última hora de julho");
        assert_eq!(c.date(tick_of(3)).month_name(), "agosto");
        let d = c.date(tick_of(18));
        assert_eq!((d.year, d.month_name()), (2, "janeiro"), "seis meses depois é ano novo");
    }

    #[test]
    fn o_jogador_chega_em_marco() {
        let c = CalendarParams::default();
        let warmup = crate::sim::Config::default().warmup_days;
        let d = c.date(tick_of(warmup));
        assert_eq!(d.month_name(), "março");
        assert_eq!(c.season(d.month), Season::Primavera);
    }

    #[test]
    fn as_estacoes_trocam_no_sul() {
        let norte = CalendarParams::default();
        let sul = CalendarParams { climate: Climate::southern(), ..CalendarParams::default() };
        let janeiro = 0;
        assert_eq!(norte.season(janeiro), Season::Inverno);
        assert_eq!(sul.season(janeiro), Season::Verao);
        // O pico dos coelhos do sul cai em setembro–outubro locais.
        assert_eq!(sul.rabbit_breeding(8), Breeding::Pico);
        assert_eq!(sul.rabbit_breeding(5), Breeding::Nao, "junho é inverno no sul");
    }

    #[test]
    fn na_iberia_os_coelhos_param_no_verao() {
        let c = CalendarParams::default();
        let meses: Vec<&str> = (0..12)
            .filter(|&m| c.rabbit_breeding(m) == Breeding::Nao)
            .map(|m| MONTHS[m])
            .collect();
        assert_eq!(meses, ["julho", "agosto", "setembro"]);
        assert_eq!(c.rabbit_breeding(3), Breeding::Pico, "abril");
    }

    #[test]
    fn a_raposa_so_dispersa_de_outubro_a_marco() {
        let c = CalendarParams::default();
        for (m, nome) in MONTHS.iter().enumerate().take(9).skip(3) {
            assert_eq!(c.fox_dispersal_per_tick(m, true), 0.0, "{nome}");
        }
        assert!(c.fox_dispersal_per_tick(11, true) > c.fox_dispersal_per_tick(11, false));
        // Somados os meses, a tabela faz dispersar cerca de metade dos machos.
        // A proporção medida em Bristol é 44 ± 26% (Devenish-Nelson 2012).
        let fica: f32 = (0..12)
            .map(|m| (1.0 - c.fox_dispersal_per_tick(m, true)).powf(c.ticks_per_month() as f32))
            .product();
        assert!((0.45..0.55).contains(&fica), "ficam {fica}");
    }
}
