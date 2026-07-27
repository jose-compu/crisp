# Crisp web documentation

This folder is the static site for the **`docs`** branch (GitHub Pages).

## Preview

```bash
cd docs && python3 -m http.server 8080
```

## Publish

GitHub → Settings → Pages → Source: branch **`docs`**, folder **`/docs`**.

Spec and contributor markdown remain on **`main`** (`docs/spec/`, `KNOWN_LIMITATIONS.md`, etc.). Keep site pages linking to `main` for those sources.
