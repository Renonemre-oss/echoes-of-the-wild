//! O jogo. O Bevy chega na etapa 4 do plano (protótipo visual).
//! Por agora, este programa só confirma que a ligação ao `eotw_sim` funciona.

use eotw_sim::{Config, Simulation};

fn main() {
    let mut sim = Simulation::new(Config::default());
    sim.run_days(1);
    println!("Echoes of the Wild: o jogo ainda não tem gráficos.");
    println!(
        "Mas a simulação já responde: ao fim de 1 dia há {} coelhos.",
        sim.rabbits.len()
    );
    println!("Para ver a simulação a sério: cargo run -p eotw_cli");
}
