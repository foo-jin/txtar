# txtar
A Rust implementation of the [txtar](https://github.com/golang/tools/tree/master/txtar) Go package.

```
cargo add txtar
```

## Example
```rust no_run
let txt = "\
comment1
comment2
-- file1 --
File 1 text.
-- foo/bar --
File 2 text.
-- empty --
-- noNL --
hello world";

let archive = txtar::from_str(txt);
archive.materialize("/tmp/somedir/").unwrap();
```

## Txtar goals
As described in the Go package:

> Package txtar implements a trivial text-based file archive format.
>
> The goals for the format are:
>
>	- be trivial enough to create and edit by hand.
>	- be able to store trees of text files describing go command test cases.
>	- diff nicely in git history and code reviews.
>
> Non-goals include being a completely general archive format,
> storing binary data, storing file modes, storing special files like
> symbolic links, and so on.


## Txtar format spec
The format spec as written in the `txtar` Go package source code:

> Txtar format
>
> A txtar archive is zero or more comment lines and then a sequence of file entries.
> Each file entry begins with a file marker line of the form "-- FILENAME --"
> and is followed by zero or more file content lines making up the file data.
> The comment or file content ends at the next file marker line.
> The file marker line must begin with the three-byte sequence "-- "
> and end with the three-byte sequence " --", but the enclosed
> file name can be surrounding by additional white space,
> all of which is stripped.
>
> If the txtar file is missing a trailing newline on the final line,
> parsers should consider a final newline to be present anyway.
>
> There are no possible syntax errors in a txtar archive.
