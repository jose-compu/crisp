# Crisp documentation site

Static site for **https://crisp-lang.org/** (GitHub Pages branch `docs`, folder `/docs`, `CNAME` → `crisp-lang.org`). Content tracks compiler **v1.7.2** public release.

## Pages

| Page | Role |
|------|------|
| `index.html` | Home (Elixir-style hero + sections) |
| `install.html` | Install / hello |
| `learning.html` | Learning hub |
| `docs.html` | Documentation hub |
| `use-cases.html` | Popular Rust domains made easier in Crisp |
| `tutorial.html` | Beginner language tutorial |
| `language.html`, `cli.html`, `limitations.html` | Doc detail pages (+ language snippets) |
| `index.html` “Language features” | Snippet gallery (enums, traits, rust import, …) |

Colors and logo match the Crisp brand (slate + blue accent, `crisp-logo.jpg`). Favicon assets (`favicon.ico`, `favicon.png`, `apple-touch-icon.png`) are derived from `assets/crisp-logo-square.jpg` on `main`. Structure inspired by [elixir-lang.org](https://elixir-lang.org/).

## SEO / discoverability

| Asset | Role |
|-------|------|
| Per-page `<meta>` + Open Graph / Twitter | Titles, descriptions, canonical URLs |
| JSON-LD on `index.html` | `WebSite`, `Organization`, `SoftwareApplication`, `SoftwareSourceCode` |
| `robots.txt` | Allow search + major AI crawlers; points at sitemap |
| `sitemap.xml` | Index of primary pages for search engines |
| `llms.txt` | LLM-oriented site summary ([llmstxt.org](https://llmstxt.org/)) |

Keep page titles/descriptions in sync when shipping major releases.

## Syntax highlighting

Web snippets use **Prism.js**:

| File | Role |
|------|------|
| `js/prism-crisp.js` | Custom Crisp grammar (`language-crisp`): keywords, builtins, strings with `{interpolation}`, `--` / `{- -}` comments, operators (`:=`, `++`, `**`, ambient `!`, …) |
| `js/prism-bash.min.js` | Shell / install snippets |
| `css/prism-crisp.css` | Theme — bright tokens on slate `.code-window`; dark ink only for light-surface `.pipe` blocks (do not apply light-band colors inside code windows) |
| `js/site-highlight.js` | Marks unmarked fences and runs Prism on each page |

Include the CSS + scripts from every HTML page that shows code (same pattern as `index.html`).

Editor TextMate grammar lives on `main` in [`editors/vscode-crisp`](https://github.com/jose-compu/crisp/tree/main/editors/vscode-crisp), not in this branch.
