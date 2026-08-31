# Tipos de fuente por dominio

## Tecnología / Software

### Fuentes primarias
- **Código fuente**: GitHub/GitLab del repo oficial (L1).
- **Release notes / changelog**: blog oficial, GitHub Releases (L1).
- **RFCs**: `https://www.rfc-editor.org/`, repos oficiales de RFCs (L1).
- **Documentación oficial**: `docs.example.com` (L2).
- **Specs**: W3C, ECMA, IETF (L1).

### Índices de paquetes
- `crates.io` (Rust), `npmjs.com` (JS), `pypi.org` (Python), `pkg.go.dev` (Go), `mvnrepository.com` (Java).

### Fuentes secundarias
- MDN (L2 para web).
- Stack Overflow (L5 — solo como navegador).
- Awesome lists en GitHub (L5).

## AI / ML

### Fuentes primarias
- **Papers originales**: arXiv (`https://arxiv.org/`) con código público (L1).
- **Papers peer-reviewed**: NeurIPS, ICML, ICLR proceedings (L1).
- **Repos oficiales**: GitHub del paper (L1).
- **Model cards**: HuggingFace model cards (L2).

### Índices
- `https://paperswithcode.com/` (navegador de papers con código).
- `https://huggingface.co/` (modelos pre-entrenados).
- `https://crfm.stanford.edu/helm/` (benchmark HELM).

### Benchmarks válidos
- SuperGLUE, GLUE, MMLU, HumanEval, GSM8K — verificar metodología.

## Systems Thinking

Ver `references/canonical-urls-systems-thinking.md` (en este skill).

## Ciencia general

### Fuentes primarias
- **Papers peer-reviewed**: PubMed, Google Scholar, Semantic Scholar (L1).
- **Datasets primarios**: repos del paper, NCBI, PDB, GenBank (L1).
- **Society statements**: APA, AMA, AAAS, Royal Society (L1).

### Índices
- `https://pubmed.ncbi.nlm.nih.gov/` (medicina/biología).
- `https://scholar.google.com/`.
- `https://www.semanticscholar.org/`.
- `https://www.biorxiv.org/` (preprints biología).

## Medicina

- **ClinicalTrials.gov** (L1 para registros de trials).
- **PubMed** (L1 para papers).
- **FDA/EMA** (L1 para aprobaciones).
- **Cochrane Library** (L1 para systematic reviews).
- **UpToDate** (L2 para guidelines).

## Economía / Política

### Datos
- BLS (`https://www.bls.gov/`), INE (`https://www.ine.es/`), Eurostat, Banco Mundial (`https://data.worldbank.org/`), IMF (`https://www.imf.org/`), OECD (`https://data.oecd.org/`).

### Análisis
- NBER papers (L1).
- IMF/World Bank reports (L2).
- Federal Reserve working papers (L1).

## Historia

### Fuentes primarias
- **Archivos nacionales**: National Archives (US), Archivo Histórico Nacional (España), British National Archives, etc.
- **Documentos de época**: libros publicados en el período, autobiografías.
- **Preprints históricos**: JSTOR, Persée.

### Historiografía
- **JSTOR** (L1 para papers peer-reviewed de historia).
- **Cambridge / Oxford History** (L1).
- **Stanford Encyclopedia of Philosophy** (L2).

## Filosofía

- **Stanford Encyclopedia of Philosophy** (`https://plato.stanford.edu/`) (L2).
- **Textos originales** (Plato, Aristotle, Kant, etc.): Perseus, Marxists Internet Archive (L1).
- **SEP entries** (L2).

---

## Cómo buscar por dominio

### Estrategia general

1. Identificar los autores canónicos / instituciones líderes del campo.
2. Buscar sus repos oficiales y publicaciones.
3. Buscar en repos de preprints (arXiv, bioRxiv, SSRN, PhilPapers).
4. Buscar en índices de papers (Google Scholar, Semantic Scholar).
5. Verificar que las fuentes secundarias (Wikipedia, blogs) NO contradicen las primarias.

### Queries recomendadas

| claim_type | Query |
|------------|-------|
| `api-existence` | `"<api_name>" site:github.com OR site:docs.rs OR site:pkg.go.dev` |
| `behavior` | `"<library>" "<feature>" tutorial site:official-docs` |
| `concept-meadows` | `"Donella Meadows" leverage points paradigm` |
| `world3-model` | `World3 standard run re-test peer-reviewed` |
| `experimental-result` | `"<finding>" site:pubmed.ncbi.nlm.nih.gov` |
| `clinical-trial` | `site:clinicaltrials.gov "<condition>"` |
| `event-date` | `"<event>" primary source archive` |
