import os
import sys


def is_dark_mode() -> None | bool:
    """
    Determine whether the operating system is in dark mode.

    Returns:
        ``True`` for dark mode, ``False`` for light mode, and ``None`` if the
        mode could not be determined.

    The implementation builds on answers to `this StackOverflow question
    <https://stackoverflow.com/questions/65294987/detect-os-dark-mode-in-python>`_
    and `the darkdetect package
    <https://github.com/albertosottile/darkdetect>`_. The latter seems both
    over- and under-engineered. In contrast, this module provides the one
    interesting bit, whether the system is in dark mode, if available and
    nothing else.
    """
    try:
        if sys.platform == "darwin":
            return _is_darkmode_macos()
        elif sys.platform == "linux":
            return _is_darkmode_linux()
        elif os.name == 'nt':
            return _is_darkmode_windows()
        else:
            return None
    except:
        return None


def _is_darkmode_windows() -> bool:
    import winreg

    with winreg.OpenKey(  # type: ignore[attr-defined]
        winreg.HKEY_CURRENT_USER,  # type: ignore[attr-defined]
        "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
    ) as key:  # type: ignore
        return not winreg.QueryValueEx(  # type: ignore[attr-defined]
            key, "AppsUseLightTheme"
        )[0]


def _is_darkmode_macos() -> bool:
    import subprocess

    # Use DEVNULL so that output of command isn't shown
    return not subprocess.run(
        ["defaults", "read", "-g", "AppleInterfaceStyle"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    ).returncode


def _is_darkmode_linux() -> None | bool:
    import subprocess

    try:
        result = subprocess.run( [
                "dbus-send",
                "--session",
                "--print-reply=literal",
                "--dest=org.freedesktop.portal.Desktop",
                "/org/freedesktop/portal/desktop",
                "org.freedesktop.portal.Settings.Read",
                "string:org.freedesktop.appearance",
                "string:color-scheme"
            ],
            capture_output=True,
            encoding='utf8',
            check=True,
        )

        stdout = result.stdout.replace("variant", "").replace("uint32", "").strip()
        if stdout in ("0", "1", "2"):
            # 0 stands for default, 1 for prefers-dark, and 2 for prefers-light.
            # Ubuntu returns 0 for light mode and 1 for dark mode.
            return stdout == "1"
    except:
        pass

    try:
        result = subprocess.run(
            ["gsettings", "get", "org.gnome.desktop.interface", "color-scheme"],
            capture_output=True,
            encoding="utf8",
            check=True,
        )
        stdout = result.stdout.strip()
    except:
        stdout = ""

    if not stdout:
        result = subprocess.run(
            ["gsettings", "get", "org.gnome.desktop.interface", "gtk-theme"],
            capture_output=True,
            encoding="utf8",
            check=True,
        )
        stdout = result.stdout.strip()
    return stdout.strip("'").endswith("-dark")
