//! O jogo: a paisagem do `eotw_sim` desenhada numa janela com Bevy.
//!
//! A simulação não sabe que o Bevy existe. Este programa só faz três coisas:
//! avança a simulação ao ritmo escolhido, desenha o que ela calculou e lê o
//! teclado. Toda a ecologia continua em `eotw_sim`.
//!
//! O mundo é uma paisagem de 5x5 regiões (120x120 células). A câmara segue
//! o jogador e só se desenha o que está à volta dele, que está sempre dentro
//! da janela simulada por inteiro (`Landscape`).
//!
//! Exemplos:
//!   cargo run -p eotw_game
//!   cargo run -p eotw_game -- --seed 7 --cabin
//!   cargo run -p eotw_game -- --screenshot janela.png
//!   cargo run -p eotw_game -- --smell   (começa com a vista do cheiro ligada)
//!   cargo run --release -p eotw_game -- --frames 600   (mede o tempo por frame e sai)
//!   cargo run --release -p eotw_game -- --speed 240     (começa a 240 horas por segundo)
//!   cargo run -p eotw_game -- --hour 22   (chega às 22h, já de noite)

use bevy::prelude::*;
use bevy::render::settings::{PowerPreference, RenderCreation, WgpuSettings};
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::render::RenderPlugin;
use eotw_sim::landscape::{Gather, Landscape, REGION, WINDOW_MARGIN};
use eotw_sim::{Breeding, Build, CabinParams, Config, Craft, Rabbit, ResourceKind, Scent, Simulation, Stage, Tool, TICKS_PER_DAY};

/// Tamanho de uma célula do mapa no ecrã, em píxeis.
const CELL: f32 = 26.0;

/// Largura reservada ao painel de texto, à esquerda do mapa.
const PANEL_WIDTH: f32 = 330.0;

/// Regiões de lado na paisagem.
const LANDSCAPE_REGIONS: usize = 5;

/// Células desenhadas à volta do jogador, de lado a lado e de cima a baixo.
/// Chegam para encher a parte da janela à direita do painel (630 x 720
/// píxeis, 25 x 28 células) e ficam dentro da janela simulada, que vai pelo
/// menos `WINDOW_MARGIN` (20) células para cada lado do jogador.
const VIEW_W: usize = 27;
const VIEW_H: usize = 31;
const _: () = assert!(VIEW_W / 2 <= WINDOW_MARGIN && VIEW_H / 2 <= WINDOW_MARGIN);

/// Tempo entre passos do jogador com a tecla carregada, em segundos.
const PLAYER_STEP_SECS: f32 = 0.12;

/// De noite, longe do jogador, as cores ficam com esta fração do brilho
/// (GDD 15.3: noites com pouca visibilidade, mas sem um ecrã preto).
const NIGHT_BRIGHTNESS: f32 = 0.35;

/// À volta do jogador vê-se melhor de noite: com luz quase plena até
/// `LANTERN_INNER` células, e a desaparecer até `LANTERN_OUTER`.
const LANTERN_INNER: f32 = 2.5;
const LANTERN_OUTER: f32 = 6.0;
const LANTERN_LIGHT: f32 = 0.8;

/// Ritmos possíveis, em horas de jogo por segundo real.
const SPEEDS: [f32; 6] = [1.0, 4.0, 12.0, 24.0, 72.0, 240.0];

fn main() {
    let options = Options::from_args();
    let smell = options.smell;
    let mut config = Config {
        seed: options.seed,
        ..Config::default()
    };
    let side = LANDSCAPE_REGIONS * REGION;
    if options.cabin {
        config.cabin = Some(CabinParams::centered(side, side, 10.0));
    }
    // O jogador chega depois do aquecimento, a um mundo já em equilíbrio,
    // ao lado da cabana (ou ao centro, se não houver cabana).
    let start = config.cabin.as_ref().map_or((side / 2, side / 2), |c| (c.x, c.y + 1));
    let mut land = Landscape::new(config, LANDSCAPE_REGIONS, LANDSCAPE_REGIONS, start);
    // O jogador chega à hora pedida (de manhã, por omissão).
    while land.window_sim().tick() % TICKS_PER_DAY != options.hour {
        land.step();
    }

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(render_plugin())
                .set(WindowPlugin {
            primary_window: Some(Window {
                title: "Echoes of the Wild".into(),
                resolution: (960.0_f32, 720.0_f32).into(),
                // A medir, sem esperar pelo ecrã: vê-se quanto o frame custa a sério.
                present_mode: if options.frames.is_some() {
                    bevy::window::PresentMode::AutoNoVsync
                } else {
                    bevy::window::PresentMode::AutoVsync
                },
                ..default()
            }),
            ..default()
        }),
        )
        .insert_resource(ClearColor(Color::srgb(0.10, 0.11, 0.10)))
        .insert_resource(World(land))
        .insert_resource(Clock {
            // O ritmo de `SPEEDS` mais perto do pedido com `--speed`.
            speed: (0..SPEEDS.len())
                .min_by(|&a, &b| (SPEEDS[a] - options.speed).abs().total_cmp(&(SPEEDS[b] - options.speed).abs()))
                .unwrap_or(3),
            paused: false,
            pending: 0.0,
        })
        .insert_resource(options.clone())
        .insert_resource(Walk { cooldown: 0.0 })
        .insert_resource(SmellView(smell))
        .init_resource::<GatherNote>()
        .init_resource::<Perf>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (controls, walk, gather, save_load, advance, follow, draw_tiles, draw_animals, draw_panel, screenshot, measure).chain(),
        )
        .run();
}

/// A placa gráfica a usar. Num portátil com duas, a integrada por omissão:
/// para um jogo 2D chega bem, e neste portátil a NVIDIA (MX250 com o
/// controlador NVK) dava um soluço de 33 ms a cada 20 frames, mesmo sem nada
/// para desenhar. A Intel integrada fica abaixo de 7 ms no pior frame. Com
/// `WGPU_POWER_PREF=high` usa-se a outra.
fn render_plugin() -> RenderPlugin {
    let mut settings = WgpuSettings::default();
    if std::env::var("WGPU_POWER_PREF").is_err() {
        settings.power_preference = PowerPreference::LowPower;
    }
    RenderPlugin {
        render_creation: RenderCreation::Automatic(settings),
        ..default()
    }
}

/// Opções da linha de comandos.
#[derive(Resource, Clone)]
struct Options {
    seed: u64,
    cabin: bool,
    /// Começar com a vista do cheiro ligada.
    smell: bool,
    /// Guardar uma imagem da janela e sair (útil para o diário e para testes).
    screenshot: Option<String>,
    /// Correr este número de frames, escrever quanto demoraram e sair.
    frames: Option<u32>,
    /// Horas de jogo por segundo ao começar (uma das de `SPEEDS`).
    speed: f32,
    /// A hora a que o jogador chega (0 a 23).
    hour: u64,
}

impl Options {
    fn from_args() -> Self {
        let mut o = Options {
            seed: 42,
            cabin: false,
            smell: false,
            screenshot: None,
            frames: None,
            speed: 24.0,
            hour: 7,
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
                "--smell" => o.smell = true,
                "--screenshot" => {
                    i += 1;
                    o.screenshot = args.get(i).cloned();
                }
                "--speed" => {
                    i += 1;
                    o.speed = args.get(i).and_then(|v| v.parse().ok()).unwrap_or(o.speed);
                }
                "--frames" => {
                    i += 1;
                    o.frames = args.get(i).and_then(|v| v.parse().ok());
                }
                "--hour" => {
                    i += 1;
                    o.hour = args.get(i).and_then(|v| v.parse().ok()).unwrap_or(o.hour) % TICKS_PER_DAY;
                }
                other => eprintln!("Argumento desconhecido (ignorado): {other}"),
            }
            i += 1;
        }
        o
    }
}

/// A paisagem, guardada como recurso do Bevy.
#[derive(Resource)]
struct World(Landscape);

/// O ritmo do tempo de jogo.
#[derive(Resource)]
struct Clock {
    /// Índice em `SPEEDS`.
    speed: usize,
    paused: bool,
    /// Fração de tick que ainda falta correr (o tempo real não chega em ticks certos).
    pending: f32,
}

/// Quanto tempo a simulação gastou no último frame (para o painel e `--frames`).
#[derive(Resource, Default)]
struct Perf {
    sim_ms: f32,
    /// O que o painel mostra: frames por segundo e o pior tempo da
    /// simulação, renovados duas vezes por segundo (mudar o texto em cada
    /// frame também custa).
    fps: f32,
    sim_peak: f32,
    frames: u32,
    elapsed: f32,
    peak: f32,
}

/// O ritmo a que o jogador anda com uma tecla carregada.
#[derive(Resource)]
struct Walk {
    /// Segundos até poder dar o próximo passo.
    cooldown: f32,
}

/// A vista do cheiro (tecla O): pinta o rasto do jogador a azul e o cheiro
/// de comida a amarelo por cima da vegetação (GDD 20.12, ver os campos de cheiro).
#[derive(Resource)]
struct SmellView(bool);

/// O que aconteceu na última recolha (tecla E), para o painel.
#[derive(Resource, Default)]
struct GatherNote(String);

/// Um dos quadrados desenhados à volta do jogador. O índice diz a posição
/// em relação a ele: coluna `i % VIEW_W` e linha `i / VIEW_W` da vista.
#[derive(Component)]
struct Tile(usize);

/// Um coelho, uma raposa ou o jogador, desenhado neste frame.
#[derive(Component)]
struct AnimalSprite;

/// O nome da parte do dia, pela luz e pela hora.
/// A data e o que os animais estão a fazer nesta altura do ano.
fn date_line(sim: &Simulation) -> String {
    let calendar = &sim.config().calendar;
    let d = sim.date();
    let coelhos = match calendar.rabbit_breeding(d.month) {
        Breeding::Nao => "não criam",
        Breeding::Pouca => "criam pouco",
        Breeding::Sim => "criam",
        Breeding::Pico => "pico das crias",
    };
    format!(
        "{} de {}, {}, ano {}\nCoelhos: {coelhos}; raposas: {}",
        d.day,
        d.month_name(),
        calendar.season(d.month).name(),
        d.year,
        calendar.fox_phase(d.month).name(),
    )
}

fn period(light: f32, hour: u64) -> &'static str {
    match light {
        l if l >= 1.0 => "dia",
        l if l <= 0.0 => "noite",
        _ if hour < 12 => "amanhecer",
        _ => "anoitecer",
    }
}

/// O texto com o dia, as populações e as teclas.
#[derive(Component)]
struct Panel;

/// O centro de uma célula da paisagem no ecrã. A linha 0 fica em cima, e
/// a câmara é que se mexe para seguir o jogador.
fn cell_to_screen(x: i64, y: i64) -> Vec2 {
    Vec2::new((x as f32 + 0.5) * CELL, -(y as f32 + 0.5) * CELL)
}

/// A célula da paisagem por baixo do quadrado `i` da vista, com o jogador ao centro.
fn tile_cell(i: usize, player: (usize, usize)) -> (i64, i64) {
    (
        player.0 as i64 + (i % VIEW_W) as i64 - (VIEW_W / 2) as i64,
        player.1 as i64 + (i / VIEW_W) as i64 - (VIEW_H / 2) as i64,
    )
}

/// A célula está à vista: a menos de meia vista do jogador.
fn on_screen((x, y): (i64, i64), player: (usize, usize)) -> bool {
    (x - player.0 as i64).unsigned_abs() as usize <= VIEW_W / 2
        && (y - player.1 as i64).unsigned_abs() as usize <= VIEW_H / 2
}

/// Uma célula da paisagem em coordenadas da janela simulada, se estiver nela.
fn in_window(land: &Landscape, (x, y): (i64, i64)) -> Option<(usize, usize)> {
    if x < 0 || y < 0 {
        return None;
    }
    land.in_window((x as usize, y as usize))
}

/// O brilho de uma célula a `d` células do jogador, com esta luz do dia:
/// 1 de dia; de noite, `NIGHT_BRIGHTNESS` longe dele e mais perto dele.
fn brightness(light: f32, d: f32) -> f32 {
    let near = ((LANTERN_OUTER - d) / (LANTERN_OUTER - LANTERN_INNER)).clamp(0.0, 1.0) * LANTERN_LIGHT;
    NIGHT_BRIGHTNESS + (1.0 - NIGHT_BRIGHTNESS) * light.max(near)
}

/// Escurece uma cor com este brilho, puxando-a um pouco para o azul: a
/// noite fica azul-escura e não cinzenta.
fn shade(color: Color, b: f32) -> Color {
    if b >= 1.0 {
        return color;
    }
    let c = color.to_srgba();
    let blue = (1.0 - b) * 0.12;
    Color::srgb(c.red * b, c.green * b, c.blue * b + blue)
}

/// A distância, em células, de uma célula ao jogador.
fn from_player((x, y): (i64, i64), player: (usize, usize)) -> f32 {
    let (dx, dy) = ((x - player.0 as i64) as f32, (y - player.1 as i64) as f32);
    (dx * dx + dy * dy).sqrt()
}

/// A cor da vegetação, de terra nua (castanho) a erva cheia (verde).
fn vegetation_color(ratio: f32) -> Color {
    let r = ratio.clamp(0.0, 1.0);
    let bare = Vec3::new(0.42, 0.33, 0.22);
    let full = Vec3::new(0.16, 0.50, 0.18);
    let c = bare.lerp(full, r);
    Color::srgb(c.x, c.y, c.z)
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn(Camera2d);
    // Os quadrados da vista: sempre os mesmos, e em cada frame mudam de
    // sítio e de cor para mostrar o que está à volta do jogador.
    for i in 0..VIEW_W * VIEW_H {
        // Um píxel de folga entre células desenha uma grelha discreta.
        commands.spawn((
            Sprite::from_color(Color::BLACK, Vec2::splat(CELL - 1.0)),
            Transform::default(),
            Tile(i),
        ));
    }
    // A fonte do Bevy não tem acentos ("Vegetação" aparecia com quadrados).
    // A DejaVu Sans tem, e a licença deixa-a ir com o jogo (ver assets/fonts).
    commands.spawn((
        Text::new(""),
        TextFont {
            font: assets.load("fonts/DejaVuSans.ttf"),
            font_size: 15.0,
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

fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut clock: ResMut<Clock>,
    mut smell: ResMut<SmellView>,
) {
    if keys.just_pressed(KeyCode::Space) {
        clock.paused = !clock.paused;
    }
    if keys.just_pressed(KeyCode::KeyO) {
        smell.0 = !smell.0;
    }
    if keys.any_just_pressed([KeyCode::Equal, KeyCode::NumpadAdd]) {
        clock.speed = (clock.speed + 1).min(SPEEDS.len() - 1);
    }
    if keys.any_just_pressed([KeyCode::Minus, KeyCode::NumpadSubtract]) {
        clock.speed = clock.speed.saturating_sub(1);
    }
}

/// Move o jogador com as setas ou WASD: um passo logo que se carrega, e
/// depois um passo a cada `PLAYER_STEP_SECS` enquanto a tecla estiver em baixo.
/// Anda em tempo real, mesmo com o jogo em pausa.
fn walk(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut walk: ResMut<Walk>,
    mut world: ResMut<World>,
) {
    let held = |a, b| keys.pressed(a) || keys.pressed(b);
    let pressed = |a, b| keys.just_pressed(a) || keys.just_pressed(b);
    let dirs = [
        (KeyCode::ArrowUp, KeyCode::KeyW, 0, -1),
        (KeyCode::ArrowDown, KeyCode::KeyS, 0, 1),
        (KeyCode::ArrowLeft, KeyCode::KeyA, -1, 0),
        (KeyCode::ArrowRight, KeyCode::KeyD, 1, 0),
    ];
    let (mut dx, mut dy): (i32, i32) = (0, 0);
    let mut fresh = false;
    for (a, b, x, y) in dirs {
        if held(a, b) {
            dx += x;
            dy += y;
        }
        fresh |= pressed(a, b);
    }
    walk.cooldown -= time.delta_secs();
    if (dx, dy) == (0, 0) {
        return;
    }
    if fresh || walk.cooldown <= 0.0 {
        world.0.move_player(dx.signum(), dy.signum());
        walk.cooldown = PLAYER_STEP_SECS;
    }
}

/// Recolhe com a tecla E da árvore ou da pedra onde o jogador está ou ao
/// lado, e faz o machado com 1 e a picareta com 2. Cada recolha e cada
/// fabrico gastam uma hora de jogo, mesmo com o jogo em pausa.
fn gather(keys: Res<ButtonInput<KeyCode>>, mut world: ResMut<World>, mut note: ResMut<GatherNote>) {
    if keys.just_pressed(KeyCode::KeyE) {
        note.0 = gather_note(world.0.gather());
    }
    for (key, tool) in [(KeyCode::Digit1, Tool::Axe), (KeyCode::Digit2, Tool::Pickaxe)] {
        if keys.just_pressed(key) {
            note.0 = craft_note(tool, world.0.craft(tool));
        }
    }
    if keys.just_pressed(KeyCode::KeyB) {
        let hours = world.0.window_sim().config().shelter.hours;
        note.0 = build_note(world.0.build(), hours);
    }
    if keys.just_pressed(KeyCode::KeyF) {
        note.0 = forage_note(world.0.forage());
    }
    if keys.just_pressed(KeyCode::KeyC) {
        note.0 = if world.0.eat() { "Comeste uma porção.".into() } else { "Não tens comida: procura-a na erva com F.".into() };
    }
}

/// Onde fica o mundo guardado, na pasta de onde se corre o jogo.
const SAVE_PATH: &str = "saves/mundo.json";

/// F5 guarda o mundo todo num ficheiro e F9 carrega-o (GDD 18.2,
/// Persistência). Só há um ficheiro: guardar outra vez substitui-o.
fn save_load(keys: Res<ButtonInput<KeyCode>>, mut world: ResMut<World>, mut note: ResMut<GatherNote>) {
    if keys.just_pressed(KeyCode::F5) {
        note.0 = match save_world(&world.0, std::path::Path::new(SAVE_PATH)) {
            Ok(()) => format!("Mundo guardado em {SAVE_PATH} (F9 para carregar)."),
            Err(e) => format!("Não consegui guardar: {e}"),
        };
    }
    if keys.just_pressed(KeyCode::F9) {
        note.0 = match load_world(std::path::Path::new(SAVE_PATH)) {
            Ok(land) => {
                world.0 = land;
                format!("Mundo carregado de {SAVE_PATH}.")
            }
            Err(e) => format!("Não consegui carregar: {e}"),
        };
    }
}

fn save_world(land: &Landscape, path: &std::path::Path) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, land.save()).map_err(|e| e.to_string())
}

fn load_world(path: &std::path::Path) -> Result<Landscape, String> {
    let text = std::fs::read_to_string(path).map_err(|_| "ainda não há nenhum mundo guardado (F5 guarda)".to_string())?;
    Landscape::load(&text)
}

/// A frase do painel para o resultado de procurar comida.
fn forage_note(result: Gather) -> String {
    match result {
        Gather::Got { .. } => "Encontraste comida: ervas, raízes e cogumelos (1 hora). C: comer.".into(),
        Gather::Full => "O inventário está cheio.".into(),
        _ => "Aqui há pouca erva para encontrar comida.".into(),
    }
}

/// A fome e a saúde, para o painel.
fn needs_line(land: &Landscape) -> String {
    let n = land.needs();
    let faints = if n.faints > 0 { format!(" (desmaiaste {} vezes)", n.faints) } else { String::new() };
    format!("Fome: {:.0} de 100, {}\nSaúde: {:.0} de 100{faints}", n.food, n.hunger_word(), n.health)
}

/// A frase do painel para o resultado de uma construção.
fn build_note(result: Build, hours: [u64; 4]) -> String {
    match result {
        Build::Built(stage) => format!("Fizeste {} ({} horas).", stage.with_article(), hours[stage as usize]),
        Build::Missing(stage, item, n) => format!("Para {} falta {n} de {}.", stage.with_article(), item.name()),
        Build::NeedsTree => "O primeiro abrigo encosta-se a uma árvore: põe-te ao lado de uma.".into(),
        Build::Blocked => "Aqui não se constrói: há uma árvore, uma pedra ou uma toca.".into(),
        Build::Elsewhere => "Já tens um abrigo: as melhorias fazem-se lá.".into(),
        Build::Finished => "O abrigo já tem lareira: está acabado.".into(),
    }
}

/// O abrigo e o que falta para a etapa seguinte, para o painel.
fn shelter_line(land: &Landscape) -> String {
    let p = &land.window_sim().config().shelter;
    let (now, next) = match land.shelter() {
        None => ("ainda não tens".to_string(), Some(Stage::Ramos)),
        Some(s) => (s.stage.name().to_string(), s.stage.next()),
    };
    let next = match next {
        None => String::new(),
        Some(stage) => {
            let cost: Vec<String> = p.cost_of(stage).iter().map(|(item, n)| format!("{n} de {}", item.name())).collect();
            format!(" (B: {}, {})", stage.name(), cost.join(" e "))
        }
    };
    format!("Abrigo: {now}{next}")
}

/// A frase do painel para o resultado de um fabrico.
fn craft_note(tool: Tool, result: Craft) -> String {
    match result {
        Craft::Made => format!("Fizeste {} (1 hora).", tool.with_article()),
        Craft::Missing(item, n) => format!("Para {} falta {n} de {}.", tool.with_article(), item.name()),
        Craft::AlreadyHave => format!("Já tens {}.", tool.with_article()),
    }
}

/// O que o jogador leva, para o painel.
fn inventory_line(land: &Landscape) -> String {
    let inv = land.inventory();
    let carry = land.window_sim().config().resources.carry;
    let items: Vec<String> = inv.iter().map(|(item, n)| format!("{n} de {}", item.name())).collect();
    let items = if items.is_empty() { "vazio".to_string() } else { items.join(", ") };
    let tools: Vec<String> = inv.tools().map(|(t, n)| format!("{} ({n} usos)", t.name())).collect();
    let tools = if tools.is_empty() { "nenhuma".to_string() } else { tools.join(", ") };
    format!("Inventário ({} de {carry}): {items}\nFerramentas: {tools}", inv.total())
}

/// A frase do painel para o resultado de uma recolha.
fn gather_note(result: Gather) -> String {
    match result {
        Gather::Got { item, amount, broke: None } => format!("Recolheste {amount} de {} (1 hora).", item.name()),
        Gather::Got { item, amount, broke: Some(tool) } => {
            let article = if tool == Tool::Axe { "O" } else { "A" };
            format!("Recolheste {amount} de {}. {article} {} partiu-se.", item.name(), tool.name())
        }
        Gather::Nothing => "Não há árvore nem pedra aqui ao lado, nem erva para folhas.".into(),
        Gather::Empty(item) => format!("Já não há {} aqui.", item.name()),
        Gather::Full => "O inventário está cheio.".into(),
    }
}

/// Corre os ticks que couberem no tempo real que passou.
fn advance(
    time: Res<Time>,
    mut clock: ResMut<Clock>,
    mut world: ResMut<World>,
    mut perf: ResMut<Perf>,
    mut note: ResMut<GatherNote>,
) {
    let start = std::time::Instant::now();
    if clock.paused {
        perf.sim_ms = 0.0;
        return;
    }
    clock.pending += time.delta_secs() * SPEEDS[clock.speed];
    // Um limite por frame, para um frame lento não congelar a janela.
    let ticks = (clock.pending as u32).min(240);
    clock.pending -= ticks as f32;
    let faints = world.0.needs().faints;
    for _ in 0..ticks {
        world.0.step();
    }
    if world.0.needs().faints > faints {
        note.0 = "Desmaiaste de fome. Acordas fraco: procura comida (F) e come (C).".into();
    }
    perf.sim_ms = start.elapsed().as_secs_f32() * 1000.0;
}

/// A câmara segue o jogador. Fica desviada para a esquerda, para o jogador
/// aparecer no meio da parte do ecrã à direita do painel.
fn follow(world: Res<World>, mut camera: Query<&mut Transform, With<Camera2d>>) {
    let (x, y) = world.0.player();
    let at = cell_to_screen(x as i64, y as i64);
    for mut t in &mut camera {
        t.translation.x = at.x - PANEL_WIDTH / 2.0;
        t.translation.y = at.y;
    }
}

fn draw_tiles(
    world: Res<World>,
    smell: Res<SmellView>,
    mut tiles: Query<(&Tile, &mut Sprite, &mut Transform, &mut Visibility)>,
) {
    let land = &world.0;
    let w = &land.window_sim().world;
    let player = land.player();
    let light = land.window_sim().light();
    for (tile, mut sprite, mut transform, mut visible) in &mut tiles {
        let cell = tile_cell(tile.0, player);
        let pos = cell_to_screen(cell.0, cell.1);
        // Só se mexe no que mudou: cada alteração obriga o Bevy a voltar a
        // enviar o sprite para a placa gráfica.
        transform.set_if_neq(Transform::from_translation(pos.extend(0.0)));
        // Fora da paisagem não há nada para desenhar.
        let Some((x, y)) = in_window(land, cell) else {
            visible.set_if_neq(Visibility::Hidden);
            continue;
        };
        visible.set_if_neq(Visibility::Visible);
        let mut color = if w.is_water(x, y) {
            WATER
        } else {
            vegetation_color(w.biomass_at(x, y) / w.capacity())
        };
        if smell.0 {
            let human = w.scent.at(Scent::Human, x, y);
            let food = w.scent.at(Scent::Food, x, y);
            color = smell_tint(color, human, food);
        }
        let color = shade(color, brightness(light, from_player(cell, player)));
        if sprite.color != color {
            sprite.color = color;
        }
    }
}

/// A cor do rio: azul-esverdeado claro, para não se confundir com o jogador
/// (azul) nem com o cheiro humano.
const WATER: Color = Color::srgb(0.30, 0.62, 0.72);

/// Mistura a cor da célula com azul (cheiro humano) e amarelo (comida).
/// O azul fica forte a partir de 1, perto do valor que já afasta as raposas.
fn smell_tint(base: Color, human: f32, food: f32) -> Color {
    let c = base.to_srgba();
    let mut rgb = Vec3::new(c.red, c.green, c.blue);
    rgb = rgb.lerp(Vec3::new(0.95, 0.85, 0.20), (food / 5.0).clamp(0.0, 0.85));
    rgb = rgb.lerp(Vec3::new(0.25, 0.45, 1.0), human.clamp(0.0, 0.85));
    Color::srgb(rgb.x, rgb.y, rgb.z)
}

/// Uma coisa a desenhar por cima dos quadrados: em que célula, com que
/// desvio dentro dela, cor, tamanho e camada.
struct Mark {
    cell: (i64, i64),
    offset: Vec2,
    color: Color,
    size: f32,
    z: f32,
}

/// Uma marca no centro da célula.
fn mark(cell: (i64, i64), color: Color, size: f32, z: f32) -> Mark {
    Mark { cell, offset: Vec2::ZERO, color, size, z }
}

/// Os coelhos que se veem, com o seu índice em `sim.rabbits`. Um coelho
/// dentro de uma toca está escondido, como para as raposas e para o
/// jogador na simulação: ao meio-dia, quando muitos descansam, veem-se menos.
fn visible_rabbits(sim: &Simulation) -> impl Iterator<Item = (usize, &Rabbit)> {
    sim.rabbits.iter().enumerate().filter(|(_, r)| !sim.world.is_burrow(r.x, r.y))
}

/// O que se desenha por cima dos quadrados (tocas, cabana, animais,
/// jogador): os sprites são sempre os mesmos, e em cada frame mudam de
/// sítio, cor e tamanho; os que sobram escondem-se. Antes, cada frame
/// apagava e criava uma entidade por cada coisa à vista, e isso, com
/// centenas de coelhos, levava a maior parte do frame (22 ms contra 5).
fn draw_animals(
    mut commands: Commands,
    world: Res<World>,
    mut pool: Query<(&mut Sprite, &mut Transform, &mut Visibility), With<AnimalSprite>>,
) {
    let land = &world.0;
    let sim = land.window_sim();
    let (ox, oy) = land.window_origin();
    let player = land.player();
    let global = |x: usize, y: usize| ((ox + x) as i64, (oy + y) as i64);
    let mut items: Vec<Mark> = Vec::new();
    for &(x, y) in sim.world.burrows() {
        items.push(mark(global(x, y), Color::srgb(0.20, 0.13, 0.08), CELL * 0.45, 1.0));
    }
    if let Some(c) = land.cabin() {
        items.push(mark((c.x as i64, c.y as i64), Color::srgb(0.55, 0.38, 0.22), CELL * 0.9, 1.5));
    }
    // O abrigo cresce e escurece a cada etapa; com lareira fica avermelhado.
    if let Some(s) = land.shelter() {
        let (color, size) = match s.stage {
            Stage::Ramos => (Color::srgb(0.62, 0.52, 0.30), 0.55),
            Stage::Folhas => (Color::srgb(0.70, 0.55, 0.20), 0.7),
            Stage::BasePedra => (Color::srgb(0.50, 0.46, 0.40), 0.85),
            Stage::Lareira => (Color::srgb(0.72, 0.30, 0.15), 0.95),
        };
        items.push(mark((s.x as i64, s.y as i64), color, CELL * size, 1.5));
    }
    // As árvores e as pedras encolhem à medida que se recolhem. Uma árvore
    // cortada fica pequena até voltar a crescer; uma pedra gasta desaparece.
    let rp = &sim.config().resources;
    for r in sim.world.resources() {
        let (color, full) = match r.kind {
            ResourceKind::Tree => (Color::srgb(0.05, 0.28, 0.10), r.amount / rp.tree_wood),
            ResourceKind::Rock if r.amount < 1.0 => continue,
            ResourceKind::Rock => (Color::srgb(0.55, 0.55, 0.58), r.amount / rp.rock_stone),
        };
        items.push(mark(global(r.x, r.y), color, CELL * (0.3 + 0.55 * full), 1.2));
    }
    for (i, r) in visible_rabbits(sim) {
        // Vários coelhos na mesma célula ficam ligeiramente desencontrados.
        let jitter = Vec2::new((i % 3) as f32 - 1.0, ((i / 3) % 3) as f32 - 1.0) * CELL * 0.22;
        items.push(Mark {
            offset: jitter,
            ..mark(global(r.x, r.y), Color::srgb(0.93, 0.90, 0.82), CELL * 0.28, 2.0)
        });
    }
    for f in &sim.foxes {
        items.push(mark(global(f.x, f.y), Color::srgb(0.90, 0.45, 0.12), CELL * 0.6, 3.0));
    }
    // De noite, o que está longe do jogador vê-se mal.
    let light = sim.light();
    for m in &mut items {
        m.color = shade(m.color, brightness(light, from_player(m.cell, player)));
    }
    // O jogador: um quadrado azul com contorno claro, para se ver bem.
    let me = (player.0 as i64, player.1 as i64);
    items.push(mark(me, Color::srgb(0.95, 0.95, 1.0), CELL * 0.8, 4.0));
    items.push(mark(me, Color::srgb(0.20, 0.45, 0.95), CELL * 0.62, 4.1));
    items.retain(|m| on_screen(m.cell, player));

    let mut sprites = pool.iter_mut();
    for &Mark { cell, offset, color, size, z } in &items {
        let pos = (cell_to_screen(cell.0, cell.1) + offset).extend(z);
        match sprites.next() {
            Some((mut sprite, mut transform, mut visible)) => {
                let size = Some(Vec2::splat(size));
                if sprite.color != color || sprite.custom_size != size {
                    sprite.color = color;
                    sprite.custom_size = size;
                }
                transform.set_if_neq(Transform::from_translation(pos));
                visible.set_if_neq(Visibility::Visible);
            }
            // Faltam sprites: criam-se mais (aparecem no frame seguinte).
            None => {
                commands.spawn((Sprite::from_color(color, Vec2::splat(size)), Transform::from_translation(pos), AnimalSprite));
            }
        }
    }
    for (_, _, mut visible) in sprites {
        visible.set_if_neq(Visibility::Hidden);
    }
}

fn draw_panel(
    world: Res<World>,
    note: Res<GatherNote>,
    clock: Res<Clock>,
    smell: Res<SmellView>,
    perf: Res<Perf>,
    mut panel: Query<&mut Text, With<Panel>>,
) {
    let land = &world.0;
    let sim = land.window_sim();
    let Ok(mut text) = panel.single_mut() else {
        return;
    };
    let state = if clock.paused { "em pausa" } else { "a correr" };
    let cabin = if land.cabin().is_some() {
        "\n  castanho claro: cabana"
    } else {
        ""
    };
    let totals = land.totals();
    // As últimas linhas do registo da cabana, se houver cabana.
    let log = sim.cabin_log.lines.iter().rev().take(3).rev();
    let log: String = log.map(|l| format!("\n{l}")).collect();
    let new = format!(
        "Echoes of the Wild\n\n\
         Dia {}, {:02}h, {}\n{}\n{state}, {} horas por segundo\n\
         {:.0} frames por segundo, simulação até {:.1} ms\n\n\
         Região {}, {} (de {} x {})\n\
         À volta (3x3 regiões): {} coelhos, {} raposas\n\
         No mundo: {} coelhos, {} raposas\n\
         Vegetação média à volta: {:.1} de {:.0}\n\n\
         {}\n{}\n{}{}\n\n\
         Setas ou WASD: andar\nE: recolher (1 hora; na erva dá folhas)\nF: procurar comida na erva (1 hora)\nC: comer\nF5: guardar o mundo; F9: carregar\n1: machado (3 madeira, 2 pedra)\n2: picareta (2 madeira, 3 pedra)\nB: construir ou melhorar o abrigo\nEspaço: pausa; + e -: velocidade\nO: ver o cheiro ({})\n\n\
         Legenda:\n  quadrado azul: tu\n  bege a ruivo: o teu abrigo\n  branco: coelho\n  laranja: raposa\n  castanho escuro: toca{cabin}\n  \
         verde escuro: árvore (abrigo)\n  cinzento: pedra\n  azul-esverdeado: rio (os animais só passam nos vaus)\n  \
         verde a castanho: muita a pouca erva{}",
        sim.day(),
        sim.tick() % TICKS_PER_DAY,
        period(sim.light(), sim.tick() % TICKS_PER_DAY),
        date_line(sim),
        SPEEDS[clock.speed],
        perf.fps,
        perf.sim_peak,
        land.player().0 / REGION,
        land.player().1 / REGION,
        land.cols,
        land.rows,
        sim.rabbits.len(),
        sim.foxes.len(),
        totals.0,
        totals.1,
        sim.world.mean_biomass(),
        sim.world.capacity(),
        needs_line(land),
        inventory_line(land),
        shelter_line(land),
        if note.0.is_empty() { String::new() } else { format!("\n{}", note.0) },
        if smell.0 { "ligado: azul é o teu rasto, amarelo a comida" } else { "desligado" },
        if log.is_empty() { String::new() } else { format!("\n\nCabana:{log}") },
    );
    // Refazer o texto obriga a voltar a compor as letras: só quando muda.
    if text.0 != new {
        text.0 = new;
    }
}

/// Conta os frames por segundo para o painel. Com `--frames N`, também
/// conta o tempo de cada frame e, ao fim de N, escreve a
/// mediana e o pior e sai. Os primeiros 30 frames não contam (a janela ainda
/// está a abrir). `sim` é o tempo gasto a avançar a simulação.
fn measure(
    options: Res<Options>,
    time: Res<Time<Real>>,
    mut perf: ResMut<Perf>,
    mut samples: Local<Vec<(f32, f32)>>,
    mut exit: EventWriter<AppExit>,
) {
    perf.frames += 1;
    perf.elapsed += time.delta_secs();
    perf.peak = perf.peak.max(perf.sim_ms);
    if perf.elapsed >= 0.5 {
        perf.fps = perf.frames as f32 / perf.elapsed;
        perf.sim_peak = perf.peak;
        (perf.frames, perf.elapsed, perf.peak) = (0, 0.0, 0.0);
    }
    let Some(n) = options.frames else {
        return;
    };
    samples.push((time.delta_secs() * 1000.0, perf.sim_ms));
    if samples.len() as u32 == n + 30 {
        let mut frame: Vec<f32> = samples[30..].iter().map(|s| s.0).collect();
        let mut sim: Vec<f32> = samples[30..].iter().map(|s| s.1).collect();
        frame.sort_by(f32::total_cmp);
        sim.sort_by(f32::total_cmp);
        let at = |v: &[f32], q: f32| v[((v.len() - 1) as f32 * q) as usize];
        println!(
            "{n} frames: frame mediana {:.1} ms ({:.0} fps), p95 {:.1} ms, pior {:.1} ms | simulação mediana {:.2} ms, pior {:.2} ms",
            at(&frame, 0.5),
            1000.0 / at(&frame, 0.5),
            at(&frame, 0.95),
            at(&frame, 1.0),
            at(&sim, 0.5),
            at(&sim, 1.0)
        );
        exit.write(AppExit::Success);
    }
}

/// Com `--screenshot`, espera 2 segundos (para a janela estar desenhada e
/// a letra carregada), guarda a imagem e fecha. Contar frames não chegava:
/// em release, 30 frames passavam antes de a letra do painel carregar.
fn screenshot(
    mut commands: Commands,
    options: Res<Options>,
    time: Res<Time<Real>>,
    mut taken: Local<Option<f32>>,
    mut exit: EventWriter<AppExit>,
) {
    let Some(path) = &options.screenshot else {
        return;
    };
    let now = time.elapsed_secs();
    match *taken {
        None if now >= 2.0 => {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path.clone()));
            *taken = Some(now);
        }
        Some(at) if now >= at + 1.0 => {
            exit.write(AppExit::Success);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Uma paisagem pequena (1 região), para os testes serem rápidos.
    fn paisagem(jogador: (usize, usize)) -> Landscape {
        Landscape::new(Config::default(), 1, 1, jogador)
    }

    #[test]
    fn mais_vegetacao_e_mais_verde() {
        let pouca = vegetation_color(0.1).to_srgba();
        let muita = vegetation_color(0.9).to_srgba();
        assert!(muita.green > pouca.green);
        assert!(muita.red < pouca.red);
    }

    /// Corre o sistema `walk` uma vez com estas teclas carregadas e devolve
    /// onde fica o jogador, que começa em (5, 5).
    fn andar_com(teclas: &[KeyCode]) -> (usize, usize) {
        let mut app = App::new();
        let mut input = ButtonInput::<KeyCode>::default();
        for &k in teclas {
            input.press(k);
        }
        app.insert_resource(input)
            .insert_resource(Time::<()>::default())
            .insert_resource(Walk { cooldown: 0.0 })
            .insert_resource(World(paisagem((5, 5))))
            .add_systems(Update, walk);
        app.update();
        app.world().resource::<World>().0.player()
    }

    #[test]
    fn as_teclas_mexem_o_jogador() {
        assert_eq!(andar_com(&[KeyCode::KeyW]), (5, 4));
        assert_eq!(andar_com(&[KeyCode::ArrowDown]), (5, 6));
        assert_eq!(andar_com(&[KeyCode::KeyA]), (4, 5));
        assert_eq!(andar_com(&[KeyCode::ArrowRight]), (6, 5));
        // Duas teclas ao mesmo tempo: diagonal.
        assert_eq!(andar_com(&[KeyCode::KeyW, KeyCode::KeyA]), (4, 4));
        assert_eq!(andar_com(&[KeyCode::ArrowDown, KeyCode::ArrowRight]), (6, 6));
        // Teclas opostas anulam-se.
        assert_eq!(andar_com(&[KeyCode::KeyW, KeyCode::KeyS]), (5, 5));
    }

    #[test]
    fn a_tecla_o_liga_e_desliga_o_cheiro() {
        let mut app = App::new();
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::KeyO);
        app.insert_resource(input)
            .insert_resource(Clock {
                speed: 3,
                paused: false,
                pending: 0.0,
            })
            .insert_resource(SmellView(false))
            .add_systems(Update, controls);
        app.update();
        assert!(app.world().resource::<SmellView>().0, "devia ter ligado");
    }

    /// A tecla E recolhe ao lado de uma árvore ou pedra, e o painel diz o
    /// que aconteceu. Longe de tudo, diz que não há nada.
    #[test]
    fn a_tecla_e_recolhe() {
        let mut land = paisagem((12, 12));
        let r = land.window_sim().world.resources()[0].clone();
        let (ox, oy) = land.window_origin();
        // Anda até em cima do recurso.
        while land.player() != (ox + r.x, oy + r.y) {
            let p = land.player();
            let dx = ((ox + r.x) as i32 - p.0 as i32).signum();
            let dy = if dx == 0 { ((oy + r.y) as i32 - p.1 as i32).signum() } else { 0 };
            land.move_player(dx, dy);
        }
        let mut app = App::new();
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::KeyE);
        app.insert_resource(input)
            .insert_resource(World(land))
            .init_resource::<GatherNote>()
            .add_systems(Update, gather);
        app.update();
        let item = r.kind.item();
        assert_eq!(app.world().resource::<World>().0.inventory().count(item), 1);
        assert_eq!(app.world().resource::<GatherNote>().0, format!("Recolheste 1 de {} (1 hora).", item.name()));
        // Com uma pedra ou uma madeira não chega para um machado.
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::Digit1);
        app.insert_resource(input);
        app.update();
        assert!(app.world().resource::<GatherNote>().0.starts_with("Para um machado falta"));
        assert!(inventory_line(&app.world().resource::<World>().0).starts_with("Inventário (1 de 20)"));
    }

    /// F procura comida na erva e C come-a; o painel mostra a fome.
    #[test]
    fn as_teclas_f_e_c_dao_de_comer() {
        let land = paisagem((12, 12));
        let (ox, oy) = land.window_origin();
        let p = land.player();
        let fome = land.needs().food;
        let w = land.window_sim().world.biomass_at(p.0 - ox, p.1 - oy);
        assert!(w >= land.window_sim().config().needs.forage_biomass, "o jogador começa na erva");
        let mut app = App::new();
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::KeyF);
        app.insert_resource(input)
            .insert_resource(World(land))
            .init_resource::<GatherNote>()
            .add_systems(Update, gather);
        app.update();
        assert!(app.world().resource::<GatherNote>().0.starts_with("Encontraste comida"));
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::KeyC);
        app.insert_resource(input);
        app.update();
        assert_eq!(app.world().resource::<GatherNote>().0, "Comeste uma porção.");
        let land = &app.world().resource::<World>().0;
        assert!(land.needs().food > fome);
        assert!(needs_line(land).starts_with("Fome: "));
    }

    /// O que se guarda num ficheiro volta igual ao carregar.
    #[test]
    fn guardar_e_carregar_um_ficheiro() {
        let dir = std::env::temp_dir().join(format!("eotw_teste_{}", std::process::id()));
        let path = dir.join("mundo.json");
        assert!(load_world(&path).err().unwrap().contains("ainda não há"));
        let land = paisagem((12, 12));
        save_world(&land, &path).unwrap();
        let Ok(de_novo) = load_world(&path) else { panic!("não carregou") };
        assert_eq!(de_novo.save(), land.save());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn o_painel_explica_a_recolha() {
        use eotw_sim::Item;
        assert_eq!(gather_note(Gather::Nothing), "Não há árvore nem pedra aqui ao lado, nem erva para folhas.");
        assert_eq!(gather_note(Gather::Empty(Item::Wood)), "Já não há madeira aqui.");
        assert_eq!(gather_note(Gather::Full), "O inventário está cheio.");
        let partiu = Gather::Got { item: Item::Wood, amount: 2, broke: Some(Tool::Axe) };
        assert_eq!(gather_note(partiu), "Recolheste 2 de madeira. O machado partiu-se.");
        assert_eq!(craft_note(Tool::Pickaxe, Craft::Made), "Fizeste uma picareta (1 hora).");
        assert_eq!(craft_note(Tool::Axe, Craft::AlreadyHave), "Já tens um machado.");
        assert!(inventory_line(&paisagem((12, 12))).ends_with("vazio\nFerramentas: nenhuma"));
    }

    /// O jogador a andar à velocidade normal do jogo (cerca de 8 células por
    /// segundo, com o tempo a 24 horas por segundo) deixa um rasto que se vê.
    #[test]
    fn o_rasto_de_quem_anda_fica_azul() {
        let mut sim = Simulation::new(Config::default());
        // Em cada tick o jogador anda 1 ou 0 células (8 células por 3 ticks).
        for t in 0..24usize {
            sim.place_player(Some((2 + t / 3, 10)));
            sim.step();
        }
        let base = vegetation_color(0.5).to_srgba();
        for x in 2..9 {
            let human = sim.world.scent.at(Scent::Human, x, 10);
            let tinted = smell_tint(vegetation_color(0.5), human, 0.0).to_srgba();
            assert!(tinted.blue > base.blue + 0.3, "célula ({x}, 10): cheiro {human:.2}, pouco azul");
        }
    }

    #[test]
    fn o_cheiro_humano_pinta_de_azul() {
        let base = vegetation_color(0.5);
        let sem = smell_tint(base, 0.0, 0.0).to_srgba();
        let com = smell_tint(base, 1.0, 0.0).to_srgba();
        assert_eq!(sem, base.to_srgba(), "sem cheiro, a cor não muda");
        assert!(com.blue > sem.blue + 0.3);
    }

    #[test]
    fn a_linha_zero_fica_em_cima() {
        assert!(cell_to_screen(0, 0).y > cell_to_screen(0, 23).y);
        assert!(cell_to_screen(0, 0).x < cell_to_screen(1, 0).x);
    }

    /// De dia vê-se tudo. De noite, longe do jogador, as cores ficam
    /// escuras mas não pretas, e perto dele quase como de dia.
    #[test]
    fn a_noite_escurece_longe_do_jogador() {
        assert_eq!(brightness(1.0, 20.0), 1.0);
        assert_eq!(brightness(0.0, 20.0), NIGHT_BRIGHTNESS);
        assert!(brightness(0.0, 0.0) > 0.8);
        assert!(brightness(0.0, 3.0) > brightness(0.0, 5.0));
        let verde = Color::srgb(0.16, 0.50, 0.18);
        assert_eq!(shade(verde, 1.0), verde);
        assert_ne!(shade(verde, NIGHT_BRIGHTNESS), Color::BLACK);
        assert_eq!(period(1.0, 12), "dia");
        assert_eq!(period(0.0, 2), "noite");
        assert_eq!(period(0.5, 6), "amanhecer");
        assert_eq!(period(0.5, 20), "anoitecer");
    }

    /// O que se desenha à volta do jogador enche o ecrã.
    #[test]
    fn a_vista_enche_o_ecra() {
        // (que a vista cabe na janela é verificado ao compilar, junto a `VIEW_W`)
        // A vista enche a parte do ecrã à direita do painel (960 x 720).
        assert!(VIEW_W as f32 * CELL >= 960.0 - PANEL_WIDTH);
        assert!(VIEW_H as f32 * CELL >= 720.0);
    }

    /// A câmara fica em cima do jogador (desviada para o painel caber à
    /// esquerda), também depois de ele andar.
    #[test]
    fn a_camara_segue_o_jogador() {
        let mut app = App::new();
        app.insert_resource(World(paisagem((5, 5)))).add_systems(Update, follow);
        let camera = app.world_mut().spawn((Camera2d, Transform::default())).id();
        for passo in 0..3 {
            app.world_mut().resource_mut::<World>().0.move_player(1, 0);
            app.update();
            let t = app.world().get::<Transform>(camera).unwrap().translation;
            let jogador = cell_to_screen(6 + passo, 5);
            assert_eq!((t.x, t.y), (jogador.x - PANEL_WIDTH / 2.0, jogador.y));
        }
    }

    /// Um coelho dentro de uma toca não se desenha.
    #[test]
    fn os_coelhos_na_toca_nao_se_veem() {
        let mut sim = Simulation::new(Config::default());
        let toca = sim.world.burrows()[0];
        sim.rabbits.truncate(2);
        (sim.rabbits[0].x, sim.rabbits[0].y) = toca;
        (sim.rabbits[1].x, sim.rabbits[1].y) = (toca.0 + 1, toca.1);
        while sim.world.is_burrow(sim.rabbits[1].x, sim.rabbits[1].y) {
            sim.rabbits[1].y = (sim.rabbits[1].y + 1) % sim.world.height;
        }
        let vistos: Vec<usize> = visible_rabbits(&sim).map(|(i, _)| i).collect();
        assert_eq!(vistos, vec![1]);
    }

    /// Os sprites dos animais reaproveitam-se: depois do primeiro frame não
    /// se criam mais, e há um visível por cada coisa à vista.
    #[test]
    fn os_sprites_dos_animais_reaproveitam_se() {
        let mut app = App::new();
        app.insert_resource(World(paisagem((12, 12)))).add_systems(Update, draw_animals);
        let contar = |app: &mut App| {
            let mut q = app.world_mut().query_filtered::<&Visibility, With<AnimalSprite>>();
            let todos: Vec<Visibility> = q.iter(app.world()).copied().collect();
            (todos.len(), todos.iter().filter(|v| **v == Visibility::Visible).count())
        };
        app.update(); // cria os sprites
        app.update(); // usa-os
        let (criados, visiveis) = contar(&mut app);
        assert!(visiveis > 2, "devia haver animais à vista, além do jogador");
        assert_eq!(criados, visiveis);
        // A simulação avança e os animais mudam: os sprites continuam os mesmos.
        app.world_mut().resource_mut::<World>().0.run_days(2);
        app.update();
        app.update();
        let (depois, visiveis) = contar(&mut app);
        assert!(depois <= criados.max(visiveis), "criou sprites a mais: {criados} → {depois}");
        assert!(depois >= visiveis);
    }

    /// Os quadrados mostram a vegetação da célula por baixo, com o jogador
    /// ao centro, e escondem-se fora da paisagem.
    #[test]
    fn os_quadrados_mostram_o_mundo_a_volta() {
        let mut app = App::new();
        app.insert_resource(World(paisagem((2, 3))))
            .insert_resource(SmellView(false))
            .add_systems(Update, draw_tiles);
        let centro = (VIEW_H / 2) * VIEW_W + VIEW_W / 2;
        let canto = 0; // em cima à esquerda: fora da paisagem
        let ids: Vec<Entity> = [centro, canto]
            .into_iter()
            .map(|i| {
                app.world_mut()
                    .spawn((Sprite::default(), Transform::default(), Visibility::default(), Tile(i)))
                    .id()
            })
            .collect();
        app.update();
        let land = &app.world().resource::<World>().0;
        let w = &land.window_sim().world;
        // Com a luz desta hora, na célula do próprio jogador.
        let luz = brightness(land.window_sim().light(), 0.0);
        let esperada = shade(vegetation_color(w.biomass_at(2, 3) / w.capacity()), luz);
        assert_eq!(app.world().get::<Sprite>(ids[0]).unwrap().color, esperada);
        assert_eq!(app.world().get::<Transform>(ids[0]).unwrap().translation.truncate(), cell_to_screen(2, 3));
        assert_eq!(*app.world().get::<Visibility>(ids[0]).unwrap(), Visibility::Visible);
        assert_eq!(*app.world().get::<Visibility>(ids[1]).unwrap(), Visibility::Hidden);
    }

    #[test]
    fn o_painel_explica_a_construcao() {
        let horas = [2, 3, 6, 4];
        assert_eq!(build_note(Build::Built(Stage::Ramos), horas), "Fizeste o abrigo de ramos (2 horas).");
        assert_eq!(
            build_note(Build::Missing(Stage::Folhas, eotw_sim::Item::Leaves, 4), horas),
            "Para a cabana de folhas falta 4 de folhas."
        );
    }
}
