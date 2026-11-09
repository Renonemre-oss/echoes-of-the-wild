# Getting to primary sources

Lessons from research rounds on this project. Read before briefing researchers.

## Open-access routes that worked

- **PubMed Central / Europe PMC** — full text and often supplementary files (Europe PMC exposed supplementary tables that the publisher page hid).
- **Semantic Scholar API** — `https://api.semanticscholar.org/graph/v1/paper/search?query=<title>&fields=title,year,externalIds,openAccessPdf` finds legal open PDFs.
- **Unpaywall** — `https://api.unpaywall.org/v2/<DOI>?email=<any>` gives the best open-access copy for a DOI.
- **OpenAlex** — abstracts and open-access links by DOI or title.
- **HAL API** (French repository) — abstracts and PDFs of French research.
- **University repositories and theses** — e.g. Durham e-Theses, Reading CentAUR, Oxford ORA, Bayreuth EPub, Bristol Research Portal. PhD theses often reproduce the key paper's data in full and compile many populations.
- **Institutional PDFs** — USGS Publications Warehouse, NPWRC, CSIRO, NZ Journal of Ecology (free), IUCN Specialist Group sites (the full IUCN canid action plan is on canids.org, not at the IUCN library link people usually quote).
- **Author PDFs** on lab pages, and **archive.org / Wayback Machine** for old free copies.

## Things that did not work, and what to do instead

- **JSTOR, ScienceDirect, Wiley and some society sites** answer automated access with 403 errors or anti-bot checks. Don't try to get around those checks, and don't use unauthorised mirror sites (sci-hub-like copies). Instead, list the papers for the user — they can sometimes download them.
- **Some sites fail on TLS certificates** (e.g. polipapers.upv.es). Try the DOI resolver, Semantic Scholar or CORE for the same paper.
- **CORE and some Australian university repositories** may block bots; try OpenAlex or Unpaywall for an alternative copy.

## Traps

- **Name collisions.** Searching by author surname plus year can return an unrelated paper: "Fox 2023" found a graph-comprehension chapter by Amy Rae Fox, and "Newsome 1989" found a neuroscience paper rather than the rabbit study by Newsome, Parer & Catling. Search by full title or DOI, and check the first page of any PDF before using it.
- **Wrong links in secondary sources.** A link labelled as one paper led to a different paper (an "Artois 1991" link opened a Goszczyński et al. 2008 study). Check that the title at the destination matches.
- **Reviews quoting reviews.** A number can be passed along through several reviews with its context lost: an "adult survival by sex" figure was later quoted as "survival by rank". Trace each number back to the table it came from.
- **Snippets mislead.** Search-result excerpts can invert a finding or merge two studies. Use them only to find the source, never as verification.
- **Secondary compilations** (e.g. wildlifeonline.me.uk) are useful maps of the literature. Follow their reference lists to the primary paper, and mark anything still second-hand as such.

## What to record per claim

Record the value, the unit, the place and years, the sample size, whether it was measured in the field, an enclosure or captivity, the citation with DOI or URL, the page or table, and the verification status (verified / abstract only / cited in a review / not verifiable / disputed).
