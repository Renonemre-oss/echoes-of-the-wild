# Echoes of the Wild

Survival com ecossistema vivo, em Rust. O desenho completo está em `docs/GDD_v0.6.md`.
O historial do que se vai construindo está em `DIARIO.md`.

## Estrutura

| Crate | Função |
|---|---|
| `eotw_sim` | A simulação (ecossistema, vegetação, coelhos, raposas). Sem dependência de motor gráfico. |
| `eotw_cli` | Corre a simulação em modo texto e gera relatórios CSV. |
| `eotw_game` | O jogo: desenha a simulação numa janela com Bevy 0.16. Não tem ecologia própria. |

## Como correr

1. Instalar o Rust: https://rustup.rs
2. Dentro desta pasta:

```bash
cargo test                      # corre os testes (incluindo os de equilíbrio)
cargo run -p eotw_cli           # simulação com os valores por omissão
cargo run -p eotw_cli -- --seed 7 --days 200 --csv relatorio.csv
cargo run -p eotw_cli -- --cabin            # teste da cabana: tempo na zona, cadeias de eventos e um registo
cargo run --release -p eotw_cli -- --sweep  # varredura dos parâmetros sensíveis (~1 min)
cargo run -p eotw_game          # a janela: setas/WASD andam, Espaço pausa, + e - velocidade, O vê o cheiro
cargo run -p eotw_game -- --cabin --seed 6
```

Se `cargo run` mostrar muito texto no início, é só o Cargo a compilar. Da segunda vez é instantâneo.
A primeira compilação do jogo (`eotw_game`, com Bevy) demora muito mais: neste portátil foram 25 minutos.

## Regras da simulação

- 1 tick = 1 hora de jogo; 24 ticks = 1 dia.
- Sem aleatoriedade global: tudo vem do `Rng` com seed. A mesma seed dá sempre o mesmo resultado.
- Os números (parâmetros) estão em `Default` de cada `*Params`; mais tarde passam para ficheiros de dados.

## Onde mexer para aprender

- `eotw_sim/src/rabbit.rs`: o comportamento do coelho (comer, procurar, fugir para a toca, reproduzir).
- `eotw_sim/src/fox.rs`: o comportamento da raposa (ver, perseguir, caçar, comer restos, travão de densidade).
- `eotw_sim/src/world.rs`: o crescimento da vegetação as tocas e os restos de comida.
- `eotw_sim/src/scent.rs`: os campos de cheiro (depositar, espalhar, decair, seguir o gradiente).
- `eotw_sim/src/sim.rs`: a ordem de cada tick, a chegada das raposas, o aquecimento, a cabana e a migração de segurança.
- `eotw_sim/src/sweep.rs`: as varreduras de parâmetros e o teste das margens do equilíbrio.
- Experimenta mudar um parâmetro e correr `cargo test`: os testes de equilíbrio dizem logo se o mundo partiu.
