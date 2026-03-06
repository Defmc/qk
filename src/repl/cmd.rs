use crate::repl::Repl;
use crate::repl::{Error, Result};

pub struct Command<'a> {
    pub cmd: &'a str,
    pub desc: &'a str,
    pub alias: &'a str,
    pub func: &'a dyn Fn(&mut Repl, &str) -> Result<()>,
}

impl<'a> Command<'a> {
    pub fn matches(&self, name: &str) -> bool {
        self.cmd == name || self.alias == name
    }
}

pub const COMMANDS: &[Command] = &[
    Command {
        cmd: "quit",
        alias: "q",
        desc: "quits the terminal",
        func: &|_r: &mut Repl, _input: &str| -> Result<()> {
            std::process::exit(0);
        },
    },
    Command {
        cmd: "set",
        alias: "s",
        desc: "manual settings",
        func: &|r: &mut Repl, input: &str| -> Result<()> {
            fn set<T: for<'a> TryFrom<&'a str>>(
                prop: &mut T,
                setting: &str,
                value: &str,
            ) -> Result<()> {
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
                    r.runner.bench = crate::repl::runner::BENCH_SETTING
                        .parse_inspired(value)
                        .map_err(|v| Error::InvalidValue(setting.to_string(), v.to_string()))?
                }
                "show" => {
                    r.runner.show = crate::repl::runner::SHOW_SETTING
                        .parse_inspired(value)
                        .map_err(|v| Error::InvalidValue(setting.to_string(), v.to_string()))?
                }
                _ => return Err(Error::UnknownSetting(setting.to_string())),
            }
            Ok(())
        },
    },
    Command {
        cmd: "context",
        alias: "ctx",
        desc: "show all the current context",
        func: &|r: &mut Repl, input: &str| -> Result<()> {
            for (k, v) in r.runner.irc.scope.definitions.iter() {
                if input.is_empty() || **k == *input {
                    print!("{k} = ");
                    r.runner
                        .irc
                        .scope
                        .pretty_print(&r.runner.irc.scope.res_pool[v.0]);
                }
            }
            Ok(())
        },
    },
    Command {
        cmd: "resources",
        alias: "r",
        desc: "show how many resources are being used",
        func: &|r: &mut Repl, _input: &str| -> Result<()> {
            fn human_size(n: usize) -> (f64, &'static str) {
                const SUFFIXES: &[&str] = &["B", "KB", "MB", "GB"];
                let mut n = n as f64;
                let mut suffix_i = 0;

                while n >= 1024.0 {
                    n /= 1024.0;
                    suffix_i += 1;
                }

                (n, SUFFIXES[suffix_i])
            }

            let scope = &r.runner.irc.scope;

            println!("context");
            println!("\tno. of definitions: {}", scope.definitions.len());
            println!("\tresources entries: {}", scope.res_pool.len());

            let (size, suffix) =
                human_size(scope.res_pool.len() * std::mem::size_of::<qk::arts::Term>());
            println!("\tresources size: {size:.2}{suffix}",);
            Ok(())
        },
    },
    Command {
        cmd: "help",
        alias: "h",
        desc: "show information about a command",
        func: &|_r: &mut Repl, name: &str| -> Result<()> {
            if name.is_empty() {
                for cmd in COMMANDS {
                    println!(
                        "{:<25} {}",
                        format!("{} (alias {})", cmd.cmd, cmd.alias),
                        cmd.desc
                    )
                }
                return Ok(());
            }
            for cmd in COMMANDS {
                if cmd.matches(name) {
                    println!("{} (alias {})", cmd.cmd, cmd.alias);
                    println!("\t{}", cmd.desc);
                    break;
                }
            }
            Err(Error::InvalidValue("help".into(), name.into()))
        },
    },
    Command {
        cmd: "clear",
        alias: "cls",
        desc: "Clear all the `runner` configuration",
        func: &|r: &mut Repl, _s: &str| -> Result<()> {
            r.runner = crate::repl::runner::Runner::new();
            Ok(())
        },
    },
    Command {
        cmd: "load",
        alias: "l",
        desc: "Load a script into the context. Each line is executed as a REPL entry",
        func: &|r: &mut Repl, path: &str| -> Result<()> {
            let mut reader = std::fs::File::open(path).map_err(|e| Error::Io { e })?;
            let content = std::io::read_to_string(&mut reader).map_err(|e| Error::Io { e })?;
            content.lines().for_each(|l| r.exec(l));
            Ok(())
        },
    },
];
