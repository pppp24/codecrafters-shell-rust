use std::{
    env,
    io::{self, BufRead, Read, Write},
    mem::MaybeUninit,
};

use crate::path::{complete_command, complete_filename, longest_common_prefix};

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

pub fn read_line(prompt: &str, builtins: &[&str]) -> Option<String> {
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();

    let mut buffer = vec![];

    let rc = unsafe { libc::isatty(libc::STDIN_FILENO) };

    if rc == 0 {
        print!("{}", prompt);
        stdout.flush().unwrap();

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

    let _ = stdout.write_all(prompt.as_bytes());
    let _ = stdout.flush();

    let mut bytes = stdin.bytes();

    let mut last_was_tab = false;

    while let Some(byte) = bytes.next() {
        let prev_was_tab = last_was_tab;
        last_was_tab = false;

        match byte {
            Ok(byte) => match byte {
                0x20..=0x7e => {
                    let _ = stdout.write_all(&[byte]);
                    let _ = stdout.flush();
                    buffer.push(byte);
                }

                b'\t' => {
                    if let Ok(buffer_str) = str::from_utf8(&buffer[..]) {
                        let segment_start = buffer_str.rfind(' ').map_or(0, |i| i + 1);

                        let segment = &buffer_str[segment_start..];

                        let (dir_part, prefix_part) = match segment.rfind('/') {
                            Some(i) => (&segment[..=i], &segment[i + 1..]),
                            None => ("", segment),
                        };

                        let matches: Vec<(String, bool)> = if !dir_part.is_empty() {
                            let cwd = env::current_dir().unwrap_or_default();
                            complete_filename(prefix_part, &cwd.join(dir_part))
                        } else if segment_start == 0 {
                            complete_command(prefix_part, builtins, env::var_os("PATH").as_deref())
                                .into_iter()
                                .map(|m| (m, false))
                                .collect()
                        } else {
                            let cwd = env::current_dir().unwrap_or_default();
                            complete_filename(prefix_part, &cwd)
                        };

                        match matches.len() {
                            0 => {
                                let _ = stdout.write_all(b"\x07");
                                let _ = stdout.flush();
                            }
                            1 => {
                                let (m, is_dir) = &matches[0];
                                let suffix_bytes = m[prefix_part.len()..].as_bytes();
                                let _ = stdout.write_all(suffix_bytes);
                                let trailer = if *is_dir { b"/" } else { b" " };
                                let _ = stdout.write_all(trailer);
                                let _ = stdout.flush();
                                buffer.extend_from_slice(suffix_bytes);
                                buffer.push(trailer[0]);
                            }
                            _ => {
                                let names: Vec<&str> =
                                    matches.iter().map(|(n, _)| n.as_str()).collect();
                                let lcp = longest_common_prefix(&names);
                                if lcp > prefix_part.len() {
                                    let suffix = &names[0][prefix_part.len()..lcp];
                                    let suffix_bytes = suffix.as_bytes();
                                    let _ = stdout.write_all(suffix_bytes);
                                    let _ = stdout.flush();
                                    buffer.extend_from_slice(suffix_bytes);
                                } else if prev_was_tab {
                                    let _ = stdout.write_all(b"\n");
                                    let _ = stdout.write_all(names.join("  ").as_bytes());
                                    let _ = stdout.write_all(b"\n");
                                    let _ = stdout.write_all(prompt.as_bytes());
                                    let _ = stdout.write_all(&buffer);
                                    let _ = stdout.flush();
                                } else {
                                    let _ = stdout.write_all(b"\x07");
                                    let _ = stdout.flush();
                                    last_was_tab = true;
                                }
                            }
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
