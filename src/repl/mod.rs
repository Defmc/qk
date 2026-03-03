use miette::Diagnostic;
use rustyline::{DefaultEditor, error::ReadlineError};
use std::fmt::Write;
use thiserror::Error;

use crate::repl::runner::Runner;

pub mod cmd;
pub mod runner;
pub mod settings;

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
    IrCompilerError(#[from] qk::ir::Error),

    #[error(transparent)]
    #[diagnostic(transparent)]
    CompilerError(#[from] qk::compiler::Error),
}

pub struct Repl {
    pub prompt: String,
    pub rl: DefaultEditor,
    pub runner: Runner,
}

impl Repl {
    pub fn run(&mut self) -> Result<()> {
        loop {
            let input = self.input();
            self.runner.reset_diagnostics();
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
                self.runner.expression(&input)
            };
            if let Err(e) = result {
                self.runner.report(e, input);
            }
        }
    }

    pub fn cmd(&mut self, input: &str) -> Result<()> {
        let (command, args) = input.split_once(' ').unwrap_or((input, ""));
        for c in cmd::COMMANDS {
            if command == c.alias || command == c.cmd {
                return (c.func)(self, args);
            }
        }
        Err(Error::UnknownCommand(command.to_string()))
    }

    pub fn input(&mut self) -> rustyline::Result<String> {
        let mut prefix = String::default();
        if self.runner.warnings > 0 {
            write!(prefix, "{}  ", self.runner.warnings).unwrap();
        }
        if self.runner.errors > 0 {
            write!(prefix, "{}  ", self.runner.errors).unwrap();
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

    pub fn new() -> Result<Self> {
        let s = Self {
            prompt: "λ> ".to_string(),
            rl: DefaultEditor::new().map_err(Error::Input)?,
            runner: Runner::new(),
        };
        Ok(s)
    }
}
