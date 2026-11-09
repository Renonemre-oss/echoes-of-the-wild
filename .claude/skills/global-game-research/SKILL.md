---
name: global-game-research
description: Research-first workflow for Echoes of the Wild. Use this skill BEFORE adding, designing, changing, recalibrating or expanding anything in the game — a new animal, plant, biome, behaviour, mechanic, tool, building, item, culture, myth, lore, parameter change or GDD section — and whenever the user asks to "pesquisar", "investigar", "aprofundar", "ver como é na natureza/no mundo", or asks how something works in reality. Covers worldwide evidence (not only Iberia or Europe), primary/trustworthy sources, cross-checking, and turning findings into simulation rules and code. Use it even when the user just says "adiciona X" or "faz Y no jogo" without mentioning research.
---

# Global game research

Echoes of the Wild is a deterministic ecosystem game (Rust workspace: `eotw_sim`, `eotw_cli`, `eotw_game` in Bevy) whose design lives in `docs/GDD_v0.7.md` and whose log lives in `DIARIO.md`. Its promise is a world where the player discovers *what is happening*, so every system should behave like something real. That only works if new content is grounded in how the world actually is — everywhere, not just where the developer lives.

The order is: **research → design → implement → validate.** Research shapes what gets built; a report that is then ignored is wasted effort.

## 1. Identify

Before searching, pin down:
- what exactly is being added or changed, and which crate/file/GDD section it touches;
- what role it plays in the game (ecology, survival, crafting, lore…);
- what the game already has that it must fit with (read the relevant code and GDD section, and the existing reports in `reports/` and `research_notes/` — don't redo research that's already there; extend it).

Scale the research to the change. A new species or system deserves a full multi-agent round; tweaking one parameter may need a handful of well-chosen primary papers. Small additions still get research — just not a 10,000-word report.

## 2. Explore the whole world

Default to a global scope. For each subject, ask which regions genuinely matter and cover them, rather than stopping at the familiar ones:

- Europe, including Eastern Europe, Scandinavia and the Balkans
- Africa: North, West, East, Central, Southern
- Asia: East, Southeast, South, Central, West
- North America; Central America and the Caribbean; South America
- Oceania and the Pacific Islands
- Indigenous peoples and less-documented communities

Not every subject needs every region (an endemic plant may live in one valley), but Iberian or Western data should never stand in for "the world". Populations of the same species often behave very differently by climate, density, food and human pressure — that variation is often the most useful thing for a game, because it gives regional presets and emergent differences.

Search in local languages where it matters (Spanish, French, German, Russian, Chinese, Japanese, Arabic, Portuguese, Swahili, etc.) — local studies and local names often don't appear in English results.

Cover the dimensions that are relevant to the subject:
- **Real-world characteristics**: appearance, behaviour, numbers (rates, distances, durations, densities, sizes), materials, function, environment.
- **Historical context**: origins, how it changed, important events.
- **Cultural context**: traditions, beliefs, symbolism, local names and meanings, sensitivities.
- **Global variations**: how it differs between places; analogous things in other cultures or related species; rare and lesser-known cases.
- **Game opportunities**: mechanics, visuals, worldbuilding, interactions and consequences.

For ecology and animal behaviour, numbers are the core deliverable: they become simulation parameters.

## 3. Use trustworthy sources

The user explicitly wants reliable, primary sources. Priority order:
1. Peer-reviewed papers, theses and scientific monographs
2. Museums, universities, research institutions
3. Archives and primary documents
4. Government and institutional reports (e.g. IUCN action plans, national red books)
5. Indigenous and local community sources
6. Specialist books
7. Credible documentaries and specialist websites

Wikipedia, popular sites and search-result snippets are starting points for finding the original, never the evidence itself. If a number is only available second-hand, say so and keep looking for the original. Never use a number from memory without confirming it. Cross-check important claims with at least two independent sources.

Full texts are often paywalled; `references/source-access.md` lists open-access routes that worked on this project and the traps to avoid (including name collisions when searching by author). Read it before a research round.

Tag every claim with how solid it is, so the user and future work can tell them apart:
- verified in the primary source (cite page/table where possible);
- from an abstract only;
- cited in a review / secondary source;
- not verifiable;
- field vs enclosure vs captivity (for behaviour data);
- disputed — state both sides.

Clearly separate **documented fact**, **disputed interpretation** and **creative inspiration**. Never invent facts, traditions, statistics or citations.

## 4. Avoid regional and cultural bias

- Don't treat Portugal/Spain (or Europe, or the anglophone world) as representative.
- Don't use European mythology as the default for anything fantastical.
- Don't treat African, Asian, Indigenous or American cultures as interchangeable; name the specific people, place and period.
- Look past the most famous examples and don't reuse the same few references for every feature.
- Credit cultural origins and keep the context and meaning instead of stripping them to an aesthetic.

## 5. Research workflow

For anything beyond a trivial check, use the `anthropic-skills:deep-research` skill: it spawns researcher subagents in parallel and a report writer. When briefing researchers:
- split by region and by angle (e.g. "species in Australia/NZ", "related species comparison", "general theory", "models other simulations use"), so the global scope is built in;
- ask for numbers with sources, and for the source-quality tags above;
- point them to `references/source-access.md` and to existing notes so they extend rather than repeat.

Then **verify**: for the claims the design will depend on, run a verification pass that checks each number against the original and marks it confirmed / corrected / not verifiable. On this project a first-pass report had ~20% of its numbers wrong or misattributed; verification is not optional for numbers that become parameters.

Output locations and language:
- notes in `research_notes/<título>/`, final report in `reports/<título>.md`;
- write in **European Portuguese** (the user's language), sentence-case titles;
- `reports/` and `research_notes/` are not committed unless the user asks.

If some sources can only be reached with library access, list them for the user — they can sometimes download PDFs themselves. Check any PDF they provide is the right paper before using it.

## 6. Research report (before implementing)

Give the user a report proportional to the change, with:
- **Funcionalidade** – what is being added
- **Âmbito** – regions, cultures, periods investigated
- **Descobertas** – the most relevant findings, with numbers
- **Variações globais** – how it differs between places
- **Fontes** – direct links to the reliable references
- **Decisões de design** – what goes into the game and why, as concrete numbered rules or parameters tied to existing code
- **Incertezas** – what couldn't be verified, conflicts, gaps

Keep the design rules visibly separate from the facts. If the research suggests the original idea should change significantly, say so and why before building it.

## 7. Implement and validate

Only after the research, implement within the existing architecture:
- follow the code conventions already there (Portuguese doc comments and test names, parameters in `*Params` structs with documented provenance, determinism via the RNG streams);
- record in parameter doc comments where each number came from and any compression of game time/space;
- don't rewrite unrelated systems;
- keep game balance in mind: real numbers sometimes need compressing for play, and that compression should be explicit.

Validate:
- `cargo test` and `cargo clippy` pass; measure the simulation's behaviour (population levels, oscillation, extinctions) against the research where it applies;
- don't loosen existing tests to make a change pass — if a test must change on purpose, explain why;
- update the GDD and add an entry to `DIARIO.md` describing what changed, why, what was tried and failed, and what is still missing.

The user tests in the game by hand first and then says "Faz commit"; don't commit before that.
