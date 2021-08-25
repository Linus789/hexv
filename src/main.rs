use std::io::{Read, StdoutLock, Write};

use clap::{App, Arg};

struct Settings<'a> {
    fontname: &'a str,
    as_bytes: bool,
    all_as_hex: bool,
    newline_as_hex: bool,
    carriage_return_as_hex: bool,
    space_as_hex: bool,
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
            Arg::new("newline")
                .short('n')
                .long("newline")
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

    let settings = Settings {
        fontname: matches.value_of("fontname").unwrap(),
        as_bytes: matches.is_present("bytes"),
        all_as_hex: matches.is_present("all"),
        newline_as_hex: matches.is_present("newline"),
        carriage_return_as_hex: matches.is_present("carriage-return"),
        space_as_hex: matches.is_present("space"),
    };

    // Lock stdout
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();

    // Read text from stdin
    let stdin = std::io::stdin();
    let mut stdin = stdin.lock();

    let mut buffer = Vec::with_capacity(256);
    stdin.read_to_end(&mut buffer).unwrap();

    if settings.all_as_hex && settings.as_bytes {
        // This option works even with invalid utf-8
        for byte in &buffer {
            write!(lock, "\\x{:02x}", byte).unwrap();
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
            families: &[fontdb::Family::Name(settings.fontname)],
            ..fontdb::Query::default()
        };

        let src = match font_db.query(&query) {
            Some(id) => {
                let (src, _) = font_db.face_source(id).unwrap();
                src
            }
            None => {
                eprintln!("Error: Font '{}' not found", settings.fontname);
                std::process::exit(1);
            }
        };

        let bin = match &*src {
            fontdb::Source::Binary(bin) => std::borrow::Cow::Borrowed(bin),
            fontdb::Source::File(path) => std::borrow::Cow::Owned(std::fs::read(path).expect("Could not read font file")),
        };

        let font = rusttype::Font::try_from_bytes(&bin).expect("Could not load font");
        process_str(&mut lock, &settings, &font, str.unwrap());
    }

    // Final newline for terminal output, if there is no ending newline
    if atty::is(atty::Stream::Stdout) && (settings.all_as_hex || settings.newline_as_hex) {
        writeln!(lock).unwrap();
    }
}

fn process_str(lock: &mut StdoutLock, settings: &Settings, font: &rusttype::Font, str: &str) {
    for char in str.chars() {
        let glyph_available = font.glyph(char).id().0 != 0;

        match char {
            c if !settings.all_as_hex && c == '\n' && !settings.newline_as_hex => write!(lock, "{}", char).unwrap(),
            c if !settings.all_as_hex && c == '\r' && !settings.carriage_return_as_hex => write!(lock, "\\r").unwrap(),
            c if settings.all_as_hex
                || c.is_ascii_control()
                || (c != ' ' && c.is_whitespace())
                || (c == ' ' && settings.space_as_hex)
                || (!c.is_ascii() && !glyph_available) =>
            {
                if settings.as_bytes {
                    for byte in char.to_string().bytes() {
                        write!(lock, "\\x{:02x}", byte).unwrap();
                    }
                } else {
                    write!(lock, "{}", char.escape_unicode()).unwrap();
                }
            }
            _ => write!(lock, "{}", char).unwrap(),
        };
    }
}
