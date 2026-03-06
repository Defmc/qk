use std::time::Instant;

use miette::{Diagnostic, NamedSource, Severity};
use qk::arts::CompArtifact;
use qk::cpu::{self, Cpu, Reductor};
use qk::{compiler::CodeUnit, ir::IrCompiler, lexer::TkTy, parser::Parser};
use smallvec::SmallVec;

use crate::repl::Result;
use crate::repl::settings::Setting;

pub const BENCH_SETTING: Setting = Setting {
    all: &[
        "lexer", "parser", "command", "ir", "compiler", "steps", "normal",
    ],
    on: SmallVec::new_const(),
};

pub const SHOW_SETTING: Setting = Setting {
    all: &[
        "lexer",
        "parser",
        "command",
        "ir",
        "compiler",
        "steps",
        "steps_raw",
        "normal",
    ],
    on: SmallVec::new_const(),
};

#[derive(Debug)]
pub struct Runner {
    pub irc: IrCompiler,
    pub art: CompArtifact,
    pub bench: Setting,
    pub show: Setting,
    pub warnings: usize,
    pub errors: usize,
}

impl Runner {
    pub fn new() -> Self {
        Self {
            irc: IrCompiler::default(),
            art: CompArtifact::default(),
            bench: BENCH_SETTING,
            show: SHOW_SETTING,
            warnings: 0,
            errors: 0,
        }
    }

    pub fn lexer(&mut self, src: &str) -> Result<Vec<qk::lexer::Token>> {
        let lexer: Vec<_> = self.bench("lexer", |_| TkTy::processed(src).collect());
        let lexer: Vec<_> = lexer
            .into_iter()
            .filter_map(|tk| match tk {
                Ok(tk) => Some(tk),
                Err(e) => {
                    self.report(e, src.to_string());
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
            self.report(report, src.to_string());
        }
        Ok(lexer)
    }

    pub fn parse(&mut self, lexer: Vec<qk::lexer::Token>, _src: &str) -> Result<qk::ast::Node> {
        // TODO: This is not ideal. But since we don't have namespaces yet, it's the only way that
        // declarations can exist
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
        Ok(t)
    }

    pub fn ir(&mut self, ast: qk::ast::Node, src: &str) -> Result<Option<qk::ir::IrObj>> {
        self.bench("ir", |s| -> Result<_> {
            if matches!(ast.item, qk::ast::Ast::Program(..)) {
                s.irc.compile_program(ast, src)?;
                Ok(None)
            } else {
                Ok(Some(s.irc.compile(ast, src)?))
            }
        })
    }

    pub fn compile(&mut self, expr: qk::ir::IrObj, src: &str) -> Result<()> {
        self.bench("compiler", |s| -> Result<()> {
            let mut art = CompArtifact::default();
            std::mem::swap(&mut art, &mut s.art);
            let mut cu = CodeUnit::with_artifacts(&mut s.irc.scope, src, art)?;
            cu.compile(&expr)?;
            s.art = cu.art;
            Ok(())
        })?;
        if self.show.is_on("compiler") {
            let aliases = self.irc.scope.get_aliases();
            println!("{}", self.art.to_string(&aliases));
        }
        Ok(())
    }

    pub fn cpu(&mut self) -> Result<()> {
        let mut root = self.art.root.unwrap();
        self.bench("normal", |s| {
            let mut art = CompArtifact::default();
            std::mem::swap(&mut art, &mut s.art);
            let mut cpu = Cpu::new(art);
            let aliases = s.irc.scope.get_aliases();
            let empty_aliases = std::collections::HashMap::new();
            loop {
                if s.show.is_on("steps") {
                    cpu.art.pretty_print(root, &aliases);
                }
                if s.show.is_on("steps_raw") {
                    cpu.art.pretty_print(root, &empty_aliases);
                    println!("{}", cpu.art.to_string(&aliases));
                }
                let op = s.bench("steps", |_| qk::cpu::Normal::step(&mut cpu, root));
                match op {
                    cpu::Op::Normal => {
                        if s.show.is_on("normal") && !s.show.is_on("steps") {
                            cpu.art.pretty_print(root, &aliases);
                        }
                        break;
                    }
                    cpu::Op::Effect(..) => todo!(),
                    cpu::Op::Reduced(idx) => {
                        root = idx;
                    }
                }
            }
        });
        Ok(())
    }

    pub fn expression(&mut self, input: &str) -> Result<()> {
        let lexer = self.lexer(input)?;
        let ast = self.parse(lexer, input)?;
        let ir = self.ir(ast, input)?;
        if let Some(expr) = ir {
            if self.show.is_on("ir") {
                println!("{expr:#?}")
            }
            self.compile(expr, input)?;

            self.cpu()?;
        }
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
