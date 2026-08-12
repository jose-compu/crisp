# Crisp for VS Code / Cursor

Syntax highlighting for Crisp (`.crp`), plus optional stdio LSP via `crisp-lsp` (#56 / #57).

## Install from VSIX

```bash
# from repo root
./scripts/package-vsix.sh
# then: Extensions → Install from VSIX… → editors/vscode-crisp/*.vsix
```

Install the language server binary:

```bash
cargo install --path crates/crisp-lsp --locked
# ensure `crisp-lsp` is on PATH
```

Settings (optional):

- `crisp.lsp.enabled` (default `true`)
- `crisp.lsp.path` (default `crisp-lsp`)

Without `vscode-languageclient` in `node_modules`, the extension loads highlighting only. For LSP client support when packaging for Marketplace:

```bash
cd editors/vscode-crisp && npm install vscode-languageclient@9
```

## Install (development)

From this folder (`editors/vscode-crisp`):

1. Open the folder in VS Code or Cursor.
2. Press **F5** (Run Extension).
3. Open any `examples/**/*.crp` file.

Or symlink:

```bash
# Cursor
ln -s "$(pwd)" "$HOME/.cursor/extensions/jose-compu.crisp-lang-0.1.0"
# VS Code: ~/.vscode/extensions/…
```

## Scope

- TextMate grammar for `.crp`
- Optional `crisp-lsp` (hover, inlay hints, analyze-on-open diagnostics)
