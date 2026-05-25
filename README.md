# folio-vitae

`folio-vitae` is a Rust CLI that **builds** and **serves locally** two static websites:
- a **main** site (portfolio / homepage)
- a **CV** site

It is intended to be installed from **crates.io** and run inside a website project directory.

---

## Mental model (important)

There are two main modes. Think of them as two separate jobs the CLI can do for you:

### `serve` (development)
Runs two local HTTP servers (main + CV), optionally watches your files, and rebuilds automatically.
This is the command you keep running while you edit templates/content/styles.

### `build` (production)
Generates a fully static `dist/` folder (HTML/CSS/assets).
This is what you deploy (GitHub Pages, Netlify, nginx, S3, …).

---

## Installation

### From crates.io

```bash
cargo install folio-vitae
```

Verify:

```bash
folio-vitae --version
folio-vitae --help
```

### From source (contributors)

```bash
git clone https://github.com/GiorgioReale/folio-vitae.git
cd folio-vitae
cargo build --release
./target/release/folio-vitae --help
```

---

## Project directory layout

A typical project directory looks like this:

```text
.
├── config.yml
├── data/
│   ├── data.it.yaml
│   └── data.en.yaml
├── templates/            # HTML templates (rendered at build time)
├── assets/               # SCSS + other source assets
├── static/               # files copied as-is to the output
├── i18n/                 # translation strings (if used)
├── .env                  # optional (ports/env)
├── .env.example
├── .folio-cache.json     # build cache (auto-generated)
├── .folio-assets-version # runtime assets version marker (auto-generated)
└── dist/                 # output (auto-generated)
```

### Notes about generated files/folders

- `dist/` is **generated** output. Do not edit by hand.
- `.folio-cache.json` is used to **skip builds** when nothing changed.
- `.folio-assets-version` is used to decide whether embedded runtime assets should be refreshed.

---

## Configuration files

### `config.yml`
This is the main configuration file. It is created by `folio-vitae init` and expected to exist in the
project root.

> The exact keys available depend on the current version of `folio-vitae`.
> If you want a complete reference, open `data_example/config.yml` in the repository source.

### `data/`
Content lives in `data/` and is loaded at build time.
The default project created by `init` uses one file per language:

- `data/data.it.yaml`
- `data/data.en.yaml`

---

## Environment variables

`folio-vitae` reads a small set of environment variables.

### `APP_ENV`
Controls development vs production behavior.

- `APP_ENV=development` (default)
- `APP_ENV=production` enables production defaults (for example minification)

Example:

```bash
APP_ENV=production folio-vitae build
```

### `MAIN_PORT`
Port for the main website server (used by `serve`).

- Default: `8080`

### `CV_PORT`
Port for the CV website server (used by `serve`).

- Default: `8081`

### `.env` file loading

Both `serve` and `build` load an env file (default: `.env`) if present.

- If the file is missing, it is not an error (it is simply skipped).
- You can change the file path via CLI.

Example `.env`:

```env
APP_ENV=development
MAIN_PORT=8080
CV_PORT=8081
```

---

## Commands and what each one does (detailed, beginner-friendly)

Below is a clear, beginner-level explanation of each command. Think of these commands as the
**main functions** of the CLI.

### `folio-vitae init`

**Purpose:** Create the starter files your project needs.

**What it does, step by step:**
1. **Creates `config.yml`** – this is the file that tells the program how your site should be built.
2. **Creates sample content** in `data/` – the file(s) that store your text, links, and other data.
3. **Creates `.env.example`** – a template for environment variables (ports, environment).

**Why you need it:**
If you are starting from an empty folder, `init` gives you a working structure so the program
knows where to find your templates, assets, and content.

**Usage:**

```bash
folio-vitae init
```

**Overwrite existing generated files:**

```bash
folio-vitae init --force
```

> `init` does **not** start a server and does **not** build output.

---

### `folio-vitae serve`

**Purpose:** Run local development servers so you can preview your sites in the browser.

**What it does, step by step:**
1. **Starts the main site server** (portfolio/homepage).
2. **Starts the CV site server**.
3. **Loads environment variables** (from `.env` by default).
4. **Optionally watches your files** and rebuilds when something changes (if you use `--watch`).

**Why you need it:**
You use `serve` while you work. It gives you fast feedback so you can edit content or templates
and instantly see the result in your browser.

**Usage (defaults):**
```bash
folio-vitae serve
```

**Common options:**

```bash
# bind to a different host interface
folio-vitae serve --host 0.0.0.0

# override ports (these override MAIN_PORT / CV_PORT)
folio-vitae serve --main-port 3000 --cv-port 3001

# load a custom env file
folio-vitae serve --env-file .env.local
```

#### Watch mode

With `--watch`, `folio-vitae` watches your project and restarts the servers when inputs change.

```bash
folio-vitae serve --watch
```

**Watched targets:**
- `assets/` (recursive)
- `templates/` (recursive)
- `data/` (recursive)
- `static/` (recursive)
- `config.yml`

Stop with `Ctrl+C`.

---

### `folio-vitae build`

**Purpose:** Create the final static website files for deployment.

**What it does, step by step:**
1. **Reads `config.yml`** to understand how to build your site.
2. **Loads `data/`** to fill templates with content.
3. **Renders templates** into HTML pages.
4. **Compiles SCSS** into CSS stylesheets.
5. **Copies `static/` files** directly into the output.
6. **Writes everything into `dist/`**, the folder you upload to your hosting provider.

**Why you need it:**
You run `build` when you are ready to publish the site. It produces pure HTML/CSS/JS files that
any static hosting service can serve.

**Usage:**
```bash
folio-vitae build
```

**Load a custom env file:**

```bash
folio-vitae build --env-file .env.production
```

---

## How the main folders work (beginner-friendly)

This section explains the “functions” of each folder in simple terms so you know where to put
your files and why they are used.

### `templates/`
Holds HTML templates. These are blueprints for your pages. During `build`, the program combines
these templates with your data to produce the final HTML.

### `data/`
Stores your content (text, links, lists, etc.). The data is inserted into the templates so your
pages are filled with real information.

### `assets/`
Contains your source assets, usually SCSS and other files that need to be processed before they
are used in the final site. SCSS is compiled into CSS during `build`.

### `static/`
Contains files that should be copied exactly as they are. Examples: images, PDFs, icons, or any
files that do not need processing.

### `i18n/`
Optional translation strings. If your templates use translations, this folder stores the text for
different languages.

### `dist/`
The final output folder. It is generated by `build` and should be uploaded to your hosting
provider. Do not edit files in `dist/` by hand.

---

## Development workflow (recommended)

```bash
# 1) create and initialize a project
mkdir my-site
cd my-site
folio-vitae init

# 2) start the dev servers with watching
folio-vitae serve --watch

# 3) edit your project:
#    - data/*        (content)
#    - templates/*   (layout)
#    - assets/*      (styles/assets)
#    - static/*      (files copied as-is)
```

---

## CI & quality gates (repository)

The repository CI typically enforces:
- `cargo fmt`
- `cargo clippy`
- `cargo test`
- dependency policy checks (via `cargo-deny`)
- MSRV checks

---

## License

MIT
