use miette::{Diagnostic, NamedSource, Severity};
use qk::lexer::TkTy;
use qk::parser::Parser;
use rustyline::{DefaultEditor, error::ReadlineError};
use smallvec::{SmallVec, ToSmallVec};
use std::{fmt::Write, time::Instant};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Diagnostic, Debug)]
pub enum Error {
    #[error("can't read the next line")]
    #[diagnostic(
        code(repl::input::readline_error),
        help("are you really running this on interactive mode?")
    )]
    Input(ReadlineError),

    #[error("unknown command")]
    #[diagnostic(code(repl::command::unknown), help("sometimes we just miss it!"))]
    UnknownCommand(String),

    #[error("missing argument")]
    #[diagnostic(
        code(repl::command::missing_arg),
        help("are you sure this is the command?")
    )]
    MissingArg(String),

    #[error("invalid setting value: {0} doesn't accept {1:?}")]
    #[diagnostic(
        code(repl::command::set::invalid_valid),
        help("are you sure this is the setting?")
    )]
    InvalidValue(String, String),

    #[error("unknown {0:?} setting")]
    #[diagnostic(code(repl::command::set::unknown_setting), help("mistyping maybe?"))]
    UnknownSetting(String),

    #[error(transparent)]
    #[diagnostic(transparent)]
    ParserError(#[from] qk::parser::Error),

    #[error(transparent)]
    #[diagnostic(transparent)]
    CompilerError(#[from] qk::pool_lambda::Error),
}

pub struct Command<'a> {
    pub cmd: &'a str,
    pub desc: &'a str,
    pub alias: &'a str,
    pub func: fn(&mut Repl, &str) -> Result<()>,
}

fn quit_cmd(_r: &mut Repl, _input: &str) -> Result<()> {
    std::process::exit(0);
}

fn set_cmd(r: &mut Repl, input: &str) -> Result<()> {
    fn set<T: for<'a> TryFrom<&'a str>>(prop: &mut T, setting: &str, value: &str) -> Result<()> {
        *prop = value
            .try_into()
            .map_err(|_| Error::InvalidValue(setting.to_string(), value.to_string()))?;
        Ok(())
    }

    let (setting, value) = input
        .split_once(' ')
        .ok_or_else(|| Error::MissingArg("setting".to_string()))?;
    match setting {
        "prompt" => set(&mut r.prompt, "prompt", value)?,
        "bench" => {
            r.bench = BENCH_SETTING
                .parse_inspired(value)
                .map_err(|v| Error::InvalidValue(setting.to_string(), v.to_string()))?
        }
        "show" => {
            r.show = SHOW_SETTING
                .parse_inspired(value)
                .map_err(|v| Error::InvalidValue(setting.to_string(), v.to_string()))?
        }
        _ => return Err(Error::UnknownSetting(setting.to_string())),
    }
    Ok(())
}

pub const COMMANDS: &[Command] = &[
    Command {
        cmd: "quit",
        alias: "q",
        desc: "quits the terminal",
        func: quit_cmd,
    },
    Command {
        cmd: "set",
        alias: "s",
        desc: "manual settings",
        func: set_cmd,
    },
];

#[derive(Default, Clone)]
pub struct Setting {
    all: &'static [&'static str],
    on: SmallVec<[&'static str; 8]>,
}

impl Setting {
    fn parse_inspired<'a>(&self, value: &'a str) -> std::result::Result<Self, &'a str> {
        if value == "all" {
            return Ok(Setting {
                all: self.all,
                on: self.all.to_smallvec(),
            });
        }
        let mut s = Self::default();
        for v in value.split(',') {
            let trimmed = v.trim();
            if let Some(set) = self.all.iter().find(|&&a_v| a_v == trimmed) {
                s.on.push(set);
            } else {
                return Err(trimmed);
            }
        }
        Ok(s)
    }
}

pub const BENCH_SETTING: Setting = Setting {
    all: &["lexer", "parser", "command", "compiler"],
    on: SmallVec::new_const(),
};

pub const SHOW_SETTING: Setting = Setting {
    all: &["lexer", "parser", "command", "compiler"],
    on: SmallVec::new_const(),
};

pub struct Repl {
    pub prompt: String,
    pub rl: DefaultEditor,
    pub warnings: usize,
    pub errors: usize,
    pub bench: Setting,
    pub show: Setting,
}

impl Repl {
    pub fn reset_diagnostics(&mut self) {
        self.warnings = 0;
        self.errors = 0;
    }

    pub fn bench<T>(&mut self, label: &str, f: impl FnOnce(&mut Self) -> T) -> T {
        if !self.bench.on.contains(&label) {
            return f(self);
        }
        let start = Instant::now();
        let r = f(self);
        let elapsed = start.elapsed();
        println!("[{label}: {elapsed:?}]");
        r
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            let input = self.input();
            self.reset_diagnostics();
            let input = match input {
                Ok(s) => s,
                Err(ReadlineError::Eof | ReadlineError::Interrupted) => {
                    return Ok(());
                }
                Err(e) => return Err(Error::Input(e)),
            };
            if input.is_empty() {
                continue;
            }
            let result = if let Some(input) = input.strip_prefix(':') {
                self.cmd(input)
            } else {
                self.expression(&input)
            };
            if let Err(e) = result {
                self.report(e, input);
            }
        }
    }

    pub fn cmd(&mut self, input: &str) -> Result<()> {
        let (command, args) = input.split_once(' ').unwrap_or((input, ""));
        for c in COMMANDS {
            if command == c.alias || command == c.cmd {
                return self.bench("command", |s| (c.func)(s, args));
            }
        }
        Err(Error::UnknownCommand(command.to_string()))
    }

    pub fn expression(&mut self, input: &str) -> Result<()> {
        let lexer: Vec<_> = self.bench("lexer", |_| TkTy::processed(input).collect());
        let lexer: Vec<_> = lexer
            .into_iter()
            .filter_map(|tk| match tk {
                Ok(tk) => Some(tk),
                Err(e) => {
                    self.report(e, input.to_string());
                    None
                }
            })
            .collect();
        if self.show.on.contains(&"lexer") {
            let report = miette::MietteDiagnostic::new("lexer's output")
                .with_labels(lexer.iter().map(|tk| {
                    miette::LabeledSpan::new_with_span(Some(format!("{:?}", tk.item)), tk.at)
                }))
                .with_severity(Severity::Advice);
            self.report(report, input.to_string());
        }
        let t = self.bench("parser", |_| Parser::new(lexer).parse_app())?;
        if self.show.on.contains(&"parser") {
            qk::ast::display_node(&t);
        }
        let compiled = self.bench("compiler", |_| qk::pool_lambda::Pool::compile(t, input))?;
        if self.show.on.contains(&"compiler") {
            println!("{compiled}");
        }
        Ok(())
    }

    pub fn input(&mut self) -> rustyline::Result<String> {
        let mut prefix = String::default();
        if self.warnings > 0 {
            write!(prefix, "{}  ", self.warnings).unwrap();
        }
        if self.errors > 0 {
            write!(prefix, "{}  ", self.errors).unwrap();
        }
        let input = if prefix.is_empty() {
            self.rl.readline(&self.prompt)?
        } else {
            prefix.push_str(&self.prompt);
            self.rl.readline(&prefix)?
        };

        self.rl.add_history_entry(&input)?;
        Ok(input)
    }

    pub fn report(&mut self, e: impl Diagnostic + Send + Sync + 'static, input: String) {
        match e.severity().unwrap_or_default() {
            Severity::Error => self.errors += 1,
            Severity::Warning => self.warnings += 1,
            _ => (),
        }
        println!(
            "{:?}",
            miette::Report::new(e).with_source_code(NamedSource::new("repl", input))
        );
    }

    fn new() -> Result<Self> {
        let s = Self {
            prompt: "λ> ".to_string(),
            rl: DefaultEditor::new().map_err(Error::Input)?,
            warnings: 0,
            errors: 0,
            bench: BENCH_SETTING.clone(),
            show: SHOW_SETTING.clone(),
        };
        Ok(s)
    }
}

fn main() -> Result<()> {
    let mut r = Repl::new()?;
    r.run()
}
