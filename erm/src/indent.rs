use std::fmt::Write;

pub struct IndentedWriter<'a> {
    newlined: bool,
    writer: &'a mut dyn Write,
    indent: &'static str,
}

pub trait Indentable<'a> {
    fn indent(self, indentation: &'static str) -> IndentedWriter<'a>;
}

impl<'a, T: Write + 'a> Indentable<'a> for &'a mut T {
    fn indent(self, indentation: &'static str) -> IndentedWriter<'a> {
        IndentedWriter {
            newlined: true,
            writer: self as &mut dyn Write,
            indent: indentation,
        }
    }
}

impl<'a> Write for IndentedWriter<'a> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        for new_line in s.split('\n') {
            if self.newlined {
                self.writer.write_str(&self.indent)?;
            }

            self.writer.write_str(new_line)?;
            self.newlined = true;
        }
        self.newlined = s.ends_with('\n');

        Ok(())
    }
}
