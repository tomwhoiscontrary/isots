extern crate regex;
extern crate chrono;

use std::io::Read;
use regex::Regex;
use chrono::TimeZone;
use std::io::Write;
use chrono::Timelike;
use chrono::Datelike;

// const BUF_SIZE: usize = 1024 * 1024;
const BUF_SIZE: usize = 25;

const POWERS_OF_10: [u64; 10] =
    [1, 10, 100, 1_000, 10_000, 100_000, 1_000_000, 10_000_000, 100_000_000, 1_000_000_000];

fn main() -> std::io::Result<()> {
    let pattern = Regex::new(r"\b1[45][0-9]{8,17}\b").expect("timestamp regex should be valid");

    let mut buffer: [u8; BUF_SIZE] = [0; BUF_SIZE];

    // raw stdin has an 8k buffer
    let shared_stdin = std::io::stdin();
    let mut stdin = shared_stdin.lock();

    // raw stdout has a line buffer, which flushes after every newline
    let shared_stdout = std::io::stdout();
    let raw_stdout = shared_stdout.lock();
    let mut stdout = std::io::BufWriter::with_capacity(BUF_SIZE, raw_stdout);

    // TODO use a Vec<u8> as the buffer, should make a lot of this simpler

    let mut leftover = 0;
    loop {
        let bytes_read = stdin.read(&mut buffer[leftover..])?;
        if bytes_read == 0 {
            if leftover == 0 {
                return Ok(());
            } else {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData,
                                               "input ends with an incomplete UTF8 sequence"));
            }
        };

        let buffer_size = leftover + bytes_read;

        let haystack_size: usize;
        {
            // TODO use unchecked from_utf8 in rescue
            let mut haystack = std::str::from_utf8(&buffer[..buffer_size]).or_else(|e| if e.error_len().is_none() {
                             std::str::from_utf8(&buffer[..e.valid_up_to()])
                         } else {
                             Err(e)
                         })
                .map_err(to_io_error)?;

            // make sure the haystack does not end with digits - defer those to the next block
            while haystack.len() > 0 &&
                  haystack.get(haystack.len() - 1..haystack.len())
                      .map(|s| {
                               s.chars()
                                   .next()
                                   .expect("nonempty string should have a first character")
                                   .is_digit(10)
                           })
                      .unwrap_or(false) {
                haystack = &haystack[..haystack.len() - 1];
            }

            if haystack.is_empty() {
                // should treat the last max_timestamp_len + 1 as leftovers and output the rest
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData,
                                               "input contains an entire buffer's worth of digits, which is too much"));
            }

            haystack_size = haystack.len();

            let mut prev_end = 0;
            for found in pattern.find_iter(haystack) {
                let fractional_seconds_digits = found.as_str().len() - 10;
                let scale = POWERS_OF_10[fractional_seconds_digits];

                let timestamp: u64 =
                found.as_str().parse().expect("sequence of digits should be parseable as integer");

                let time = chrono::Utc.timestamp((timestamp / scale) as i64,
                                                 (timestamp % scale) as u32);

                stdout.write(&haystack[prev_end..found.start()].as_bytes())?;

                if fractional_seconds_digits == 0 {
                    write!(stdout,
                           "{}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                           time.year(),
                           time.month(),
                           time.day(),
                           time.hour(),
                           time.minute(),
                           time.second());
                } else {
                    write!(stdout,
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

                prev_end = found.end();
            }
            stdout.write(&haystack[prev_end..haystack.len()].as_bytes())?;
        }

        leftover = buffer_size - haystack_size;
        if leftover > 0 {
            for i in 0..leftover {
                buffer[i] = buffer[haystack_size + i];
            }
        }
    }
}

fn to_io_error(e: std::str::Utf8Error) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, e)
}
