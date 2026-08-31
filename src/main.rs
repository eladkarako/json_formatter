use serde_json::Value;
use std::{
    env,
    fs,
    io::{self, Read, Write},
    process,
};
use is_terminal::IsTerminal;


#[derive(Clone, Copy)]
enum OutputFormat {
    Minified,
    Beautified,
}

#[derive(Clone, Copy)]
enum SortMode {
    NoSort,
    SortKeys,
    SortKeysArrays,
}

#[derive(Clone, Copy)]
enum OutputEol {
    Os,
    Win,
    Nx,
}

impl OutputEol {
    fn bytes(self) -> &'static [u8] {
        match self {
            OutputEol::Win => b"\r\n",
            OutputEol::Nx => b"\n",
            OutputEol::Os => {
                if cfg!(windows) {
                    b"\r\n"
                } else {
                    b"\n"
                }
            }
        }
    }
}

fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    let mut left = a.chars().peekable();
    let mut right = b.chars().peekable();

    loop {
        match (left.peek(), right.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,

            (Some(a), Some(b)) if a.is_ascii_digit() && b.is_ascii_digit() => {
                let mut a_number = String::new();
                let mut b_number = String::new();

                while left.peek().is_some_and(|c| c.is_ascii_digit()) {
                    a_number.push(left.next().unwrap());
                }

                while right.peek().is_some_and(|c| c.is_ascii_digit()) {
                    b_number.push(right.next().unwrap());
                }

                let a_trimmed = a_number.trim_start_matches('0');
                let b_trimmed = b_number.trim_start_matches('0');

                let a_trimmed = if a_trimmed.is_empty() {
                    "0"
                } else {
                    a_trimmed
                };

                let b_trimmed = if b_trimmed.is_empty() {
                    "0"
                } else {
                    b_trimmed
                };

                match a_trimmed.len().cmp(&b_trimmed.len()) {
                    Ordering::Equal => match a_trimmed.cmp(b_trimmed) {
                        Ordering::Equal => continue,
                        ordering => return ordering,
                    },
                    ordering => return ordering,
                }
            }

            (Some(a), Some(b)) => {
                let ordering = a.cmp(b);

                left.next();
                right.next();

                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
        }
    }
}

fn value_sort_key(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

fn sort_value(value: &mut Value, mode: SortMode) {
    match value {
        Value::Object(object) => {
            // Always recursively visit object values.
            for child in object.values_mut() {
                sort_value(child, mode);
            }

            if matches!(mode, SortMode::SortKeys | SortMode::SortKeysArrays) {
                let mut entries: Vec<_> = object
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect();

                entries.sort_by(|(a, _), (b, _)| natural_cmp(a, b));

                object.clear();

                for (key, value) in entries {
                    object.insert(key, value);
                }
            }
        }

        Value::Array(array) => {
            // Always recursively visit array entries.
            for child in array.iter_mut() {
                sort_value(child, mode);
            }

            // Only sort the array itself in this mode.
            if matches!(mode, SortMode::SortKeysArrays) {
                array.sort_by(|a, b| {
                    natural_cmp(&value_sort_key(a), &value_sort_key(b))
                });
            }
        }

        Value::Null
        | Value::Bool(_)
        | Value::Number(_)
        | Value::String(_) => {}
    }
}

fn parse_output_format(value: &str) -> Result<OutputFormat, String> {
    match value {
        "json_minified" => Ok(OutputFormat::Minified),
        "json_beautified" => Ok(OutputFormat::Beautified),
        _ => Err(format!("Invalid output format: {value}")),
    }
}

fn parse_sort_mode(value: &str) -> Result<SortMode, String> {
    match value {
        "no-sort" => Ok(SortMode::NoSort),
        "sort-keys" => Ok(SortMode::SortKeys),
        "sort-keys-arrays" => Ok(SortMode::SortKeysArrays),
        _ => Err(format!("Invalid output sort mode: {value}")),
    }
}

fn parse_output_eol(value: &str) -> Result<OutputEol, String> {
    match value {
        "os" => Ok(OutputEol::Os),
        "win" => Ok(OutputEol::Win),
        "nx" => Ok(OutputEol::Nx),
        _ => Err(format!("Invalid Output EOL mode: {value}")),
    }
}

fn main() {
    let mut input_file: Option<String> = None;
    let mut output_file: Option<String> = None;
    let mut raw_input: Option<String> = None;

    let mut output_format = OutputFormat::Beautified;
    let mut sort_mode = SortMode::NoSort;
    let mut output_eol = OutputEol::Os;

    for argument in env::args().skip(1) {
        if let Some(value) = argument.strip_prefix("--input-file=") {
            input_file = Some(value.to_string());
        } else if let Some(value) = argument.strip_prefix("--output-file=") {
            output_file = Some(value.to_string());
        } else if let Some(value) = argument.strip_prefix("--output-format=") {
            output_format = parse_output_format(value).unwrap_or_else(|error| {
                eprintln!("{error}");
                process::exit(2);
            });
        } else if let Some(value) = argument.strip_prefix("--output-sort=") {
            sort_mode = parse_sort_mode(value).unwrap_or_else(|error| {
                eprintln!("{error}");
                process::exit(2);
            });
        } else if let Some(value) = argument.strip_prefix("--output-eol=") {
            output_eol = parse_output_eol(value).unwrap_or_else(|error| {
                eprintln!("{error}");
                process::exit(2);
            });
        } else if argument == "--help" || argument == "-h" {
            print_help();
            return;
        } else if argument.starts_with("--") {
            eprintln!("Unknown option: {argument}");
            process::exit(2);
        } else if raw_input.is_none() {
            // This is raw JSON5 content, not a filename.
            raw_input = Some(argument);
        } else {
            eprintln!("Only one raw JSON5 argument is allowed");
            process::exit(2);
        }
    }

    let input = if let Some(filename) = input_file {
        // --input-file= overrides both stdin and the raw argument.
        fs::read_to_string(&filename).unwrap_or_else(|error| {
            eprintln!("Failed to read input file {filename}: {error}");
            process::exit(1);
        })
    } else if !io::stdin().is_terminal() {
        let mut input = String::new();

        io::stdin()
            .read_to_string(&mut input)
            .unwrap_or_else(|error| {
                eprintln!("Failed to read stdin: {error}");
                process::exit(1);
            });

        input
    } else if let Some(input) = raw_input {
        input
    } else {
        eprintln!("No input provided");
        print_help();
        process::exit(2);
    };

    // JSON5 is always used for reading.
    let mut value: Value = json5::from_str(&input).unwrap_or_else(|error| {
        eprintln!("Invalid JSON5 input: {error}");
        process::exit(1);
    });

    sort_value(&mut value, sort_mode);

    let serialized = match output_format {
        OutputFormat::Minified => serde_json::to_string(&value),
        OutputFormat::Beautified => serde_json::to_string_pretty(&value),
    }
        .unwrap_or_else(|error| {
            eprintln!("Failed to serialize JSON: {error}");
            process::exit(1);
        });

    let mut output = Vec::new();

    // serde_json pretty output uses \n. Normalize it to the requested Output EOL.
    let normalized = serialized.replace('\n', std::str::from_utf8(output_eol.bytes()).unwrap());

    output.extend_from_slice(normalized.as_bytes());
    output.extend_from_slice(output_eol.bytes());

    if let Some(filename) = output_file {
        fs::write(&filename, output).unwrap_or_else(|error| {
            eprintln!("Failed to write output file {filename}: {error}");
            process::exit(1);
        });
    } else {
        io::stdout().write_all(&output).unwrap_or_else(|error| {
            eprintln!("Failed to write stdout: {error}");
            process::exit(1);
        });
    }
}

fn print_help() {
    eprintln!(
        "Usage:
  json-format [RAW_JSON5] [OPTIONS]

Input precedence:
  1. --input-file=PATH
  2. piped stdin
  3. first argument not starting with --

Options:
  --input-file=PATH
  --output-file=PATH
  --output-format=json_minified|json_beautified
  --output-sort=no-sort|sort-keys|sort-keys-arrays
  --output-eol=os|win|nx

Defaults:
  --output-format=json_beautified
  --output-sort=no-sort
  --output-eol=os"
    );
}
