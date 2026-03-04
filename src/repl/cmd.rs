use crate::repl::Repl;
use crate::repl::{Error, Result};

pub struct Command<'a> {
    pub cmd: &'a str,
    pub desc: &'a str,
    pub alias: &'a str,
    pub func: &'a dyn Fn(&mut Repl, &str) -> Result<()>,
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
}

fn context_cmd(r: &mut Repl, input: &str) -> Result<()> {
    for (k, v) in r.runner.irc.scope.definitions.iter() {
        if input.is_empty() || **k == *input {
            print!("{k} = ");
            r.runner
                .irc
                .scope
                .pretty_print(&r.runner.irc.scope.res_pool[v.0]);
            println!();
        }
    }
    Ok(())
}

fn res_cmd(r: &mut Repl, _input: &str) -> Result<()> {
    println!("context's size: {}", r.runner.irc.scope.definitions.len());
    println!("resources used: {}", r.runner.irc.scope.res_pool.len());
    Ok(())
}

fn help_cmd(_r: &mut Repl, name: &str) -> Result<()> {
    for cmd in COMMANDS {
        if cmd.matches(name) {
            println!("{} (alias {})", cmd.cmd, cmd.alias);
            println!("\t{}", cmd.desc);

            break;
        }
    }
    Ok(())
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
        func: set_cmd,
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
                    println!();
                }
            }
            Ok(())
        },
    },
    Command {
        cmd: "res",
        alias: "r",
        desc: "show how many resources are being used",
        func: res_cmd,
    },
];
