# Crisp documentation site

Static site for GitHub Pages (branch `docs`, folder `/docs`). Content tracks compiler **v1.4.1** public preview.

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

Colors and logo match the Crisp brand (slate + blue accent, `crisp-logo.jpg`). Structure inspired by [elixir-lang.org](https://elixir-lang.org/).

## Syntax highlighting

Prism.js with a custom **Crisp** grammar (`js/prism-crisp.js`) plus bash for shell snippets. Theme: `css/prism-crisp.css`. Auto-detection in `js/site-highlight.js`.
