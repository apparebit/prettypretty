//! Querying a terminal for its current color theme.
//!
//! WHILE EXPORTED OUTSIDE THIS CRATE, THE FUNCTIONS IN THIS MODULE ARE NOT PART
//! OF PRETTYPRETTY'S PUBLIC API. THEY MAY CHANGE OR GO AWAY AT ANY MOMENT. USE
//! AT YOUR OWN RISK.
//!
//! This module implements three distinct strategies for querying the terminal.
//! While functionally equivalent, they differ in how they schedule writing to
//! the requests, reading the responses, and parsing the responses:
//!
//!  1. Perform the query with one loop that writes one request, flushes the
//!     output, reads the response, and parses the responses.
//!  2. Perform the query with two loops. The first loop writes all requests and
//!     only afterwards flushes the output once. The second loop reads and
//!     parses one response at a time.
//!  3. Perform the query with three loops. The first loop writes all requests
//!     and only afterwards flushes the output once. The second loop reads all
//!     responses. The third loop parses all responses.
//!
//! Performance measurements show that `query1` takes about 800ms, `query2`
//! about 280ms, and `query3` about 340ms.
//!
//! The two-loop and three-loop versions are much faster than the one-loop
//! version because they only flush the output once. Since each request is
//! between 9 and 10 bytes long, all 18 requests easily fit into the output
//! buffer and hence are written in one system call for the two- and three-loop
//! versions.
//!
//! The two-loop version beats the three-loop version because it does not need
//! to store read responses on the heap, but rather immediately parses them. At
//! the same time, there isn't enough input to fall behind while parsing.
//!
//! WHILE EXPORTED OUTSIDE THIS CRATE, THE FUNCTIONS IN THIS MODULE ARE NOT PART
//! OF PRETTYPRETTY'S PUBLIC API. THEY MAY CHANGE OR GO AWAY AT ANY MOMENT. USE
//! AT YOUR OWN RISK.

use std::io::{Result, Write};

use super::{Theme, ThemeEntry};
use crate::cmd::{Query, WriteCommand};
use crate::term::{terminal, Options, TerminalAccess, VtScanner};

/// Use the terminal access and scanner to fill in the theme.
///
/// This function uses one loop for writing queries as well as reading and
/// parsing the responses.
///
/// THIS FUNCTION IS NOT PART OF PRETTYPRETTY'S PUBLIC API!
#[doc(hidden)]
#[inline]
pub fn query1(tty: &mut TerminalAccess, scanner: &mut VtScanner, theme: &mut Theme) -> Result<()> {
    for entry in ThemeEntry::all() {
        tty.write_cmd(entry)?;
        tty.flush()?;
        let response = scanner.scan_bytes(tty)?;
        theme[entry] = <ThemeEntry as Query>::parse(&entry, response)?;
    }

    Ok(())
}

/// Use the terminal access and scanner to fill in the theme.
///
/// This function uses two loops, one for writing queries and one for reading
/// and parsing responses.
///
/// THIS FUNCTION IS NOT PART OF PRETTYPRETTY'S PUBLIC API!
#[doc(hidden)]
#[inline]
pub fn query2(tty: &mut TerminalAccess, scanner: &mut VtScanner, theme: &mut Theme) -> Result<()> {
    for entry in ThemeEntry::all() {
        tty.write_cmd(entry)?;
    }

    tty.flush()?;

    for entry in ThemeEntry::all() {
        let response = scanner.scan_bytes(tty)?;
        theme[entry] = <ThemeEntry as Query>::parse(&entry, response)?;
    }

    Ok(())
}

/// Use the terminal access and scanner to fill in the theme.
///
/// This function uses three loops, one for writing queries, one for reading
/// responses, and one for parsing responses. parse loop.
///
/// THIS FUNCTION IS NOT PART OF PRETTYPRETTY'S PUBLIC API!
#[doc(hidden)]
#[inline]
pub fn query3(tty: &mut TerminalAccess, scanner: &mut VtScanner, theme: &mut Theme) -> Result<()> {
    for entry in ThemeEntry::all() {
        tty.write_cmd(entry)?;
    }

    tty.flush()?;

    let mut all_responses = Vec::new();
    for _entry in ThemeEntry::all() {
        let response = scanner.scan_bytes(tty)?;
        all_responses.push(response.to_owned());
    }

    for (entry, response) in ThemeEntry::all().zip(all_responses.into_iter()) {
        theme[entry] = <ThemeEntry as Query>::parse(&entry, &response)?;
    }

    Ok(())
}

/// Prepare for applying a query function.
///
/// THIS FUNCTION IS NOT PART OF PRETTYPRETTY'S PUBLIC API!
#[doc(hidden)]
#[inline]
pub fn prepare(verbose: bool) -> Result<(TerminalAccess<'static>, VtScanner, Theme)> {
    let tty = terminal().access_with(Options::builder().verbose(verbose).build())?;
    let scanner = VtScanner::new();
    let theme = Theme::default();

    Ok((tty, scanner, theme))
}

/// Apply the query function to terminal, scanner, and theme.
///
/// THIS FUNCTION IS NOT PART OF PRETTYPRETTY'S PUBLIC API!
#[doc(hidden)]
pub fn apply<F>(query: F, options: Options) -> Result<Theme>
where
    F: Fn(&mut TerminalAccess, &mut VtScanner, &mut Theme) -> Result<()>,
{
    let mut tty = terminal().access_with(options)?;
    let mut scanner = VtScanner::new();
    let mut theme = Theme::default();
    query(&mut tty, &mut scanner, &mut theme)?;
    Ok(theme)
}
