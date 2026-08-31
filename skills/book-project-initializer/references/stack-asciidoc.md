# Stack editorial: AsciiDoc + Asciidoctor

Plantillas por defecto para `book-project-initializer`.

## book.adoc (master)

```asciidoc
= {book-title}
{author}
:v{book-version}:
:doctype: book
:toc: left
:toclevels: 3
:sectlinks:
:icons: font
:source-highlighter: rouge
:lang: es

= Parte I: Fundamentos

include::chapters/ch01-introduccion.adoc[]
```

## _chapter-template.adoc

```asciidoc
[id="ch{n}-{slug}"]
== {n}. {Title}

[abstract]
Resumen de una línea.

=== Objetivos de aprendizaje

* ...

=== Sección

Contenido. Incluir ejemplos desde el proyecto real:

include::../../examples/ch{n}-{slug}/src/main.adoc[tag=main]
```

## justfile

```makefile
default:
    @just --list

build-html:
    asciidoctor -D build/html src/book.adoc

build-pdf:
    asciidoctor-pdf -D build/pdf src/book.adoc

build-epub:
    asciidoctor-epub3 -D build/epub src/book.adoc

build: build-html build-pdf build-epub

test:
    # Probar ejemplos ejecutables (rust)
    find examples -maxdepth 2 -name Cargo.toml -execdir cargo test \;

lint:
    vale src/ || true

clean:
    rm -rf build
```

## ci.yml (GitHub Actions, esqueleto)

```yaml
name: book-ci
on: [push, pull_request]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Setup Ruby
        uses: ruby/setup-ruby@v1
        with:
          ruby-version: '3.3'
      - name: Install Asciidoctor
        run: gem install asciidoctor asciidoctor-pdf asciidoctor-epub3 rouge
      - name: Build
        run: just build
      - name: Test examples
        run: just test
```
