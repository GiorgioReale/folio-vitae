# folio-vitae

A small Rust CLI that runs two personal sites from YAML:

1. a main website
2. a CV website

Edit your content. Customize templates and styles if you want. That is it.

## Live examples

Sites built with folio-vitae:

- [giorgioreale.xyz](https://giorgioreale.xyz/en) — main site
- [cv.giorgioreale.xyz](https://cv.giorgioreale.xyz/en) — CV site

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

Edit `data/data.en.yaml` (and other `data/data.<code>.yaml` files you keep). The servers reload when files change.

`folio-vitae` without a subcommand prints help; use `init`, `dev`, or `prod` explicitly.

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

## Docker / GHCR

Alpine-based images are published to:

```text
ghcr.io/giorgioreale/folio-vitae
```

### Run your own instance

You only need Docker. No local Rust or `folio-vitae` install. Templates, assets, and i18n are already in the image; you only provide content under `data/`.

```bash
mkdir folio-vitae && cd folio-vitae

curl -fsSL -o compose.yaml \
  https://raw.githubusercontent.com/GiorgioReale/folio-vitae/main/compose.yaml

# Create starter data/ + .env.example using the image itself
docker run --rm \
  -v "$PWD:/work" \
  -w /work \
  --user "$(id -u):$(id -g)" \
  ghcr.io/giorgioreale/folio-vitae:latest \
  init

# Optional settings (ports, language, SMTP, …)
cp .env.example .env

# Edit your content, then start
# $EDITOR data/data.en.yaml
docker compose up -d
```

Open:

- main site: <http://127.0.0.1:36546>
- CV site: <http://127.0.0.1:36547>

Edit files in `./data` anytime. Restart after changes if the process does not pick them up:

```bash
docker compose restart
```

Equivalent `docker run` (after `init` has created `./data`):

```bash
docker run --rm \
  --name folio-vitae \
  -p 36546:36546 \
  -p 36547:36547 \
  -v "$PWD/data:/app/data:ro" \
  ghcr.io/giorgioreale/folio-vitae:latest
```

Add `--env-file .env` when you use a local `.env`.

Notes:

- The image runs `folio-vitae prod --host 0.0.0.0` and reads content from `/app/data`.
- Put source images under `data/images/` when your YAML references them.
- Pin a version in Compose (`image: ...:<version>`) if you do not want `latest` to move.
- Compose publishes both ports on `127.0.0.1` only so a reverse proxy (Caddy, Traefik, nginx) on the same host can reach them without exposing the app directly. For a public deploy, put that proxy in front and point your domains at it.
- Compose runs the container with a read-only root filesystem; a tmpfs mount on `/app` holds runtime asset materialization while `./data` stays read-only.
- SMTP variables in `.env` enable the CV contact form (built into the default binary).
- Upgrading the crate fills in missing `assets/`, `templates/`, `static/`, and `i18n/` files but does not overwrite local edits.

### Image tags

- version tags (`x.y.z`, `x.y`) and `latest` after a successful crates.io publish on each GitHub Release
- manual rebuild via Actions → Docker → Run workflow (refresh `latest` / version tags without a new app release)

### Build locally

```bash
docker build -t folio-vitae .
# or: docker compose up -d --build  (after enabling `build: .` in compose.yaml)
```

## Contributing

```bash
cargo fmt
cargo clippy
cargo test
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for more.

## License

[MIT](LICENSE)
