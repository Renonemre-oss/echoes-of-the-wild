# ECHOES OF THE WILD

## Documento de Visão e Design do Jogo (Game Design Document)

**Versão:** 0.7  
**Estado:** Conceito em desenvolvimento  
**Idioma do documento:** Português  
**Tecnologia:** Rust  
**Motor/framework:** Bevy (Rust puro)  
**Última consolidação:** 1 de novembro de 2026

> **Frase do projeto:** Um mundo selvagem. Uma sobrevivência. Milhares de histórias que não foram escritas por ninguém.
>
> **Princípio central:** O jogador não deve explorar o mundo apenas para descobrir o que existe. Deve explorar para descobrir o que está a acontecer.

---

## Índice

1. [Resumo executivo](#1-resumo-executivo)
2. [Identidade e visão do jogo](#2-identidade-e-visão-do-jogo)
3. [Pilares de design](#3-pilares-de-design)
4. [Jogos de referência e aprendizagens](#4-jogos-de-referência-e-aprendizagens)
5. [Fantasia naturalista e universo](#5-fantasia-naturalista-e-universo)
6. [Mundo, biomas e clima](#6-mundo-biomas-e-clima)
7. [Ecossistema vivo](#7-ecossistema-vivo)
8. [Criaturas e domesticação](#8-criaturas-e-domesticação)
9. [Ciclo de jogo e sobrevivência](#9-ciclo-de-jogo-e-sobrevivência)
10. [Construção, crafting e recursos](#10-construção-crafting-e-recursos)
11. [Exploração e descoberta ecológica](#11-exploração-e-descoberta-ecológica)
12. [Progressão e objetivos](#12-progressão-e-objetivos)
13. [Eventos e perigos](#13-eventos-e-perigos)
14. [Modos de jogo e multiplayer](#14-modos-de-jogo-e-multiplayer)
15. [Direção artística e apresentação](#15-direção-artística-e-apresentação)
16. [Direção técnica: Rust](#16-direção-técnica-rust)
17. [Arquitetura técnica proposta](#17-arquitetura-técnica-proposta)
18. [Protótipo inicial](#18-protótipo-inicial)
19. [Plano de desenvolvimento e aprendizagem](#19-plano-de-desenvolvimento-e-aprendizagem)
20. [Riscos, limites e decisões de design](#20-riscos-limites-e-decisões-de-design)
21. [Decisões tomadas e questões em aberto](#21-decisões-tomadas-e-questões-em-aberto)
22. [Glossário](#22-glossário)

---

# 1. Resumo executivo

**Echoes of the Wild** é um jogo de sobrevivência, exploração e construção num mundo aberto de fantasia naturalista. A sua característica principal é um ecossistema simulado que continua a funcionar mesmo quando o jogador não está a observar diretamente.

O jogador começa com poucos recursos, aprende a sobreviver, constrói um abrigo, explora diferentes biomas, observa animais e descobre como as suas ações alteram o ambiente. Os animais não existem apenas para atacar o jogador ou fornecer materiais: procuram comida, água e abrigo, evitam ameaças, defendem territórios, reproduzem-se e interagem com outras espécies.

A identidade do projeto combina quatro referências principais:

- **Minecraft:** liberdade de exploração, construção e criação de objetivos próprios.
- **Rust:** tensão da sobrevivência, preparação de expedições, risco e histórias emergentes.
- **Muck:** ciclo direto de recolha, preparação, melhoramento de equipamento e perigo crescente.
- **Rain World e Eco:** inspiração para ecossistemas, relações entre espécies e consequências ambientais.

**Valheim** é uma referência adicional para a ligação entre biomas, equipamento, construção e progressão.

O objetivo não é copiar estes jogos. É aprender com o que fazem bem e desenhar soluções próprias para evitar problemas como grind excessivo, repetição, progressão pouco significativa, mundos que parecem estáticos e penalizações demasiado frustrantes.

## 1.1. Experiência desejada

O jogador entra para sobreviver durante uma hora e acaba por passar muito mais tempo porque encontrou uma pista, observou um comportamento animal inesperado, construiu uma estrutura nova ou decidiu investigar uma alteração no ecossistema.

Um exemplo da experiência pretendida:

1. O jogador constrói uma cabana perto de um rio.
2. Deixa restos de comida no exterior.
3. Pequenos animais começam a aproximar-se.
4. Um predador identifica a nova fonte de alimento e passa a visitar a zona.
5. Os herbívoros alteram os seus percursos e evitam a área.
6. O jogador intervém, por exemplo, afugentando ou caçando o predador.
7. Nas semanas seguintes, a população de presas e a vegetação podem mudar.
8. O jogador percebe que a sua presença e as suas decisões influenciaram o vale.

Este tipo de consequência sistémica é o coração do projeto.

---

# 2. Identidade e visão do jogo

## 2.1. Género

- Survival / sobrevivência.
- Mundo aberto e exploração.
- Crafting e construção livre ou modular.
- Simulação ecológica.
- Aventura com elementos de fantasia.
- Possibilidade de jogo cooperativo e multiplayer numa fase posterior.

## 2.2. Conceito central

Um mundo naturalista que funciona segundo regras ecológicas compreensíveis, mas que contém espécies, locais e fenómenos que não existem no nosso mundo.

O jogador é uma presença dentro do ecossistema, não o centro de toda a vida. Os animais continuam a procurar alimento, a deslocar-se, a fugir, a caçar e a reproduzir-se mesmo quando o jogador não está perto.

## 2.3. O que torna o jogo diferente?

A proposta diferenciadora é juntar:

1. **Sobrevivência e construção:** o jogador recolhe recursos, fabrica ferramentas, prepara abrigos e constrói uma base.
2. **Ecologia simulada:** populações, recursos naturais e relações entre espécies podem mudar com o tempo.
3. **Descoberta baseada em observação:** o jogador aprende sobre espécies e ambientes ao investigar, em vez de receber toda a informação imediatamente.
4. **Fantasia discreta:** os elementos fantásticos são raros, misteriosos e integrados na natureza.
5. **Consequências persistentes:** ações como caçar em excesso, desflorestar, cultivar ou alterar cursos de água podem afetar a região.

## 2.4. O que o jogo não pretende ser

- Não pretende ser apenas um clone de Minecraft com outros animais.
- Não pretende ser um Rust em que o jogador tem de estar permanentemente online para proteger a base.
- Não pretende transformar a sobrevivência numa lista constante de tarefas e barras a preencher.
- Não pretende encher o mundo de monstros e magia sem contexto.
- Não pretende simular cada organismo de forma igualmente detalhada a toda a distância.
- Não pretende começar com dezenas de biomas, centenas de criaturas ou multiplayer complexo.

---

# 3. Pilares de design

Estes pilares devem orientar as decisões futuras. Quando uma mecânica nova for proposta, devemos perguntar se reforça um ou mais deles.

## Pilar 1 — Um mundo que não espera pelo jogador

O ecossistema tem atividade própria. Os animais têm necessidades, comportamentos e relações. O mundo muda mesmo quando o jogador não está a olhar.

## Pilar 2 — Liberdade para criar a própria aventura

O jogador pode escolher como sobreviver: explorar, construir, cultivar, estudar animais, caçar, domesticar, restaurar habitats ou simplesmente viver numa região.

## Pilar 3 — Preparação com significado

Uma expedição deve exigir escolhas interessantes: equipamento, comida, roupa, ferramentas, rota e riscos. A preparação não deve transformar-se em grind obrigatório.

## Pilar 4 — Descoberta através da observação

Informação importante pode ser descoberta ao seguir pegadas, observar animais, estudar plantas, reconhecer sons, encontrar abrigos ou comparar comportamentos.

## Pilar 5 — Fantasia rara, integrada e memorável

As criaturas fantásticas e os fenómenos sobrenaturais devem parecer parte do ecossistema, não elementos colocados aleatoriamente para tornar tudo mais espetacular.

## Pilar 6 — Consequências sem punição arbitrária

As ações devem ter resultados legíveis e interessantes. O jogo pode ser desafiante, mas deve evitar perdas frustrantes que pareçam existir apenas para obrigar o jogador a repetir trabalho.

---

# 4. Jogos de referência e aprendizagens

As referências abaixo são fontes de inspiração de design, não especificações para copiar. As críticas e problemas mencionados são padrões discutidos durante a fase inicial de pesquisa; não devem ser tratados como consenso universal de todos os jogadores.

## 4.1. Minecraft — liberdade, construção e exploração

**O que queremos aproveitar:**

- Construção criativa e flexível.
- Exploração sem um percurso obrigatório.
- Crafting e descoberta de materiais.
- Progressão que permite ao jogador definir objetivos próprios.
- Mundo que incentiva experiências e projetos pessoais.

**O que queremos desenvolver de forma diferente:**

- Fazer com que os biomas tenham maior impacto ecológico e funcional.
- Ligar melhor as descobertas à progressão e às decisões do jogador.
- Dar às criaturas comportamentos mais relevantes para o mundo.
- Fazer com que a exploração revele processos e histórias, não apenas estruturas e recursos.

## 4.2. Rust — tensão, risco e histórias emergentes

**O que queremos aproveitar:**

- Começar com poucos recursos.
- Sentir que uma base representa investimento e progresso.
- Planear expedições para locais perigosos.
- Encontrar recursos raros em zonas de risco.
- Possibilidade de histórias inesperadas, especialmente em multiplayer.

**O que queremos evitar ou repensar:**

- Dependência de grind excessivo.
- Perder todo o progresso devido a ataques offline, especialmente em modos PvE ou cooperativos.
- Fazer o jogador sentir que tem de viver em função do servidor.
- Tornar a progressão excessivamente dependente de repetição.

**Possível solução:** proteção de bases em modos PvE/co-op, regras próprias para servidores PvP e progressão orientada para descoberta, habilidade e construção, não apenas horas de recolha.

## 4.3. Muck — ritmo e ciclo de preparação

**O que queremos aproveitar:**

- Começar rapidamente.
- Recolher materiais e melhorar ferramentas.
- Preparar abrigo e equipamento antes de períodos perigosos.
- Ter objetivos claros a curto prazo.
- Criar experiências caóticas e divertidas com amigos.

**O que queremos evitar:**

- Repetição excessiva de vagas de inimigos.
- Progressão final pouco variada.
- Perigos que pareçam arbitrários.

**Possível solução:** o dia e a noite alteram o comportamento do mundo, mas nem todas as noites têm de ser um ataque. Podem ocorrer noites tranquilas, migrações, tempestades, atividade de predadores ou fenómenos raros.

## 4.4. Rain World — comportamentos independentes

**O que queremos aproveitar:**

- Criaturas que procuram presas, fogem de ameaças e procuram abrigo.
- Relações entre espécies que não dependem do jogador.
- Um mundo que parece vivo e imprevisível.

**O que queremos adaptar:**

- Usar comportamentos modulares e necessidades simples que possam produzir resultados emergentes.
- Tornar a observação do ecossistema acessível e legível.
- Evitar uma curva de aprendizagem excessivamente punitiva.

## 4.5. Eco — consequências ambientais

**O que queremos aproveitar:**

- Ações humanas que alteram habitats e populações.
- Relação entre exploração de recursos, biodiversidade e equilíbrio ambiental.
- Consequências que se manifestam ao longo do tempo.

**O que queremos adaptar:**

- Fazer com que essas consequências sejam visíveis e compreensíveis dentro de um survival game.
- Evitar que a simulação ambiental se transforme numa tarefa administrativa pesada.

## 4.6. Valheim — progressão ligada aos biomas

**O que queremos aproveitar:**

- Biomas que representam novos desafios.
- Ligação entre exploração, equipamento, alimentação e construção.
- Sensação de avanço para regiões desconhecidas.
- Construção de bases com utilidade prática e valor emocional.

**O que queremos melhorar:**

- Reduzir a repetição desnecessária na recolha e transporte de recursos.
- Fazer com que novos biomas também introduzam mudanças ecológicas e novas formas de descoberta.

---

# 5. Fantasia naturalista e universo

## 5.1. Direção escolhida

**Fantasia naturalista.** Esta é uma decisão já tomada durante a conversa.

A regra artística e narrativa é:

> O mundo deve parecer real até o jogador descobrir que não é completamente real.

A maior parte do mundo será composta por natureza reconhecível: florestas, rios, montanhas, pântanos, animais, plantas, clima e ciclos naturais. Os elementos fantásticos surgem com moderação e têm uma explicação dentro do universo, ainda que nem sempre seja revelada imediatamente.

**Decisão (v0.5):** quantidade moderada de fantasia, com várias criaturas fantásticas e ruínas, mantendo a regra de que cada região só tem uma pequena parte delas e que cada uma tem função ecológica.

## 5.2. Princípios para a fantasia

- Não colocar criaturas mágicas em todos os cantos do mapa.
- Dar a cada espécie fantástica uma função ou relação ecológica.
- Fazer com que o jogador descubra a existência destas criaturas através de indícios.
- Usar ruínas, plantas, sons, marcas e alterações ambientais para construir mistério.
- Evitar que todas as criaturas especiais sejam inimigos ou bosses.

## 5.3. Exemplo de criatura especial: The Elder Stag

**Nome provisório:** The Elder Stag  
**Categoria:** criatura ancestral rara  
**Bioma associado:** florestas antigas, especialmente The Ancient Wilds.

Características propostas:

- Aparência semelhante à de um veado, mas de dimensões e idade extraordinárias.
- Chifres com padrões bioluminescentes discretos.
- Evita o contacto humano.
- Aparece raramente e pode deslocar-se por grandes áreas.
- A sua presença pode estar associada a alterações subtis na vegetação ou no comportamento de outras espécies.
- Não é necessariamente um boss e não deve existir apenas para ser morto.

A intenção é que o jogador possa passar muito tempo a encontrar pistas da criatura antes de a observar diretamente. A sua descoberta deve ser memorável.

Esta criatura é um conceito inicial, não uma especificação definitiva.

---

# 6. Mundo, biomas e clima

## 6.1. Filosofia dos biomas

Os biomas não devem ser apenas mudanças de cor, árvores e materiais. Cada bioma deve ter:

- Clima e condições ambientais próprios.
- Vegetação característica.
- Espécies animais próprias ou populações diferentes.
- Recursos e materiais específicos.
- Riscos e oportunidades diferentes.
- Comportamentos ecológicos que afetem a forma como o jogador explora e sobrevive.
- Segredos ou fenómenos raros, quando adequado.

## 6.2. Biomas propostos

Os nomes são provisórios e podem ser alterados.

### 1. The Greenwood — floresta inicial

**Função:** introduzir o jogador às regras principais do mundo.

Elementos propostos:

- Florestas densas e clareiras.
- Pequenos rios e lagos.
- Veados, coelhos, javalis, raposas e lobos.
- Recursos básicos de madeira, pedra, plantas e frutos.
- Abrigos naturais e locais adequados para construir a primeira base.

### 2. The Hollow Marsh — pântano

Elementos propostos:

- Pântanos, zonas alagadas e nevoeiro.
- Fungos e plantas bioluminescentes.
- Criaturas adaptadas a água pouco profunda e ambientes húmidos.
- Recursos raros ligados a plantas, fungos e minerais.
- Espécies ou comportamentos que se tornam mais visíveis em certas condições climáticas.

### 3. The Frostlands — montanhas geladas

Elementos propostos:

- Montanhas, lagos congelados e florestas de coníferas.
- Animais adaptados ao frio.
- Cavernas e passagens montanhosas.
- Maior importância da roupa, abrigo, fogo e gestão da temperatura.
- Recursos que não existem nos biomas mais quentes.

### 4. The Ancient Wilds — floresta ancestral

Elementos propostos:

- Árvores muito antigas e estruturas naturais invulgares.
- Ruínas parcialmente cobertas pela vegetação.
- Espécies raras e comportamentos incomuns.
- Indícios sobre a história do mundo e a origem dos fenómenos fantásticos.
- Possíveis habitats de criaturas ancestrais, incluindo The Elder Stag.

### 5. The Ashen Reach — território vulcânico

Elementos propostos:

- Terreno vulcânico e zonas de calor extremo.
- Cinzas, rochas e minerais raros.
- Criaturas adaptadas a temperaturas elevadas.
- Perigos geológicos e ambientais.
- Preparação especializada para exploração.

## 6.3. Clima, estações e ciclos

A existência de estações completas é uma ideia para explorar, não uma funcionalidade confirmada para o primeiro protótipo.

> **Nota de implementação (calendário, novembro de 2026):** o protótipo já tem um calendário (`eotw_sim/src/calendar.rs`). O ano é comprimido em 36 dias de jogo, 3 dias por mês. A escala sai das idades que o jogo já usa: o coelho amadurece aos 10 dias de jogo e o coelho-bravo do sul de Espanha cria aos 3–4 meses; a raposa amadurece aos 30 dias e a raposa real acasala aos 10 meses. A simulação começa em julho, e com 60 dias de aquecimento o jogador chega em março.
>
> Para cada mês, o calendário dá a estação, a época de reprodução do coelho, a fase do ano da raposa e a probabilidade mensal de uma raposa jovem dispersar. Há quatro climas: mediterrânico ibérico (o padrão), temperado oceânico, temperado continental e temperado do sul, com as estações trocadas.
>
> - **Coelhos (ibérico):** criam de novembro a junho, com pico em março–abril, e param no verão seco (Gonçalves 2002; Delibes & Calderón; Soriguer). Num ano seco criam só de janeiro a março.
> - **Raposas (ibérico):** acasalam em janeiro e têm partos do fim de fevereiro a março. As crias saem da toca em maio (Doñana). A dispersão vai de outubro a março, com a tabela mensal de Bristol (Smith & Harris 1991, em Devenish-Nelson 2012), que coincide com o outubro–janeiro de Doñana (Travaini et al. 1993).
>
> O ecrã mostra a data e o que os animais estão a fazer. Por agora os animais ainda não usam o calendário: o resultado da simulação é igual ao de antes. As regras de reprodução por época e de dispersão (relatório "Vida social mundial coelho e raposa") vão depender dele. O comprimento do dia ainda não muda com a estação.

Possíveis sistemas:

- Chuva e tempestades.
- Nevoeiro e alterações de visibilidade.
- Neve e frio.
- Períodos de seca.
- Ciclos sazonais que alterem alimento disponível, migração e reprodução.
- Eventos climáticos que modifiquem temporariamente a acessibilidade de certas regiões.

O clima deve influenciar o jogo de forma legível: o jogador deve conseguir aprender a preparar-se e antecipar alguns riscos.

---

# 7. Ecossistema vivo

Esta é a principal característica diferenciadora do jogo.

## 7.1. Necessidades das criaturas

Cada espécie poderá ter uma combinação das seguintes necessidades:

- **Alimentação:** procurar plantas, frutos, presas ou outras fontes de alimento.
- **Água:** encontrar rios, lagos ou outras fontes.
- **Abrigo:** procurar tocas, ninhos, cavernas, vegetação ou zonas protegidas.
- **Segurança:** evitar predadores, ameaças e perturbações.
- **Reprodução:** encontrar parceiros e condições adequadas para criar descendência.
- **Território:** ocupar, defender ou abandonar determinadas áreas.
- **Descanso:** alternar atividade e repouso.

Nem todas as espécies precisam de todos estes sistemas. A profundidade deve ser implementada de acordo com o papel de cada criatura e com a prioridade do protótipo.

## 7.2. Interações ecológicas

O jogo poderá simular relações como:

- Herbívoros alimentam-se de vegetação.
- Predadores caçam herbívoros ou animais menores.
- Animais competem por recursos e território.
- Algumas espécies vivem em grupos.
- Algumas espécies defendem crias ou ninhos.
- Organismos decompositores reciclam matéria orgânica.
- A disponibilidade de água e vegetação influencia a distribuição das populações.

### Exemplo de cadeia ecológica

**Vegetação → herbívoros → predadores → decompositores → nutrientes → vegetação.**

Esta cadeia não precisa de ser completamente simulada em termos científicos. O objetivo é produzir consequências consistentes e compreensíveis durante o jogo.

## 7.3. Consequências de intervenção do jogador

Exemplos de situações emergentes:

- Se o jogador caçar demasiados lobos, poderá reduzir a pressão sobre algumas populações de herbívoros.
- Se os herbívoros aumentarem demasiado, podem consumir mais vegetação.
- A desflorestação pode remover abrigos e alterar percursos dos animais.
- As plantações podem atrair herbívoros e predadores.
- Fogueiras, ruído, cheiros e construções podem afastar ou atrair certas espécies.
- Barragens e canais podem alterar o acesso à água.
- Incêndios, secas e inundações podem transformar temporariamente os habitats.

Estas consequências devem depender do contexto e não ser garantidas como resultados fixos. O sistema ecológico deve permitir resultados diferentes consoante as condições locais.

## 7.4. Simulação em camadas

Não é necessário calcular todos os animais com o mesmo detalhe a toda a distância. A proposta técnica é usar níveis de simulação.

### Perto do jogador — simulação completa

- Entidades individuais presentes no mundo.
- Movimento e animações.
- Decisões detalhadas.
- Caça e fuga.
- Interações com o jogador e com outras criaturas.
- Sons, rastos e reações imediatas.

### Nas proximidades — simulação reduzida

- Atualização de necessidades e estados principais.
- Deslocação simplificada.
- Alimentação, caça ou fuga representadas através de eventos abstratos.
- Menos cálculos de movimento e apresentação.

### Longe do jogador — simulação populacional

Em vez de atualizar individualmente centenas de animais, o jogo poderá manter dados agregados por região:

- População aproximada por espécie.
- Disponibilidade de vegetação e água.
- Tendência de crescimento ou diminuição.
- Pressão de predadores.
- Estado geral do habitat.

Quando o jogador se aproximar, a população poderá ser convertida novamente em indivíduos simulados. A transição deve preservar coerência suficiente para não parecer que os animais surgiram do nada.

**Nota:** números como "10.000 animais" ou distâncias como "10 metros, 500 metros e 5 km" são apenas exemplos conceptuais. Os valores reais terão de ser definidos por testes de performance.

## 7.5. Objetivos de qualidade do ecossistema

- Os comportamentos devem parecer naturais, mas não precisam de ser cientificamente perfeitos.
- As consequências devem ser legíveis para o jogador.
- A simulação deve ser eficiente e previsível.
- As populações não devem desaparecer ou crescer sem controlo devido a erros de sistema.
- O jogador deve conseguir aprender padrões através da observação.
- Os sistemas devem produzir variedade sem depender de scripts manuais para cada acontecimento.

## 7.6. Perceção

O teste central do Prototype 01 (comida deixada junto à cabana) só funciona se os animais conseguirem detetar a comida sem o jogo lhes dar essa informação diretamente. A perceção tem, por isso, de ser um sistema próprio.

**Três canais:**

- **Visão:** alcance e ângulo por espécie, reduzidos de noite e com nevoeiro.
- **Audição:** eventos de ruído com posição, raio e intensidade (cortar madeira, correr, combate). Os animais dentro do raio reagem no passo de simulação seguinte.
- **Olfato:** campos de cheiro por célula.

**Campos de cheiro:**

- O mundo tem uma grelha (por chunk) com valores de cheiro por tipo: *alimento*, *carcaça*, *presa*, *predador*, *humano*.
- Fontes (restos de comida, carcaças, animais, jogador, fogueira) depositam intensidade na sua célula. A cada passo o cheiro espalha-se um pouco para as células vizinhas e decai.
- Os animais não "sabem" onde está a comida: amostram o cheiro das células à volta e seguem o gradiente do tipo que lhes interessa.
- O vento pode alterar a difusão numa fase posterior; não faz parte do primeiro protótipo.

**Sensibilidade por espécie** (valores ilustrativos, a calibrar):

| Espécie | Visão | Audição | Olfato |
|---|---|---|---|
| Coelho | média | alta | média |
| Veado | alta | alta | média |
| Javali | baixa | média | alta |
| Raposa | média | alta | alta |
| Lobo | média | alta | muito alta |

**Custo:** os campos só são atualizados nas zonas em simulação completa ou reduzida. Longe do jogador, o efeito do cheiro é absorvido pelos dados agregados da região.

**Como isto serve o teste da cabana:** os restos de comida emitem cheiro de *alimento*. Coelhos e raposas seguem-no; o lobo reage ao cheiro de *presa* e *carcaça* que a raposa e os coelhos passam a deixar na zona. A cadeia de acontecimentos surge sem ser escrita.

**No Prototype 01 (marco 8):** existem os cheiros de *alimento* (restos) e de *presa* (coelhos). O cheiro decai 1% por hora, e um coelho dentro da toca não deixa cheiro. O coelho dá prioridade à erva que vê e só segue o cheiro se não houver comida à volta: sem esta regra, o cheiro de restos já comidos prendia os coelhos junto à cabana até morrerem de fome. Existe também o cheiro *humano*: o jogador deixa rasto por onde passa, e a raposa sai de células onde ele é forte e não entra nelas (o coelho ignora-o e só reage ao que vê). Um rasto de patrulha afasta as raposas durante uns dois dias. A raposa usa o nariz quando não vê presas e é necrófaga: um cheiro forte de restos faz-lhe largar a caça. Esta necrofagia tem de ser moderada, porque raposas a mais acampadas na cabana afastam os coelhos e, com o travão de densidade, deixam de ter crias.

## 7.7. Vegetação como sistema

A cadeia ecológica da secção 7.2 só tem consequências reais se a vegetação for um recurso que se gasta e se regenera.

- Cada célula do terreno tem **capacidade** (máximo de biomassa, definido pelo solo, pela água e pelo bioma) e **biomassa atual**.
- A biomassa regenera-se com uma curva logística: cresce devagar quando está muito baixa, mais depressa a meio, e estabiliza perto da capacidade.
- Os herbívoros consomem biomassa da célula onde estão. Se a biomassa ficar abaixo de um limiar durante demasiado tempo, a capacidade da célula baixa temporariamente (sobrepastoreio) e recupera devagar quando a pressão desaparece.
- Árvores e recursos que o jogador corta são entidades separadas. Desflorestar reduz o abrigo e a capacidade das células à volta.

  **No Prototype 01 (etapa 6):** cortar só reduz o abrigo, e não a capacidade da erva. Uma árvore inteira abriga as 9 células à sua volta. A raposa apanha um coelho abrigado como se ele estivesse em erva com 60% da altura máxima, e o coelho, ao procurar erva, prefere as células abrigadas. À volta de uma árvore cortada fica uma clareira, onde os coelhos se sentem expostos e contam a erva com 6 a menos. A árvore leva 40 dias a voltar a ficar inteira. Na zona cortada há cerca de 70% menos coelhos e mais do dobro da erva durante esse tempo. Num mundo sem cortes não há clareiras, por isso o equilíbrio não muda. A cabana fica numa clareira própria, sem árvores à volta. Um abrigo mais forte tornava o mundo diferente consoante o tamanho do mapa (menos 17% de raposas em 48x48).
- Plantações são células com biomassa de alto valor, o que explica porque atraem herbívoros e, indiretamente, predadores.
- Frutos sazonais ficam para uma fase posterior.

## 7.8. Equilíbrio das populações

Sistemas predador-presa simples tendem a oscilar com amplitude crescente até uma espécie desaparecer. O jogo vai precisar de mecanismos estabilizadores desde o início.

**Mecanismos propostos:**

1. **Capacidade de carga:** o número de herbívoros que uma região sustenta é limitado pela vegetação (secção 7.7).
2. **Limites ao predador:** cada predador só consegue caçar um número limitado de presas por dia, e depois de comer fica saciado durante um período.
3. **Refúgios:** tocas e vegetação densa reduzem a probabilidade de sucesso da caça, para que as presas nunca fiquem totalmente expostas.
4. **Território:** lobos ocupam e defendem território, o que limita a sua densidade.
   No Prototype 01 a raposa usa uma versão mais simples, um **travão de densidade**: não ocupa nem defende uma área, mas só tem crias se não houver outra raposa a menos de um certo raio (`territory_radius`). O travão conta também com a comida: restos de comida perto dão lugar a mais raposas (cada 40 de energia em restos permite mais uma rival), senão raposas juntas numa fonte de comida rica deixavam de ter crias. O território verdadeiro, com ocupação e defesa, fica para o lobo.
5. **Reprodução condicionada:** os animais só se reproduzem com energia acima de um limiar e depois de um período de recuperação.
6. **Migração de segurança:** se uma espécie ficar abaixo de um mínimo durante vários dias, indivíduos chegam pelas margens do mapa. Num mapa pequeno, isto impede extinções locais irreversíveis e pode ser apresentado como migração natural.

**Dados fora do código:** todos os parâmetros (taxas, limiares, sensibilidades) vivem em ficheiros de dados (por exemplo RON ou TOML), para ajustar o equilíbrio sem recompilar.

**Como medir:** o modo headless (secção 17.8) corre dezenas de dias simulados com várias seeds e produz um relatório de populações. Os critérios de aceitação estão na secção 18.6.

## 7.9. Transição entre simulação agregada e individual

Este é o ponto tecnicamente mais delicado da simulação em camadas (7.4).

**Ao reduzir o detalhe de uma região** (o jogador afasta-se), guardar por espécie:

- número de animais;
- distribuição simples de idade e energia (histograma);
- pontos de ancoragem: tocas, pontos de água, centro dos grupos.

**Ao aumentar o detalhe** (o jogador aproxima-se), gerar os indivíduos a partir desses dados, com um gerador aleatório derivado de *seed do mundo + id do chunk + tick + id da espécie*. Assim a conversão é reproduzível, o que simplifica guardar/carregar e os testes.

**Tamanho mínimo de região:** a simulação de coelhos e raposas só é estável em regiões de pelo menos 24×24 células (`MIN_REGION_SIZE`). Numa varredura com 40 seeds, 20×20 ainda aguenta, mas com 16×16 ou menos há tão poucas raposas que desaparecem por azar e a migração passa a ser precisa muitas vezes. Os chunks podem ser mais pequenos, mas a simulação agregada junta-os em regiões deste tamanho ou maiores; uma região pequena e isolada (uma ilha, um vale fechado) tem de contar com a migração vinda das vizinhas.

**No Prototype 01 (etapa 5):** a região agregada (`region.rs`) guarda coelhos e raposas por idade em dias e a vegetação média, e avança um dia de cada vez. As regras são tabelas medidas na simulação completa (nascimentos por adulto e mortes por fome em função da vegetação, 0,21 coelhos caçados por raposa por dia, crias de raposa em função da densidade de raposas). A partir do mesmo estado, tem o mesmo equilíbrio que a completa (275 coelhos contra 269, 10,8 raposas contra 10,9, entre os dias 500 e 900) e recupera de choques à mesma velocidade. As raposas só chegam ao equilíbrio depois de as primeiras morrerem de velhice, ao fim de 400 dias: a tabela das crias de raposa é medida a partir do dia 500. Varia menos de dia para dia.

A conversão inversa (`AggregateRegion::expand`) volta a criar animais individuais com a mesma população por idade, a mesma vegetação média e as mesmas tocas. Sorteia o resto (posições, energia, vegetação de cada célula) com distribuições medidas na completa. Nenhum animal aparece a 8 células ou menos do jogador, e a mesma regra vale para a migração e para a chegada das raposas. Os sorteios vêm da seed, da região, do dia e da espécie. As raposas são postas umas perto das outras: espalhadas ao acaso, as que ficavam sozinhas não tinham travão de densidade e tinham crias todas de uma vez.

A paisagem (`landscape.rs`) junta as regiões num mundo. À volta do jogador há uma janela de 3x3 regiões simuladas por inteiro numa só simulação, e as restantes são agregadas. Quando o jogador entra mais de 4 células numa região vizinha, a janela desloca-se: o que sai é agregado, o que entra é expandido, e o que fica guarda cada animal. Estas 4 células de folga (histerese) evitam que andar em cima de uma fronteira desloque a janela a cada passo. As trocas de nível de detalhe acontecem a 20 células ou mais do jogador. Uma vez por dia, alguns animais passam para as regiões vizinhas, à taxa medida na completa (2,3% dos coelhos e 0,29% das raposas por fronteira). Uma região vazia recupera assim os coelhos numa semana e as raposas em um ou dois meses. Antes de o jogador chegar, a paisagem corre 600 dias só com regiões agregadas, para as raposas já estarem em equilíbrio (cerca de 10 por região) no primeiro dia de jogo. O jogo corre uma paisagem de 5x5 regiões, com a câmara a seguir o jogador. Só se desenha o que está à volta dele, e isso está sempre dentro da janela simulada por inteiro.

**Regras:**

- A população total conserva-se na conversão (com tolerância pequena).
- Nunca surgem animais dentro do campo de visão do jogador nem abaixo de uma distância mínima.
- O estado gerado é compatível com a hora do dia (animais noturnos ativos de noite, etc.).
- Indivíduos importantes (domesticados, marcados pelo jogador, criaturas especiais como o Elder Stag) nunca são agregados: continuam a existir como entidades.
- Perde-se o detalhe individual de animais comuns (um veado ferido deixa de o estar depois de a região ser agregada). Isto é aceitável e deve ser assumido como regra do jogo.

## 7.10. Legibilidade para o jogador

Um ecossistema só é interessante se o jogador o consegue ler. Os sinais devem existir em vários canais, sem HUD técnico:

- **Rastos:** pegadas, ervas pisadas e excrementos como depósitos temporários na grelha, com tipo de espécie e idade aproximada.
- **Som:** cada espécie tem sons reconhecíveis para alerta, chamamento e alimentação.
- **Linguagem corporal:** cabeça levantada, orelhas viradas, grupo a juntar-se, fuga em direção a abrigo.
- **Vestígios:** carcaças, tocas, ninhos, zonas de vegetação rapada.
- **Diário:** regista o que o jogador efetivamente observou (secção 11.2), não o estado interno da simulação.

**Critério de qualidade:** perante um comportamento observado (por exemplo, um veado a fugir de repente), o jogador deve conseguir identificar uma causa plausível pelos sinais disponíveis.

---

# 8. Criaturas e domesticação

## 8.1. Primeiras espécies propostas

### Animais pacíficos ou geralmente não agressivos

| Espécie | Comportamento proposto | Papel ecológico |
|---|---|---|
| Veado | Vive em grupos, procura vegetação e foge de ameaças | Herbívoro de médio/grande porte |
| Coelho | Procura alimento, cria tocas e reproduz-se | Presa de pequenos e médios predadores |
| Javali | Procura alimento e pode reagir agressivamente quando ameaçado | Omnívoro que remexe o solo |
| Raposa | Caça animais pequenos e explora restos de comida | Predador pequeno e oportunista |

### Predadores

| Espécie | Comportamento proposto | Papel ecológico |
|---|---|---|
| Lobo | Pode caçar em grupo, comunicar e defender território | Predador de médio/grande porte |
| Urso | Procura alimento, pode defender crias e ser perigoso | Omnívoro de grande porte |

### Criaturas especiais

| Criatura | Conceito | Função possível |
|---|---|---|
| The Elder Stag | Veado ancestral com marcas luminosas | Mistério ecológico e descoberta rara |
| Predador ancestral | Criatura rara com território próprio | Desafio de observação e sobrevivência; não necessariamente um boss |

A lista é um ponto de partida. A quantidade de espécies do jogo final ainda não está definida.

## 8.2. Domesticação

A domesticação deve ser um processo baseado em comportamento e confiança, não apenas num botão.

Exemplo de processo para um lobo:

1. Descobrir onde vive a alcateia.
2. Observar os seus comportamentos e hierarquia.
3. Aproximar-se com cuidado e aprender o que representa uma ameaça.
4. Criar oportunidades de associação positiva, por exemplo através de alimento adequado.
5. Desenvolver confiança gradualmente.
6. Conseguir estabelecer uma relação de acompanhamento ou cooperação.

Possíveis utilidades de animais domesticados:

- Acompanhar o jogador.
- Detetar ameaças.
- Ajudar a localizar recursos.
- Apoiar expedições.
- Proteger rebanhos ou a base.
- Participar em tarefas específicas, dependendo da espécie.

A domesticação não deve permitir transformar qualquer animal selvagem num servo instantâneo. Algumas espécies podem não ser domesticáveis.

## 8.3. Reprodução e hereditariedade

**Decisão (v0.5):** hereditariedade completa, numa fase tardia do desenvolvimento. Ideia para essa fase:

- Reprodução entre animais domesticados ou populações selvagens.
- Características transmitidas entre gerações.
- Variações físicas ou comportamentais moderadas.
- Adaptação gradual a certos ambientes.

Este sistema é ambicioso e não pertence ao primeiro protótipo.

---

# 9. Ciclo de jogo e sobrevivência

## 9.1. Ciclo principal

O ciclo central proposto é:

**Explorar → Recolher → Aprender → Construir → Preparar → Enfrentar riscos → Melhorar → Explorar mais longe.**

A descoberta ecológica pode surgir em qualquer ponto do ciclo e deve dar motivos para o jogador alterar os seus planos.

## 9.2. Dia e noite

O ciclo dia/noite deve ter impacto real, mas não transformar todas as noites numa vaga obrigatória de inimigos.

| Período | Atividades e características possíveis |
|---|---|
| Manhã | Exploração, recolha, observação e agricultura |
| Tarde | Expedições, construção e preparação |
| Pôr do sol | Regresso, organização e proteção da base |
| Noite | Menor visibilidade, predadores noturnos e espécies específicas |

A noite poderá ser tranquila ou perigosa. O perigo depende de fatores como o bioma, clima, presença do jogador, alimento disponível e atividade das espécies.

**No Prototype 01 (etapa 6):** a luz vai de 1 a 0, com o amanhecer das 5h às 7h e o anoitecer das 19h às 21h (`daylight.rs`). De noite o coelho vê 2 células em vez de 3, e a raposa 4 em vez de 5. De dia a raposa descansa e só caça abaixo de 20 de energia (de noite, abaixo de 70), por isso quase todas as caçadas (98%) são ao anoitecer e de noite. Só a visão reduzida não chegava: as caçadas ficavam iguais em todas as horas. No jogo, a noite é azul-escura longe do jogador e mais clara até 6 células dele. Ao meio-dia (das 9h às 17h), os coelhos com energia entre 40 e 60 descansam numa toca, onde não gastam energia e recuperam 1,5 por hora, como os coelhos bravos, que digerem na lura de dia e comem ao crepúsculo e de noite. Os outros continuam ativos. Aos 60 saem para comer, porque só a comer chegam aos 80 de que precisam para ter crias: sem este teto, a energia do descanso desligava a população da comida. Ao meio-dia estão na toca 27% dos coelhos, contra 4% à meia-noite. De noite o coelho vê pouco, mas ouve os passos do jogador a 3 células. A raposa tem o êxito medido na natureza (25% das tentativas) e gasta 2,4 por hora ativa e 0,4 a descansar, por isso um coelho lhe chega para um dia e meio. O resto do que come são roedores, que não são animais da simulação: um salto na erva, que acerta menos na erva alta. Com isto, a população de coelhos oscila 20% (antes 8,8%), em ciclos de predador e presa (relatório "Mecânicas quantitativas dos animais"). A região agregada passou a guardar a reserva (a fração dos coelhos com 60 ou mais de energia), que é o que prevê as crias.

## 9.3. Sistemas de sobrevivência

**Decisão (v0.5):** necessidades completas (alimentação, água, temperatura e energia). O ritmo de cada uma deve ser lento e calibrado nos testes.

Os sistemas fundamentais são:

| Sistema | Função | Princípio de design |
|---|---|---|
| Alimentação | Manter o jogador nutrido | Alimentos diferentes podem oferecer benefícios diferentes |
| Água | Manter o jogador hidratado | Fontes de água naturais ligadas ao ecossistema e aos animais |
| Temperatura | Proteger contra frio e calor | Roupa, fogo e abrigo devem permitir preparação |
| Saúde | Representar ferimentos e recuperação | O dano deve ter consequências compreensíveis |
| Energia | Representar esforço e descanso | Evitar interromper constantemente a exploração |
| Preparação | Equipamento, ferramentas e provisões | As escolhas devem importar sem exigir grind excessivo |

A fome, sede ou temperatura não devem obrigar o jogador a interromper a atividade de poucos em poucos minutos. A sobrevivência deve gerar decisões, não tarefas repetitivas.

**No Prototype 01 (versão simplificada, ainda sem pesquisa):** o jogador tem fome e saúde, de 0 a 100 (`survival.rs`). A saciedade desce 1,4 por hora: cheio, só fica sem comida ao fim de três dias. Com a saciedade a 0 perde 2 de saúde por hora; acima de 30 recupera 1 por hora. A comida (ervas, raízes e cogumelos) procura-se na erva com F: 1 hora e 3 de vegetação por porção, por isso o jogador compete com os coelhos. C come uma porção (+25). Com a saúde a 0 o jogador desmaia e acorda com 25 de saúde e de saciedade: ainda não há morte. Os números são decisões de jogo, a rever com pesquisa (necessidades reais, alimentos silvestres por estação, caça).

## 9.4. Combate

**Decisão (v0.5):** combate de peso médio, com armas variadas e técnica. Direção:

- Deve ser uma parte da sobrevivência, não a única forma de progressão.
- O comportamento do animal deve importar mais do que barras de vida exageradas.
- Algumas criaturas devem preferir fugir a lutar.
- Preparação, terreno, equipamento e leitura do comportamento devem fazer diferença.
- Nem todos os encontros devem terminar em combate.

---

# 10. Construção, crafting e recursos

## 10.1. Filosofia de construção

A construção deve ser uma atividade central e criativa, com liberdade inspirada em Minecraft e utilidade prática inspirada em jogos survival.

A abordagem exata — blocos, peças modulares ou sistema híbrido — foi adiada até existir motor visual (decisão v0.5).

## 10.2. Estruturas propostas

| Estrutura | Utilidade |
|---|---|
| Cabana | Abrigo inicial (o jogador constrói-o; ver a nota abaixo) |
| Torre de vigia | Observar o território e detetar ameaças |
| Armazém | Guardar materiais e alimentos |
| Oficina | Criar e melhorar ferramentas e equipamento |
| Estábulo | Abrigar animais domesticados |
| Estufa | Cultivar plantas em condições controladas |
| Cercas | Proteger plantações e animais |
| Armadilhas | Capturar animais ou obter recursos, com regras próprias |
| Torre defensiva | Proteger a base em situações de ameaça |

> **Nota de implementação (abrigo, novembro de 2026):** o jogador começa sem abrigo e sem tenda. O objetivo dos primeiros dias é construir o seu. O abrigo faz-se em quatro etapas, no mesmo sítio (tecla B), e cada etapa acrescenta algo que a anterior não tinha. A escada segue os quatro tipos de chozo de pastor da Estremadura (Martín Galindo 2006), e o primeiro passo segue o abrigo de ramos do manual FM 21-76:
>
> | Etapa | Custo | Horas |
> |---|---|---|
> | Abrigo de ramos, encostado a uma árvore | 6 madeira | 2 |
> | Cabana de folhas (teto e cama de folhas) | 10 folhas | 3 |
> | Chozo com base de pedra | 8 pedra, 6 madeira | 6 |
> | Chozo com lareira | 6 pedra | 4 |
>
> As folhas recolhem-se da erva onde não há árvore nem pedra, e gastam vegetação: tirar folhas tira comida aos coelhos. Os custos e os tempos são decisões de jogo, ancoradas em medidas: 5–8 varas para o abrigo de ramos, poucas horas para uma cabana leve, dias para uma de pedra.
>
> Por agora o abrigo ainda não protege o jogador de nada, porque o jogador ainda não tem frio nem cansaço. O que faz é cheirar a gente. Deixa cheiro humano por hora, menos do que o jogador nas três primeiras etapas e mais com a lareira, por causa do fumo, e os animais afastam-se. A pesquisa está no relatório "Abrigos construídos pelo jogador".

## 10.3. Construção ligada ao ambiente

As construções devem interagir com as condições do mundo:

- Pedra poderá isolar melhor ou resistir melhor a certos danos.
- Madeira poderá ser rápida de construir, mas mais vulnerável ao fogo.
- Estruturas elevadas poderão proteger contra algumas ameaças ou inundações.
- Fogueiras e fontes de luz poderão atrair ou afastar espécies.
- Plantações podem atrair herbívoros.
- Construções em zonas ribeirinhas poderão ser afetadas por cheias.
- A localização de uma base deve ter vantagens e desvantagens ecológicas.

## 10.4. Crafting

Categorias iniciais propostas:

- Ferramentas de recolha.
- Armas e equipamento de defesa.
- Abrigos e peças de construção.
- Recipientes e armazenamento.
- Equipamento de exploração.
- Roupa e proteção climática.
- Utensílios para agricultura e domesticação.

As receitas específicas, níveis de qualidade e árvore de tecnologia ainda estão por definir.

**No Prototype 01 (etapa 6):** cada região de 24x24 tem 20 árvores (4 madeiras, que voltam a crescer 0,1 de madeira por dia) e 8 pedras (6 pedras, que não voltam), sorteadas com números à parte para não mudarem o resto do mundo (`resources.rs`). O jogador recolhe com a tecla E, na célula onde está ou numa das 4 ao lado: 1 unidade por recolha, e cada recolha gasta uma hora de jogo. O inventário leva 20 coisas. Por agora, cortar árvores não mexe na erva nem no abrigo dos animais (7.7). As primeiras ferramentas são o machado (3 madeiras e 2 pedras) e a picareta (2 madeiras e 3 pedras). Fazem-se em qualquer sítio e cada uma gasta 1 hora. Com a ferramenta certa recolhem-se 2 unidades por hora em vez de 1, e cada ferramenta parte-se ao fim de 30 recolhas.

## 10.5. Evitar o grind

- Reduzir receitas que exigem quantidades absurdas de materiais comuns.
- Evitar obrigar o jogador a repetir a mesma atividade sem novas decisões.
- Permitir que conhecimento, ferramentas e planeamento reduzam o esforço.
- Dar utilidade a diferentes materiais e regiões sem criar deslocações artificiais.
- Fazer com que as melhorias alterem possibilidades de jogo, não apenas números.

---

# 11. Exploração e descoberta ecológica

## 11.1. Mecânica proposta: Ecological Discovery

O jogador não recebe automaticamente uma enciclopédia completa das espécies. Aprende através de observação, pistas e experiência.

Exemplo de descoberta:

1. Encontrar pegadas perto de um rio.
2. Identificar que pertencem a uma espécie ainda desconhecida.
3. Observar a área e procurar sinais adicionais.
4. Descobrir o que a espécie come.
5. Seguir os seus movimentos.
6. Observar um comportamento raro, como caça, migração ou reprodução.
7. Registar a descoberta num diário.

## 11.2. Diário de exploração

O diário pode funcionar como uma enciclopédia pessoal, preenchida à medida que o jogador aprende.

Entradas possíveis:

- Espécies observadas.
- Pegadas e rastos reconhecidos.
- Plantas e materiais identificados.
- Preferências alimentares conhecidas.
- Horários de atividade aproximados.
- Habitats preferidos.
- Relações entre espécies.
- Fenómenos climáticos e eventos raros.
- Locais e ruínas descobertos.

## 11.3. Conhecimento como progressão

A informação pode desbloquear oportunidades sem se transformar numa árvore de tecnologia convencional.

Exemplos:

- Identificar uma planta permite utilizá-la em receitas.
- Descobrir o comportamento de uma espécie permite aproximar-se sem a assustar.
- Conhecer os hábitos de um predador ajuda a atravessar o seu território.
- Estudar materiais pode revelar técnicas de crafting.
- Descobrir condições de crescimento pode melhorar a agricultura.

O conhecimento deve ser útil, mas não deve bloquear todas as ações básicas atrás de tarefas de investigação.

---

# 12. Progressão e objetivos

A progressão deve permitir que o jogador escolha o seu próprio estilo, em vez de obrigar todos a seguir uma única rota.

## Fase 1 — Sobrevivente

- Começar sem equipamento significativo.
- Encontrar água e alimento.
- Recolher madeira e pedra.
- Criar ferramentas básicas.
- Construir um abrigo.
- Aprender os perigos imediatos do bioma inicial.

## Fase 2 — Explorador

- Melhorar ferramentas.
- Explorar áreas mais distantes.
- Descobrir novos materiais e espécies.
- Aprender a lidar com clima e predadores.
- Encontrar ruínas e pistas sobre o mundo.

## Fase 3 — Construtor

- Construir uma base mais desenvolvida.
- Criar zonas agrícolas.
- Domesticar determinadas espécies.
- Estabelecer postos avançados.
- Melhorar armazenamento e produção.
- Gerir a relação da base com o ambiente.

## Fase 4 — Guardião da Natureza

- Explorar regiões remotas.
- Compreender ecossistemas complexos.
- Enfrentar desafios ambientais de grande escala.
- Descobrir fenómenos ancestrais.
- Escolher entre explorar, proteger, transformar ou simplesmente habitar o mundo.

Estas fases são uma estrutura conceptual. Não implicam que o jogo tenha uma campanha linear ou que todos os jogadores tenham de as completar pela mesma ordem.

**Decisão (v0.5):** não há campanha principal. A história é ambiental, contada por ruínas, vestígios e pelo diário de exploração (secção 11.2).

## Estilos de jogo possíveis

- Explorador solitário.
- Construtor e arquiteto.
- Agricultor e criador de animais.
- Naturalista e investigador.
- Caçador e sobrevivente.
- Restaurador de habitats.
- Aventureiro focado em ruínas e mistérios.

---

# 13. Eventos e perigos

Os eventos devem emergir do mundo e do clima, não ser apenas ataques programados.

Exemplos propostos:

- Tempestades que reduzem a visibilidade e dificultam a deslocação.
- Incêndios que alteram habitats e rotas de animais.
- Migrações que levam espécies a atravessar regiões habitadas.
- Escassez alimentar que aproxima predadores de zonas de construção.
- Nevoeiro denso que torna a orientação difícil.
- Cheias que alteram margens de rios.
- Atividade de criaturas raras em determinadas condições.
- Alterações sazonais na disponibilidade de recursos.

Os eventos devem ser antecipáveis até certo ponto. O jogador deve poder observar sinais e preparar-se, mesmo que não consiga prever tudo.

---

# 14. Modos de jogo e multiplayer

O multiplayer é desejado como possibilidade, mas não deve fazer parte do primeiro protótipo.

## 14.1. Modos propostos

| Modo | Experiência |
|---|---|
| Solo | Mundo próprio, persistente e adaptado ao jogador |
| Co-op | Grupo pequeno, inicialmente pensado para 2 a 4 jogadores |
| Hardcore | Sobrevivência mais exigente e recursos limitados |

**Decisão (v0.5):** o multiplayer é solo e co-op de 2 a 4 jogadores. Servidores partilhados e PvP ficam fora do projeto por agora.

## 14.2. Filosofia multiplayer

- O jogo deve ser divertido a solo antes de ser divertido em multiplayer.
- O co-op deve facilitar a construção, exploração e especialização de tarefas.
- Os modos PvE/co-op devem proteger a base contra destruição offline por outros jogadores.
- As regras do co-op devem definir claramente perda de itens e persistência do mundo partilhado.

## 14.3. Multiplayer mais tarde

A arquitetura pode ser preparada para uma futura autoridade de servidor, mas implementar networking cedo demais pode atrasar a validação do núcleo do jogo. Primeiro é necessário provar que a simulação local funciona e é divertida.

---

# 15. Direção artística e apresentação

## 15.1. Decisão conceptual tomada

O universo será de **fantasia naturalista**.

## 15.2. Decisão visual ainda em aberto

Foram identificadas três opções:

### A. Voxel / block-based

Um mundo construído com blocos, com iluminação e ambientes mais sofisticados.

**Potenciais vantagens:**

- Liberdade de construção muito elevada.
- Leitura visual clara.
- Identidade associada à criatividade de Minecraft.
- Destruição e modificação do terreno potencialmente mais fáceis de conceptualizar.

### B. 3D tradicional

Modelos tridimensionais detalhados, terreno contínuo, árvores realistas e construção modular.

**Potenciais vantagens:**

- Maior detalhe visual para animais e ambientes.
- Boa integração com uma estética naturalista.
- Maior liberdade para criar silhuetas e animações orgânicas.

### C. Híbrido

Mundo 3D detalhado com construção modular ou baseada em blocos.

**Potenciais vantagens:**

- Combinar a liberdade de construção de Minecraft com a atmosfera de um survival moderno.
- Permitir ambientes e criaturas detalhados sem abandonar a construção criativa.

**Estado (v0.5):** decisão adiada. O primeiro protótipo usa formas simples; o estilo visual escolhe-se quando a simulação estiver a funcionar. A opção híbrida continua a ser uma hipótese, não uma decisão.

## 15.3. Direção atmosférica

- Florestas com profundidade, nevoeiro e luz natural.
- Rios, lagos, clareiras e montanhas com identidade própria.
- Bioluminescência muito pontual em espécies ou plantas especiais.
- Ruínas parcialmente reclamadas pela natureza.
- Noites com visibilidade limitada, mas sem tornar o jogo permanentemente escuro.
- Sons ambientais que ajudam o jogador a reconhecer espécies e perigos.

## 15.4. Perspetiva

**Decisão (v0.5):** o jogador pode alternar entre primeira e terceira pessoa. Com formas simples no protótipo, o custo é baixo; o trabalho extra (animações do corpo, câmara) surge quando houver modelos finais.

## 15.5. Ideias de arte *(inspiração, não decisões)*

A direção visual continua adiada (secção 15.3): o primeiro protótipo usa formas simples e o estilo escolhe-se quando a simulação estiver viva. Esta secção regista o *sentimento* pretendido, a partir de sete imagens de referência reunidas em novembro de 2026, e não como o desenhar.

**O que as referências têm em comum:**

| Traço | O que se vê nas referências | Ligação ao jogo |
|---|---|---|
| Pintura, não realismo | Pinceladas visíveis, formas simplificadas, cor decidida; estudos que parecem aguarela | Estilo estilizado, que também reduz o trabalho de arte (secção 20.15) |
| Escala e figuras pequenas | Viajantes minúsculos diante de ruínas, canyons e picos enormes | A terceira pessoa alternável (15.4) permite este enquadramento; sensação de espanto e calma, não de ameaça constante |
| Ruínas recuperadas pela natureza | Estruturas antigas cobertas de vegetação, poços e arcos com água e luz | História ambiental contada por ruínas e diário (secção 12 e 11.2) |
| Marcos visíveis ao longe | Torre suspensa, pico de pedra, arco monumental | O gerador do mundo pode colocar marcos de propósito (picos, ruínas, poços) para orientar a exploração |
| A luz como protagonista | Raios de luz, luz filtrada pelas copas, luz rasante nas encostas | Iluminação e névoa como principais ferramentas de ambiente |
| Paleta contida com um acento | Base sóbria com uma cor forte (vermelho sobre neve, verde vivo sobre azul) | Cada bioma com uma paleta base e um acento reconhecível |
| Fantasia moderada | Luas e estruturas flutuantes, uma ave montada; o resto é natureza | Coerente com a decisão de fantasia moderada (secção 5.1) |

**Cor por hora e estação:** uma das referências mostra a mesma floresta em três paletas diferentes (dourada, verde-azulada e outonal). Serve de ponto de partida para o ciclo dia/noite e para as estações, como *color scripts* por bioma e hora.

**Consequências para a simulação (a ter em conta desde já):**

- **Os animais têm de se ler à distância.** Nenhuma referência mostra animais pequenos. Coelhos e raposas precisam de silhuetas, cores e movimento que os distingam do fundo, ou o ecossistema fica invisível (liga-se à legibilidade, secção 7.10).
- **Os marcos entram na geração do mundo** como peças que o gerador coloca de propósito, e não só como terreno.
- **Prioridades baratas e eficazes para um projeto individual:** cor estilizada, névoa/perspetiva atmosférica, luz forte e silhuetas marcantes, em vez de modelos muito detalhados.

**Por confirmar:** quais destes traços passam a *princípios de arte* e quais ficam só como referência (por exemplo, escala e solidão, ruínas com natureza, ou cor e luz). Até haver essa escolha, nada nesta secção obriga o projeto.

---

# 16. Direção técnica: Rust

## 16.1. Preferência do projeto

Foi expressa uma preferência clara por desenvolver o projeto em **Rust**, sobretudo pela segurança, controlo e potencial de performance da linguagem.

O facto de o utilizador ainda não saber programar em Rust não é um bloqueio. O projeto pode também servir como percurso de aprendizagem gradual.

**Importante:** Rust não torna automaticamente um jogo rápido. O desempenho depende da arquitetura, do motor, da renderização, da física, da gestão de memória, da frequência de atualização e da quantidade de trabalho feita em cada frame. A linguagem oferece ferramentas fortes para construir sistemas eficientes e concorrentes, mas o resultado precisa de ser medido.

## 16.2. Opção a investigar: Bevy

**Bevy** é a primeira opção a investigar se quisermos construir o jogo principalmente em Rust.

Razões para a considerar:

- É um motor/framework de desenvolvimento de jogos baseado em Rust.
- Utiliza uma arquitetura ECS (Entity Component System).
- Pode encaixar bem em sistemas com muitas entidades, componentes e lógica de simulação.
- Permite manter grande parte da lógica do jogo na mesma linguagem.

**Decisão (v0.5):** o motor é o Bevy. O teste técnico inicial deve, ainda assim, validar a maturidade das funcionalidades de que o projeto precisa (animação, áudio, física, exportação e co-op). Como o `eotw_sim` é independente do motor (secção 17.8), a troca para outra opção continua possível se for necessária.

## 16.3. Alternativas

- **Godot com integração em Rust:** possibilidade de utilizar um motor com editor e fluxo de trabalho visual mais tradicional, enquanto Rust trata sistemas específicos.
- **Outra engine/framework:** poderá ser considerada se oferecer vantagens claras em ferramentas, renderização, produção de conteúdo ou suporte.

A decisão deve ser tomada depois de um protótipo técnico pequeno, não apenas com base em preferência pela linguagem.

## 16.4. Sistemas adequados para Rust

Rust poderá ser utilizado para implementar:

- Simulação de ecossistemas.
- IA e tomada de decisões das criaturas.
- Geração procedural do mundo.
- Gestão de chunks e streaming.
- Sistemas de física ou integração com física.
- Inventário e crafting.
- Gestão de recursos.
- Clima e ciclos temporais.
- Salvamento e carregamento.
- Networking e lógica autoritativa de servidor, numa fase posterior.

---

# 17. Arquitetura técnica proposta

Esta secção é uma direção inicial para investigação. Não representa uma arquitetura já implementada ou testada.

## 17.1. Separação conceptual dos sistemas

```text
                         GAME WORLD
                             |
          +------------------+------------------+
          |                  |                  |
          v                  v                  v
      ECOSYSTEM              AI               WEATHER
          |                  |                  |
          v                  v                  v
       POPULATIONS          ANIMALS            CLIMATE
          |                  |                  |
          +------------------+------------------+
                             |
                             v
                        WORLD STATE
                             |
             +---------------+---------------+
             |               |               |
             v               v               v
          PLAYER          BUILDING        EXPLORATION
             |               |               |
             +---------------+---------------+
                             |
                             v
                      SAVE / LOAD
```

## 17.2. ECS (Entity Component System)

Se for escolhido Bevy, o modelo ECS poderá representar entidades e os seus dados através de componentes e sistemas.

Exemplo conceptual de uma entidade animal:

```rust
struct Animal;

struct Species {
    // Identificador da espécie.
}

struct Health {
    current: f32,
    maximum: f32,
}

struct Age {
    value: f32,
}

struct Needs {
    hunger: f32,
    thirst: f32,
    energy: f32,
    safety: f32,
}
```

Este código é apenas ilustrativo. Os tipos, unidades, intervalos e componentes reais terão de ser desenhados durante a implementação.

## 17.3. IA baseada em necessidades e estados

Uma criatura pode tomar decisões a partir de necessidades e perceções.

Exemplo conceptual:

```text
Se detetar uma ameaça próxima:
    avaliar o perigo
    procurar uma rota de fuga
    fugir ou procurar abrigo

Caso contrário, se tiver fome:
    procurar uma fonte de alimento adequada

Caso contrário, se tiver sede:
    procurar água

Caso contrário, se precisar de descansar:
    procurar um local seguro

Caso contrário:
    explorar, socializar ou permanecer no território
```

O comportamento deve variar por espécie. Um coelho, um lobo e um urso não devem utilizar exatamente as mesmas prioridades ou respostas.

### IA de utilidade (v0.4)

A lista de prioridades acima é um bom ponto de partida, mas tende a produzir comportamentos previsíveis. Como as necessidades já são valores contínuos (`Needs`), propõe-se uma **IA de utilidade**:

- Cada ação possível (fugir, comer, beber, descansar, explorar, seguir cheiro, defender território) recebe uma pontuação calculada a partir das necessidades, da perceção e das características da espécie.
- Escolhe-se a ação com maior pontuação.
- **Histerese:** depois de escolher, a criatura mantém a ação durante um tempo mínimo, para evitar mudar de ideias a cada passo e "tremer" entre duas ações.
- **Fuga com prioridade absoluta:** se a ameaça passar um limiar, ignora-se a pontuação das restantes ações.
- As curvas de pontuação de cada espécie ficam em ficheiros de dados, o que permite criar variedade comportamental sem escrever código novo por espécie.


## 17.4. Escalonamento e concorrência

Rust permite desenhar sistemas concorrentes com garantias de segurança importantes. No entanto, nem todos os sistemas devem correr em paralelo automaticamente.

Proposta:

- Separar atualizações independentes, como certos cálculos populacionais por região.
- Evitar partilha desnecessária de estado mutável.
- Agrupar trabalho por regiões ou chunks.
- Utilizar filas de eventos para comunicar sistemas quando adequado.
- Medir custos antes de otimizar prematuramente.

## 17.5. Mundo dividido em chunks

Um mundo aberto poderá ser dividido em regiões ou chunks para facilitar:

- Carregamento e descarga de áreas.
- Geração procedural.
- Persistência de alterações.
- Simulação ecológica por região.
- Atualização de apenas zonas relevantes.

A dimensão dos chunks e a representação do terreno dependem da decisão visual e do motor.

**Decisão (v0.5):** o mundo é procedural e extenso (quase infinito). Consequências de design: os chunks são gerados a partir da seed; só se guardam as diferenças (alterações do jogador e estado ecológico das regiões visitadas); a simulação regional agregada só existe para regiões carregadas ou visitadas. O Prototype 01 continua a ser um mapa pequeno.

## 17.6. Estado persistente

O sistema de salvamento deverá guardar, entre outras coisas:

- Seed e configuração do mundo.
- Posição e estado do jogador.
- Construções e alterações feitas ao terreno.
- Inventário e equipamento.
- Estado das regiões importantes.
- Populações ou dados agregados das espécies.
- Descobertas registadas no diário.
- Estado de missões ou acontecimentos, se vierem a existir.

Não é necessário guardar cada decisão e cada movimento de cada animal. A estratégia de persistência deverá guardar o estado necessário para reconstruir o mundo de forma coerente.

## 17.7. Multiplayer e autoridade do servidor

Se o multiplayer avançar, uma arquitetura de servidor autoritativo será uma opção a estudar para reduzir inconsistências e exploração de falhas.

O servidor poderá controlar:

- Estado oficial do mundo.
- Inventário e crafting.
- Alterações às construções.
- Simulação ecológica partilhada.
- Regras de PvP e proteção de bases.

Este sistema deve ser desenhado depois de validado o jogo local. Não deve ser implementado no primeiro protótipo.

## 17.8. Organização do projeto em *crates*

Separar a simulação do motor gráfico, usando um *workspace* do Cargo:

| Crate | Tipo | Função |
|---|---|---|
| `eotw_sim` | biblioteca | Ecossistema, vegetação, perceção, IA e dados de espécies. Sem dependência do Bevy. |
| `eotw_cli` | binário | Corre a simulação em modo headless, imprime eventos em texto e gera relatórios CSV. É o resultado da etapa 3. |
| `eotw_game` | binário | Bevy: renderização, input, interface, áudio. Liga-se ao `eotw_sim`. |

**Regras da simulação:**

- Passo de tempo fixo, independente da taxa de fotogramas.
- Determinística para uma dada seed: sem aleatoriedade global nem leitura do relógio.
- Parâmetros em ficheiros de dados.
- Testes automáticos, incluindo os de equilíbrio (secção 18.6).

**Vantagens:** o código da etapa 3 não é descartável; a simulação pode ser testada sem janela; a mudança de versões do Bevy afeta apenas o `eotw_game`; e uma futura arquitetura de servidor autoritativo reutiliza o `eotw_sim`.

---

# 18. Protótipo inicial

## 18.1. Objetivo

Antes de construir o jogo completo, queremos responder a uma pergunta:

> **É divertido simplesmente existir neste mundo?**

O primeiro protótipo deve provar que um pequeno ecossistema consegue criar situações interessantes sem exigir uma grande quantidade de conteúdo.

## 18.2. Conteúdo do Prototype 01

| Elemento | Conteúdo inicial |
|---|---|
| Mundo | Uma pequena floresta com rio e clareira |
| Terreno | Um mapa pequeno, ainda sem mundo infinito |
| Animais | Veados, coelhos, javalis, raposas e lobos |
| Vegetação | Árvores, arbustos, ervas e frutos |
| Ecossistema | Necessidades básicas, alimentação, fuga e predação simples |
| Perceção | Visão simples, campos de cheiro e eventos de ruído (ver 7.6) |
| Vegetação | Biomassa por célula, consumida pelos herbívoros e regenerada (ver 7.7) |
| Jogador | Movimento e interação básica |
| Construção | Poucas peças de madeira e abrigo simples |
| Crafting | Ferramentas muito básicas |
| Sobrevivência | Saúde e alimentação simples |
| Tempo | Ciclo de dia e noite |
| Persistência | Guardar e carregar o estado básico do mundo |

## 18.3. Fora do âmbito do primeiro protótipo

- Multiplayer.
- Mundo infinito.
- Cinco ou mais biomas.
- Centenas de criaturas.
- Reprodução genética avançada.
- Sistema de domesticação completo.
- Economia ou aldeias de NPCs.
- Combate complexo.
- História ou campanha completa.
- Sistema climático avançado.

## 18.4. Teste principal

Criar uma pequena cabana, deixar comida no exterior e observar como os animais reagem.

O teste será bem-sucedido se:

- As criaturas procurarem recursos sem instruções diretas do jogador.
- Algumas espécies reagirem a ameaças de forma diferente.
- As relações entre espécies produzirem acontecimentos compreensíveis.
- O jogador conseguir perceber que uma ação sua alterou o comportamento local.
- A simulação funcionar com performance aceitável no hardware-alvo, ainda por definir.

## 18.5. Critérios de validação

O protótipo não deve ser avaliado apenas pelo número de funcionalidades. Devemos testar:

1. O jogador consegue compreender o que os animais estão a fazer?
2. Os comportamentos parecem suficientemente naturais?
3. As interações produzem situações que não foram escritas manualmente?
4. As consequências são visíveis e fazem sentido?
5. O mundo continua interessante sem combate constante?
6. A performance é estável com várias criaturas ativas?
7. Guardar e carregar preserva a coerência do mundo?

## 18.6. Critérios mensuráveis

Os valores são ponto de partida e devem ser ajustados com os primeiros resultados.

**Teste de equilíbrio (headless):**

- Correr, por exemplo, 100 dias simulados com 20 seeds diferentes.
- Nenhuma espécie extingue-se sem a migração de segurança, ou, se necessitar dela, o número de ativações fica abaixo de um limite definido.
- Nenhuma população ultrapassa a capacidade de carga da região por uma margem definida.
- A vegetação não colapsa de forma permanente sem intervenção do jogador.
- O relatório de populações é guardado para comparar versões.

**Teste da cabana (medido por registos):**

- Com comida deixada no exterior, o número de visitas por espécie à zona aumenta de forma mensurável face a uma execução sem comida.
- Pelo menos uma sequência de eventos encadeados (por exemplo, herbívoro, depois raposa, depois lobo) acontece sem ter sido escrita.
- Em execuções com seeds diferentes, os resultados variam.

*Como está medido no Prototype 01 (marco 9):* as visitas medem-se em horas passadas na zona, porque uma raposa que chega fica. A cadeia é "um coelho come os restos e, em menos de 2 dias, uma raposa caça junto à cabana". Como sem comida a cadeia é impossível por definição, o controlo são as caçadas junto à cabana, que têm de ser mais com comida do que sem. O valor nutritivo dos restos decide o resultado: muito nutritivos alimentam a raposa e protegem as presas, pouco nutritivos atraem-na e levam-na a caçar.

**Transição agregado/individual:**

- A população total antes e depois da conversão é igual dentro da tolerância definida.
- Nenhum animal é gerado dentro da distância mínima ao jogador.

**Performance:** o passo de simulação cabe no orçamento de tempo definido para o hardware-alvo (a definir).

---

# 19. Plano de desenvolvimento e aprendizagem

O utilizador ainda não sabe programar em Rust. O projeto deve, por isso, ser desenvolvido como uma aprendizagem progressiva, sem exigir que toda a linguagem seja dominada antes de começar.

## Etapa 1 — Preparar o ambiente

- Instalar Rust e Cargo.
- Escolher editor de código.
- Criar um projeto mínimo.
- Aprender a compilar, executar e organizar módulos.
- Fazer um pequeno programa que represente espécies e animais.

## Etapa 2 — Fundamentos de Rust através do jogo

Conceitos a aprender à medida que forem necessários:

- `let` e mutabilidade.
- Funções (`fn`).
- Estruturas (`struct`).
- Enumerações (`enum`).
- Implementações (`impl`).
- Coleções como `Vec` e `HashMap`.
- `Option` e `Result`.
- Ownership e borrowing.
- Módulos e organização de projeto.
- Testes e tratamento de erros.

## Etapa 3 — Simulação de animais sem gráficos

Criar primeiro uma simulação simples em texto ou numa representação mínima:

- Criar uma população de animais.
- Atualizar fome e energia.
- Procurar alimento.
- Simular fuga de ameaças.
- Registar eventos.
- Implementar perceção por campos de cheiro e eventos de ruído.
- Gerar relatórios de população (CSV) para testar o equilíbrio.

Isto permite aprender lógica e estruturas de dados sem misturar logo dificuldades de renderização.

## Etapa 4 — Protótipo visual

- Investigar Bevy e alternativas.
- Mostrar terreno e vegetação básicos.
- Adicionar o jogador e movimento.
- Representar animais no mundo.
- Ligar a simulação de necessidades à apresentação visual.

## Etapa 5 — Ecossistema mínimo

- Adicionar presas e predadores.
- Implementar recursos alimentares.
- Fazer populações reagirem à disponibilidade de recursos.
- Adicionar simulação em diferentes níveis de detalhe.
- Medir performance.

## Etapa 6 — Sobrevivência e construção

- Inventário.
- Recolha de recursos.
- Ferramentas.
- Crafting básico.
- Construção de abrigo.
- Dia e noite.

## Etapa 7 — Persistência e expansão

- Guardar e carregar mundos.
- Testar alterações ambientais persistentes.
- Acrescentar novas espécies.
- Acrescentar novas regiões e biomas.
- Melhorar a geração do mundo.

## Etapa 8 — Multiplayer, se o núcleo estiver validado

- Escolher modelo de servidor.
- Definir autoridade do estado do mundo.
- Sincronizar jogadores, construções e eventos.
- Testar latência e consistência.
- Definir regras de co-op, PvE e PvP.

---

# 20. Riscos, limites e decisões de design

## 20.1. Risco: tentar criar um mundo demasiado grande demasiado cedo

**Resposta:** começar por uma única floresta pequena e um ecossistema limitado.

## 20.2. Risco: simulação ecológica demasiado cara

**Resposta:** usar níveis de detalhe, agregação populacional, atualizações por intervalos e profiling. Não simular tudo com a mesma frequência.

## 20.3. Risco: ecossistema difícil de compreender

**Resposta:** utilizar sinais visuais, sons, rastos, diário de descoberta e consequências legíveis.

## 20.4. Risco: sobrevivência repetitiva

**Resposta:** reduzir tarefas frequentes e fazer com que alimentação, temperatura e preparação criem escolhas interessantes.

## 20.5. Risco: a fantasia enfraquecer a identidade naturalista

**Resposta:** limitar criaturas fantásticas, dar-lhes uma função ecológica e apresentá-las através de descoberta gradual.

## 20.6. Risco: Rust e a engine escolhida aumentarem a dificuldade inicial

**Resposta:** desenvolver um protótipo técnico pequeno e comparar ferramentas antes de assumir uma tecnologia para todo o projeto.

## 20.7. Risco: multiplayer aumentar demasiado o âmbito

**Resposta:** validar primeiro a experiência a solo e só depois desenhar a arquitetura multiplayer.

## 20.8. Risco: confundir realismo com diversão

**Resposta:** a simulação serve a experiência do jogador. Não é necessário reproduzir a natureza com rigor científico se isso tornar o jogo lento, opaco ou frustrante.

## 20.9. Risco: o âmbito total das decisões ser grande demais para um projeto individual

As decisões tomadas (mundo quase infinito, domesticação completa, hereditariedade, sobrevivência completa, fantasia moderada, co-op) somam-se a um projeto de equipa.

**Resposta:** tratar o GDD como destino e não como plano. Cada etapa da secção 19 funciona como porta: só se avança quando o sistema anterior for compreensível, divertido e medido. A ordem de corte, se o tempo apertar, está na secção 20.17.

## 20.10. Risco: documentar mais depressa do que construir

O GDD é extenso e ainda não há código. O ecossistema só pode ser avaliado ao correr.

**Resposta:** congelar o design na versão 0.6. O documento só volta a mudar quando um teste o exigir. A próxima entrega é código, não texto.

## 20.11. Risco: aprender Rust e Bevy ao mesmo tempo

O Bevy muda de API com frequência entre versões, e a curva de aprendizagem do Rust já é exigente por si.

**Resposta:** aprender Rust no `eotw_cli`, sem gráficos, e só depois ligar o Bevy. Fixar uma versão do Bevy e atualizar apenas entre marcos. Na simulação, usar estruturas simples (`Vec`, índices) antes de recorrer a ECS mais avançado.

## 20.12. Risco: a simulação ficar instável ou aborrecida

Sistemas predador-presa oscilam ou colapsam com facilidade e, mesmo equilibrados, podem ser invisíveis para o jogador.

**Resposta:** relatórios CSV de população desde a primeira versão, parâmetros em ficheiros de dados, registo de eventos legível (por exemplo "a raposa seguiu o cheiro até à cabana"), e mais tarde um modo observador com visualização dos campos de cheiro.

## 20.13. Risco: determinismo, mundo infinito e co-op

A simulação tem de dar o mesmo resultado a partir da mesma seed, e o co-op depende disso. Números decimais e aleatoriedade global podem causar divergências.

**Resposta:** passo de tempo fixo, sem aleatoriedade global, e todas as ações do jogador entram na simulação como comandos. Isto deixa o co-op possível mais tarde sem o implementar já. No início, usar um mapa finito com a API já pensada em chunks.

## 20.14. Risco: performance

Perceção por campos, muitos animais e chunks gerados em tempo real podem pesar.

**Resposta:** definir um orçamento de milissegundos por passo de simulação e medir desde o início. Os campos de cheiro só existem perto do jogador, como previsto na secção 7.6.

## 20.15. Risco: conteúdo e arte

Um mundo procedural com muitas espécies exige muitos modelos e animações, e a decisão visual está adiada.

**Resposta:** manter a renderização separada da simulação, para que trocar de estilo não obrigue a refazer a lógica. Quando chegar a decisão, preferir um estilo simples e estilizado que reduza o trabalho de arte.

## 20.16. Risco: perder a motivação num projeto longo e individual

**Resposta:** marcos pequenos e jogáveis a cada poucas semanas, e um diário de desenvolvimento onde se registam as aprendizagens e as decisões.

## 20.17. Ordem de corte se o tempo apertar

Se o ritmo de desenvolvimento ficar abaixo do previsto, cortar por esta ordem:

1. Hereditariedade completa dos animais.
2. Co-op.
3. Domesticação completa (reduzir para poucas espécies).

O núcleo nunca se corta: ecossistema legível, perceção, vegetação e equilíbrio das populações.

---

# 21. Decisões tomadas e questões em aberto

## 21.1. Decisões já tomadas

| Tema | Decisão | Estado |
|---|---|---|
| Nome | Echoes of the Wild | Nome de trabalho aceite |
| Conceito | Survival, exploração, construção e ecossistema vivo | Definido a nível conceptual |
| Direção do universo | Fantasia naturalista | **Escolhida** |
| Princípio central | Mundo que continua a funcionar sem o jogador | **Escolhido** |
| Referências principais | Minecraft, Rust e Muck | Confirmadas pelo gosto do utilizador |
| Referência ecológica | Rain World e Eco | Selecionadas para estudo |
| Referência de progressão | Valheim | Selecionada para estudo |
| Linguagem | Rust | **Escolhida** |
| Engine | Bevy (Rust puro) | **Escolhida** |
| Aprendizagem | Aprender Rust ao construir o projeto | Abordagem pretendida |
| Perspetiva | Primeira e terceira pessoa, alternáveis | **Escolhida** |
| Direção visual | Adiada; formas simples no protótipo | Adiada |
| Mundo | Procedural extenso (quase infinito), por chunks | **Escolhida** |
| Construção | Sistema adiado até haver motor visual | Adiada |
| Combate | Peso médio: armas variadas e técnica | **Escolhida** |
| Sobrevivência | Necessidades completas: alimentação, água, temperatura e energia | **Escolhida** |
| Fantasia | Moderada: várias criaturas e ruínas | **Escolhida** |
| História | Ambiental (ruínas, vestígios e diário); sem campanha principal | **Escolhida** |
| Domesticação | Completa: confiança gradual e várias espécies | **Escolhida** |
| Reprodução | Hereditariedade completa numa fase tardia | **Escolhida** |
| Multiplayer | Solo e co-op de 2 a 4 jogadores; sem servidores partilhados nem PvP | **Escolhida** |
| Plataforma | PC primeiro, outras plataformas depois | **Escolhida** |
| Equipa | Projeto individual | **Escolhida** |
| Monetização | Gratuito, sem monetização | **Escolhida** |

## 21.2. Questões ainda em aberto

| Tema | Questão |
|---|---|
| Visual | Voxel, 3D tradicional ou híbrido; decidir quando a simulação estiver a funcionar. Ideias de arte registadas em 15.5 |
| Construção | Blocos livres, peças modulares ou mistura; decidir depois de haver motor visual |
| Hardcore | Se existe como modo e que regras de perda e recursos tem |
| Necessidades | Ritmo de cada necessidade, de forma a não interromper o jogador (a calibrar) |
| Domesticação | Que espécies são domesticáveis e quais não são |
| Fantasia | Que criaturas fantásticas e ruínas entram em cada bioma |
| Co-op | Anfitrião ou servidor dedicado; como sincronizar a simulação |
| Hardware-alvo | Requisitos mínimos de PC para o orçamento de simulação |

## 21.3. Próximas decisões recomendadas

A ordem recomendada é:

1. Teste técnico com Bevy e Rust: criar o workspace (`eotw_sim`, `eotw_cli`, `eotw_game`).
2. Primeira simulação de animais sem gráficos (etapa 3), com perceção e vegetação.
3. Relatórios de equilíbrio das populações (secção 18.6).
4. Protótipo visual com formas simples e câmara alternável entre primeira e terceira pessoa.
5. Floresta pequena do Prototype 01.
6. Só depois: crafting, construção e progressão (o sistema de construção decide-se nessa altura).

---

# 22. Glossário

| Termo | Significado no projeto |
|---|---|
| Bioma | Região com clima, vegetação, espécies e condições ambientais próprias |
| Ecossistema | Conjunto de organismos e fatores ambientais que interagem |
| ECS | Entity Component System, arquitetura que organiza entidades, componentes e sistemas |
| Chunk | Secção do mundo usada para geração, carregamento e simulação |
| Simulação em camadas | Utilizar níveis diferentes de detalhe conforme a distância e relevância |
| População agregada | Representação estatística de um grupo de animais sem simular cada indivíduo |
| Procedural | Conteúdo criado por regras e algoritmos, em vez de ser todo colocado manualmente |
| Seed | Valor inicial usado para reproduzir a geração de um mundo |
| Mundo persistente | Mundo que conserva alterações e estado entre sessões |
| Servidor autoritativo | Servidor que mantém e valida o estado oficial de uma sessão multiplayer |
| Campo de cheiro | Grelha de valores por célula onde alimento, animais e jogador depositam odores que se espalham e decaem |
| IA de utilidade | Decisão por pontuação: cada ação recebe um valor a partir das necessidades e da perceção, e vence a mais alta |
| Capacidade de carga | Número máximo de animais que uma região consegue sustentar com a vegetação e a água disponíveis |
| Headless | Execução da simulação sem gráficos, usada para testes e relatórios |

---

# Nota final do projeto

**Echoes of the Wild** ainda está em fase de conceito. Este documento consolida as ideias discutidas e propõe uma estrutura de desenvolvimento, mas não significa que todas as funcionalidades estejam decididas ou implementadas.

A prioridade é preservar a identidade do projeto: um survival em que a liberdade de construção e exploração se encontra com um ecossistema convincente e uma fantasia discreta.

Não precisamos de construir um continente inteiro para provar a ideia. Precisamos de construir uma floresta pequena que pareça viva.

> **O jogador não deve ser o centro do mundo. Deve ser mais uma parte dele.**
