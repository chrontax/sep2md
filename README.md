# sep2md

Turns [SEP](https://plato.stanford.edu/index.html) pages into markdown for nicer reading and easy conversion into other formats.

## Usage

Build it and run:

```
sep2md <URL> [OUTPUT-PATH]
```

If `OUTPUT-PATH` is not provided, the default path will be "<name of the article>.md" (e.g. "heidegger.md" for <https://plato.stanford.edu/entries/heidegger/>).

## Conversion to EPUB

To convert to EPUB you'll need [pandoc](https://pandoc.org/). Once it's installed, you can run:

```
pandoc something.md -o something.epub
```

If you want an accurate title in metadata and make sure the table of contents isn't truncated, you can run:

```
pandoc something.md --toc-depth=5 --metadata title="Lorem Ipsum" -o something.epub
```
