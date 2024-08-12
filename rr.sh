#!/bin/sh

set -e

# Simple styles
BOLD=$(printf "\e[1m")
REGULAR=$(printf "\e[22m")
UNDERLINE=$(printf "\e[4m")
NOLINE=$(printf "\e[24m")
RESET=$(printf "\e[m")

help() {
    >&2 printf "%s\n" "${BOLD}$(basename "$0") [install|build|check|docs|all|help]${RESET}"
    >&2 printf "\n"
    >&2 printf "%s\n" "${BOLD}install${REGULAR} : Install or update required tools using apt or brew."
    >&2 printf "%s\n" "${BOLD}build${REGULAR}   : Build and locally install the Python extenion module."
    >&2 printf "%s\n" "${BOLD}check${REGULAR}   : Check that the source code is well-formatted,"
    >&2 printf "%s\n" "          free of lint, and altogether in good shape."
    >&2 printf "%s\n" "${BOLD}docs${REGULAR}    : Build the user guide, the Rust API documentation,"
    >&2 printf "%s\n" "          and the Python API documentation in ${UNDERLINE}target/doc${NOLINE} dir."
    >&2 printf "%s\n" "${BOLD}all${REGULAR}     : Perform build, check, and docs tasks in that order."
    >&2 printf "%s\n" "${BOLD}help${REGULAR}    : Show this help message and exit."
    exit 1
}

# shellcheck disable=SC2034
TRACE=""
# shellcheck disable=SC2034
INFO=$(printf "\e[1;35m")
# shellcheck disable=SC2034
SUCCESS=$(printf "\e[1;32m")
# shellcheck disable=SC2034
WARNING=$(printf "\e[1;38;5;208m")
# shellcheck disable=SC2034
ERROR=$(printf "\e[1;31m")

terminal=$(tty)
# The joys of Unix: On macOS, stty -a prints the number of columns before the
# word "columns", whereas on Linux it does just the opposite. Meanwhile, the BSD
# version of grep accepts \d in an extended regex, whereas POSIX and Linux do
# not.
columns=$(stty -a <"$terminal" | grep -Eo '; ([0-9]+ )?columns( [0-9]+)?;' | grep -Eo '[0-9]+')

log() {
    LEVEL="$1"
    shift
    eval "STYLE=\"\${$LEVEL}\""
    # Don't use print, it's zsh only.
    >&2 printf "%s\n" "${STYLE}〔r²〕${LEVEL}: ${*}${RESET}"
}

print_run_header() {
    # Don't use {1..$columns}. Bash does not expand variables.
    >&2 printf "━%.0s" $(seq 1 "$columns")
    >&2 printf "\n"
    >&2 echo "$@"
    >&2 printf "─%.0s" $(seq 1 "$columns")
    >&2 printf "\n"
}

run() {
    print_run_header "$@"
    "$@"
    >&2 echo
}

# ===========================================================================================================
# For simplicity and uniformity, use the same package manager for all dependencies!

if [ -x "$(command -v brew)" ]; then
    INSTALLER=Homebrew
    installer_update() {
        brew update
        brew upgrade
    }
    installer_install() {
        brew install "$1"
    }
elif [ -x "$(command -v apt)" ]; then
    INSTALLER="APT"
    installer_update() {
        sudo apt update
        sudo apt upgrade
    }
    installer_install() {
        sudo apt install "$1"
    }
else
    log ERROR "Could not find apt or brew package manager!"
    exit 1
fi

get_package_name() {
    case $1 in
        cargo)
            if [ "$INSTALLER" = "APT" ]; then
                echo "rust-all"
            else
                echo "rust"
            fi
            ;;
        *)  echo "$1" ;;
    esac
}

install_tool() {
    tool_name="$1"
    pkg_name="$(get_package_name "$tool_name")"

    if [ -x "$(command -v "$tool_name")" ]; then
        log TRACE "Skipping ${BOLD}${pkg_name}${REGULAR}, since it is already installed."
    else
        installer_install "$pkg_name"
    fi
}

install() {
    print_run_header "Update $INSTALLER and its packages"
    installer_update
    echo

    for tool in git curl cargo maturin mdbook node python; do
        install_tool "$tool"
    done

    if [ -d ./.venv ]; then
        log TRACE "Skipping creation of virtual env in ${UNDERLINE}.venv${NOLINE}, since it already exists."
    else
        run python -m venv .venv
    fi
}

# ===========================================================================================================

build() {
    run cargo fmt
    run maturin dev --all-features

    if [ -f "prettypretty/color.abi3.so" ]; then
        suffix=".abi3.so"
    elif [ -f "prettypretty/color.pyd" ]; then
        suffix=".pyd"
    else
        log ERROR "Unable to locate extension module library!"
    fi

    mkdir -p prettypretty/color

    # Creating symbolic links to reify submodules of an extension module is a
    # bit weird, though blessed by PEP 489 (https://peps.python.org/pep-0489/).
    for mod in gamut spectrum term trans; do
        if ! [ -h "prettypretty/color/${mod}${suffix}" ]; then
            ln -s "../color${suffix}" "prettypretty/color/${mod}${suffix}"
        fi
    done
}

check() {
    run cargo fmt --check
    run cargo check
    run cargo check --all-features
    run cargo clippy
    run cargo clippy --all-features
    run cargo test
    if [ -d prettypretty ]; then
        run npm run pyright -- --pythonpath ./.venv/bin/python
    fi
    if [ -d test ]; then
        run run_python_tests
    fi
}

run_python_tests() {
    ./.venv/bin/python -c '
import sys
import unittest
from test.runtime import ResultAdapter

unittest.main(
    module="test",
    testRunner=unittest.TextTestRunner(
        stream=sys.stdout,
        resultclass=ResultAdapter
    ),
)
'
}

doc() {
    if [ -d docs ]; then
        run mdbook build docs
    fi

    run cargo rustdoc --all-features -- -e "$(realpath docs/pretty.css)"

    if [ -d docs ]; then
        run ./.venv/bin/sphinx-build -a -b html docs target/doc/python
        run rm -rf target/doc/python/.doctrees
    fi
}

# ===========================================================================================================

if [ -z "$1" ]; then
    target="build"
else
    target="$1"
fi

case $target in
    "-h" | "--help" | "help")
        help;;
    install)
        install;;
    build)
        build;;
    check)
        check;;
    doc)
        doc;;
    all)
        build
        check
        docs
        ;;
    *)
        log ERROR "\"$target\" is not a valid runner target!"
        exit 1
        ;;
esac

log SUCCESS "Happy, happy, joy, joy!"
