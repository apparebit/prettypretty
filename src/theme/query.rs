use std::io::{Result, Write};

use super::{Theme, ThemeEntry};
use crate::error::{ThemeError, ThemeErrorKind};
use crate::term::{terminal, Options, TerminalAccess, VtScanner};
use crate::Color;

// ----------------------------------------------------------------------------------------------------------

/// Access the terminal.
#[inline]
fn access_with(options: Options) -> Result<TerminalAccess<'static>> {
    let tty = terminal()
        .access_with(options)
        .map_err(|e| ThemeError::new(ThemeErrorKind::AccessDevice, e.into()))?;
    Ok(tty)
}

/// Write the query for the theme entry.
#[inline]
fn write(tty: &mut TerminalAccess, entry: ThemeEntry) -> Result<()> {
    write!(tty, "{}", entry)
        .map_err(|e| ThemeError::new(ThemeErrorKind::WriteQuery(entry), e.into()))?;
    Ok(())
}

/// Write and flush the query for the theme entry.
#[inline]
fn write_and_flush(tty: &mut TerminalAccess, entry: ThemeEntry) -> Result<()> {
    write!(tty, "{}", entry)
        .and_then(|()| tty.flush())
        .map_err(|e| ThemeError::new(ThemeErrorKind::WriteQuery(entry), e.into()))?;
    Ok(())
}

/// Read the response to a query.
#[inline]
fn read<'a>(
    tty: &mut TerminalAccess,
    scanner: &'a mut VtScanner,
    entry: ThemeEntry,
) -> Result<&'a str> {
    let response = scanner
        .scan_str(tty)
        .map_err(|e| ThemeError::new(ThemeErrorKind::ScanEscape(entry), e.into()))?;
    Ok(response)
}

/// Parse the response to a query.
#[inline]
fn parse(entry: ThemeEntry, response: &str) -> Result<Color> {
    let color = entry
        .parse_response(response)
        .map_err(|e| ThemeError::new(ThemeErrorKind::ParseColor(entry), e.into()))?;
    Ok(color)
}

// ----------------------------------------------------------------------------------------------------------

/// Use the terminal access and scanner to fill in the theme.
///
/// This function uses one loop for writing queries as well as reading and
/// parsing the responses.
#[doc(hidden)]
#[inline]
pub fn query1(tty: &mut TerminalAccess, scanner: &mut VtScanner, theme: &mut Theme) -> Result<()> {
    for entry in ThemeEntry::all() {
        write_and_flush(tty, entry)?;
        let response = read(tty, scanner, entry)?;
        theme[entry] = parse(entry, response)?;
    }

    Ok(())
}

/// Use the terminal access and scanner to fill in the theme.
///
/// This function uses two loops, one for writing queries and one for reading
/// and parsing responses.
#[doc(hidden)]
#[inline]
pub fn query2(tty: &mut TerminalAccess, scanner: &mut VtScanner, theme: &mut Theme) -> Result<()> {
    for entry in ThemeEntry::all() {
        write(tty, entry)?;
    }

    tty.flush()?;

    for entry in ThemeEntry::all() {
        let response = read(tty, scanner, entry)?;
        theme[entry] = parse(entry, response)?;
    }

    Ok(())
}

/// Use the terminal access and scanner to fill in the theme.
///
/// This function uses three loops, one for writing queries, one for reading
/// responses, and one for parsing responses. parse loop.
#[doc(hidden)]
#[inline]
pub fn query3(tty: &mut TerminalAccess, scanner: &mut VtScanner, theme: &mut Theme) -> Result<()> {
    for entry in ThemeEntry::all() {
        write(tty, entry)?;
    }

    tty.flush()?;

    let mut all_responses = Vec::new();
    for entry in ThemeEntry::all() {
        let response = read(tty, scanner, entry)?;
        all_responses.push(String::from(response));
    }

    for (entry, response) in ThemeEntry::all().zip(all_responses.into_iter()) {
        theme[entry] = parse(entry, &response)?;
    }

    Ok(())
}

/// Prepare for applying a query function.
#[doc(hidden)]
#[inline]
pub fn prepare(verbose: bool) -> Result<(TerminalAccess<'static>, VtScanner, Theme)> {
    prepare_with(Options::builder().verbose(verbose).build())
}

/// Prepare for applying a query function.
#[doc(hidden)]
#[inline]
pub fn prepare_with(options: Options) -> Result<(TerminalAccess<'static>, VtScanner, Theme)> {
    let tty = access_with(options)?;
    let scanner = VtScanner::new();
    let theme = Theme::default();

    Ok((tty, scanner, theme))
}

/// Apply the query function to terminal, scanner, and theme.
#[doc(hidden)]
pub fn apply<F>(query: F, options: Options) -> Result<Theme>
where
    F: Fn(&mut TerminalAccess, &mut VtScanner, &mut Theme) -> Result<()>,
{
    let (mut tty, mut scanner, mut theme) = prepare_with(options)?;
    query(&mut tty, &mut scanner, &mut theme)?;
    Ok(theme)
}
