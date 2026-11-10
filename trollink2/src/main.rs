use terminal::Terminal;
use trollink2::{app::App, cli::Args};

fn main() -> std::process::ExitCode {
    let args: Args = clap::Parser::parse();

    if let Err(err) = run(args) {
        eprintln!("Error: {err}");

        let mut source = err.source();
        while let Some(cause) = source {
            eprintln!("Caused by: {cause}");
            source = cause.source();
        }

        return std::process::ExitCode::FAILURE;
    }

    std::process::ExitCode::SUCCESS
}

fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let terminal = Terminal::stdout();
    let mut app = App::new(args)?;
    terminal.enter(|term| {
        app.run(term)?;
        Ok(())
    })
}
