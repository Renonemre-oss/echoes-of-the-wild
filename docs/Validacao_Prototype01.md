# Validação do Prototype 01

Data: 2026-11-13, depois da recalibração da fase 1 e da dispersão das raposas (a primeira validação foi a 2026-11-11). Critérios das secções 18.4, 18.5 e 18.6 do GDD. Os números foram medidos com o código deste commit; para repetir:

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
| Nenhuma população passa da capacidade de carga por uma margem definida (18.6) | ✅ com uma nota | Margem: 2 vezes a capacidade. Coelhos entre ≈4700 no fim do inverno e 9905 no pico depois das crias (média ≈7450): agora têm o ciclo do ano, e antes ficavam quase parados em ≈5900; veados 21 (capacidade 46); javalis 22 (capacidade 16), num pico curto depois dos partos da primavera; a média fica em 6 |
| A vegetação não colapsa (18.6) | ✅ | A erva média mais baixa em todas as seeds foi 25 de 100 (no verão mediterrânico a erva não cresce) |
| Migração de segurança abaixo de um limite (18.6) | ⚠️ não medido na paisagem | A paisagem não conta as migrações das regiões; os testes de uma região (`eotw_sim`) continuam a passar |
| O relatório de populações fica guardado (18.6) | ✅ | Este ficheiro (tabela abaixo) |
| Teste da cabana: mais visitas com comida (18.6) | ✅ | Horas na zona: raposas ×4,94, coelhos ×1,15; entradas de raposas ×2,44, de coelhos ×1,70 |
| Teste da cabana: uma cadeia de acontecimentos sem ser escrita (18.6) | ✅ com uma nota | "Coelho come os restos → raposa vem atrás → caça": 102 vezes, em 13 de 20 seeds. A cadeia com o lobo (lobo mata → carcaça → raposa) existe, mas à escala real o lobo quase não entra: 0 a 4 presas em 100 dias por seed |
| Os resultados variam entre seeds (18.6) | ✅ | Ver as tabelas: as populações, a erva e as visitas dos lobos mudam de seed para seed |
| Transição agregado/individual conserva a população (18.6) | ✅ | Testes `agregar_conserva_a_populacao`, `expandir_conserva_a_populacao`, `ida_e_volta_mantem_o_equilibrio` |
| Nenhum animal aparece perto do jogador (18.6) | ✅ | Teste `nada_aparece_perto_do_jogador`; os veados e os javalis entram pelas bordas, longe do jogador |
| Desempenho no hardware-alvo (18.4, 18.5.6, 18.6) | ✅ com uma nota | A 240 horas por segundo: simulação mediana 1,5–1,6 ms por frame, pior 5,9–7,0 ms (orçamento 8 ms); frame no percentil 95 7,4–8,6 ms (orçamento 10). Uma hora custa 0,54 ms e a viragem do dia 1,06 ms. O jogo dá à simulação no máximo 5 ms por frame e as horas que não cabem passam para o frame seguinte. Medido com o portátil carregado (carga média 7 em 4 núcleos): numa de 3 corridas houve um pico de 12,9 ms, de uma só hora interrompida pelo sistema. Repetido com o portátil mais livre: simulação pior 5,05 ms, frame p95 7,9 ms, pior frame 9,4 ms |
| Guardar e carregar preserva a coerência (18.5.7) | ✅ | Testes `guardar_e_carregar_continua_igual` e `as_alteracoes_do_jogador_ficam`: o mundo carregado continua igual, até ao último número |
| As criaturas procuram recursos sem instruções (18.4) | ✅ | Coelhos à erva, raposas à caça e ao cheiro dos restos, javalis aos arbustos com frutos, lobo atrás de veados e javalis |
| Espécies diferentes reagem de forma diferente às ameaças (18.4) | ✅ | Coelho foge para a toca; raposa afasta-se do jogador a 4 células; javali foge a 9 (10 com o machado); veado a 15; lobo a 18 |
| O jogador percebe que mudou o comportamento local (18.4, 18.5.4) | ✅ | Comida na cabana: raposas ×4,9; machado: os coelhos fogem para a toca e as raposas afastam-se; árvores cortadas: menos coelhos na zona. O jogador respondeu que vê as consequências e que fazem sentido |

## As perguntas do GDD 18.5 (respostas do jogador, 2026-11-13)

| Pergunta | Resposta | Estado |
|---|---|---|
| 1. Consegues compreender o que os animais estão a fazer? | Raramente | ❌ |
| 2. Os comportamentos parecem naturais? | Sim | ✅ |
| 3. As interações produzem situações que não foram escritas à mão? | Não reparei | ❌ não se vê (as medições contam 102 cadeias em 20 seeds, mas o jogador não dá por elas) |
| 4. As consequências são visíveis e fazem sentido? | Sim, vejo e faz sentido | ✅ |
| 5. O mundo continua interessante sem combate constante? | Não | ❌ |

**O que ajudaria a perceber os animais** (escolha do jogador): clicar num animal e ver no painel o que está a fazer e porquê; mostrar as crias e quantos animais estão dentro de cada toca.

**Conclusão:** a simulação funciona e passa nos critérios medidos, mas o jogador não a consegue ler. As perguntas 1 e 3 falham pela mesma razão: o que acontece não se vê. Antes de dar o Prototype 01 como fechado, falta tornar a simulação legível, e depois voltar a fazer estas perguntas. A pergunta 5 depende em parte disso: um mundo que não se percebe não prende.

## Teste de equilíbrio (paisagem do jogo, todas as espécies)

Teste de equilíbrio: paisagem de 5x5 regiões, 100 dias, 20 seeds

| Seed | Coelhos (mín–méd–máx) | Raposas | Veados | Javalis | Erva média | Lobos: visitas, presas |
|---|---|---|---|---|---|---|
| 1 | 4919–7597–9716 | 131–183–250 | 6–13–18 | 3–6–13 | 25.82–46.22 | 12, 3 |
| 2 | 4668–7256–9210 | 139–199–270 | 3–13–19 | 3–6–18 | 28.43–47.77 | 11, 1 |
| 3 | 4758–7287–9436 | 129–191–249 | 2–13–18 | 3–5–10 | 27.96–48.92 | 6, 0 |
| 4 | 4789–7436–9640 | 139–190–250 | 4–12–18 | 3–6–21 | 27.43–46.74 | 10, 2 |
| 5 | 4773–7428–9418 | 130–188–243 | 6–14–21 | 3–6–14 | 26.63–46.83 | 12, 1 |
| 6 | 4662–7281–9379 | 143–197–263 | 2–14–18 | 3–5–8 | 28.16–46.78 | 12, 0 |
| 7 | 4896–7570–9784 | 137–188–250 | 4–13–18 | 3–6–9 | 25.58–46.89 | 13, 0 |
| 8 | 4814–7533–9693 | 135–192–277 | 8–13–18 | 3–6–22 | 26.01–45.90 | 8, 0 |
| 9 | 4840–7444–9526 | 137–189–252 | 4–14–18 | 3–6–9 | 27.06–46.68 | 7, 2 |
| 10 | 4788–7417–9681 | 116–185–259 | 6–14–18 | 3–6–15 | 26.47–46.74 | 9, 3 |
| 11 | 4823–7417–9471 | 135–188–239 | 5–13–18 | 3–6–9 | 26.53–48.15 | 8, 0 |
| 12 | 4751–7441–9717 | 134–185–247 | 4–13–19 | 3–6–13 | 26.78–46.05 | 10, 0 |
| 13 | 4867–7471–9537 | 133–182–233 | 4–14–19 | 3–6–14 | 26.75–46.78 | 13, 0 |
| 14 | 4850–7499–9700 | 129–184–256 | 5–13–18 | 3–6–16 | 26.27–47.57 | 13, 5 |
| 15 | 4925–7582–9681 | 130–182–242 | 3–12–18 | 3–6–10 | 25.50–46.91 | 9, 0 |
| 16 | 4861–7443–9558 | 139–185–243 | 4–12–18 | 3–5–9 | 27.05–46.86 | 12, 1 |
| 17 | 4919–7569–9833 | 131–188–246 | 4–13–18 | 3–6–20 | 25.45–46.98 | 8, 1 |
| 18 | 4759–7326–9462 | 138–194–273 | 3–14–19 | 3–6–19 | 27.47–47.92 | 11, 2 |
| 19 | 4886–7619–9905 | 132–180–235 | 5–14–18 | 3–6–14 | 25.28–44.70 | 10, 1 |
| 20 | 4914–7471–9501 | 138–184–233 | 4–13–20 | 3–7–16 | 25.85–47.11 | 12, 0 |

Seeds em que alguma espécie chegou a 0:
  coelhos 0, raposas 0, veados 0, javalis 0
Máximos de todas as seeds: coelhos 9905, raposas 277, veados 21, javalis 22
Erva média mais baixa de todas as seeds: 25.28

## O que fica para depois do Prototype 01

- Os herbívoros não passam fome; o javali não come láparos; o lobo não deixa cheiro, pegadas nem uivos; a raposa não foge do lobo.
- Contar as migrações de segurança na paisagem inteira.
- Pesquisa a fundo de cada animal, um de cada vez, e recalibrar.
