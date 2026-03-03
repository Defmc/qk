pub mod repl;

fn main() -> repl::Result<()> {
    let mut r = repl::Repl::new()?;
    r.run()
}
