# Diário de Construção: Echoes of the Wild

Aqui registamos o que foi feito, quando, e que marcos foram alcançados, para celebrar cada passo.

**Como funciona:** cada entrada tem a data, o que foi feito, o marco (se houve) e o que se aprendeu.
As entradas mais recentes ficam no topo. Quando um marco acontece, passa a ✅ no painel e ganha uma celebração. 🎉

---

## Painel de marcos

| # | Marco | Estado | Data |
|---|---|---|---|
| 1 | Design congelado (GDD v0.6) | ✅ | 2026-11-01 |
| 2 | Workspace criado (`eotw_sim`, `eotw_cli`, `eotw_game`) | ✅ | 2026-11-01 |
| 3 | Primeira simulação a correr (coelhos e vegetação) | ✅ | 2026-11-01 |
| 4 | Primeiro equilíbrio estável (50 seeds, 300 dias, sem extinções) | ✅ | 2026-11-01 |
| 5 | Primeiro `cargo run` no teu computador | ✅ | 2026-11-02 |
| 6 | Primeiro teste escrito por ti | ✅ | 2026-11-02 |
| 7 | Primeiro predador (raposa) | ✅ | 2026-11-02 |
| 8 | Perceção por cheiro a funcionar | ✅ | 2026-11-02 |
| 9 | Teste da cabana em modo texto | ✅ | 2026-11-02 |
| 10 | Primeira janela com Bevy (etapa 4) | ✅ | 2026-11-02 |
| 11 | Prototype 01 completo | ⬜ | |

Os marcos 5 a 11 seguem o plano da secção 19 do GDD e podem mudar.

---

## 2026-11-02 (segunda-feira)

**O que foi feito**

- O projeto compilou e correu no teu computador.
- Escreveste o teu primeiro teste: `sem_vegetacao_os_coelhos_morrem_de_fome`.
- Na primeira versão o teste falhou: os coelhos passaram de 20 para 120. Corrigido ao desligar o crescimento da vegetação nesse teste.

**Marcos** 🎉

- Marco 5: **primeiro `cargo run` no teu computador**. 💻
- Marco 6: **primeiro teste escrito por ti**. 🧪

**O que se aprendeu**

1. **Um teste que falha também ensina:** esvaziar o mundo uma vez não chega, porque o `seed_growth` faz a vegetação voltar a crescer. Ao fim de cerca de um dia e meio as células já passam o `min_food`, antes de os coelhos morrerem de fome.
2. **Configurar só o que interessa:** `Config { vegetation: VegetationParams { regrowth: 0.0, seed_growth: 0.0, ..VegetationParams::default() }, ..Config::default() }` altera dois valores e mantém todos os outros por omissão.

**Próximo passo:** acrescentar a raposa (marco 7).

### Mais tarde, no mesmo dia: a raposa 🦊

**O que foi feito**

- Nova espécie em `eotw_sim/src/fox.rs`: a raposa vê coelhos até 5 células, persegue-os a 2 células por tick e salta quando chega ao lado. Depois de comer fica saciada.
- O coelho agora foge: corre para a toca mais próxima ou, se não houver nenhuma perto, para longe da raposa.
- Tocas no mapa (`o` no mapa de texto): dentro de uma toca o coelho não se vê nem se pode caçar. A vegetação densa também protege.
- Território: uma raposa só tem crias se não houver outra raposa a 9 células ou menos.
- Migração de segurança (GDD 7.8, mecanismo 6): se uma espécie desaparecer durante 3 dias, chega um animal pela margem do mapa. Fica registada no relatório.
- CSV e tabela do `eotw_cli` com colunas novas: caçados, raposas, crias e mortes de raposas, migrações.
- 28 testes (eram 13), incluindo o teste de equilíbrio predador-presa da GDD 18.6: 20 seeds, 100 dias.

**Marco** 🎉

- Marco 7: **o primeiro predador**. Pela primeira vez, a cadeia vegetação → coelhos → raposas funciona: com raposas há menos coelhos e mais vegetação. Em 70 seeds com 300 dias, e em 20 seeds com 1000 dias, as duas espécies coexistem **sem uma única migração de segurança**. 🦊🐇🌱

**O que se aprendeu**

1. **A perseguição infinita:** na primeira versão as raposas quase nunca caçavam. A raposa parava ao lado do coelho, o coelho afastava-se uma célula, e repetiam isto para sempre. A solução foi a raposa saltar logo no mesmo tick em que chega ao lado.
2. **O ciclo de expansão e colapso (GDD 7.8 tinha razão):** as raposas multiplicavam-se até cerca de 20, comiam todos os coelhos e depois morriam de fome. As tocas e o território ajudaram, mas não chegaram.
3. **O medo também mata:** a dada altura morriam mais coelhos de fome do que caçados, com a vegetação quase cheia à volta. Estavam escondidos ou a fugir em vez de comer. A solução foi o coelho esfomeado arriscar e ir comer (`desperate_energy`). É a IA de utilidade da GDD 17.3 em versão pequena.
4. **O parâmetro decisivo foi a gula da raposa:** passar `hunt_below` de 85 para 65 (a raposa só caça quando tem fome a sério), com 30 tocas e 2 raposas iniciais, transformou um sistema que colapsava num que coexiste. Mais uma vez, um número pequeno decidiu entre vida e extinção.
5. **Testar muitas seeds de uma vez vale ouro:** um programa à parte corria 20 a 70 seeds com várias combinações de parâmetros em segundos, e mostrava logo qual era a mais robusta.

**Revisão do código (no mesmo dia)**

- O teu teste da fome deixou de isolar a fome: com raposas no `Config::default()`, podia passar só porque os coelhos foram caçados. Agora usa `Config::without_foxes()` e verifica também que `deaths_hunger` é maior que zero.
- As crias nasciam com 30 de energia, que é exatamente o limiar `desperate_energy`, por isso nunca fugiam. Foi acidente: agora nascem com 35 e há um teste (`cria_foge_desde_que_nasce`).
- As tocas eram criadas depois das raposas, por isso mudar `initial_foxes` mudava o sítio das tocas e estragava a comparação com/sem raposas. Passaram para antes. Isto mudou os resultados (uma seed em 70 voltou a precisar de migração); com as crias a 35 voltou a 70/70 sem migrações em 300 dias e 20/20 em 1000 dias.
- `rust-version = "1.82"` no `Cargo.toml`, para o Cargo dizer claramente quando o Rust está desatualizado.
- Fica para os chunks: `nearest_rabbit` e `nearest_burrow` percorrem listas inteiras. Chega para um mapa de 24×24, mas vai precisar de uma grelha espacial.

**Lição:** uma revisão de fora vê o que quem escreveu já não vê. E mudar a ordem em que se usa o `Rng` muda o mundo todo: depois de qualquer mudança dessas, volta-se a correr os testes de equilíbrio.

**Próximo passo:** perceção por cheiro (marco 8). A raposa passa a seguir o rasto dos coelhos em vez de só os ver.

### Ao fim do dia: arrumação e arte 🎨

**O que foi feito**

- **Hardlinks quebrados.** Os ficheiros de fora e os da subpasta `echoes-of-the-wild/` eram o mesmo ficheiro com dois nomes. Um `cp` seguido de `mv` em cada um tornou-os independentes, e a subpasta repetida passou a ser só uma cópia antiga, que se pode apagar.
- **GDD v0.7** com a nova secção 15.5, *Ideias de arte (inspiração, não decisões)*: sete referências de arte e o que têm em comum (pintura em vez de realismo, escala e figuras pequenas, ruínas recuperadas pela natureza, marcos visíveis ao longe, luz como protagonista, paleta contida com um acento, fantasia moderada). A decisão visual continua adiada.

**O que se aprendeu**

1. **Dois nomes para o mesmo ficheiro pregam partidas:** editar um muda o outro, e ferramentas que querem evitar ler "por um atalho" recusam-se a abri-lo.
2. **A arte também empurra a simulação:** nenhuma referência mostra animais pequenos, por isso coelhos e raposas vão precisar de silhuetas e cores que se leiam à distância (ligação à legibilidade, GDD 7.10).

### Segunda revisão: as margens do equilíbrio 📏

A revisão mostrou que o equilíbrio era estável, mas com margens estreitas: `hunt_below` 65 estava a 5 pontos de um colapso, 3 ou mais raposas iniciais faziam os coelhos descer a 2, e um mapa de 16×16 colapsava.

**O que foi feito**

- **As varreduras passaram para o projeto.** `cargo run --release -p eotw_cli -- --sweep` corre cada parâmetro sensível de um lado ao outro com 40 seeds (módulo `sweep.rs`). Há um teste novo, `margens_de_equilibrio`, que falha se os valores por omissão ficarem perto de uma beira: `hunt_below` ±10, 1 raposa, o triplo das raposas, um terço das tocas e o mapa mínimo.
- **As raposas chegam no dia 30, e o jogador no dia 60.** Antes disso só há coelhos (`foxes_arrive_day`), e `Simulation::warmed_up` corre os primeiros 60 dias antes de o jogador aparecer (`warmup_days`).
- **`hunt_below` passou de 65 para 60,** o meio da nova zona estável.
- **Tamanho mínimo de região: 24×24** (`MIN_REGION_SIZE`), escrito na GDD 7.9.
- **O teste da cabana existe antes do cheiro.** Há restos de comida no mundo, que coelhos e raposas comem se pisarem a célula, e uma cabana opcional (`CabinParams`) cuja comida é reposta todos os dias. Cada espécie tem um contador de entradas na zona. O teste `comida_junto_a_cabana_atrai_animais` está marcado com `#[ignore]` até ao marco 8.
- **O "território" da raposa tem o nome certo nos comentários e na GDD:** é um travão de densidade, não um território que ela ocupe.

| Varredura (40 seeds, 300 dias) | Antes | Depois |
|---|---|---|
| `hunt_below` estável | 50 a 65 (70 colapsa) | 40 a 75 (80 colapsa; a 1000 dias, 50 a 70) |
| Raposas iniciais estáveis | 1 a 2 | 1 a 8 |
| Tocas estáveis | 10 a 40 (0 colapsa) | 0 a 40 |
| Mapa estável | 24×24 ou mais | 20×20 ou mais |

**Ponto de partida do teste da cabana (sem cheiro)**, com `eotw_cli --cabin`, 20 seeds e 120 dias depois de o jogador chegar:

- coelhos: 17 550 visitas sem comida e 17 538 com comida (×1,00);
- raposas: 21 visitas sem comida e 24 com comida (×1,14), um número demasiado pequeno para tirar conclusões.

É este o número que o cheiro tem de melhorar.

**O que se aprendeu**

1. **A sensibilidade não era às raposas, era ao arranque.** Com 20 coelhos no dia 0, poucas raposas a mais comiam-nos antes de se reproduzirem. Com só 20 dias de avanço para os coelhos, 2 a 8 raposas ficaram todas estáveis, mesmo com a migração desligada. O problema não estava onde parecia.
2. **Um valor que funciona pode estar à beira do precipício.** Os testes de equilíbrio passavam com 65, mas 67,5 já colapsava. Saber onde estão as beiras vale tanto como saber que o valor por omissão funciona.
3. **Perto da capacidade de carga, a raposa troca uma morte por outra.** Com raposas há só 1 a 9% menos coelhos, mas as mortes por fome caem de cerca de 2,2 para 1 por dia: muitos dos coelhos caçados teriam morrido de fome na mesma. O teste `as_raposas_reduzem_os_coelhos` teve de passar a medir a partir do dia 100, quando já há raposas suficientes para a diferença se ver.
4. **Sem cheiro, a comida junto à cabana não atrai ninguém.** Os coelhos já estão por todo o lado, e as raposas descansam quase sempre e raramente passam lá. Agora sabemos que qualquer diferença que apareça depois vem do cheiro.
5. **Com 2 a 8 raposas, perde-se uma por azar de vez em quando.** A 1000 dias, cerca de 1 seed em 20 precisa de uma migração de raposa, seja qual for o `hunt_below` entre 45 e 70. Não é um limiar: é o que acontece com populações tão pequenas, e é para isso que serve a migração.

**Fica para depois**

- Em duas seeds (3 e 4) as duas raposas iniciais nunca têm crias em 250 dias, provavelmente porque andam sempre a menos de 9 células uma da outra. Vale a pena investigar.
- `cargo test` demora cerca de 30 segundos por causa das varreduras em modo debug. Se começar a incomodar, `opt-level = 1` no perfil de teste resolve.

**Próximo passo:** perceção por cheiro (marco 8). Fica feito quando `comida_junto_a_cabana_atrai_animais` passar sem o `#[ignore]`.

### Já de noite: o cheiro 👃

**O que foi feito**

- **Campos de cheiro** (`scent.rs`, GDD 7.6): uma grelha por tipo, por agora *alimento* (restos) e *presa* (coelhos). As fontes depositam cheiro na sua célula, e em cada tick o cheiro espalha-se para as vizinhas e decai 1%. Um coelho dentro da toca não deixa cheiro, por isso a toca continua a ser refúgio.
- **O coelho com fome** vai primeiro para a erva que vê. Só segue o cheiro dos restos se nenhuma vizinha tiver comida que chegue, e sente-o até cerca de 7 células.
- **A raposa que não vê nenhum coelho** segue primeiro o cheiro de restos, depois o de coelhos, e só deambula se não sentir nada. Também come os restos.
- **O teste da cabana passou, já sem o `#[ignore]`.** Mede horas na zona em vez de entradas.
- **`cargo test` passou de 73 para 6 segundos** com `opt-level = 2` no perfil de teste.

**Marco** 🎉

- Marco 8: **os animais encontram comida que não veem.** Com 10 restos por dia, os coelhos passam mais tempo junto à cabana em 20 de 20 seeds (×1,15 com 10 restos, ×1,44 com 30, ×1,75 com 50), e as raposas ×2,7. Uma raposa que encontra a cabana fica lá a comer os restos e a caçar os coelhos que vêm pela comida: a cadeia coelho, raposa aparece sem ter sido escrita.

**O que se aprendeu**

1. **Medir antes valeu a pena.** Sem cheiro a diferença era ×1,00, por isso o que aparece agora vem do cheiro.
2. **A pergunta do teste estava errada duas vezes, e foi corrigida às claras.**
   - **Entradas → horas.** As entradas de raposas *desciam* com comida, porque a raposa chega e fica.
   - **O ×1,5 dos coelhos era um número inventado antes de haver dados.** Os dados mostram um efeito pequeno mas sempre presente e proporcional à comida. O teste passou a pedir isso: sobe em todas as seeds e sobe mais com mais comida. Para as raposas continua a ser ×1,5 no total.
3. **Uma armadilha ecológica.** Na primeira versão, com 50 restos por dia, os coelhos caíam para 10 em todo o mapa. Os restos acabavam em poucas horas, mas o cheiro durava dias: os coelhos ficavam presos à volta da cabana, em células rapadas, a morrer de fome com erva cheia a duas células. A correção foi o coelho confiar primeiro na erva que vê. Há um teste novo, `muita_comida_nao_e_uma_armadilha`, e agora mais comida dá mais coelhos (311 com 200 restos por dia, contra 280 sem comida).
4. **O equilíbrio aguentou o cheiro:** a varredura dá os mesmos resultados de antes.

**Fica para depois**

- A raposa só encontra a cabana em cerca de 1 seed em 4, porque quase sempre vê um coelho antes de cheirar a comida. Uma raposa que prefira restos fáceis a uma caçada podia mudar isso.
- O marco 9 (teste da cabana em modo texto) já mede o que a GDD pede às visitas, com `eotw_cli --cabin`. Falta o registo de uma sequência de eventos encadeados, que na GDD inclui o lobo.

**Próximo passo:** decidir entre fechar o marco 9 (eventos encadeados no registo) e afinar a raposa necrófaga.

### A raposa necrófaga 🦴

**O que foi feito**

- **A raposa é necrófaga.** Com fome e um cheiro forte de restos (`scavenge_smell_min`), vai aos restos mesmo que veja um coelho: comer restos não custa nada.
- **`sniff` deixou de gastar números do `Rng` quando não há cheiro.** Assim um mundo sem cabana não é alterado por um cheiro que não existe.
- **Dois testes novos:**
  - `raposa_necrofaga_prefere_restos_a_caca`;
  - `necrofagia_nao_esteriliza_as_raposas`.

**Com 10 restos por dia, 20 seeds e 60 dias, segundo o limiar:**

| Limiar | As raposas vêm à cabana | Os coelhos sobem | Raposas a 1000 dias com 50 restos |
|---|---|---|---|
| desligado | 3/20 seeds | 20/20 | média 9, 0 migrações |
| **8 (escolhido)** | 7/20 (19/20 com 50 restos) | 18/20 | média 7, 0 migrações |
| 2 | 16/20 | 14/20 | média 2,4, 16 migrações |
| 1 | 20/20 | 12/20 | média 2, 18 migrações |

**O que se aprendeu**

1. **A raposa na cabana afasta os coelhos.** Quanto mais necrófaga, mais vezes acampa junto à comida, e os coelhos fogem dela. É uma consequência que o jogador consegue ler ("desde que a raposa vem cá, os coelhos desapareceram"), mas em excesso apaga o efeito da comida nos coelhos.
2. **O travão de densidade tem um ponto cego.** Quando todas as raposas se juntam num sítio com comida, cada uma tem rivais perto e nenhuma tem crias, mesmo bem alimentadas. A população envelhece e só a migração a repõe. O travão conta vizinhas, não comida disponível. É um argumento para o território verdadeiro do lobo e, talvez, para um travão que olhe também para a comida.
3. **Os casos de beira mudam com qualquer coisa.** Só por mudar a sequência do `Rng`, "raposas chegam no dia 0" e o mapa de 16×16 passaram a precisar de migração em algumas seeds. Já tinham trocado de estado em varreduras anteriores. É por isso que os valores por omissão ficam longe dessas beiras.

**Próximo passo:** fechar o marco 9, com eventos encadeados no registo (coelho, raposa, e um dia o lobo).

### O registo da cabana e o marco 9 📜

**O que foi feito**

- **Registo da cabana** (`CabinLog`) com frases legíveis e a hora, por exemplo:
  - "Dia 62, 09h: um coelho comeu os restos junto à cabana."
  - "uma raposa seguiu o cheiro dos restos até à cabana."
  - "uma raposa caçou um coelho junto à cabana."
- **Deteção de cadeias:** um coelho come os restos e, em menos de 2 dias, uma raposa caça junto à cabana. A raposa pode ter vindo atrás ou já lá estar à espera, e o registo diz qual foi.
- **`eotw_cli --cabin`** mostra as cadeias por seed e o registo da primeira história com cadeia.
- **Teste `a_cadeia_da_cabana_acontece_sem_ser_escrita`:**
  - há cadeias;
  - variam entre seeds;
  - com comida há mais caçadas junto à cabana do que sem comida.

**Marco** 🎉

- Marco 9: **o teste da cabana corre em modo texto e cumpre a GDD 18.6.**
  - **Mais tempo junto à cabana com comida:** coelhos ×1,12, raposas ×3,35.
  - **Há cadeias que ninguém escreveu:** 36 em 7 de 20 seeds, em 60 dias depois de o jogador chegar.
  - **Os resultados variam entre seeds:** de 0 a 7 cadeias por seed.
  - **A comida causa mais caçadas junto à cabana:** 26 sem comida, 49 com comida.
  - **O lobo da cadeia da GDD** fica para quando houver lobo.

**O que se aprendeu**

1. **Uma cadeia contada não é uma cadeia causada.** À primeira, as "cadeias" eram coincidências: havia caçadas junto à cabana com ou sem comida. Só o controlo a sério (caçadas com comida contra sem comida) o mostrou: com comida havia *menos* caçadas, 14 contra 26. Sem esse controlo, o marco tinha fechado com uma causalidade falsa.
2. **O valor de um resto muda o jogo.** Com 10 de energia por unidade, 10 restos por dia valiam 2 coelhos. A raposa enchia-se e deixava de caçar, e a comida protegia os coelhos. É a alimentação de diversão, que existe na ecologia real. Com 2, os restos são um petisco: atraem a raposa, que depois caça os coelhos que vieram pela comida (×1,9 caçadas). Escolhi 2 porque é a cadeia que a GDD descreve. Fica como ideia de jogo: restos de carne (muito nutritivos, protegem as presas) e restos de vegetais (atraem coelhos e, por arrasto, raposas).
3. **A definição da cadeia também estava estreita demais.** Exigia que a raposa *chegasse* depois do coelho, mas quase todas as caçadas eram de raposas que já estavam acampadas na cabana. A causa é a mesma (foi a comida que as trouxe), por isso as duas contam.

**Fica para depois**

- **O ponto cego do travão de densidade piorou com comida em excesso.** Com 200 restos por dia, as raposas juntam-se todas na cabana e deixam de ter crias: ficam em média 2, com 38 migrações em 20 seeds e 1000 dias. Com o valor antigo já eram 19. Com 10 restos por dia não há problema. A solução é um travão que conte com a comida disponível, ou o território verdadeiro.

**Próximo passo:** o travão de densidade com comida, ou começar a etapa 4 (primeira janela com Bevy, marco 10).

### O travão de densidade com comida, e a raposa que debicava 🍖

**O que foi feito**

- **O travão de densidade conta com a comida.** Cada 40 de energia em restos dentro do raio dá lugar a mais uma raposa rival (`scrap_energy_per_extra_fox`).
- **Com restos aos pés, a raposa come até estar cheia** (`full_energy` 95), mesmo sem fome.
- **Os restos valem 0,5 de energia por unidade para a raposa** (eram 2).
- **Dois testes novos:**
  - `raposa_com_restos_come_ate_estar_cheia`;
  - `muita_comida_nao_esteriliza_as_raposas`, que substitui o da necrofagia e testa 50 e 200 restos por dia.

**Com 200 restos por dia, 20 seeds e 1000 dias:**

| Versão | Raposas em média | Migrações |
|---|---|---|
| Antes | 2,0 | 38 |
| Só o travão com comida | 2,5 | 33 |
| Só comer até estar cheia | 2,2 | 20 |
| As duas | 16,7 | 0 |
| As duas, com restos a 0,5 (final) | 4,8 | 0 |

**O que se aprendeu**

1. **O diagnóstico de ontem estava incompleto.** Achei que o problema era só o travão de densidade, mas corrigi-lo quase não mudou nada. Ao seguir as raposas de uma seed, a energia delas estava sempre entre 60 e 64. Só comiam restos com fome (abaixo de 60), cada dentada dava +4, e paravam: debicavam para sempre logo acima do limiar e nunca chegavam aos 75 para ter crias. Eram precisas as duas correções, e nenhuma resolvia sozinha.
2. **Corrigir um comportamento muda outro.** Uma raposa que come até estar cheia é alimentada pela cabana e deixa de caçar lá (3 caçadas com comida, contra 26 sem). A alimentação de diversão voltou. Baixar os restos para 0,5 tornou-os outra vez um petisco: 48 caçadas com comida, 20 cadeias em 7 de 20 seeds.
3. **Medir em energia e não em unidades.** O travão diz quanta energia em restos justifica mais uma raposa. Assim, se o valor dos restos mudar outra vez, o travão continua coerente.

**Próximo passo:** começar a etapa 4 (primeira janela com Bevy, marco 10).

### Arrumação, git e a primeira janela 🪟

**O que foi feito**

- **A cópia antiga `echoes-of-the-wild/` foi apagada,** depois de confirmar que só tinha versões mais antigas dos mesmos ficheiros.
- **`git init` e primeiro commit** (`4b4846f`).
- **`eotw_game` com Bevy 0.16.** A janela desenha a simulação de `eotw_sim`, que continua sem saber que o Bevy existe:
  - a vegetação, de castanho (pouca erva) a verde (muita);
  - as tocas, os coelhos, as raposas e a cabana;
  - um painel com o dia, as populações, a legenda e as últimas linhas do registo da cabana.
- **Teclas:**
  - Espaço: pausa;
  - `+` e `-`: velocidade, de 1 a 240 horas de jogo por segundo.
- **Opções:** `--seed`, `--cabin` e `--screenshot FICHEIRO`. Esta última guarda uma imagem da janela e fecha, e foi assim que se tirou a imagem `docs/janela_marco10.png`.
- **Fonte DejaVu Sans em `eotw_game/assets/fonts`,** com a licença ao lado. A fonte do Bevy não tem acentos, e "Vegetação" aparecia com quadrados.
- **`rust-version` passou para 1.85,** o mínimo do Bevy 0.16.

**Marco** 🎉

- Marco 10: **o ecossistema tem cara.** Pela primeira vez vê-se o mundo a mexer: os coelhos espalhados pela erva, as raposas a laranja, e a erva a ficar castanha onde os coelhos comem.

**O que se aprendeu**

1. **A primeira compilação do Bevy demora:** 25 minutos neste portátil. As seguintes recompilam só o nosso código (cerca de 25 segundos). O `target/` fica grande, porque o executável em modo debug tem 1,3 GB com a informação de depuração.
2. **Escolhi o Bevy 0.16 e não o mais recente (0.20).** É a versão cuja API conheço com segurança. Atualizar é um passo a planear à parte, porque o Bevy muda a API em cada versão.
3. **Separar a simulação do motor compensa já:** o jogo tem cerca de 300 linhas e só desenha. Toda a ecologia, os testes e as varreduras continuam em `eotw_sim`.

**Verificado por ti:** a janela abre e fecha sem erros, e a pausa e a velocidade funcionam.

**Próximo passo:** etapa 4, a seguir: o jogador e o movimento.

---

## 2026-11-01 (domingo)

**O que foi feito**

- O GDD passou de v0.3 para v0.6: buracos tapados (perceção, vegetação, equilíbrio, transição de camadas, legibilidade), todas as decisões em aberto tomadas por quiz, riscos e ordem de corte na secção 20.
- Criado o workspace Cargo com `eotw_sim`, `eotw_cli` e `eotw_game`.
- Primeira simulação: gerador aleatório determinístico, mundo em grelha com vegetação que cresce e se gasta, coelhos com energia que procuram comida, comem e reproduzem-se.
- Relatório diário em CSV e mapa em texto.
- 12 testes automáticos, incluindo o primeiro teste de equilíbrio (GDD 18.6).

**Marcos** 🎉

- Marco 1: design congelado.
- Marco 2: workspace criado.
- Marco 3: **primeira vida no mundo**. 20 coelhos, e no dia 8 já eram 50. 🐇
- Marco 4: **o primeiro ecossistema estável**. Em 50 seeds com 300 dias cada: zero extinções, entre 253 e 291 coelhos no fim, e a vegetação nunca caiu abaixo de 3,35 (de 10). 🌱

**O que se aprendeu**

1. **Sobrepastoreio:** na primeira versão a vegetação ficava sempre em cerca de 0,3 de 10, fosse qual fosse o crescimento. Os coelhos comiam tudo até ao chão. A solução foi uma resposta funcional: comem menos quando a comida é escassa (GDD 7.8).
2. **O precipício do `min_food`:** com o valor 0,5 os coelhos ficavam a comer em células pobres, em vez de procurarem melhores, e extinguiam-se todos. Com 1,5 o sistema estabilizou. Um parâmetro pequeno decidiu entre vida e extinção.
3. **Os testes de equilíbrio valem a pena:** foi o teste automático que mostrou o colapso da vegetação logo à primeira. Sem ele, só se veria muito mais tarde.

**Próximo passo:** correr tudo no teu computador (marco 5) e depois acrescentar a raposa.

---

*Modelo para entradas novas:*

```
## AAAA-MM-DD (dia da semana)

**O que foi feito**
- ...

**Marcos** 🎉
- ...

**O que se aprendeu**
- ...

**Próximo passo:** ...
```
