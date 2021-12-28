use std::{
    env,
    io::Write,
    str::FromStr,
    sync::atomic::{AtomicBool, AtomicU8, Ordering},
};

use anyhow::{bail, format_err, Error, Result};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Coloring {
    Auto = 0,
    Always,
    Never,
}

impl Coloring {
    const AUTO: u8 = Coloring::Auto as _;
    const ALWAYS: u8 = Coloring::Always as _;
    const NEVER: u8 = Coloring::Never as _;
}

impl FromStr for Coloring {
    type Err = Error;

    fn from_str(color: &str) -> Result<Self, Self::Err> {
        match color {
            "auto" => Ok(Self::Auto),
            "always" => Ok(Self::Always),
            "never" => Ok(Self::Never),
            other => bail!("must be auto, always, or never, but found `{}`", other),
        }
    }
}

static COLORING: AtomicU8 = AtomicU8::new(Coloring::AUTO);
pub(crate) fn set_coloring(color: Option<&str>) -> Result<()> {
    let mut coloring = match color {
        Some(color) => color.parse().map_err(|e| format_err!("argument for --color {}", e))?,
        // https://doc.rust-lang.org/nightly/cargo/reference/config.html#termcolor
        None => match env::var_os("CARGO_TERM_COLOR") {
            Some(color) => color
                .to_string_lossy()
                .parse()
                .map_err(|e| format_err!("CARGO_TERM_COLOR {}", e))?,
            None => Coloring::Auto,
        },
    };
    if coloring == Coloring::Auto && !atty::is(atty::Stream::Stderr) {
        coloring = Coloring::Never;
    }
    // Relaxed is fine because only the argument parsing step updates this value.
    COLORING.store(coloring as _, Ordering::Relaxed);
    Ok(())
}
fn coloring() -> ColorChoice {
    match COLORING.load(Ordering::Relaxed) {
        Coloring::AUTO => ColorChoice::Auto,
        Coloring::ALWAYS => ColorChoice::Always,
        Coloring::NEVER => ColorChoice::Never,
        _ => unreachable!(),
    }
}

static HAS_ERROR: AtomicBool = AtomicBool::new(false);
pub(crate) fn set_error() {
    HAS_ERROR.store(true, Ordering::SeqCst)
}
pub(crate) fn has_error() -> bool {
    HAS_ERROR.load(Ordering::SeqCst)
}

static VERBOSE: AtomicBool = AtomicBool::new(false);
pub(crate) fn set_verbose() {
    // Relaxed is fine because only the argument parsing step updates this value.
    VERBOSE.store(true, Ordering::Relaxed)
}
pub(crate) fn verbose() -> bool {
    VERBOSE.load(Ordering::Relaxed)
}

pub(crate) fn print_status(status: &str, color: Option<Color>, justified: bool) -> StandardStream {
    let mut stream = StandardStream::stderr(coloring());
    let _ = stream.set_color(ColorSpec::new().set_bold(true).set_fg(color));
    if justified {
        let _ = write!(stream, "{:>12}", status);
    } else {
        let _ = write!(stream, "{}", status);
        let _ = stream.set_color(ColorSpec::new().set_bold(true));
        let _ = write!(stream, ":");
    }
    let _ = stream.reset();
    let _ = write!(stream, " ");
    stream
}

macro_rules! error {
    ($($msg:expr),* $(,)?) => {{
        use std::io::Write;
        crate::term::set_error();
        let mut stream = crate::term::print_status("error", Some(termcolor::Color::Red), false);
        let _ = writeln!(stream, $($msg),*);
    }};
}

// macro_rules! warn {
//     ($($msg:expr),* $(,)?) => {{
//         use std::io::Write;
//         let mut stream = crate::term::print_status("warning", Some(termcolor::Color::Yellow), false);
//         let _ = writeln!(stream, $($msg),*);
//     }};
// }

macro_rules! info {
    ($($msg:expr),* $(,)?) => {{
        use std::io::Write;
        let mut stream = crate::term::print_status("info", None, false);
        let _ = writeln!(stream, $($msg),*);
    }};
}

// macro_rules! status {
//     ($status:expr, $($msg:expr),* $(,)?) => {{
//         use std::io::Write;
//         let mut stream = crate::term::print_status($status, Some(termcolor::Color::Cyan), true);
//         let _ = writeln!(stream, $($msg),*);
//     }};
// }
