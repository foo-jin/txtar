#![doc=include_str!("../README.md")]

use std::{
    borrow::Cow,
    fmt::Display,
    fs,
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
};

mod error;

const NEWLINE_MARKER: &str = "\n-- ";
const MARKER: &str = "-- ";
const MARKER_END: &str = " --";

pub use error::MaterializeError;

#[derive(Debug, Eq, PartialEq)]
pub struct Archive<'a> {
    // internal invariant:
    // comment is fix_newlined
    comment: Cow<'a, [u8]>,
    files: Vec<File<'a>>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct File<'a> {
    name: &'a [u8],
    // internal invariant:
    // data is fix_newlined
    data: Cow<'a, [u8]>,
}

impl<'a> File<'a> {
    pub fn new(name: &'a str, data: &'a str) -> File<'a> {
        let mut data = Cow::Borrowed(data.as_bytes());
        fix_newline(&mut data);

        File {
            name: name.as_bytes(),
            data,
        }
    }
}

impl<'a> Archive<'a> {
    fn new(comment: &'a str, files: Vec<File<'a>>) -> Archive<'a> {
        let mut comment = Cow::Borrowed(comment.as_bytes());
        fix_newline(&mut comment);

        Archive { comment, files }
    }

    /// Serialize the archive as txtar into the I/O stream.
    pub fn to_writer<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.comment)?;
        for File { name, data } in &self.files {
            writer.write_all(name)?;
            writer.write_all(data)?;
        }
        Ok(())
    }

    /// Writes each file in this archive to the directory at the given
    /// path.
    ///
    /// # Errors
    ///
    /// This function will error in the event a file would be written
    /// outside of the directory or if an existing file would be
    /// overwritten. Additionally, any errors caused by the underlying
    /// I/O operations will be propagated.
    pub fn materialize<P: AsRef<Path>>(&self, path: P) -> Result<(), MaterializeError> {
        let path = path.as_ref();
        for File { name, data } in &self.files {
            // this is disgusting, TODO
            let name_path = PathBuf::from(path_clean::clean(&String::from_utf8_lossy(name)));
            if name_path.starts_with("../") || name_path.is_absolute() {
                return Err(MaterializeError::DirEscape(
                    name_path.to_string_lossy().to_string(),
                ));
            }

            let rel_path = name_path;
            let path = path.join(rel_path);
            if let Some(p) = path.parent() {
                fs::create_dir_all(p)?;
            }

            let mut file = fs::File::options()
                .write(true)
                .create_new(true)
                .open(path)?;
            let mut w = BufWriter::new(&mut file);
            w.write_all(data)?;
        }

        Ok(())
    }
}

impl<'a> From<&'a str> for Archive<'a> {
    fn from(s: &'a str) -> Archive<'a> {
        let (comment, mut name, mut s) = split_file_markers(s);
        let mut files = Vec::new();

        while !name.is_empty() {
            let (data, next_name, rest) = split_file_markers(s);

            let file = File::new(name, data);
            files.push(file);

            name = next_name;
            s = rest;
        }

        Archive::new(comment, files)
    }
}

fn split_file_markers(s: &str) -> (&str, &str, &str) {
    let (prefix, rest) = match s.find(NEWLINE_MARKER) {
        None => return (s, "", ""),
        Some(offset) => s.split_at(offset + 1),
    };
    debug_assert!(rest.starts_with(MARKER));

    let (name, postfix) = match rest.split_once('\n') {
        None => return (s, "", ""),
        Some((n, pf)) => (n, pf),
    };

    let name = name.trim_end_matches('\r');
    debug_assert!(name.ends_with(MARKER_END));

    let name = name
        .strip_prefix(MARKER)
        .and_then(|name| name.strip_suffix(MARKER_END))
        .unwrap();
    (prefix, name, postfix)
}

impl<'a> Display for Archive<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let comment = String::from_utf8_lossy(&self.comment);
        write!(f, "{comment}")?;

        for File { name, data } in &self.files {
            let name = String::from_utf8_lossy(name);
            writeln!(f, "-- {name} --")?;
            let data = String::from_utf8_lossy(data);
            write!(f, "{data}")?;
        }

        Ok(())
    }
}

fn fix_newline(s: &mut Cow<'_, [u8]>) {
    if !s.is_empty() && !s.ends_with(&[b'\n']) {
        s.to_mut().push(b'\n');
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
