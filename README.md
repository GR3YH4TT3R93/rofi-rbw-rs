```markdown
# rofi-rbw-rs

A fast, native frontend for [rbw](https://github.com/doy/rbw) — the unofficial
Bitwarden CLI — built in Rust. It presents your vault through rofi (or wofi,
fuzzel, bemenu), lets you pick an entry, and then types it, copies it, or
prints it, depending on what you ask for.

It is a faithful port of [rofi-rbw](https://github.com/fdw/rofi-rbw) with the
same CLI surface and config format, packaged as a single static binary with no
runtime dependencies beyond the helper tools you already have installed.

## Why use it

- **Instant startup.** No Python interpreter, no imports, no venv. Launching
  from a keyboard shortcut feels immediate.
- **Single binary.** Install one file. No `pip`, no `configargparse`, no
  broken Python environment when your distro upgrades.
- **Multi-call pinentry.** The same binary doubles as an `rbw` pinentry
  server that stores your master password in the system keyring, so
  `rbw unlock` can be silent across reboots.
- **Tested.** Every non-trivial behavior — target parsing, URI indices,
  card expiry composition, cache scoring, config-file syntax, pinentry
  dispatch — is covered by unit tests.
- **Compatible.** Same CLI flags, same `rofi-rbw.rc` config file format, same
  keybinding syntax as upstream. Drop-in replacement.

## Features

- Four selector backends: **rofi**, **wofi**, **fuzzel**, **bemenu**.
- Five typer backends: **wtype**, **ydotool**, **dotool**, **xdotool**, and a
  no-op fallback that errors clearly.
- Three clipboard backends: **wl-copy**, **xclip**, **xsel**, plus a no-op.
- Supports **Login**, **Card**, and **Note** entries.
- Handles **custom fields**, **URIs** (including multiple), **TOTP**,
  **notes**, and the `_autotype` override field.
- Configurable **display fields** (name, folder, user, first URI) with
  automatic column alignment.
- Custom **keybindings** for both the entry list and the target menu.
- **MRU cache** so your most-used entries rise to the top.
- **Keyring-backed pinentry** so `rbw` doesn't re-prompt after a reboot.
- Reads config from `~/.config/rofi-rbw.rc` or
  `~/.config/rofi-rbw/config.rc`.

## Requirements

Runtime:

- `rbw` — the Bitwarden CLI, available in `extra/rbw` on Arch.
- At least one selector: `rofi`, `wofi`, `fuzzel`, or `bemenu`.
- At least one typer if you intend to type: `wtype` (Wayland),
  `ydotool` (Wayland/X11), `dotool` (AUR), or `xdotool` (X11).
- At least one clipboarder if you intend to copy: `wl-clipboard` (Wayland),
  `xclip` or `xsel` (X11).
- `libsecret` and a running Secret Service daemon (`ksecretd`, `gnome-keyring`,
  or `oo7-daemon`) for keyring-backed pinentry.
- `pinentry` for the initial interactive prompt.

Build:

- Rust (stable) and Cargo.
- `git`, `base-devel`, and `makepkg` if you're using the PKGBUILD.

## Installation

### Arch Linux (PKGBUILD)

Clone the PKGBUILD repository and build:

```bash
git clone https://git.ourserver.fun/GR3YH4TT3R93/rofi-rbw-rs.git
cd rofi-rbw-rs
makepkg -si
```

The package installs /usr/bin/rofi-rbw-rs and its license. It does not ship
any symlinks or wrappers.

From source

```bash
git clone https://git.ourserver.fun/GR3YH4TT3R93/rofi-rbw-rs.git
cd rofi-rbw-rs
cargo build --release
sudo install -Dm755 target/release/rofi-rbw-rs /usr/local/bin/rofi-rbw-rs
```

Verify

```bash
rofi-rbw-rs --help
```

You should see the full CLI. If you have rbw configured with at least one
login entry:

```bash
rofi-rbw-rs
```

Rofi should open with your vault entries. Escape closes without doing
anything.

Setup

1. Configure rbw

If you haven't already, follow the rbw
setup to log in and sync your vault:

```bash
rbw config set email you@example.com
rbw login
rbw sync
rbw list
```

1. Wire up silent unlock (recommended)

rbw cannot pass arguments to its configured pinentry, and the default
pinentry prompts every unlock. rofi-rbw-rs doubles as a pinentry server
that stores your master password in the system keyring, so subsequent
unlocks are silent.

Point rbw at the binary:

```bash
rbw config set pinentry /usr/bin/rofi-rbw-rs
```

Verify:

```bash
rbw config show | grep pinentry
```

You should see:

```json
"pinentry": "/usr/bin/rofi-rbw-rs",
```

The binary detects that it was invoked by rbw (parent process name plus a
piped stdin) and switches into pinentry mode automatically. No arguments, no
wrapper script, no second binary.

Now lock and unlock once to seed the keyring:

```bash
rbw lock
rbw unlock
```

You will be prompted for your master password one time. After that, the
password is stored in the keyring and future unlocks are silent. Verify:

```bash
secret-tool lookup application rbw profile rbw type master_password
```

This should return your master password. Then:

```bash
rbw lock && rbw unlock
```

Should return immediately with no prompt.

To clear the stored password later:

```bash
rofi-rbw-rs --pinentry-clear
```

For multiple rbw profiles, set RBW_PROFILE in the environment before
running any rbw command; each profile gets its own keyring entry.

1. Bind to a keyboard shortcut

The intended usage is a global hotkey. In Sway:

```
bindsym $mod+p exec rofi-rbw-rs
```

In i3:

```
bindsym $mod+p exec --no-startup-id rofi-rbw-rs
```

In Hyprland:

```
bind = $mainMod, P, exec, rofi-rbw-rs
```

Focus the target application (browser login form, SSH prompt, etc.), press
the hotkey, pick an entry from rofi, and the credentials are typed or copied
automatically.

Usage

CLI

```
rofi-rbw-rs [OPTIONS]
```

Common invocations:

```bash
# Default: pick an entry, type username + Tab + password
rofi-rbw-rs

# Copy the password to the clipboard instead
rofi-rbw-rs -a copy -t password

# Print notes to stdout (no clipboard, no typing)
rofi-rbw-rs -a print -t notes

# Type username, Tab, password, then press Enter
rofi-rbw-rs -a type -t username -t tab -t password -t enter

# Force a specific selector
rofi-rbw-rs --selector fuzzel

# Clear the password from the clipboard 30 seconds after copying
rofi-rbw-rs -a copy -t password --clear-after 30

# Open the target menu, then act on the selected field
rofi-rbw-rs -t menu
```

Full flag list:

Flag Description
-a, --action <type\|copy\|print\|sync> What to do with the selected entry. Default: type.
-t, --target <TARGET> Which part of the entry to use. Repeatable.
-r, --prompt <TEXT> Selector prompt. Default: Choose entry.
--selector <rofi\|wofi\|fuzzel\|bemenu> Force a specific selector. Default: autodetect.
--clipboarder <xsel\|xclip\|wl-copy> Force a specific clipboard backend.
--typer <xdotool\|wtype\|ydotool\|dotool> Force a specific typer backend.
--selector-args <ARGS> Extra arguments to pass to the selector.
--display-fields <F1> <F2> ... Ordered list of fields to show.
--clear-after <SECONDS> Auto-clear the clipboard after N seconds. 0 disables.
--no-help Don't show the keybinding help line in the selector.
--no-cache Don't use the MRU cache.
--use-notify-send Send a desktop notification after copying TOTP.
--keybindings <SPEC> Custom keybindings for the entry list.
--menu-keybindings <SPEC> Custom keybindings for the target menu.
--typing-key-delay <MS> Delay between keystrokes when typing.
--typing-start-delay <MS> Delay before typing starts.
--action-sequence-delay <MS> Delay for the delay target.

Display fields

The --display-fields option accepts, in order:

· name_only — entry name (e.g. github.com)
· name_with_folder — folder-prefixed name (e.g. work/github.com)
· folder — folder only
· user — username
· uri — first URI

Columns are right-padded with spaces to align. For example,
--display-fields name_with_folder user:

```
work/github.com    alice
work/forgejo       alice
personal/mastodon  alice@example.com
```

Targets

Valid targets for -t and keybindings:

· username, password, totp — login credentials
· notes — entry notes (newlines replaced with | in the target menu)
· uri, uri 2, uri 3, … — indexed URIs
· number, cardholder, brand, expiry, code — card fields
· menu — open the target-selection menu instead of using a fixed target
· enter, tab, delay — special sequence steps for the type action

Keybindings

The default keybindings, pressed inside the rofi entry list:

Shortcut Action
Alt+1 Type username, Tab, password
Alt+2 Type username
Alt+3 Type password
Alt+4 Type TOTP
Alt+c Copy password
Alt+u Copy username
Alt+t Copy TOTP
Alt+m Open the target menu
Alt+s Sync the vault

Customize with --keybindings or the config file. The syntax is
shortcut:action:target1:target2:..., comma-separated:

```
keybindings = Alt+1:type:username:tab:password,Alt+c:copy:password
```

In the target menu, the default keybindings are Alt+t for type and Alt+c
for copy.

Config file

Options can be set in ~/.config/rofi-rbw.rc (or
~/.config/rofi-rbw/config.rc), and are overridden by CLI flags. The format
accepts key = value, key: value, or key value, one per line. # and
; start comments; bare keys are treated as booleans.

Example:

```ini
# Force rofi even when wofi is available
selector = rofi

# Default to type action, username then password
action = type
target = username
target = tab
target = password
target = enter

# Show the URL and username in the list
display-fields = uri user

# Clear passwords from the clipboard after 30 seconds
clear-after = 30

# Custom keybindings
keybindings = Alt+1:type:username:tab:password,\
              Alt+2:copy:password,\
              Alt+m::menu

# Target-menu keybindings
menu-keybindings = Alt+t:type,Alt+c:copy
```

How it differs from upstream rofi-rbw

The CLI and config-file format are intentionally compatible, so you can
switch back and forth without changing anything in your workflow. The
differences are:

· Written in Rust, compiled to a single binary. No Python runtime.
· The pinentry integration is built into the same binary via a runtime
  dispatch that detects when rbw is the parent process. Upstream uses a
  shell script.
· All behavior is covered by tests.
· The internal data model is stricter (typed enums for targets and actions
  rather than string comparison), so unusual entries fail loudly rather than
  silently.

Troubleshooting

"Could not find a valid way to show the selection"

None of the four selector backends is installed. Install at least one
(rofi, wofi, fuzzel, or bemenu).

"Could not find a valid way to type characters"

No typer is installed. Install one (wtype on Wayland, xdotool on X11,
etc.) or use -a copy / -a print instead.

"Could not find a valid way to copy to clipboard"

No clipboarder is installed. Install one (wl-clipboard, xclip, or xsel).

rbw unlock still prompts after setup

Check that rbw config show reports "pinentry": "/usr/bin/rofi-rbw-rs",
with no arguments. If you have a stale value, reset it:

```bash
rbw config set pinentry /usr/bin/rofi-rbw-rs
```

Also verify the keyring entry is present:

```bash
secret-tool lookup application rbw profile rbw type master_password
```

If it's empty, the password was never stored — re-run rbw unlock and type
it in when prompted.

Empty rofi list

Run rbw list --raw to confirm rbw returns entries. If it does and rofi
still shows nothing, run with RUST_LOG=debug rofi-rbw-rs and check that the
backend parser sees the entries.

Password typed into the wrong field

This is usually the target application's focus handling, not the tool. Try a
different sequence:

```bash
rofi-rbw-rs -a type -t username -t tab -t password
rofi-rbw-rs -a type -t username -t enter -t password -t enter
```

License

PPL. See LICENSE.
