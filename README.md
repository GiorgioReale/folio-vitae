# folio-vitae

A small Rust CLI that runs two personal sites from YAML:

1. a main website
2. a CV website

Edit your content. Customize templates and styles if you want. That is it.

## Install

```bash
cargo install folio-vitae
```

Or from source:

```bash
git clone https://github.com/GiorgioReale/folio-vitae.git
cd folio-vitae
cargo run -- --help
```

## Quick start

```bash
mkdir my-site
cd my-site
folio-vitae init
folio-vitae dev
```

Then open:

- main site: <http://127.0.0.1:36546>
- CV site: <http://127.0.0.1:36547>

Edit `data/data.en.yaml` (and `data/data.it.yaml` if you use Italian). The servers reload when files change.

## Commands

| Command | What it does |
| --- | --- |
| `folio-vitae init` | Creates starter files in the current folder |
| `folio-vitae init --force` | Same, but overwrites existing starter files |
| `folio-vitae dev` | Local servers with auto-reload |
| `folio-vitae prod` | Same servers, without file watching |

Useful options for `dev` and `prod`:

```bash
folio-vitae dev --main-port 3000 --cv-port 3001
folio-vitae dev --host 0.0.0.0
folio-vitae dev --env-file .env
```

## Project layout

```text
.
├── data/          # your content (YAML, one file per language)
├── templates/     # HTML templates
├── assets/        # styles, scripts, fonts, icons
├── static/        # files served as-is
├── i18n/          # UI translations
└── .env.example   # optional settings
```

Language files are named `data/data.<code>.yaml` (for example `data.en.yaml`, `data.it.yaml`).

## Optional settings

Copy `.env.example` to `.env` if you need custom ports or other options:

```env
MAIN_PORT=36546
CV_PORT=36547
DEFAULT_LANGUAGE=en
```

CLI flags override values from `.env`.

## Contributing

```bash
cargo fmt
cargo clippy
cargo test
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for more.

## License

[MIT](LICENSE)
