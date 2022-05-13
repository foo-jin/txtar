#![doc=include_str!("../README.md")]

use std::{
    borrow::Cow,
    fmt::Display,
    io::{self, Write},
};

const NEWLINE_MARKER: &str = "\n-- ";
const NEWLINE_MARKER_END: &str = " --\n";

pub struct Archive<'a> {
    comment: &'a str,
    files: Vec<File<'a>>,
}

pub struct File<'a> {
    name: &'a str,
    data: &'a str,
}

impl<'a> Archive<'a> {
    /// Serialize the archive as txtar into the IO stream.
    pub fn to_writer<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        write!(writer, "{}", self)
    }
}

impl<'a> From<&'a str> for Archive<'a> {
    fn from(s: &'a str) -> Archive<'a> {
        let (comment, mut name, mut s) = split_file_markers(s);
        let mut files = Vec::new();

        while !name.is_empty() {
            let (data, next_name, rest) = split_file_markers(s);

            let file = File { name, data };
            files.push(file);

            name = next_name;
            s = rest;
        }

        Archive { comment, files }
    }
}

fn split_file_markers(s: &str) -> (&str, &str, &str) {
    let (prefix, rest) = match s.split_once(NEWLINE_MARKER) {
        Some(split) => split,
        None => return (s, "", ""),
    };

    let (name, postfix) = match rest.split_once(NEWLINE_MARKER_END) {
        Some(split) => split,
        None => return (s, "", ""),
    };

    (prefix, name, postfix)
}

impl<'a> Display for Archive<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut comment = Cow::Borrowed(self.comment);
        fix_newline(&mut comment);
        write!(f, "{}", comment)?;

        for File { name, data } in &self.files {
            writeln!(f, "-- {name} --")?;

            let mut data = Cow::Borrowed(*data);
            fix_newline(&mut data);
            write!(f, "{}", data)?;
        }

        Ok(())
    }
}

fn fix_newline(s: &mut Cow<'_, str>) {
    if !s.is_empty() && !s.ends_with('\n') {
        s.to_mut().push('\n');
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
