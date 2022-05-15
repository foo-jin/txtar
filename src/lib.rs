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
    let (prefix, rest) = if s.starts_with(MARKER) {
        ("", s)
    } else {
        match s.find(NEWLINE_MARKER) {
            None => return (s, "", ""),
            Some(offset) => s.split_at(offset + 1),
        }
    };
    debug_assert!(rest.starts_with(MARKER));

    let (name, suffix) = match rest.split_once('\n') {
        None if rest.ends_with(MARKER_END) => (rest, ""),
        None => return (s, "", ""),
        Some((n, pf)) => (n, pf),
    };

    let name = name.trim_end_matches('\r');
    debug_assert!(name.ends_with(MARKER_END));

    let name = name
        .strip_prefix(MARKER)
        .and_then(|name| name.strip_suffix(MARKER_END))
        .unwrap();
    (prefix, name, suffix)
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
    use super::*;
    use assert_fs::{prelude::*, TempDir};
    use predicates::prelude::{predicate::str::contains, *};
    use similar_asserts::{assert_eq, assert_str_eq};

    const BASIC: &str = "\
comment1
comment2
-- file1 --
File 1 text.
-- foo --
File 2 text.
-- empty --
-- noNL --
hello world";

    #[test]
    fn parse_format() {
        // Test simplest
        {
            let simplest = "-- simplest.txt --";
            let expected = format!("{simplest}\n");
            check_parse_format("simplest", Archive::from(simplest), &expected);
        }

        // Test basic variety of inputs
        {
            let basic = BASIC;
            let expected = format!("{basic}\n");
            check_parse_format("basic", Archive::from(basic), &expected);
        }

        // Test CRLF input
        {
            let crlf = "blah\r\n-- hello --\r\nhello\r\n";
            let expected = Archive {
                comment: Cow::Borrowed(b"blah\r\n"),
                files: vec![File {
                    name: b"hello",
                    data: Cow::Borrowed(b"hello\r\n"),
                }],
            };

            let arch = Archive::from(crlf);
            assert_eq!(arch, expected, "parse[CRLF input]",);
        }
    }

    fn check_parse_format(name: &str, arch: Archive, expected: &str) {
        let txtar = arch.to_string();
        assert_str_eq!(txtar, expected, "parse[{name}]");
    }

    #[test]
    fn materialize_basic() {
        let dir = TempDir::new().unwrap();
        let exists = predicate::path::exists();
        let empty = predicate::str::is_empty().from_utf8().from_file_path();
        {
            let good = dbg!(Archive::from("-- good.txt --"));
            good.materialize(&dir)
                .expect("good.materialize should not error");
            dir.child("good.txt").assert(exists).assert(empty);
        }
        {
            let basic = Archive::from(BASIC);
            basic
                .materialize(&dir)
                .expect("basic.materialize should not error");

            check_contents(&dir, "file1", "File 1 text.");
            check_contents(&dir, "foo", "File 2 text.");
            check_contents(&dir, "noNL", "hello world");
            dir.child("empty").assert(exists).assert(empty);
        }
        {
            let bad_rel = Archive::from("-- ../bad.txt --");
            check_bad_materialize(&dir, bad_rel, "../bad.txt");

            let bad_abs = Archive::from("-- /bad.txt --");
            check_bad_materialize(&dir, bad_abs, "/bad.txt");
        }
    }

    #[test]
    fn materialize_nested() {
        let dir = TempDir::new().unwrap();

        {
            let nested = Archive::from(
                "comment\n\
			 -- foo/foo.txt --\nThis is foo.\n\
			 -- bar/bar.txt --\nThis is bar.\n\
			 -- bar/deep/deeper/abyss.txt --\nThis is in the DEEPS.",
            );
            nested
                .materialize(&dir)
                .expect("nested.materialize should not error");

            check_contents(&dir, "foo/foo.txt", "This is foo.");
            check_contents(&dir, "bar/bar.txt", "This is bar.");
            check_contents(&dir, "bar/deep/deeper/abyss.txt", "This is in the DEEPS.");
        }
        {
            let bad_nested_rel = Archive::from("-- bar/deep/deeper/../../../../escaped.txt --");
            check_bad_materialize(&dir, bad_nested_rel, "../escaped.txt");
        }
    }

    fn check_contents(dir: &TempDir, child: &str, contents: &str) {
        let exists = predicate::path::exists();
        let newline_ending = predicate::str::ends_with("\n").from_utf8().from_file_path();
        dir.child(child)
            .assert(exists)
            .assert(contains(contents))
            .assert(newline_ending);
    }

    fn check_bad_materialize(dir: &TempDir, bad_rel: Archive, expected: &str) {
        let err = bad_rel.materialize(dir);
        match err {
            Err(MaterializeError::DirEscape(p)) => assert_eq!(p, expected.to_string()),
            Err(e) => panic!("expected `MaterializeError::DirEscape`, got {:?}", e),
            Ok(_) => panic!(
                "materialize({}) outside of parent dir should have failed",
                expected
            ),
        }
    }
}
