extern crate chrono;

use std::io::Read;
use chrono::TimeZone;
use chrono::Timelike;
use chrono::Datelike;

const BUF_SIZE: usize = 1024 * 1024;

const POWERS_OF_10: [u64; 10] =
    [1, 10, 100, 1_000, 10_000, 100_000, 1_000_000, 10_000_000, 100_000_000, 1_000_000_000];

#[derive(PartialEq, Eq)]
enum State {
    Hunting,
    Skipping,
    MatchStarted,
    Matching,
}

fn main() -> std::io::Result<()> {
    // raw stdin has an 8k buffer
    let shared_stdin = std::io::stdin();
    let raw_stdin = shared_stdin.lock();
    let mut stdin = std::io::BufReader::with_capacity(BUF_SIZE, raw_stdin).bytes();

    // raw stdout has a line buffer, which flushes after every newline
    let shared_stdout = std::io::stdout();
    let raw_stdout = shared_stdout.lock();
    let mut stdout = std::io::BufWriter::with_capacity(BUF_SIZE, raw_stdout);

    let mut state = Some(State::Hunting);
    let mut buffer = Vec::with_capacity(19);

    while state.is_some() {
        state = match stdin.next() {
            Some(Ok(ch)) if ch >= '0' as u8 && ch <= '9' as u8 => {
                // digit

                match state.expect("state should exist while running") {
                    State::Hunting => {
                        if ch == '1' as u8 {
                            Some(State::MatchStarted)
                        } else {
                            emit(&mut stdout, ch)?;
                            Some(State::Skipping)
                        }
                    }
                    State::Skipping => {
                        emit(&mut stdout, ch)?;
                        Some(State::Skipping)
                    }
                    State::MatchStarted => {
                        if ch == '4' as u8 || ch == '5' as u8 {
                            buffer.push('1' as u8);
                            buffer.push(ch);
                            Some(State::Matching)
                        } else {
                            emit(&mut stdout, '1' as u8)?;
                            emit(&mut stdout, ch)?;
                            Some(State::Skipping)
                        }
                    }
                    State::Matching => {
                        if buffer.len() < buffer.capacity() {
                            buffer.push(ch);
                            Some(State::Matching)
                        } else {
                            emit_buffer(&mut stdout, &mut buffer)?;
                            Some(State::Skipping)
                        }
                    }
                }
            }
            Some(Ok(ch)) => {
                // non-digit

                match state.expect("state should exist while running") {
                    State::Hunting => {
                        emit(&mut stdout, ch)?;
                        Some(State::Hunting)
                    }
                    State::Skipping => {
                        emit(&mut stdout, ch)?;
                        Some(State::Hunting)
                    }
                    State::MatchStarted => {
                        emit(&mut stdout, '1' as u8)?;
                        emit(&mut stdout, ch)?;
                        Some(State::Hunting)
                    }
                    State::Matching => {
                        emit_date_or_buffer(&mut stdout, &mut buffer)?;
                        emit(&mut stdout, ch)?;
                        Some(State::Hunting)
                    }
                }
            }
            Some(Err(e)) => {
                // IO error!

                return Err(e);
            }
            None => {
                // end of file

                match state.expect("state should exist while running") {
                    State::Hunting => None,
                    State::Skipping => None,
                    State::MatchStarted => {
                        emit(&mut stdout, '1' as u8)?;
                        None
                    }
                    State::Matching => {
                        emit_date_or_buffer(&mut stdout, &mut buffer)?;
                        None
                    }
                }
            }
        };
    }

    assert!(buffer.is_empty());

    Ok(())
}

fn emit_date_or_buffer(out: &mut impl std::io::Write, buffer: &mut Vec<u8>) -> std::io::Result<()> {
    if buffer.len() >= 10 {
        emit_date(out, buffer)
    } else {
        emit_buffer(out, buffer)
    }
}

fn emit_date(out: &mut impl std::io::Write, buffer: &mut Vec<u8>) -> std::io::Result<()> {
    let fractional_seconds_digits = buffer.len() - 10;
    let scale = POWERS_OF_10[fractional_seconds_digits];

    let mut timestamp: u64 = 0;
    for d in buffer.iter() {
        assert!((*d as char).is_digit(10),
                "{} is not a digit (in {:?})",
                *d,
                buffer);
        timestamp = timestamp * 10 + (*d - '0' as u8) as u64;
    }

    let time = chrono::Utc.timestamp((timestamp / scale) as i64, (timestamp % scale) as u32);

    if fractional_seconds_digits == 0 {
        write!(out,
               "{}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
               time.year(),
               time.month(),
               time.day(),
               time.hour(),
               time.minute(),
               time.second());
    } else {
        write!(out,
               "{}-{:02}-{:02}T{:02}:{:02}:{:02}.{:0width$}Z",
               time.year(),
               time.month(),
               time.day(),
               time.hour(),
               time.minute(),
               time.second(),
               time.nanosecond(),
               width = fractional_seconds_digits);
    }

    buffer.clear();

    Ok(())
}

fn emit_buffer(out: &mut impl std::io::Write, buffer: &mut Vec<u8>) -> std::io::Result<()> {
    for d in buffer.into_iter() {
        emit(out, *d)?;
    }

    buffer.clear();

    Ok(())
}

fn emit(out: &mut impl std::io::Write, ch: u8) -> std::io::Result<()> {
    let written = out.write(&[ch])?;

    if written == 0 {
        return Err(std::io::Error::new(std::io::ErrorKind::WriteZero, "output refused write"));
    }

    Ok(())
}
