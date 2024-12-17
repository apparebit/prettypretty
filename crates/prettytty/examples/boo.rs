use prettytty::cmd::{EraseScreen, MoveTo, RequestCursorPosition};
use prettytty::err::ErrorKind;
use prettytty::{Connection, Query, Scan, Token};

fn main() -> std::io::Result<()> {
    let rcp = &RequestCursorPosition;
    let tty = Connection::open()?;
    let pos = {
        let (mut output, mut input) = (tty.output(), tty.input());
        output.exec(EraseScreen)?;
        output.exec(MoveTo(6, 65))?;
        output.exec(rcp)?;

        match input.read_token()? {
            Token::Sequence(ctrl, payload) if ctrl == rcp.control() => rcp.parse(payload)?,
            Token::Sequence(_, _) => return Err(ErrorKind::BadControl.into()),
            _ => return Err(ErrorKind::NotASequence.into()),
        }
    };
    drop(tty);

    println!("{}/{}\n\n", pos.0, pos.1);
    assert_eq!(pos, (6, 65));

    Ok(())
}
