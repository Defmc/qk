use std::{collections::HashMap, time::Instant};

use miette::{Diagnostic, NamedSource, Severity};
use qk::{
    compiler::{CodeUnit, CompArtifact},
    ir::IrCompiler,
    lexer::TkTy,
    parser::Parser,
};
use smallvec::SmallVec;

use crate::repl::Result;
use crate::repl::settings::Setting;

pub const BENCH_SETTING: Setting = Setting {
    all: &["lexer", "parser", "command", "ir", "compiler"],
    on: SmallVec::new_const(),
};

pub const SHOW_SETTING: Setting = Setting {
    all: &["lexer", "parser", "command", "ir", "compiler"],
    on: SmallVec::new_const(),
};

#[derive(Debug)]
pub struct Runner {
    pub irc: IrCompiler,
    pub shared_cache: CompArtifact,
    pub bench: Setting,
    pub show: Setting,
    pub warnings: usize,
    pub errors: usize,
}

impl Runner {
    pub fn new() -> Self {
        Self {
            irc: IrCompiler::default(),
            shared_cache: CompArtifact::default(),
            bench: BENCH_SETTING,
            show: SHOW_SETTING,
            warnings: 0,
            errors: 0,
        }
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

        if self.show.is_on("lexer") {
            let report = miette::MietteDiagnostic::new("lexer's output")
                .with_labels(lexer[..lexer.len() - 1].iter().map(|tk| {
                    miette::LabeledSpan::new_with_span(Some(format!("{:?}", tk.item)), tk.at)
                }))
                .with_severity(Severity::Advice);
            self.report(report, input.to_string());
        }

        let is_decl = lexer.iter().any(|t| t.item == TkTy::Assign);

        let t = self.bench("parser", |_| {
            let mut p = Parser::new(lexer);
            if is_decl {
                p.parse_program()
            } else {
                p.parse_app()
            }
        })?;

        if self.show.is_on("parser") {
            qk::ast::display_node(&t);
        }

        if matches!(t.item, qk::ast::Ast::Program(..)) {
            self.declare_code(t, input)
        } else {
            self.run_executable(t, input)
        }
    }

    pub fn run_executable(&mut self, ast: qk::ast::Node, src: &str) -> Result<()> {
        let ir = self.bench("ir", |s| s.irc.compile(ast, src))?;
        if self.show.is_on("ir") {
            println!("{ir:#?}")
        }

        let (art, _entry_point) =
            self.bench("compiler", |s| match CodeUnit::new(&mut s.irc.scope, src) {
                Err(e) => Result::Err(e.into()),
                Ok(mut cu) => match cu.compile(&ir) {
                    Ok(id) => Ok((cu.art, id)),
                    Err(e) => Result::Err(e.into()),
                },
            })?;
        if self.show.is_on("compiler") {
            let hp = HashMap::default();
            println!("{}", art.to_string(&hp));
        }
        Ok(())
    }

    pub fn declare_code(&mut self, ast: qk::ast::Node, src: &str) -> Result<()> {
        self.bench("ir", |s| s.irc.compile_program(ast, src))?;
        Ok(())
    }
    pub fn bench<T>(&mut self, label: &str, f: impl FnOnce(&mut Self) -> T) -> T {
        if !self.bench.is_on(label) {
            return f(self);
        }
        let start = Instant::now();
        let r = f(self);
        let elapsed = start.elapsed();
        println!("[{label}: {elapsed:?}]");
        r
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

    pub fn reset_diagnostics(&mut self) {
        self.warnings = 0;
        self.errors = 0;
    }
}
