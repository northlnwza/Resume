#!/usr/bin/env bash
set -euo pipefail

TEX_FILE="${1:-resume.tex}"
OUT_DIR="${2:-build}"

if [[ ! -f "$TEX_FILE" ]]; then
  echo "Missing LaTeX source: $TEX_FILE" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"

if command -v latexmk >/dev/null 2>&1; then
  latexmk -pdf -interaction=nonstopmode -halt-on-error -output-directory="$OUT_DIR" "$TEX_FILE"
elif command -v pdflatex >/dev/null 2>&1; then
  pdflatex -interaction=nonstopmode -halt-on-error -output-directory="$OUT_DIR" "$TEX_FILE"
  pdflatex -interaction=nonstopmode -halt-on-error -output-directory="$OUT_DIR" "$TEX_FILE"
elif command -v tectonic >/dev/null 2>&1; then
  tectonic --outdir "$OUT_DIR" "$TEX_FILE"
else
  echo "No LaTeX compiler found. Install one of: latexmk, pdflatex, or tectonic." >&2
  exit 1
fi

PDF_NAME="$(basename "${TEX_FILE%.tex}.pdf")"
echo "Built $OUT_DIR/$PDF_NAME"
