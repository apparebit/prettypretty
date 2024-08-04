#!/bin/zsh

set -e

BOLD="\e[1m"
ERROR="\e[1;31m"
WARNING="\e[1;38;5;208m"
SUCCESS="\e[1;32m"
EM="\e[1;34m"
INFO="\e[1;35m"
RESET="\e[0m"

terminal=$(tty)
columns=$(stty -a <"$terminal" | egrep -Eo '\d+ columns;' | egrep -Eo '\d+')

h1() {
    hx "━" "$1"
}

h2() {
    hx "─" "$1"
}

hx() {
    printf "$1%.0s" {1..4}
    printf " $2 "
    printf "$1%.0s" {1..$((80 - ${#2}))}
    printf "\n"
}

log() {
    eval "STYLE=\"\${$1}\""
    print -u 2 "${STYLE}$1: ${@:2}${RESET}"
}

trace() {
    printf "━%.0s" {1..$columns}
    printf "\n"
    echo "$*"
    printf "─%.0s" {1..$columns}
    printf "\n"
    $1 "${@:2}"
    echo
}

run-python-tests() {
    python -c '
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

check() {
    trace cargo fmt --check
    trace cargo check
    trace cargo check --all-features
    trace cargo clippy
    trace cargo clippy --all-features
    trace cargo test
    if [ -d prettypretty ]; then
        trace npm run pyright
    fi
    if [ -d test ]; then
        trace run-python-tests
    fi
}

build() {
    trace cargo fmt
    trace maturin dev --all-features
}

docs() {
    if [ -d docs ]; then
        trace mdbook build docs
    fi

    trace cargo rustdoc --all-features -- -e $(realpath docs/pretty.css)

    if [ -d docs ]; then
        trace sphinx-build -a -b html docs target/doc/python
        trace rm -rf target/doc/python/.doctrees
    fi
}

help() {
    echo "${EM}./rr.sh [check|build|docs|all|help]${RESET}"
    echo
    echo "${BOLD}check${RESET} :  Check that the source code is well-formatted,"
    echo "         free of lint, and altogether in good shape."
    echo "${BOLD}build${RESET} :  Build and locally install the Python extenion module."
    echo "${BOLD}docs${RESET}  :  Build the user guide, the Rust API documentation,"
    echo "         and the Python API documentation; see target/doc"
    echo "${BOLD}all${RESET}   :  Perform all of the above tasks in the listed order."
    echo "${BOLD}help${RESET}  :  Show this help message and exit."
    exit 1
}

target="$1"

if [ -z "$1" ]; then
    target="build"
fi


case $target in
    "-h" | "--help" )
        help
        ;;
    check )
        check
        ;;
    build )
        build
        ;;
    docs )
        docs
        ;;
    all )
        check
        build
        docs
        ;;
    * )
        log ERROR "\"$target\" is not a valid runner target!"
        exit 1
        ;;
esac

log SUCCESS Happy, happy, joy, joy!
