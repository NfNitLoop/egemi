mod browser;
mod editor;
mod gemtext;
mod gemtext_widget;
mod widgets;
mod util;

use std::error::Error;

use clap::{builder::{styling::{Color, RgbColor, Style, Styles}}, Parser as _};
use eframe::{
    egui::{self, Context, Link, RichText, ScrollArea, TextEdit, TextStyle, Widget}, Frame, NativeOptions
};

use gemtext::Block;

use crate::gemtext_widget::GemtextWidget;

#[derive(clap::Parser, Debug)]
#[command(name = "egemi", version, about, styles = CLAP_STYLING)]
/// egemi, an egui gemini/web browser.
struct Cli {
    /// A URL to browse.
    url: Option<String>
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    Open(OpenCommand)
}

/// Browse to a URL.
#[derive(clap::Args, Debug)]
struct OpenCommand {
}


pub const CLAP_STYLING: Styles = Styles::styled()
    .usage(Style::new().fg_color(Some(Color::Rgb(RgbColor(0, 255, 0)))))
    .literal(Style::new().bold().fg_color(Some(Color::Rgb(RgbColor(220, 220, 0)))))
    .error(Style::new().fg_color(Some(Color::Rgb(RgbColor(255, 0, 0)))))    
;
// ;

type DynResult<T = ()> = Result<T,Box<dyn Error>>;
type DynResultSend<T = ()> = Result<T, Box<dyn Error + Send>>;

fn main() -> DynResult {
    let cli = Cli::parse();
    let url = cli.url.unwrap_or("about:egemi".into());
    
    if url == "editor:" {
        editor::main()?;
        return Ok(());
    }

    browser::main(url)?;
    Ok(())
}

