use std::{
    io::{BufRead, IsTerminal, Read, StdoutLock, Write},
    ops::Deref,
};

use ab_glyph::Font;
use bstr::ByteSlice;
use clap::{arg, crate_authors, crate_description, crate_name, crate_version, Arg, ArgAction, Command};

struct Formatter<'a> {
    stdout_lock: StdoutLock<'a>,
    fontnames: &'a str,
    as_bytes: bool,
    all_as_hex: bool,
    hex_as_decimal: bool,
    newline_escaped: bool,
    newline_as_hex: bool,
    carriage_return_as_hex: bool,
    tab_as_hex: bool,
    space_as_circle: bool,
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
        } else if self.hex_as_decimal {
            write!(self.stdout_lock, "\\u{{{}}}", char as u32).unwrap();
        } else {
            write!(self.stdout_lock, "{}", char.escape_unicode()).unwrap();
        }
    }

    fn process_str(&mut self, fonts: &[FontCow], buffer: &[u8]) {
        for (start, end, char) in buffer.char_indices() {
            let char_as_string = char.to_string();
            let original_bytes = &buffer[start..end];
            let new_bytes = char_as_string.as_bytes();

            if original_bytes != new_bytes {
                for byte in original_bytes {
                    self.write_byte(*byte);
                }

                continue;
            }

            if self.all_as_hex {
                self.write_char(char);
                continue;
            }

            match char {
                c if c == '\n' && self.newline_escaped => write!(self.stdout_lock, "\\n").unwrap(),
                c if c == '\n' && !self.newline_as_hex => write!(self.stdout_lock, "{}", char).unwrap(),
                c if c == '\r' && !self.carriage_return_as_hex => write!(self.stdout_lock, "\\r").unwrap(),
                c if c == '\t' && !self.tab_as_hex => write!(self.stdout_lock, "\\t").unwrap(),
                c if c == ' ' && self.space_as_circle => write!(self.stdout_lock, "🞄").unwrap(),
                c if c.is_ascii_control()
                    || (c != ' ' && c.is_whitespace())
                    || (c == ' ' && self.space_as_hex)
                    || (!c.is_ascii() && !is_char_in_fonts(fonts, char)) =>
                {
                    self.write_char(char);
                }
                _ => write!(self.stdout_lock, "{}", char).unwrap(),
            };
        }
    }
}

fn is_char_in_fonts(fonts: &[FontCow], char: char) -> bool {
    for font in fonts {
        if font.glyph_id(char).0 != 0 {
            return true;
        }
    }

    false
}

fn main() {
    // Parse args
    let matches = Command::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(
            arg!(
                -b --"bytes" "Show bytes instead of unicode values"
            )
            .required(false)
            .action(ArgAction::SetTrue),
        )
        .arg(
            arg!(
                -a --"all" "Print everything as hex values"
            )
            .required(false)
            .action(ArgAction::SetTrue),
        )
        .arg(
            arg!(
                -d --"decimal" "Print hex values as decimal values"
            )
            .required(false)
            .action(ArgAction::SetTrue),
        )
        .arg(
            arg!(
                -n --"newline-escaped" "Print new line as \\n"
            )
            .required(false)
            .action(ArgAction::SetTrue),
        )
        .arg(
            arg!(
                -N --"newline-hex" "Print new line as hex value"
            )
            .conflicts_with("newline-escaped")
            .required(false)
            .action(ArgAction::SetTrue),
        )
        .arg(
            arg!(
                -r --"carriage-return-hex" "Print carriage return as hex value instead of \\r"
            )
            .required(false)
            .action(ArgAction::SetTrue),
        )
        .arg(
            arg!(
                -t --"tab-hex" "Print tab as hex value instead of \\t"
            )
            .required(false)
            .action(ArgAction::SetTrue),
        )
        .arg(
            arg!(
                -s --"space-circle" "Print space as circle (🞄)"
            )
            .required(false)
            .action(ArgAction::SetTrue),
        )
        .arg(
            arg!(
                -S --"space-hex" "Print space as hex value"
            )
            .conflicts_with("space-circle")
            .required(false)
            .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("fontname")
                .short('f')
                .long("fontname")
                .value_name("FONT1[,FONT2,...]")
                .help("Sets the font to check whether a glyph is present")
                .required(true),
        )
        .arg(
            arg!(
                -l --"line-by-line" "Read lines by line"
            )
            .required(false)
            .action(ArgAction::SetTrue),
        )
        .get_matches();

    let stdout = std::io::stdout();

    let mut formatter = Formatter {
        stdout_lock: stdout.lock(),
        fontnames: matches.get_one::<String>("fontname").unwrap(),
        as_bytes: matches.get_flag("bytes"),
        all_as_hex: matches.get_flag("all"),
        hex_as_decimal: matches.get_flag("decimal"),
        newline_escaped: matches.get_flag("newline-escaped"),
        newline_as_hex: matches.get_flag("newline-hex") && !matches.get_flag("newline-escaped"),
        carriage_return_as_hex: matches.get_flag("carriage-return-hex"),
        tab_as_hex: matches.get_flag("tab-hex"),
        space_as_circle: matches.get_flag("space-circle"),
        space_as_hex: matches.get_flag("space-hex") && !matches.get_flag("space-circle"),
    };
    let line_by_line = matches.get_flag("line-by-line");

    // Prepare to read text from stdin
    let stdin = std::io::stdin();
    let mut stdin = stdin.lock();
    let mut buffer = Vec::with_capacity(256);

    if formatter.all_as_hex && formatter.as_bytes {
        if line_by_line {
            loop {
                let read = stdin.read_until(b'\n', &mut buffer).unwrap();

                if read == 0 {
                    break;
                }

                for byte in &buffer[..read] {
                    formatter.write_byte(*byte);
                }

                formatter.stdout_lock.flush().unwrap();
                buffer.clear();
            }
        } else {
            stdin.read_to_end(&mut buffer).unwrap();

            for byte in &buffer {
                formatter.write_byte(*byte);
            }
        }
    } else {
        // Init font database
        let mut font_db = fontdb::Database::new();
        font_db.load_system_fonts();

        // Load fonts
        let font_sources: Vec<_> = formatter
            .fontnames
            .split(',')
            .map(|fontname| get_font_source(&font_db, fontname))
            .collect();

        let mut fonts = Vec::with_capacity(font_sources.len());
        for src in &font_sources {
            fonts.push(load_font(src));
        }

        if line_by_line {
            loop {
                let read = stdin.read_until(b'\n', &mut buffer).unwrap();

                if read == 0 {
                    break;
                }

                formatter.process_str(&fonts, &buffer[..read]);
                formatter.stdout_lock.flush().unwrap();
                buffer.clear();
            }
        } else {
            stdin.read_to_end(&mut buffer).unwrap();
            formatter.process_str(&fonts, &buffer);
        }
    }

    // Final newline for terminal output, if there is no ending newline
    if formatter.stdout_lock.is_terminal()
        && (formatter.all_as_hex || formatter.newline_escaped || formatter.newline_as_hex)
    {
        writeln!(formatter.stdout_lock).unwrap();
    }
}

#[allow(clippy::large_enum_variant)]
enum FontCow<'a> {
    FontVec(ab_glyph::FontVec),
    FontRef(ab_glyph::FontRef<'a>),
}

impl FontCow<'_> {
    fn glyph_id(&self, c: char) -> ab_glyph::GlyphId {
        match self {
            FontCow::FontVec(f) => f.glyph_id(c),
            FontCow::FontRef(f) => f.glyph_id(c),
        }
    }
}

fn get_font_source(font_db: &fontdb::Database, fontname: &str) -> fontdb::Source {
    let query = fontdb::Query {
        families: &[fontdb::Family::Name(fontname)],
        ..fontdb::Query::default()
    };

    match font_db.query(&query) {
        Some(id) => {
            let (src, _) = font_db.face_source(id).unwrap();
            src
        }
        None => {
            eprintln!("Error: Font '{}' not found", fontname);
            std::process::exit(1);
        }
    }
}

fn load_font(font_source: &fontdb::Source) -> FontCow<'_> {
    match font_source {
        fontdb::Source::Binary(bin) | fontdb::Source::SharedFile(_, bin) => {
            FontCow::FontRef(ab_glyph::FontRef::try_from_slice(bin.deref().as_ref()).expect("Could not load font"))
        }
        fontdb::Source::File(path) => FontCow::FontVec(
            ab_glyph::FontVec::try_from_vec(std::fs::read(path).expect("Could not read font file"))
                .expect("Could not load font"),
        ),
    }
}
