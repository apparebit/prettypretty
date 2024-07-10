#!/bin/zsh

set -e

BOLD="\e[1m"
ERROR="\e[1;31m"
WARNING="\e[1;38;5;208m"
SUCCESS="\e[1;32m"
INFO="\e[1;35m"
RESET="\e[0m"


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
    printf "━%.0s" {1..80}
    printf "\n"
    echo "$*"
    printf "─%.0s" {1..80}
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
    trace cargo check
    trace cargo check --all-features
    trace cargo clippy
    trace cargo clippy --all-features
    trace cargo test
    trace npm run pyright
    trace run-python-tests
}

build() {
    trace cargo fmt
    trace maturin dev --all-features
}

docs() {
    trace mdbook build docs
    trace cargo rustdoc --all-features -- -e $(realpath docs/pretty.css)
    trace sphinx-build -a -b html docs target/doc/python
    trace rm -rf target/doc/python/.doctrees
}

case $1 in
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
        log ERROR "\"$1\" is not a valid run target!"
        exit 1
        ;;
esac

log SUCCESS Happy, happy, joy, joy!
