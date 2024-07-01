import os
import subprocess
import sys


def identify_terminal() -> None | tuple[str, str]:
    """
    Identify the current terminal.

    This function implements the fallback strategies for
    :meth:`.Terminal.request_terminal_identity`.
    """
    name = lookup_term_program()
    version = lookup_term_program_version()
    if name and version:
        return normalize_terminal_name(name), version

    if sys.platform != "darwin":
        return None

    bundle = lookup_macos_bundle_id()
    if bundle is None:
        return None

    version = lookup_macos_bundle_version(bundle)
    return normalize_terminal_name(bundle), version or ''


def _init_registry() -> dict[str, str]:
    registry: dict[str, str] = {}

    for name, *aliases in (
        ("Alacritty", "org.alacritty", "org.alacritty.Alacritty"),
        ("Hyper", "co.zeit.hyper"),
        ("iTerm", "com.googlecode.iterm2", "iTerm2"),
        ("Kitty", "net.kovidgoyal.kitty"),
        ("Rio", "com.raphaelamorim.rio"),
        ("Tabby", "org.tabby"),
        ("Terminal.app", "com.apple.Terminal", "Apple_Terminal"),
        ("VSCode", "com.microsoft.VSCode", "Code", "Visual Studio Code"),
        ("Warp", "dev.warp.Warp-Stable", "WarpTerminal"),
        ("WezTerm", "com.github.wez.wezterm", "org.wezfurlong.wezterm"),
    ):
        registry[name.casefold()] = name
        for alias in aliases:
            registry[alias.casefold()] = name

    return registry

_REGISTRY = _init_registry()


def normalize_terminal_name(name: str) -> str:
    """
    Normalize the terminal name or bundle ID.

    If the given name is an alias for a well-known terminal, this function
    returns the canonical name. Otherwise, it just returns the given name.
    """
    normal = _REGISTRY.get(name.casefold())
    return normal if normal else name


def lookup_term_program() -> None | str:
    """Look up the term program."""
    return os.getenv('TERM_PROGRAM')


def lookup_term_program_version() -> None | str:
    """Look up the term program version."""
    return os.getenv('TERM_PROGRAM_VERSION')


def lookup_macos_bundle_id() -> None | str:
    """Look up the macOS bundle identifier for the current terminal."""
    return os.getenv('__CFBundleIdentifier')


def lookup_macos_bundle_version(bundle: str) -> None | str:
    """
    Look up the macOS bundle version for the given bundle ID.

    This function only runs on macOS.
    """
    if sys.platform != "darwin":
        raise NotImplementedError("only runs on macOS")

    # Locate actual bundle...
    paths = subprocess.run(
        ["mdfind", f"kMDItemCFBundleIdentifier == '{bundle}'"],
        stdout=subprocess.PIPE,
        encoding="utf8",
    ).stdout.strip().splitlines()

    if not paths:
        return None

    # ...to extract its version
    for path in paths:
        version = subprocess.run(
            ["mdls", "-name", "kMDItemVersion", "-raw", path],
            stdout=subprocess.PIPE,
            encoding="utf8",
        ).stdout.strip()

        if version:
            return version

    return None
