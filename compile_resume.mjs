import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const inputFile = process.argv[2] ?? "resume.txt";
const outputFile = process.argv[3] ?? "resume.tex";
const shouldBuildPdf = process.argv.includes("--pdf");

function splitRightColumn(line) {
  const marker = " :: ";
  const index = line.lastIndexOf(marker);
  if (index === -1) return [line.trim(), ""];
  return [line.slice(0, index).trim(), line.slice(index + marker.length).trim()];
}

function escapeLatex(value) {
  return value
    .replace(/\\/g, "\\textbackslash{}")
    .replace(/&/g, "\\&")
    .replace(/%/g, "\\%")
    .replace(/\$/g, "\\$")
    .replace(/#/g, "\\#")
    .replace(/_/g, "\\_")
    .replace(/{/g, "\\{")
    .replace(/}/g, "\\}")
    .replace(/~/g, "\\textasciitilde{}")
    .replace(/\^/g, "\\textasciicircum{}")
    .replace(/\|/g, "$|$");
}

function contactItem(token) {
  const text = token.trim();
  const escaped = escapeLatex(text.replace(/^https?:\/\//, ""));
  if (text.includes("@") && !text.startsWith("http")) {
    return `\\href{mailto:${text}}{${escaped}}`;
  }
  if (/^(https?:\/\/|linkedin\.com|github\.com)/i.test(text)) {
    const href = text.startsWith("http") ? text : `https://${text}`;
    return `\\href{${href}}{${escaped}}`;
  }
  return escaped;
}

function parseResume(source) {
  const resume = { meta: {}, sections: [] };
  let currentSection = null;
  let currentEntry = null;

  for (const rawLine of source.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith("//")) continue;

    if (line.startsWith("@")) {
      const index = line.indexOf(":");
      if (index !== -1) {
        const key = line.slice(1, index).trim().toLowerCase();
        resume.meta[key] = line.slice(index + 1).trim();
      }
      continue;
    }

    if (line.startsWith("# ")) {
      currentSection = { title: line.slice(2).trim(), content: [], entries: [], skills: [] };
      resume.sections.push(currentSection);
      currentEntry = null;
      continue;
    }

    if (!currentSection) continue;

    if (line.startsWith("> ")) {
      const [title, right] = splitRightColumn(line.slice(2));
      currentEntry = { title, right, detail: "", detailRight: "", bullets: [] };
      currentSection.entries.push(currentEntry);
      continue;
    }

    if (line.startsWith("- ")) {
      if (currentEntry) currentEntry.bullets.push(line.slice(2).trim());
      continue;
    }

    const skillIndex = line.indexOf(":");
    if (!currentEntry && skillIndex > 0 && currentSection.title.toLowerCase().includes("skill")) {
      currentSection.skills.push({
        label: line.slice(0, skillIndex).trim(),
        value: line.slice(skillIndex + 1).trim(),
      });
      continue;
    }

    if (currentEntry && !currentEntry.detail) {
      const [detail, detailRight] = splitRightColumn(line);
      currentEntry.detail = detail;
      currentEntry.detailRight = detailRight;
    } else {
      currentSection.content.push(line);
    }
  }

  return resume;
}

function renderHeading(meta) {
  const name = escapeLatex(meta.name ?? "");
  const contact = (meta.contact ?? "")
    .split("|")
    .map(contactItem)
    .filter(Boolean)
    .join(" $|$ ");

  return String.raw`\begin{center}
    {\Huge \textbf{${name}}} \\ \vspace{4pt}
    \small ${contact}
\end{center}`;
}

function renderBullets(bullets) {
  if (bullets.length === 0) return "";
  return [
    "    \\resumeItemListStart",
    ...bullets.map((bullet) => `      \\resumeItem{${escapeLatex(bullet)}}`),
    "    \\resumeItemListEnd",
  ].join("\n");
}

function renderEntry(section, entry) {
  const title = escapeLatex(entry.title);
  const right = escapeLatex(entry.right);
  const detail = escapeLatex(entry.detail);
  const detailRight = escapeLatex(entry.detailRight);
  const bullets = renderBullets(entry.bullets);

  if (entry.detail || !["projects", "awards & activities"].includes(section.title.toLowerCase())) {
    return [
      "  \\resumeSubheading",
      `    {${title}}{${right}}`,
      `    {${detail}}{${detailRight}}`,
      bullets,
    ].filter(Boolean).join("\n");
  }

  return [
    "  \\resumeProjectHeading",
    `    {${title}}{${right}}`,
    bullets,
  ].filter(Boolean).join("\n");
}

function renderSection(section) {
  const lines = [`\\section{${escapeLatex(section.title)}}`];
  if (section.content.length) {
    lines.push("\\small");
    lines.push(section.content.map(escapeLatex).join("\n\n"));
    lines.push("\\normalsize");
  }
  if (section.skills.length) {
    lines.push("\\begin{itemize}[leftmargin=0.15in, label={}]");
    lines.push("  \\small{\\item{");
    lines.push(section.skills.map((skill) => `    \\textbf{${escapeLatex(skill.label)}}: ${escapeLatex(skill.value)}`).join(" \\\\\n"));
    lines.push("  }}");
    lines.push("\\end{itemize}");
  }
  if (section.entries.length) {
    lines.push("\\resumeSubHeadingListStart");
    lines.push(section.entries.map((entry) => renderEntry(section, entry)).join("\n\n"));
    lines.push("\\resumeSubHeadingListEnd");
  }
  return lines.join("\n");
}

function renderLatex(resume) {
  return String.raw`\documentclass[letterpaper,11pt]{article}

\usepackage[empty]{fullpage}
\usepackage{titlesec}
\usepackage{enumitem}
\usepackage[hidelinks]{hyperref}
\usepackage{fancyhdr}
\usepackage[english]{babel}
\usepackage{tabularx}
\input{glyphtounicode}

\pagestyle{fancy}
\fancyhf{}
\fancyfoot{}
\renewcommand{\headrulewidth}{0pt}
\renewcommand{\footrulewidth}{0pt}

\addtolength{\oddsidemargin}{-0.5in}
\addtolength{\evensidemargin}{-0.5in}
\addtolength{\textwidth}{1in}
\addtolength{\topmargin}{-.6in}
\addtolength{\textheight}{1.2in}
\setlength{\footskip}{5pt}

\urlstyle{same}
\raggedbottom
\raggedright
\setlength{\tabcolsep}{0in}
\pdfgentounicode=1

\titleformat{\section}{
  \vspace{-6pt}\scshape\raggedright\large
}{}{0em}{}[\titlerule \vspace{-6pt}]

\newcommand{\resumeItem}[1]{
  \item\small{#1}
}

\newcommand{\resumeSubheading}[4]{
  \vspace{-2pt}\item
  \begin{tabular*}{0.97\textwidth}[t]{l@{\extracolsep{\fill}}r}
    \textbf{#1} & #2 \\
    \textit{\small #3} & \textit{\small #4} \\
  \end{tabular*}\vspace{-6pt}
}

\newcommand{\resumeProjectHeading}[2]{
  \vspace{-2pt}\item
  \begin{tabularx}{0.97\textwidth}[t]{@{}Xr@{}}
    \small\textbf{#1} & #2 \\
  \end{tabularx}\vspace{-6pt}
}

\newcommand{\resumeSubHeadingListStart}{\begin{itemize}[leftmargin=0.15in, label={}]}
\newcommand{\resumeSubHeadingListEnd}{\end{itemize}}
\newcommand{\resumeItemListStart}{\begin{itemize}[leftmargin=0.25in]}
\newcommand{\resumeItemListEnd}{\end{itemize}\vspace{-5pt}}

\begin{document}

${renderHeading(resume.meta)}

${resume.sections.map(renderSection).join("\n\n")}

\end{document}
`;
}

function hasCommand(command) {
  const checker = process.platform === "win32" ? "where" : "sh";
  const args = process.platform === "win32" ? [command] : ["-c", `command -v ${command}`];
  return spawnSync(checker, args, { stdio: "ignore" }).status === 0;
}

function run(command, args) {
  return spawnSync(command, args, { stdio: "inherit", shell: process.platform === "win32" });
}

function buildPdf(texFile) {
  fs.mkdirSync("build", { recursive: true });

  if (hasCommand("latexmk")) {
    return run("latexmk", ["-pdf", "-interaction=nonstopmode", "-halt-on-error", "-output-directory=build", texFile]);
  }

  if (hasCommand("pdflatex")) {
    const first = run("pdflatex", ["-interaction=nonstopmode", "-halt-on-error", "-output-directory=build", texFile]);
    if (first.status !== 0) return first;
    return run("pdflatex", ["-interaction=nonstopmode", "-halt-on-error", "-output-directory=build", texFile]);
  }

  if (hasCommand("tectonic")) {
    return run("tectonic", ["--outdir", "build", texFile]);
  }

  console.error("No LaTeX compiler found. Install one of: latexmk, pdflatex, or tectonic.");
  return { status: 1 };
}

const sourcePath = path.resolve(inputFile);
const outputPath = path.resolve(outputFile);
const resume = parseResume(fs.readFileSync(sourcePath, "utf8"));
fs.writeFileSync(outputPath, renderLatex(resume), "utf8");
console.log(`Wrote ${outputFile} from ${inputFile}`);

if (shouldBuildPdf) {
  const build = buildPdf(outputFile);
  process.exit(build.status ?? 1);
}
