# Crisp for VS Code / Cursor

Syntax highlighting for Crisp (`.crp`) — line comments `--`, block comments `{- -}`, keywords, strings with `{interpolation}`, and operators (`:=`, `->`, `**`, …).

## Install (development)

From this folder (`editors/vscode-crisp`):

1. Open the folder in VS Code or Cursor.
2. Press **F5** (Run Extension) — opens a new window with the extension loaded.
3. Open any `examples/**/*.crp` file.

Or symlink / copy into your extensions directory:

```bash
# Cursor
ln -s "$(pwd)" "$HOME/.cursor/extensions/jose-compu.crisp-lang-0.1.0"

# VS Code
ln -s "$(pwd)" "$HOME/.vscode/extensions/jose-compu.crisp-lang-0.1.0"
```

Then reload the window (`Developer: Reload Window`).

## Package (optional)

```bash
npx @vscode/vsce package
# installs crisp-lang-0.1.0.vsix via Extensions: Install from VSIX…
```

## Scope

Highlighting only (TextMate grammar). LSP features stay in `crisp-lsp` / QUICKSTART §11 until a stdio host ships.
