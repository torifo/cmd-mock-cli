use anyhow::Result;
use clap::Parser;
use cmdock::app::{App, Cli, list_modes};

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.list {
        print!("{}", list_modes());
        return Ok(());
    }
    let mut app = App::bootstrap(cli)?;
    app.run()
}
