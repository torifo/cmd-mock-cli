use anyhow::Result;
use clap::Parser;
use cmd_mock_cli::app::{App, Cli};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut app = App::bootstrap(cli)?;
    app.run()
}
