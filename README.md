# folio-vitae

`folio-vitae` is a simple Rust CLI to create and run two personal websites:

- a main website
- a CV website

It is made for people who want something easy to start and easy to edit.

## What it does

With `folio-vitae` you can:

- create the starter files for a new project
- run the two websites in development or production
- edit your content in YAML
- customize templates, styles, images, and translations

## Install

From crates.io:

```bash
cargo install folio-vitae
```

Check that it works:

```bash
folio-vitae --help
```

From source:

```bash
git clone https://github.com/GiorgioReale/folio-vitae.git
cd folio-vitae
cargo run -- --help
```

## Quick start

Create a new folder for your site:

```bash
mkdir my-site
cd my-site
folio-vitae init
```

Then start the local websites in development mode:

```bash
folio-vitae dev
```

By default the websites run here:

- main website: `http://127.0.0.1:8080`
- CV website: `http://127.0.0.1:8081`

## Main commands

### `folio-vitae init`

Creates the basic files you need to begin:

- `data/data.it.yaml`
- `data/data.en.yaml`
- `.env.example`

If you want to overwrite existing starter files:

```bash
folio-vitae init --force
```

### `folio-vitae dev`

Starts the two local websites in development mode with automatic restart when project files change.

Useful options:

```bash
folio-vitae dev --main-port 3000 --cv-port 3001
folio-vitae dev --host 0.0.0.0
folio-vitae dev --env-file .env
```

Other available flags:

- `--quiet`
- `--json`

Watched paths:

- `assets/`
- `templates/`
- `data/`
- `static/`
- `i18n/`
- `config.yml`
- `.env`

### `folio-vitae prod`

Starts the two local websites in production mode without file watching.

Useful options:

```bash
folio-vitae prod --main-port 3000 --cv-port 3001
folio-vitae prod --host 0.0.0.0
folio-vitae prod --env-file .env
```

Other available flags:

- `--quiet`
- `--json`

## Project structure

After the first run, a project usually contains:

```text
.
├── data/
│   ├── data.it.yaml
│   └── data.en.yaml
├── templates/
├── assets/
├── static/
├── i18n/
└── .env.example
```

What each folder is for:

- `data/`: your content
- `templates/`: page templates
- `assets/`: styles, scripts, fonts, icons
- `static/`: files copied as they are
- `i18n/`: translations

## Environment variables

You can use a `.env` file if you want custom ports.

Example:

```env
MAIN_PORT=8080
CV_PORT=8081
```

Command line options override environment variables.

## Typical workflow

```bash
folio-vitae init
folio-vitae dev
```

Then edit:

- `data/` for text and content
- `templates/` for HTML structure
- `assets/` for styles and scripts
- `static/` for static files

When you want the production runtime behavior instead of local watch mode:

```bash
folio-vitae prod
```

## For contributors

Run the usual checks:

```bash
cargo fmt
cargo clippy
cargo test
```

## License

MIT
