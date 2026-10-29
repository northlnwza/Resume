#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
if [[ ${1:-} == --help || ${1:-} == -h ]]; then
  echo "Usage: $0 [resume.txt|resume.tex] [output-directory]"
  exit 0
fi
if [[ $# -gt 2 ]]; then
  echo "Expected a .txt or .tex input and an optional output directory." >&2
  exit 1
fi
# Build the editable text source by default; explicit .tex inputs stay supported.
if [[ $# -eq 0 ]]; then
  cd "$SCRIPT_DIR"
  cargo run -- resume.txt resume.tex --pdf
  exit $?
fi

TEX_FILE="${1:-resume.tex}"
OUT_DIR="${2:-build}"

if [[ "$TEX_FILE" == *.txt ]]; then
  TEX_NAME="$(basename "${TEX_FILE%.txt}.tex")"
  exec cargo run --manifest-path "$SCRIPT_DIR/Cargo.toml" -- \
    "$TEX_FILE" "$OUT_DIR/$TEX_NAME" --pdf --out-dir "$OUT_DIR"
fi
if [[ "$TEX_FILE" != *.tex ]]; then
  echo "Input must be a .txt or .tex file." >&2
  exit 1
fi

if [[ ! -f "$TEX_FILE" ]]; then
  echo "Missing LaTeX source: $TEX_FILE" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"
# Compile in a temporary directory so failures keep the last successful PDF.
BUILD_DIR="$(mktemp -d "$OUT_DIR/.resume-build.XXXXXX")"
trap 'rm -rf -- "$BUILD_DIR"' EXIT

if command -v latexmk >/dev/null 2>&1; then
  latexmk -pdf -interaction=nonstopmode -halt-on-error -output-directory="$BUILD_DIR" "$TEX_FILE"
elif command -v pdflatex >/dev/null 2>&1; then
  pdflatex -interaction=nonstopmode -halt-on-error -output-directory="$BUILD_DIR" "$TEX_FILE"
  pdflatex -interaction=nonstopmode -halt-on-error -output-directory="$BUILD_DIR" "$TEX_FILE"
elif command -v tectonic >/dev/null 2>&1; then
  tectonic --outdir "$BUILD_DIR" "$TEX_FILE"
else
  echo "No LaTeX compiler found. Install one of: latexmk, pdflatex, or tectonic." >&2
  exit 1
fi

PDF_NAME="$(basename "${TEX_FILE%.tex}.pdf")"
mv -f -- "$BUILD_DIR/$PDF_NAME" "$OUT_DIR/$PDF_NAME"
echo "Built $OUT_DIR/$PDF_NAME"
