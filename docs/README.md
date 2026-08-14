# Crisp documentation site

Static site for GitHub Pages (branch `docs`, folder `/docs`). Content tracks compiler **v1.5.1** public release.

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
