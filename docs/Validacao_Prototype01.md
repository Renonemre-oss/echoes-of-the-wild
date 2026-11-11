# Validação do Prototype 01

Data: 2026-11-11. Critérios das secções 18.4, 18.5 e 18.6 do GDD. Os números foram medidos com o código deste commit; para repetir:

```bash
cargo run --release -p eotw_cli -- --validate --days 100
cargo run --release -p eotw_cli -- --cabin
cargo test --release --workspace
cargo run --release -p eotw_game -- --frames 600 --speed 240
```

## Resumo

| Critério | Estado | Como se mediu |
|---|---|---|
| Nenhuma espécie se extingue (18.6) | ✅ | 20 seeds, 100 dias, paisagem de 5x5 com as cinco espécies: nenhuma chegou a 0 |
| Nenhuma população passa da capacidade de carga por uma margem definida (18.6) | ✅ com uma nota | Margem: 2 vezes a capacidade. Coelhos no máximo 6372 (média ≈5900); veados 21 (capacidade 46); javalis 21 (capacidade 16), num pico curto depois dos partos da primavera; a média fica em 6 |
| A vegetação não colapsa (18.6) | ✅ | A erva média mais baixa em todas as seeds foi 4,51 de 10 |
| Migração de segurança abaixo de um limite (18.6) | ⚠️ não medido na paisagem | A paisagem não conta as migrações das regiões; os testes de uma região (`eotw_sim`) continuam a passar |
| O relatório de populações fica guardado (18.6) | ✅ | Este ficheiro (tabela abaixo) |
| Teste da cabana: mais visitas com comida (18.6) | ✅ | Horas na zona: raposas ×4,88, coelhos ×1,04; entradas de raposas ×1,88 |
| Teste da cabana: uma cadeia de acontecimentos sem ser escrita (18.6) | ✅ com uma nota | "Coelho come os restos → raposa vem atrás → caça": 103 vezes, em 11 de 20 seeds. A cadeia com o lobo (lobo mata → carcaça → raposa) existe, mas à escala real o lobo quase não entra: 0 a 4 presas em 100 dias por seed |
| Os resultados variam entre seeds (18.6) | ✅ | Ver as tabelas: cadeias de 0 a 23 por seed |
| Transição agregado/individual conserva a população (18.6) | ✅ | Testes `agregar_conserva_a_populacao`, `expandir_conserva_a_populacao`, `ida_e_volta_mantem_o_equilibrio` |
| Nenhum animal aparece perto do jogador (18.6) | ✅ | Teste `nada_aparece_perto_do_jogador`; os veados e os javalis entram pelas bordas, longe do jogador |
| Desempenho no hardware-alvo (18.4, 18.5.6, 18.6) | ✅ | A 240 horas por segundo: simulação ≈1 ms por frame, picos de 4–7 ms (orçamento 8 ms); uma hora de simulação custa 0,54 ms, a viragem do dia 0,93 ms |
| Guardar e carregar preserva a coerência (18.5.7) | ✅ | Testes `guardar_e_carregar_continua_igual` e `as_alteracoes_do_jogador_ficam`: o mundo carregado continua igual, até ao último número |
| As criaturas procuram recursos sem instruções (18.4) | ✅ | Coelhos à erva, raposas à caça e ao cheiro dos restos, javalis aos arbustos com frutos, lobo atrás de veados e javalis |
| Espécies diferentes reagem de forma diferente às ameaças (18.4) | ✅ | Coelho foge para a toca; raposa afasta-se do jogador a 4 células; javali foge a 9 (10 com o machado); veado a 15; lobo a 18 |
| O jogador percebe que mudou o comportamento local (18.4, 18.5.4) | ✅ medido; ❓ falta a tua opinião | Comida na cabana: raposas ×4,9; machado: coelhos fora da toca de 1059 para 497 e raposas de 13 para 1; árvores cortadas: 70% menos coelhos na zona |

## Perguntas que só tu podes responder (18.5)

1. Consegues compreender o que os animais estão a fazer?
2. Os comportamentos parecem naturais?
3. As interações produzem situações que não foram escritas à mão?
4. As consequências são visíveis e fazem sentido?
5. O mundo continua interessante sem combate constante?

## Teste de equilíbrio (paisagem do jogo, todas as espécies)

Teste de equilíbrio: paisagem de 5x5 regiões, 100 dias, 20 seeds

| Seed | Coelhos (mín–méd–máx) | Raposas | Veados | Javalis | Erva média | Lobos: visitas, presas |
|---|---|---|---|---|---|---|
| 1 | 5663–5944–6316 | 236–250–262 | 7–13–17 | 3–7–20 | 4.66–5.23 | 8, 0 |
| 2 | 5760–6017–6310 | 247–254–260 | 6–14–19 | 3–6–9 | 4.65–5.13 | 7, 1 |
| 3 | 5631–5870–6232 | 242–259–269 | 4–12–18 | 3–6–17 | 4.74–5.25 | 3, 0 |
| 4 | 5944–6054–6372 | 230–235–242 | 2–14–18 | 3–7–21 | 4.51–5.02 | 7, 0 |
| 5 | 5685–5914–6345 | 232–239–246 | 7–14–21 | 3–6–11 | 4.58–5.19 | 11, 0 |
| 6 | 5936–6073–6324 | 234–245–253 | 2–13–18 | 3–6–12 | 4.64–5.09 | 16, 2 |
| 7 | 5709–5891–6209 | 247–254–264 | 4–13–17 | 3–6–10 | 4.76–5.22 | 13, 0 |
| 8 | 5979–6076–6358 | 232–238–248 | 6–13–18 | 3–6–13 | 4.64–5.10 | 6, 0 |
| 9 | 5800–5912–6124 | 253–260–268 | 5–13–18 | 3–6–9 | 4.86–5.16 | 12, 1 |
| 10 | 5696–5864–6161 | 258–268–277 | 4–12–18 | 3–7–18 | 4.76–5.21 | 6, 0 |
| 11 | 5552–5833–6313 | 263–268–277 | 5–13–18 | 3–5–9 | 4.73–5.29 | 9, 0 |
| 12 | 5783–5966–6291 | 239–246–254 | 5–14–19 | 3–6–13 | 4.64–5.09 | 9, 0 |
| 13 | 5868–6005–6361 | 236–245–254 | 5–13–18 | 3–6–14 | 4.64–5.00 | 12, 3 |
| 14 | 5605–5853–6267 | 249–257–262 | 5–14–18 | 3–6–18 | 4.71–5.26 | 16, 1 |
| 15 | 5710–5960–6255 | 245–251–258 | 3–13–18 | 3–7–14 | 4.71–5.18 | 7, 0 |
| 16 | 5731–5924–6372 | 244–251–258 | 5–13–18 | 3–6–11 | 4.62–5.15 | 13, 1 |
| 17 | 5755–5952–6142 | 243–251–259 | 4–12–18 | 3–6–14 | 4.78–5.22 | 7, 0 |
| 18 | 5859–6083–6363 | 230–241–253 | 3–13–18 | 3–6–20 | 4.57–5.07 | 7, 4 |
| 19 | 5705–5884–6339 | 247–257–265 | 7–14–18 | 3–6–15 | 4.65–5.22 | 9, 3 |
| 20 | 5837–6030–6372 | 229–238–248 | 4–13–19 | 3–7–18 | 4.57–5.08 | 10, 0 |

Seeds em que alguma espécie chegou a 0:
  coelhos 0, raposas 0, veados 0, javalis 0
Máximos de todas as seeds: coelhos 6372, raposas 277, veados 21, javalis 21
Erva média mais baixa de todas as seeds: 4.51

## O que fica para depois do Prototype 01

- Os herbívoros não passam fome; o javali não come láparos; o lobo não deixa cheiro, pegadas nem uivos; a raposa não foge do lobo.
- Contar as migrações de segurança na paisagem inteira.
- Pesquisa a fundo de cada animal, um de cada vez, e recalibrar.
