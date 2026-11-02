//! O jogo: a simulação do `eotw_sim` desenhada numa janela com Bevy (etapa 4).
//!
//! A simulação não sabe que o Bevy existe. Este programa só faz três coisas:
//! avança a simulação ao ritmo escolhido, desenha o que ela calculou e lê o
//! teclado. Toda a ecologia continua em `eotw_sim`.
//!
//! Exemplos:
//!   cargo run -p eotw_game
//!   cargo run -p eotw_game -- --seed 7 --cabin
//!   cargo run -p eotw_game -- --screenshot janela.png

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use eotw_sim::{CabinParams, Config, Simulation, TICKS_PER_DAY};

/// Tamanho de uma célula do mapa no ecrã, em píxeis.
const CELL: f32 = 26.0;

/// Largura reservada ao painel de texto, à esquerda do mapa.
const PANEL_WIDTH: f32 = 330.0;

/// Ritmos possíveis, em horas de jogo por segundo real.
const SPEEDS: [f32; 6] = [1.0, 4.0, 12.0, 24.0, 72.0, 240.0];

fn main() {
    let options = Options::from_args();
    let mut config = Config {
        seed: options.seed,
        ..Config::default()
    };
    if options.cabin {
        config.cabin = Some(CabinParams::centered(config.width, config.height, 10.0));
    }
    // O jogador chega depois do aquecimento, a um mundo já em equilíbrio.
    let sim = Simulation::warmed_up(config);

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Echoes of the Wild".into(),
                resolution: (960.0_f32, 720.0_f32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.10, 0.11, 0.10)))
        .insert_resource(World(sim))
        .insert_resource(Clock {
            speed: 3,
            paused: false,
            pending: 0.0,
        })
        .insert_resource(options)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (controls, advance, draw_tiles, draw_animals, draw_panel, screenshot).chain(),
        )
        .run();
}

/// Opções da linha de comandos.
#[derive(Resource, Clone)]
struct Options {
    seed: u64,
    cabin: bool,
    /// Guardar uma imagem da janela e sair (útil para o diário e para testes).
    screenshot: Option<String>,
}

impl Options {
    fn from_args() -> Self {
        let mut o = Options {
            seed: 42,
            cabin: false,
            screenshot: None,
        };
        let args: Vec<String> = std::env::args().skip(1).collect();
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--seed" => {
                    i += 1;
                    o.seed = args.get(i).and_then(|v| v.parse().ok()).unwrap_or(o.seed);
                }
                "--cabin" => o.cabin = true,
                "--screenshot" => {
                    i += 1;
                    o.screenshot = args.get(i).cloned();
                }
                other => eprintln!("Argumento desconhecido (ignorado): {other}"),
            }
            i += 1;
        }
        o
    }
}

/// A simulação, guardada como recurso do Bevy.
#[derive(Resource)]
struct World(Simulation);

/// O ritmo do tempo de jogo.
#[derive(Resource)]
struct Clock {
    /// Índice em `SPEEDS`.
    speed: usize,
    paused: bool,
    /// Fração de tick que ainda falta correr (o tempo real não chega em ticks certos).
    pending: f32,
}

/// Uma célula do mapa (o índice é `y * largura + x`).
#[derive(Component)]
struct Tile(usize);

/// Um coelho ou uma raposa desenhado neste frame.
#[derive(Component)]
struct AnimalSprite;

/// O texto com o dia, as populações e as teclas.
#[derive(Component)]
struct Panel;

/// O centro de uma célula no ecrã. A linha 0 do mapa fica em cima.
fn cell_to_screen(x: usize, y: usize, width: usize, height: usize) -> Vec2 {
    Vec2::new(
        (x as f32 + 0.5 - width as f32 / 2.0) * CELL,
        (height as f32 / 2.0 - y as f32 - 0.5) * CELL,
    )
}

/// A cor da vegetação, de terra nua (castanho) a erva cheia (verde).
fn vegetation_color(ratio: f32) -> Color {
    let r = ratio.clamp(0.0, 1.0);
    let bare = Vec3::new(0.42, 0.33, 0.22);
    let full = Vec3::new(0.16, 0.50, 0.18);
    let c = bare.lerp(full, r);
    Color::srgb(c.x, c.y, c.z)
}

fn setup(mut commands: Commands, world: Res<World>, assets: Res<AssetServer>) {
    // A câmara desvia-se para a esquerda, para o mapa ficar à direita do painel.
    commands.spawn((Camera2d, Transform::from_xyz(-PANEL_WIDTH / 2.0, 0.0, 0.0)));
    let sim = &world.0;
    let (w, h) = (sim.world.width, sim.world.height);
    for y in 0..h {
        for x in 0..w {
            let pos = cell_to_screen(x, y, w, h);
            // Um píxel de folga entre células desenha uma grelha discreta.
            commands.spawn((
                Sprite::from_color(Color::BLACK, Vec2::splat(CELL - 1.0)),
                Transform::from_xyz(pos.x, pos.y, 0.0),
                Tile(y * w + x),
            ));
            if sim.world.is_burrow(x, y) {
                commands.spawn((
                    Sprite::from_color(Color::srgb(0.20, 0.13, 0.08), Vec2::splat(CELL * 0.45)),
                    Transform::from_xyz(pos.x, pos.y, 1.0),
                ));
            }
        }
    }
    if let Some(c) = &sim.config().cabin {
        let pos = cell_to_screen(c.x, c.y, w, h);
        commands.spawn((
            Sprite::from_color(Color::srgb(0.55, 0.38, 0.22), Vec2::splat(CELL * 0.9)),
            Transform::from_xyz(pos.x, pos.y, 1.5),
        ));
    }
    // A fonte do Bevy não tem acentos ("Vegetação" aparecia com quadrados).
    // A DejaVu Sans tem, e a licença deixa-a ir com o jogo (ver assets/fonts).
    commands.spawn((
        Text::new(""),
        TextFont {
            font: assets.load("fonts/DejaVuSans.ttf"),
            font_size: 17.0,
            ..default()
        },
        TextColor(Color::srgb(0.92, 0.92, 0.88)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(14.0),
            width: Val::Px(PANEL_WIDTH - 28.0),
            ..default()
        },
        Panel,
    ));
}

fn controls(keys: Res<ButtonInput<KeyCode>>, mut clock: ResMut<Clock>) {
    if keys.just_pressed(KeyCode::Space) {
        clock.paused = !clock.paused;
    }
    if keys.any_just_pressed([KeyCode::Equal, KeyCode::NumpadAdd, KeyCode::ArrowUp]) {
        clock.speed = (clock.speed + 1).min(SPEEDS.len() - 1);
    }
    if keys.any_just_pressed([KeyCode::Minus, KeyCode::NumpadSubtract, KeyCode::ArrowDown]) {
        clock.speed = clock.speed.saturating_sub(1);
    }
}

/// Corre os ticks que couberem no tempo real que passou.
fn advance(time: Res<Time>, mut clock: ResMut<Clock>, mut world: ResMut<World>) {
    if clock.paused {
        return;
    }
    clock.pending += time.delta_secs() * SPEEDS[clock.speed];
    // Um limite por frame, para um frame lento não congelar a janela.
    let ticks = (clock.pending as u32).min(240);
    clock.pending -= ticks as f32;
    for _ in 0..ticks {
        world.0.step();
    }
}

fn draw_tiles(world: Res<World>, mut tiles: Query<(&Tile, &mut Sprite)>) {
    let w = &world.0.world;
    for (tile, mut sprite) in &mut tiles {
        let (x, y) = (tile.0 % w.width, tile.0 / w.width);
        sprite.color = vegetation_color(w.biomass_at(x, y) / w.capacity());
    }
}

/// Apaga os animais do frame anterior e desenha os de agora. São poucas
/// centenas, por isso não vale a pena complicar.
fn draw_animals(
    mut commands: Commands,
    world: Res<World>,
    old: Query<Entity, With<AnimalSprite>>,
) {
    for e in &old {
        commands.entity(e).despawn();
    }
    let sim = &world.0;
    let (w, h) = (sim.world.width, sim.world.height);
    for (i, r) in sim.rabbits.iter().enumerate() {
        // Vários coelhos na mesma célula ficam ligeiramente desencontrados.
        let jitter = Vec2::new((i % 3) as f32 - 1.0, ((i / 3) % 3) as f32 - 1.0) * CELL * 0.22;
        let pos = cell_to_screen(r.x, r.y, w, h) + jitter;
        commands.spawn((
            Sprite::from_color(Color::srgb(0.93, 0.90, 0.82), Vec2::splat(CELL * 0.28)),
            Transform::from_xyz(pos.x, pos.y, 2.0),
            AnimalSprite,
        ));
    }
    for f in &sim.foxes {
        let pos = cell_to_screen(f.x, f.y, w, h);
        commands.spawn((
            Sprite::from_color(Color::srgb(0.90, 0.45, 0.12), Vec2::splat(CELL * 0.6)),
            Transform::from_xyz(pos.x, pos.y, 3.0),
            AnimalSprite,
        ));
    }
}

fn draw_panel(world: Res<World>, clock: Res<Clock>, mut panel: Query<&mut Text, With<Panel>>) {
    let sim = &world.0;
    let Ok(mut text) = panel.single_mut() else {
        return;
    };
    let state = if clock.paused { "em pausa" } else { "a correr" };
    let cabin = if sim.config().cabin.is_some() {
        "\n  castanho claro: cabana"
    } else {
        ""
    };
    // As últimas linhas do registo da cabana, se houver cabana.
    let log = sim.cabin_log.lines.iter().rev().take(4).rev();
    let log: String = log.map(|l| format!("\n{l}")).collect();
    text.0 = format!(
        "Echoes of the Wild\n\n\
         Dia {}, {:02}h\n{state}, {} horas por segundo\n\n\
         Coelhos: {}\nRaposas: {}\nVegetação média: {:.1} de {:.0}\n\n\
         Espaço: pausa\n+ e -: velocidade\n\n\
         Legenda:\n  branco: coelho\n  laranja: raposa\n  castanho escuro: toca{cabin}\n  \
         verde a castanho: muita a pouca erva{}",
        sim.day(),
        sim.tick() % TICKS_PER_DAY,
        SPEEDS[clock.speed],
        sim.rabbits.len(),
        sim.foxes.len(),
        sim.world.mean_biomass(),
        sim.world.capacity(),
        if log.is_empty() { String::new() } else { format!("\n\nCabana:{log}") },
    );
}

/// Com `--screenshot`, espera uns frames (para a janela estar desenhada),
/// guarda a imagem e fecha.
fn screenshot(
    mut commands: Commands,
    options: Res<Options>,
    mut frames: Local<u32>,
    mut exit: EventWriter<AppExit>,
) {
    let Some(path) = &options.screenshot else {
        return;
    };
    *frames += 1;
    if *frames == 30 {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.clone()));
    }
    if *frames == 60 {
        exit.write(AppExit::Success);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mais_vegetacao_e_mais_verde() {
        let pouca = vegetation_color(0.1).to_srgba();
        let muita = vegetation_color(0.9).to_srgba();
        assert!(muita.green > pouca.green);
        assert!(muita.red < pouca.red);
    }

    #[test]
    fn a_linha_zero_fica_em_cima() {
        let topo = cell_to_screen(0, 0, 24, 24);
        let fundo = cell_to_screen(0, 23, 24, 24);
        assert!(topo.y > fundo.y);
        // O mapa fica centrado na origem.
        assert_eq!(cell_to_screen(0, 0, 24, 24).x, -cell_to_screen(23, 0, 24, 24).x);
    }
}
