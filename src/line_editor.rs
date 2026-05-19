use std::{
    io::{self, BufRead, Read, Write},
    mem::MaybeUninit,
};

pub struct RawMode {
    original: libc::termios,
}

impl RawMode {
    pub fn new() -> io::Result<RawMode> {
        let mut termios = MaybeUninit::<libc::termios>::uninit();
        let rc = unsafe { libc::tcgetattr(libc::STDIN_FILENO, termios.as_mut_ptr()) };

        if rc == -1 {
            return Err(std::io::Error::last_os_error());
        }

        let termios = unsafe { termios.assume_init() };

        let mut termios_clone = termios;

        termios_clone.c_lflag &= !(libc::ICANON | libc::ECHO);

        let rc = unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, &termios_clone) };

        if rc == -1 {
            return Err(std::io::Error::last_os_error());
        }

        Ok(RawMode { original: termios })
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, &mut self.original) };
    }
}

fn complete_builtin(prefix: &str, builtins: &[&str]) -> Option<String> {
    let matches: Vec<&&str> = builtins
        .iter()
        .filter(|builtin| builtin.starts_with(prefix))
        .collect();

    if matches.len() != 1 {
        return None;
    }

    return Some(matches[0].to_string());
}

pub fn read_line(prompt: &str, builtins: &[&str]) -> Option<String> {
    print!("{}", prompt);
    io::stdout().flush().unwrap();

    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();

    let mut buffer = vec![];

    let rc = unsafe { libc::isatty(libc::STDIN_FILENO) };

    if rc == 0 {
        let mut line = String::new();
        let bytes = stdin.read_line(&mut line).unwrap();
        if bytes == 0 {
            return None;
        } else {
            return Some(line);
        }
    }

    let _raw = match RawMode::new() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("{e}");
            return None;
        }
    };

    let mut bytes = stdin.bytes();

    while let Some(byte) = bytes.next() {
        match byte {
            Ok(byte) => match byte {
                0x20..=0x7e => {
                    let _ = stdout.write_all(&[byte]);
                    let _ = stdout.flush();
                    buffer.push(byte);
                }

                b'\t' => {
                    if let Ok(prefix) = str::from_utf8(&buffer[..]) {
                        if let Some(builtin) = complete_builtin(prefix, builtins) {
                            let suffix_bytes = builtin[prefix.len()..].as_bytes();
                            let _ = stdout.write_all(suffix_bytes);
                            let _ = stdout.write_all(b" ");
                            let _ = stdout.flush();
                            buffer.extend_from_slice(suffix_bytes);
                            buffer.push(b' ');
                        }
                    }
                }
                b'\r' | b'\n' => {
                    let _ = stdout.write_all(b"\n");
                    let _ = stdout.flush();
                    buffer.push(b'\n');
                    break;
                }
                0x7f | 0x08 => {
                    if buffer.pop().is_some() {
                        let _ = stdout.write_all(b"\x08 \x08");
                        let _ = stdout.flush();
                    };
                }
                0x04 => {
                    if buffer.is_empty() {
                        return None;
                    }
                }
                0x1b => {
                    if let Some(byte) = bytes.next() {
                        match byte {
                            Ok(b'[') | Ok(b'O') => {
                                while let Some(byte) = bytes.next() {
                                    match byte {
                                        Ok(0x40..=0x7e) => break,
                                        Err(_) => break,
                                        _ => continue,
                                    }
                                }
                            }
                            Err(_) => return None,
                            _ => {}
                        }
                    }
                }
                _ => {}
            },
            Err(_) => return None,
        }
    }

    if let Ok(line) = String::from_utf8(buffer) {
        Some(line)
    } else {
        None
    }
}
