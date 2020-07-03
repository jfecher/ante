use difference::{ Changeset, Difference };
use colored::Colorize;
use std::fmt::{ Formatter, Display, Error };

pub struct DiffPrinter(pub Changeset);

fn fmt_lines<F, D: Display>(lines: &str, mut current_line: Option<&mut usize>, f: &mut Formatter, colorizer: F) -> Result<(), Error>
    where F: Fn(&str) -> D,
          D: Display
{
    for line in lines.lines() {
        if let Some(line) = &current_line {
            write!(f, "{:3}| ", line)?;
        } else {
            write!(f, "{:3}| ", " ")?;
        }

        writeln!(f, "{}", colorizer(line))?;
        current_line.as_deref_mut().map(|x| *x += 1);
    }
    Ok(())
}

impl Display for DiffPrinter {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let mut line = 1;
        for i in 0 .. self.0.diffs.len() {
            match &self.0.diffs[i] {
                Difference::Same(lines) => {
                    fmt_lines(lines, Some(&mut line), f, |x| x.normal())?;
                },
                Difference::Add(lines) => {
                    // Don't show/increment the line number if the previous change was a Removal
                    if i > 0 && matches!(self.0.diffs[i - 1], Difference::Rem(..)) {
                        fmt_lines(lines, None, f, |x| x.green())?;
                    } else {
                        fmt_lines(lines, Some(&mut line), f, |x| x.green())?;
                    }
                },
                Difference::Rem(lines) => {
                    // Don't show/increment the line number unless the next change is an Addition
                    if i < self.0.diffs.len() - 1 && matches!(self.0.diffs[i + 1], Difference::Add(..)) {
                        fmt_lines(lines, Some(&mut line), f, |x| x.red())?;
                    } else {
                        fmt_lines(lines, None, f, |x| x.red())?;
                    }
                },
            }
        }
        Ok(())
    }
}
