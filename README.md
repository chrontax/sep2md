# sep2md

Turns [Stanford Encyclopedia of Philosophy](https://plato.stanford.edu/index.html) pages into markdown for more comfortable reading and easy conversion into other formats.

If you find the articles useful, consider [supporting the SEP](https://plato.stanford.edu/fundraising/).

## Usage

Build and run it from the repo:

```sh
cargo run --release -- <URL> [OUTPUT-PATH]
```

or install from crates.io and run:

```sh
cargo install sep2md
sep2md <URL> [OUTPUT-PATH]
```

If `OUTPUT-PATH` is not provided, the default path will be "*name of the article*.md" (e.g. "heidegger.md" for <https://plato.stanford.edu/entries/heidegger/>).

## Conversion to EPUB

To convert to EPUB you'll need [pandoc](https://pandoc.org/). Once it's installed, you can run:

```sh
pandoc something.md --toc --toc-depth=5 --shift-heading-level-by=-1 -o something.epub
```
