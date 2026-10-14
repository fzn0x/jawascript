//! The command-line interface: `jawac`, `jawac run` and `jawac check`.

use crate::builder::Emit;
use crate::{codegen, link};
use jawa_frontend::diag::Diagnostic;
use jawa_frontend::source::SourceFile;
use jawa_frontend::{parser, sema};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const USAGE: &str = "\
jawac: the compiler for Jawa, a programming language with Javanese keywords

Usage:
  jawac [options] <file.jw>       compile to an executable
  jawac run [options] <file.jw>   compile and run
  jawac check <file.jw>           check for errors without compiling

Options:
  -o <path>          where to write the output
  -O0, -O1, -O2, -O3 optimization level (default: -O2)
  --emit <kind>      what to produce:
                       exe        an executable (default)
                       obj        an object file
                       llvm       optimized LLVM IR
                       mlir       the jawa dialect, as the frontend builds it
                       mlir-opt   the jawa dialect after optimization
                       mlir-llvm  the LLVM dialect
                     Text output goes to standard output unless -o is given.
  --color <when>     auto, always or never
  -h, --help         show this help
  -V, --version      show the version

Environment:
  JAWA_CC            the C compiler used to link (default: the one jawac was built with)
  JAWA_RUNTIME_LIB   the path to libjawa_rt.a
";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Build,
    Run,
    Check,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Output {
    Exe,
    Object,
    LlvmIr,
    Mlir,
    MlirOpt,
    MlirLlvm,
}

#[derive(Debug)]
struct Options {
    mode: Mode,
    input: PathBuf,
    output: Option<PathBuf>,
    opt_level: u32,
    emit: Output,
    color: bool,
}

enum Parsed {
    Compile(Options),
    Help,
    Version,
}

pub fn main() -> i32 {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    match parse_args(args) {
        Ok(Parsed::Help) => {
            print!("{USAGE}");
            0
        }
        Ok(Parsed::Version) => {
            println!("jawac {}", env!("CARGO_PKG_VERSION"));
            0
        }
        Ok(Parsed::Compile(options)) => run(&options),
        Err(message) => {
            eprintln!("error: {message}");
            eprintln!("\nRun `jawac --help` for usage.");
            2
        }
    }
}

fn parse_args(args: Vec<OsString>) -> Result<Parsed, String> {
    let mut mode = Mode::Build;
    let mut input = None;
    let mut output = None;
    let mut opt_level = 2;
    let mut emit = Output::Exe;
    let mut color = None;

    let mut args = args.into_iter().peekable();
    if let Some(first) = args.peek().and_then(|a| a.to_str()) {
        match first {
            "run" => mode = Mode::Run,
            "check" => mode = Mode::Check,
            _ => {}
        }
        if mode != Mode::Build {
            args.next();
        }
    }

    while let Some(arg) = args.next() {
        let Some(text) = arg.to_str() else {
            set_input(&mut input, PathBuf::from(arg))?;
            continue;
        };
        let mut value_of = |name: &str| -> Result<String, String> {
            args.next()
                .and_then(|value| value.into_string().ok())
                .ok_or_else(|| format!("`{name}` needs a value"))
        };
        match text {
            "-h" | "--help" => return Ok(Parsed::Help),
            "-V" | "--version" => return Ok(Parsed::Version),
            "-o" => output = Some(PathBuf::from(value_of("-o")?)),
            "-O" => opt_level = 2,
            "-O0" | "-O1" | "-O2" | "-O3" => opt_level = u32::from(text.as_bytes()[2] - b'0'),
            "--emit" => emit = parse_emit(&value_of("--emit")?)?,
            "--color" => color = Some(parse_color(&value_of("--color")?)?),
            _ => {
                if let Some(kind) = text.strip_prefix("--emit=") {
                    emit = parse_emit(kind)?;
                } else if let Some(when) = text.strip_prefix("--color=") {
                    color = Some(parse_color(when)?);
                } else if let Some(path) = text.strip_prefix("-o").filter(|p| !p.is_empty()) {
                    output = Some(PathBuf::from(path));
                } else if text.starts_with('-') && text != "-" {
                    return Err(format!("unknown option `{text}`"));
                } else {
                    set_input(&mut input, PathBuf::from(text))?;
                }
            }
        }
    }

    let input = input.ok_or("no input file; pass a `.jw` file to compile")?;
    if mode == Mode::Run && emit != Output::Exe {
        return Err("`jawac run` builds an executable; drop `--emit`".into());
    }
    let color = color
        .unwrap_or_else(|| std::io::stderr().is_terminal() && env::var_os("NO_COLOR").is_none());
    Ok(Parsed::Compile(Options {
        mode,
        input,
        output,
        opt_level,
        emit,
        color,
    }))
}

fn set_input(input: &mut Option<PathBuf>, path: PathBuf) -> Result<(), String> {
    if input.is_some() {
        return Err("jawac compiles one file at a time".into());
    }
    *input = Some(path);
    Ok(())
}

fn parse_emit(kind: &str) -> Result<Output, String> {
    Ok(match kind {
        "exe" => Output::Exe,
        "obj" => Output::Object,
        "llvm" => Output::LlvmIr,
        "mlir" => Output::Mlir,
        "mlir-opt" => Output::MlirOpt,
        "mlir-llvm" => Output::MlirLlvm,
        _ => return Err(format!("unknown `--emit` kind `{kind}`")),
    })
}

fn parse_color(when: &str) -> Result<bool, String> {
    match when {
        "always" => Ok(true),
        "never" => Ok(false),
        "auto" => Ok(std::io::stderr().is_terminal() && env::var_os("NO_COLOR").is_none()),
        _ => Err(format!(
            "`--color` takes auto, always or never, not `{when}`"
        )),
    }
}

/// Runs the compiler and returns the process exit code.
fn run(options: &Options) -> i32 {
    match compile(options) {
        Ok(code) => code,
        Err(message) => {
            if !message.is_empty() {
                let label = if options.color {
                    "\x1b[1;31merror\x1b[0m"
                } else {
                    "error"
                };
                eprintln!("{label}: {message}");
            }
            1
        }
    }
}

fn compile(options: &Options) -> Result<i32, String> {
    let text = fs::read_to_string(&options.input)
        .map_err(|error| format!("cannot read `{}`: {error}", options.input.display()))?;
    let file = SourceFile::new(options.input.display().to_string(), text);

    let parsed = parser::parse(file.text());
    let mut diagnostics = parsed.diagnostics;
    let program = if diagnostics.iter().any(Diagnostic::is_error) {
        None
    } else {
        let checked = sema::check(&parsed.program);
        diagnostics.extend(checked.diagnostics);
        Some(checked.program)
    };
    // Report in source order, whichever stage found the problem.
    diagnostics.sort_by_key(|diagnostic| diagnostic.span.start);
    for diagnostic in &diagnostics {
        eprint!("{}", diagnostic.render(&file, options.color));
    }
    let errors = diagnostics.iter().filter(|d| d.is_error()).count();
    if errors > 0 {
        return Err(format!(
            "could not compile `{}` because of {errors} error{}",
            file.name(),
            if errors == 1 { "" } else { "s" }
        ));
    }
    let program = program.expect("checked when there are no errors");
    if options.mode == Mode::Check {
        return Ok(0);
    }

    let mut builder = codegen::generate(&program, &file);
    builder.verify().map_err(internal_error)?;

    match options.emit {
        Output::Mlir | Output::MlirOpt | Output::MlirLlvm | Output::LlvmIr => {
            let kind = match options.emit {
                Output::Mlir => Emit::JawaIr,
                Output::MlirOpt => Emit::OptimizedIr,
                Output::MlirLlvm => Emit::LlvmDialect,
                _ => Emit::LlvmIr,
            };
            let path = options
                .output
                .as_ref()
                .map_or_else(|| "-".to_string(), |p| p.display().to_string());
            builder
                .emit(kind, options.opt_level, &path)
                .map_err(internal_error)?;
            Ok(0)
        }
        Output::Object => {
            let path = options
                .output
                .clone()
                .unwrap_or_else(|| PathBuf::from(stem(&options.input) + ".o"));
            builder
                .emit(Emit::Object, options.opt_level, &path.display().to_string())
                .map_err(internal_error)?;
            Ok(0)
        }
        Output::Exe => {
            let object = TempFile::new("o");
            builder
                .emit(
                    Emit::Object,
                    options.opt_level,
                    &object.path.display().to_string(),
                )
                .map_err(internal_error)?;
            if options.mode == Mode::Run {
                let exe = TempFile::new(env::consts::EXE_EXTENSION);
                link::link(&object.path, &exe.path)?;
                drop(object);
                let status = Command::new(&exe.path)
                    .status()
                    .map_err(|error| format!("cannot run the program: {error}"))?;
                return Ok(status.code().unwrap_or(1));
            }
            let output = options.output.clone().unwrap_or_else(|| {
                let mut name = PathBuf::from(stem(&options.input));
                if !env::consts::EXE_EXTENSION.is_empty() {
                    name.set_extension(env::consts::EXE_EXTENSION);
                }
                name
            });
            link::link(&object.path, &output)?;
            Ok(0)
        }
    }
}

/// The input's file name without `.jw`, used for default output names in
/// the current directory.
fn stem(input: &Path) -> String {
    input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "program".to_string())
}

fn internal_error(message: String) -> String {
    format!(
        "internal compiler error: {message}\n  = note: please report this at https://github.com/fzn0x/jawascript/issues"
    )
}

/// A file in the temporary directory, deleted when dropped.
struct TempFile {
    path: PathBuf,
}

impl TempFile {
    fn new(extension: &str) -> TempFile {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut path = env::temp_dir().join(format!("jawac-{}-{n}", std::process::id()));
        if !extension.is_empty() {
            path.set_extension(extension);
        }
        TempFile { path }
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
