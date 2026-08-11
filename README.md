# Errand Boy

A desktop shopping assistant. You describe what you want to buy; it searches Google Shopping, then uses an LLM to group the noisy results by actual product, score them, and filter out the junk — so you compare a handful of real options instead of forty listings for the same thing.

## How it works

1. **Search** — queries the [Value SERP](https://www.valueserp.com/) shopping API across several result pages in parallel. Negative keywords you supply are appended to the query as `-term`.
2. **Analyse** — sends the results to your chosen LLM (Anthropic Claude, Google Gemini, or OpenAI ChatGPT), which groups listings by brand + model and scores each group 0–100 against what you asked for.
3. **Review** — results render as cards. Anything below your minimum score is auto-filtered into a tray, and you can dismiss cards you're not interested in.

## Platform support

> [!IMPORTANT]
> **Linux and Windows only. macOS is not supported and there are no plans to add it.**
>
> Testing has been done on **Debian 12** and **Windows 10**. Other distributions and Windows versions may work but are untested — if a build or launch fails there, that's expected territory, not a regression.

## Download

Prebuilt binaries for both platforms are on the [Releases page](https://github.com/PSarapata/errand_boy/releases). Download the archive matching your OS and unpack it anywhere:

| Platform | Asset | Contains |
| --- | --- | --- |
| Debian / Ubuntu Linux | `errand_boy-vX.Y.Z-linux_debian.zip` | `linux_debian/errand_boy` + `assets/` |
| Windows 10 | `errand_boy-vX.Y.Z-windows10.zip` | `windows10/errand_boy.exe` + `assets/` |

The "Source code (zip)" and "(tar.gz)" entries GitHub generates on each release contain source only — the runnable builds are the two platform assets above.

> [!WARNING]
> **Keep the executable and its `assets/` folder together.** The app loads its stylesheet and images from `assets/` next to the binary. Moving the executable out on its own gives you an unstyled window with no images. Run it from inside the unpacked folder.

### Runtime requirements

You do **not** need Rust or the Dioxus CLI to run a release build — those are build-time tools only.

| Platform | Needs |
| --- | --- |
| Linux | GTK 3, WebKitGTK 4.1, libsoup 3, OpenSSL 3 shared libraries. Present by default on most desktop installs; on Debian 12 install with `sudo apt install libgtk-3-0 libwebkit2gtk-4.1-0` if missing. |
| Windows | Microsoft Edge WebView2 Runtime. Ships with Windows 11 and any Windows 10 with a current Edge. Otherwise install the [Evergreen Bootstrapper](https://developer.microsoft.com/en-us/microsoft-edge/webview2/). |

## First run

The app needs two API keys, entered in the setup wizard on first launch:

- **Value SERP** — for the shopping search ([get a key](https://www.valueserp.com/))
- **One LLM provider** — Anthropic, Google, or OpenAI, depending on which you pick

Both are paid third-party services billed to your own accounts. Errand Boy sends your search query and the search results to whichever LLM you configure; it has no backend of its own and collects nothing.

Settings are stored locally and can be changed later via the cog icon:

| Platform | Config file |
| --- | --- |
| Linux | `~/.config/errand_boy/config.toml` |
| Windows | `%APPDATA%\errand_boy\config.toml` |

There is a 60-second cooldown between searches to keep API costs predictable.

## Building from source

Requires Rust ≥ 1.85 (edition 2024) and the [Dioxus CLI](https://dioxuslabs.com/).

```sh
cargo install dioxus-cli
dx build --release
```

> [!NOTE]
> Build with `dx`, not `cargo`. Plain `cargo build` compiles the binary but does not bundle the `asset!()` files, so the result launches without styling or images. `dx` prints the finished bundle location as `path=…` on its final line — that directory holds the executable plus its `assets/`.

Additional build-time system packages:

- **Debian/Ubuntu** — `sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev libssl-dev pkg-config build-essential`
- **Windows** — the MSVC toolchain (`rustup default stable-x86_64-pc-windows-msvc`) plus Visual Studio Build Tools with the "Desktop development with C++" workload. MinGW is not supported.

Cross-compiling between Linux and Windows is not supported — build each platform on that platform.

## License

[MIT](LICENSE) © 2026 Pawel Sarapata
