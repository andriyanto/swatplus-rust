//! `septic.str`, read as `sep_read` reads it: each system is one list-directed read of 28 items
//! (a name, three integers, two reals, an integer and 21 reals) that goes on into the next lines
//! when a line has fewer, so a short line takes the first tokens of the next system's line: its
//! name, read as a number, ends that read with an error (the items not read keep 0), and the
//! next read starts on the line after. A read that reaches the end of the file stops the
//! reading. (The editor writes all 28 values, under a header that names 22.)

use swat_model::SepticSys;

/// What a value read: a number, or an error (the token isn't one of that kind).
fn int(tok: &str) -> Option<i32> {
    tok.parse::<i32>().ok()
}

fn real(tok: &str) -> Option<f32> {
    let t = tok.replace(['d', 'D'], "e");
    t.parse::<f32>().ok().filter(|_| !tok.chars().any(|c| c.is_ascii_alphabetic() && !"eEdD".contains(c)))
}

/// Reads `septic.str`. Element 0 is the empty system SWAT+ points HRUs without one at.
pub fn read_septic(text: &str) -> Vec<SepticSys> {
    let lines: Vec<&str> = text.split('\n').map(|l| l.trim_end_matches('\r')).collect();
    // The title and header, then as many systems as lines are left (a list-directed read of a
    // character variable reads a blank line too, but not the end of the file).
    let body = if lines.len() > 2 { &lines[2..] } else { &[][..] };
    let nlines = if text.ends_with('\n') { body.len().saturating_sub(1) } else { body.len() };
    let mut out = vec![SepticSys::default()];
    let mut line = 0usize;
    for _ in 0..nlines {
        let mut s = SepticSys::default();
        let mut item = 0usize;
        let mut ended = false;
        // Items are taken from this line on; the read ends after its last item or an error.
        loop {
            if line >= nlines {
                ended = true;
                break;
            }
            let toks: Vec<&str> = body[line].split([' ', ',', '\t']).filter(|t| !t.is_empty()).collect();
            line += 1;
            let mut err = false;
            for tok in toks {
                if item >= 28 {
                    break;
                }
                match item {
                    0 => s.name = tok.to_string(),
                    1 | 2 | 3 | 6 => match int(tok) {
                        Some(v) => match item {
                            1 => s.typ = v,
                            2 => s.yr = v,
                            3 => s.opt = v,
                            _ => s.tfail = v,
                        },
                        None => err = true,
                    },
                    _ => {
                        let k = if item < 6 { item - 4 } else { item - 5 };
                        match real(tok) {
                            Some(v) => s.r[k] = v,
                            None => err = true,
                        }
                    }
                }
                if err {
                    break;
                }
                item += 1;
            }
            if err || item >= 28 {
                break;
            }
        }
        if ended && item < 28 {
            // The end of the file: SWAT+ leaves the loop (what was read stays).
            out.push(s);
            break;
        }
        out.push(s);
    }
    // Systems after the end of the file keep their defaults.
    while out.len() < nlines + 1 {
        out.push(SepticSys::default());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_rows_take_the_next_line() {
        let text = "septic.str: t\nname typ yr operation ...\n\
            standard 1 0 1 2.5 100.0 70 500.0 50.0 0.5 1.5 1000.0 0.5 0.32 30.0 0.8 1.3 0.1 0.5 0.16 0.3 0.5\n\
            failing 1 0 0 2.5 100.0 1 500.0 50.0 0.5 1.5 1000.0 0.5 0.32 30.0 0.8 1.3 0.1 0.5 0.16 0.3 0.5\n";
        let s = read_septic(text);
        assert_eq!(s.len(), 3);
        assert_eq!((s[1].name.as_str(), s[1].typ, s[1].opt, s[1].tfail), ("standard", 1, 1, 70));
        assert_eq!(s[1].r[SepticSys::Z], 500.0);
        assert_eq!(s[1].r[SepticSys::SLG2], 0.5);
        assert_eq!(s[1].r[SepticSys::NITR], 0.0);
        // The second system's line was read by the first: the second read meets the end.
        assert_eq!(s[2], SepticSys::default());
    }
}
