use smallvec::SmallVec;

use crate::repl::settings::Setting;

pub const BENCH_SETTING: Setting = Setting {
    all: &["lexer", "parser", "command", "ir", "compiler"],
    on: SmallVec::new_const(),
};

pub const SHOW_SETTING: Setting = Setting {
    all: &["lexer", "parser", "command", "ir", "compiler"],
    on: SmallVec::new_const(),
};
