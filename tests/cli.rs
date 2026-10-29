use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "resume compiler test {} {}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn write(&self, path: &str, text: &str) {
        fs::write(self.0.join(path), text).unwrap();
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_resume-compiler"))
            .current_dir(&self.0)
            .args(args)
            .output()
            .unwrap()
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
const VALID: &str = "@name: Test Person\n# Portfolio\n> Game | Java\n- Built a game";

#[test]
fn invalid_source_does_not_overwrite_existing_tex() {
    let work = Workspace::new();
    work.write("resume.txt", "@name: Test\n# Projects\n>");
    work.write("resume.tex", "previous valid output");
    let result = work.run(&[]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("resume.txt: line 3:"));
    assert_eq!(
        fs::read_to_string(work.0.join("resume.tex")).unwrap(),
        "previous valid output"
    );
}

#[test]
fn check_validates_without_creating_outputs() {
    let work = Workspace::new();
    work.write("resume.txt", VALID);
    assert!(work.run(&["--check"]).status.success());
    assert!(!work.0.join("resume.tex").exists());
    assert!(!work.0.join("build").exists());
    work.write("resume.txt", "# Projects\n> Game");
    assert!(!work.run(&["--check"]).status.success());
    work.write("resume.txt", "@name: Test\n# Empty");
    assert!(!work.run(&["--check"]).status.success());
}

#[test]
fn generation_accepts_paths_with_spaces_and_creates_parent_directories() {
    let work = Workspace::new();
    work.write("custom resume.txt", VALID);
    let result = work.run(&["custom resume.txt", "output folder/custom resume.tex"]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(work.0.join("output folder/custom resume.tex").exists());
}

#[test]
fn bad_arguments_and_source_overwrites_are_rejected() {
    let work = Workspace::new();
    work.write("resume.txt", VALID);
    for args in [
        vec!["--unknown"],
        vec!["--out-dir"],
        vec!["--check", "--pdf"],
        vec!["resume.txt", "resume.txt"],
        vec!["a", "b", "c"],
    ] {
        assert!(!work.run(&args).status.success(), "{args:?}");
    }
    assert_eq!(
        fs::read_to_string(work.0.join("resume.txt")).unwrap(),
        VALID
    );
    work.write("source.tex", VALID);
    assert!(!work.run(&["source.tex", "./source.tex"]).status.success());
    assert_eq!(
        fs::read_to_string(work.0.join("source.tex")).unwrap(),
        VALID
    );
    assert!(work.run(&["--help"]).status.success());
}

#[cfg(unix)]
fn executable(path: &Path, contents: &str) {
    use std::os::unix::fs::PermissionsExt;
    fs::write(path, contents).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
#[cfg(unix)]
fn failed_compilation_keeps_previous_pdf_and_cleans_temporary_files() {
    let work = Workspace::new();
    work.write("resume.txt", VALID);
    fs::create_dir(work.0.join("bin")).unwrap();
    fs::create_dir(work.0.join("pdf output")).unwrap();
    work.write("pdf output/resume.pdf", "previous PDF");
    // First available engine is deliberately broken, even if host engines exist.
    executable(&work.0.join("bin/latexmk"), "#!/bin/sh\nexit 1\n");
    let result = Command::new(env!("CARGO_BIN_EXE_resume-compiler"))
        .current_dir(&work.0)
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin", work.0.join("bin").display()),
        )
        .args(["--pdf", "--out-dir", "pdf output"])
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("previous PDF was kept"));
    assert_eq!(
        fs::read_to_string(work.0.join("pdf output/resume.pdf")).unwrap(),
        "previous PDF"
    );
    assert_eq!(fs::read_dir(work.0.join("pdf output")).unwrap().count(), 1);
}

#[test]
#[cfg(unix)]
fn successful_compilation_publishes_only_the_new_pdf() {
    let work = Workspace::new();
    work.write("resume.txt", VALID);
    fs::create_dir(work.0.join("bin")).unwrap();
    executable(
        &work.0.join("bin/latexmk"),
        r##"#!/bin/sh
for arg do
  case "$arg" in
    -output-directory=*) out=${arg#-output-directory=} ;;
  esac
done
printf 'new PDF' > "$out/resume.pdf"
printf 'auxiliary file' > "$out/resume.aux"
"##,
    );
    let result = Command::new(env!("CARGO_BIN_EXE_resume-compiler"))
        .current_dir(&work.0)
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin", work.0.join("bin").display()),
        )
        .args(["--pdf", "--out-dir", "pdf output"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        fs::read_to_string(work.0.join("pdf output/resume.pdf")).unwrap(),
        "new PDF"
    );
    assert_eq!(fs::read_dir(work.0.join("pdf output")).unwrap().count(), 1);
}
