use std::io::Result;

use prettypretty::style::{stylist, Fidelity};
use prettypretty::theme::{Theme, VGA_COLORS};
use prettypretty::{OkVersion, Translator};
use prettytty::opt::Options;
use prettytty::Connection;

fn main() -> Result<()> {
    // 1. Assemble your styles
    let chic = stylist()
        .bold()
        .underlined()
        .rgb(215, 40, 39)
        .fg()
        .et_voila();

    // 2. Adjust your styles
    let options = Options::builder().timeout(10).build();
    let (has_tty, theme) = match Connection::with_options(options) {
        Ok(tty) => (true, Theme::query(&tty)?),
        Err(_) => (false, VGA_COLORS),
    };
    let fidelity = Fidelity::from_environment(has_tty);
    let translator = Translator::new(OkVersion::Revised, theme);
    let effective_chic = &chic.cap(fidelity, &translator);

    // 3. Apply your styles
    println!("\n    {}Wow!{}\n", effective_chic, -effective_chic);

    Ok(())
}
