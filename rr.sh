#!/usr/bin/env bash

set -e

# Simple styles
BOLD="\e[1m"
REGULAR="\e[22m"

UNDERLINE="\e[4m"
NOLINE="\e[24m"

RESET="\e[m"

# Log styles
TRACE=""
INFO="\e[1;35m"
SUCCESS="\e[1;32m"
WARNING="\e[1;38;5;208m"
ERROR="\e[1;31m"

terminal=$(tty)
# Don't use \d; it doesn't work on Linux.
columns=$(stty -a <"$terminal" | egrep -Eo '; ([0-9]+ )?columns( [0-9]+)?;' | egrep -Eo '[0-9]+')

log() {
    eval "STYLE=\"\${$1}\""
    # Use printf only, since Bash doesn't have print
    >&2 printf "${STYLE}$1: ${@:2}${RESET}\n"
}

trace_header() {
    >&2 printf "━%.0s" $(seq 1 $columns)
    >&2 printf "\n"
    >&2 echo "$*"
    >&2 printf "─%.0s" $(seq 1 $columns)
    >&2 printf "\n"
}


trace() {
    trace_header "$*"
    "$@"
    >&2 echo
}

help() {
    >&2 printf "${BOLD}$0 [install|build|check|docs|all|help]${RESET}\n"
    >&2 printf "\n"
    >&2 printf "${BOLD}install${REGULAR} : Install or update required tools as follows:\n"
    >&2 printf "              rustup   : cargo, rustc\n"
    >&2 printf "              cargo    : maturin, mdbook\n"
    >&2 printf "              apt/brew : curl, git, node, python\n"
    >&2 printf "${BOLD}build${REGULAR}   : Build and locally install the Python extenion module.\n"
    >&2 printf "${BOLD}check${REGULAR}   : Check that the source code is well-formatted,\n"
    >&2 printf "          free of lint, and altogether in good shape.\n"
    >&2 printf "${BOLD}docs${REGULAR}    : Build the user guide, the Rust API documentation,\n"
    >&2 printf "          and the Python API documentation in ${UNDERLINE}target/doc${NOLINE} dir.\n"
    >&2 printf "${BOLD}all${REGULAR}     : Perform build, check, and docs tasks in that order.\n"
    >&2 printf "${BOLD}help${REGULAR}    : Show this help message and exit.\n"
    exit 1
}

# ===========================================================================================================
# Between the Rust project's rustup, Homebrew's brew, and Ubuntu's apt, there
# are at least two ways of installing Rust on macOS and three on Linux. So
# what's the right approach? To keep installation *lightweight*, this script
# uses Homebrew on macOS and the distro's package manager on Linux for
# installing arbitrary tools such as curl and git. To keep it *reliable*, it
# uses Rust's own rustup for installing Rust and cargo for installing mdbook and
# maturin.

install_prepare() {
    if [ -x "$(command -v brew)" ]; then
        installer="brew install"
        installer_update="brew update && brew upgrade"
    elif [ -x "$(command -v apt)" ]; then
        installer="sudo apt install"
        installer_update="sudo apt update && sudo apt upgrade"
    else
        log ERROR "Could not find apt or brew package manager!"
        exit 1
    fi
}

install_tool() {
    tool="$1"
    if [ -x "$(command -v $tool)" ]; then
        log TRACE "Skipping already installed ${BOLD}${tool}${REGULAR}"
    else
        trace $installer "$tool"
    fi
}

install() {
    install_prepare
    trace $installer_update
    install_tool git
    install_tool curl

    if [ -x "$(command -v rustup)" ]; then
        log TRACE "Skipping already installed ${BOLD}${tool}${REGULAR}"
    else
        trace_header "Installing rustup"
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    fi

    trace rustup update
    trace cargo install --locked maturin
    trace cargo install --locked mdbook

    install_tool node
    install_tool python

    if [ ! -d ./.venv ]; then
        trace python -m venv .venv
    fi
}

# ===========================================================================================================

build() {
    trace cargo fmt
    trace maturin dev --all-features
}

check() {
    trace cargo fmt --check
    trace cargo check
    trace cargo check --all-features
    trace cargo clippy
    trace cargo clippy --all-features
    trace cargo test
    if [ -d prettypretty ]; then
        trace npm run pyright -- --pythonpath ./.venv/bin/python
    fi
    if [ -d test ]; then
        trace run_python_tests
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

docs() {
    if [ -d docs ]; then
        trace mdbook build docs
    fi

    trace cargo rustdoc --all-features -- -e "$(realpath docs/pretty.css)"

    if [ -d docs ]; then
        trace ./.venv/bin/sphinx-build -a -b html docs target/doc/python
        trace rm -rf target/doc/python/.doctrees
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
    docs)
        docs;;
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
