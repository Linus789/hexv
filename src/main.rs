use std::io::{Read, StdoutLock, Write};

use clap::{App, Arg};

struct Formatter<'a> {
    stdout_lock: StdoutLock<'a>,
    fontname: &'a str,
    as_bytes: bool,
    all_as_hex: bool,
    hex_as_decimal: bool,
    newline_escaped: bool,
    newline_as_hex: bool,
    carriage_return_as_hex: bool,
    space_as_hex: bool,
}

impl<'a> Formatter<'a> {
    fn write_byte(&mut self, byte: u8) {
        if self.hex_as_decimal {
            write!(self.stdout_lock, "\\d{:03}", byte).unwrap();
        } else {
            write!(self.stdout_lock, "\\x{:02x}", byte).unwrap();
        }
    }

    fn write_char(&mut self, char: char) {
        if self.as_bytes {
            for byte in char.to_string().bytes() {
                self.write_byte(byte);
            }
        } else {
            if self.hex_as_decimal {
                write!(self.stdout_lock, "\\u{{{}}}", char as u32).unwrap();
            } else {
                write!(self.stdout_lock, "{}", char.escape_unicode()).unwrap();
            }
        }
    }

    fn process_str(&mut self, font: &rusttype::Font, str: &str) {
        for char in str.chars() {
            let glyph_available = font.glyph(char).id().0 != 0;
    
            match char {
                c if !self.all_as_hex && c == '\n' && self.newline_escaped => write!(self.stdout_lock, "\\n").unwrap(),
                c if !self.all_as_hex && c == '\n' && !self.newline_as_hex => write!(self.stdout_lock, "{}", char).unwrap(),
                c if !self.all_as_hex && c == '\r' && !self.carriage_return_as_hex => write!(self.stdout_lock, "\\r").unwrap(),
                c if self.all_as_hex
                    || c.is_ascii_control()
                    || (c != ' ' && c.is_whitespace())
                    || (c == ' ' && self.space_as_hex)
                    || (!c.is_ascii() && !glyph_available) =>
                {
                    self.write_char(char);
                }
                _ => write!(self.stdout_lock, "{}", char).unwrap(),
            };
        }
    }
}

fn main() {
    // Parse args
    let matches = App::new("hexv")
        .version("0.1")
        .author("Linus789")
        .about("View text with hex values")
        .arg(
            Arg::new("bytes")
                .short('b')
                .long("bytes")
                .about("Show bytes instead of unicode values")
                .takes_value(false),
        )
        .arg(
            Arg::new("all")
                .short('a')
                .long("all")
                .about("Print everything as hex values")
                .takes_value(false),
        )
        .arg(
            Arg::new("decimal")
                .short('d')
                .long("decimal")
                .about("Print hex values as decimal values")
                .takes_value(false),
        )
        .arg(
            Arg::new("newline-escaped")
                .short('n')
                .long("newline-escaped")
                .about("Print new line as \\n (takes precedence of newline-hex)")
                .takes_value(false),
        )
        .arg(
            Arg::new("newline-hex")
                .short('N')
                .long("newline-hex")
                .about("Print new line as hex value")
                .takes_value(false),
        )
        .arg(
            Arg::new("carriage-return")
                .short('r')
                .long("carriage-return")
                .about("Print carriage return as hex value instead of \\r")
                .takes_value(false),
        )
        .arg(
            Arg::new("space")
                .short('s')
                .long("space")
                .about("Print space as hex value")
                .takes_value(false),
        )
        .arg(
            Arg::new("fontname")
                .short('f')
                .long("fontname")
                .about("Sets the font to check whether a glyph is present")
                .required(true)
                .takes_value(true),
        )
        .get_matches();

    let stdout = std::io::stdout();

    let mut formatter = Formatter {
        stdout_lock: stdout.lock(),
        fontname: matches.value_of("fontname").unwrap(),
        as_bytes: matches.is_present("bytes"),
        all_as_hex: matches.is_present("all"),
        hex_as_decimal: matches.is_present("decimal"),
        newline_escaped: matches.is_present("newline-escaped"),
        newline_as_hex: matches.is_present("newline-hex") && !matches.is_present("newline-escaped"),
        carriage_return_as_hex: matches.is_present("carriage-return"),
        space_as_hex: matches.is_present("space"),
    };

    // Read text from stdin
    let stdin = std::io::stdin();
    let mut stdin = stdin.lock();

    let mut buffer = Vec::with_capacity(256);
    stdin.read_to_end(&mut buffer).unwrap();

    if formatter.all_as_hex && formatter.as_bytes {
        // This option works even with invalid utf-8
        for byte in &buffer {
            formatter.write_byte(*byte);
        }
    } else {
        let str = std::str::from_utf8(&buffer);

        if str.is_err() {
            eprintln!("Error: Invalid UTF-8 from stdin\nYou might want to use the following flags: --all --bytes");
            std::process::exit(1);
        }

        // Init fonts
        let mut font_db = fontdb::Database::new();
        font_db.load_system_fonts();

        let query = fontdb::Query {
            families: &[fontdb::Family::Name(formatter.fontname)],
            ..fontdb::Query::default()
        };

        let src = match font_db.query(&query) {
            Some(id) => {
                let (src, _) = font_db.face_source(id).unwrap();
                src
            }
            None => {
                eprintln!("Error: Font '{}' not found", formatter.fontname);
                std::process::exit(1);
            }
        };

        let bin = match &*src {
            fontdb::Source::Binary(bin) => std::borrow::Cow::Borrowed(bin),
            fontdb::Source::File(path) => {
                std::borrow::Cow::Owned(std::fs::read(path).expect("Could not read font file"))
            }
        };

        let font = rusttype::Font::try_from_bytes(&bin).expect("Could not load font");
        formatter.process_str(&font, str.unwrap());
    }

    // Final newline for terminal output, if there is no ending newline
    if atty::is(atty::Stream::Stdout) && (formatter.all_as_hex || formatter.newline_escaped || formatter.newline_as_hex) {
        writeln!(formatter.stdout_lock).unwrap();
    }
}
