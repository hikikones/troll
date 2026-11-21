use terminal::Terminal;
use trollink2::{APP_NAME, APP_ORGANIZATION, APP_QUALIFIER, app::App, cli::Args};

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
    let app_dir = shared::env::get_app_dir(APP_QUALIFIER, APP_ORGANIZATION, APP_NAME);
    let mut app = App::new(args, app_dir)?;
    let terminal = Terminal::stdout();
    terminal.enter(|term| {
        app.run(term)?;
        Ok(())
    })
}
