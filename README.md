# Resume

This folder contains:

- `resume.tex`: the editable LaTeX resume source
- `build_resume.sh`: a small build script that compiles `resume.tex` into PDF

## Build

```bash
./build_resume.sh
```

The generated PDF will be written to `build/resume.pdf`.

## Install a LaTeX compiler on WSL

The build script supports `latexmk`, `pdflatex`, or `tectonic`.

Option 1: install a minimal TeX Live toolchain

```bash
sudo apt update
sudo apt install -y latexmk texlive-latex-base texlive-latex-recommended texlive-latex-extra
```

Option 2: install `tectonic`

If you already have Rust tooling:

```bash
cargo install tectonic
```

## Rebuild

```bash
./build_resume.sh resume.tex build
```
