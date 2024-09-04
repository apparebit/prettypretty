#!/bin/sh

set -e

if [ -t 2 ]; then
    MARK=$(printf "\e[1;34m〔r²〕\e[m")

    BOLD=$(printf "\e[1m")
    NOBOLD=$(printf "\e[22m")
    UNDERLINE=$(printf "\e[4m")
    NOLINE=$(printf "\e[24m")
    RESET=$(printf "\e[m")

    terminal=$(tty)
    # The joys of Unix: On macOS, stty -a prints the number of columns before the
    # word "columns", whereas on Linux it does just the opposite. Meanwhile, the BSD
    # version of grep accepts \d in an extended regex, whereas POSIX and Linux do
    # not.
    columns=$(stty -a <"$terminal" | grep -Eo '; ([0-9]+ )?columns( [0-9]+)?;' | grep -Eo '[0-9]+')

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
else
    MARK="〔r²〕"

    BOLD=""
    NOBOLD=""
    UNDERLINE=""
    NOLINE=""
    RESET=""

    columns=80

    # shellcheck disable=SC2034
    TRACE=""
    # shellcheck disable=SC2034
    INFO=""
    # shellcheck disable=SC2034
    SUCCESS=""
    # shellcheck disable=SC2034
    WARNING=""
    # shellcheck disable=SC2034
    ERROR=""
fi

help() {
    >&2 cat <<EOF
${BOLD}Usage:${NOBOLD} $(basename "$0") [COMMAND]

${BOLD}Commands:${NOBOLD}
    ${BOLD}install${NOBOLD} : Install or update required tools using apt or brew.
    ${BOLD}build${NOBOLD}   : Build and locally install the Python extenion module.
    ${BOLD}check${NOBOLD}   : Check that the source code is well-formatted,
              free of lint, and altogether in good shape.
    ${BOLD}docs${NOBOLD}    : Build the user guide, the Rust API documentation,
              and the Python API documentation in ${UNDERLINE}target/doc${NOLINE} dir.
    ${BOLD}all${NOBOLD}     : Perform build, check, and docs tasks in that order.
    ${BOLD}help${NOBOLD}    : Show this help message and exit.
EOF
    exit 0
}

log() (
    if [ "$1" = "TRACE" ]; then
        STYLE=""
        LEVEL=""
    else
        eval "STYLE=\"\${$1}\""
        LEVEL="$1: "
    fi
    shift

    # Don't use print, it's zsh only.
    >&2 printf "%s\n" "${MARK}${STYLE}${LEVEL}${*}${RESET}"
)

print_header() {
    # Don't use {1..$columns}. Bash does not expand variables.
    >&2 printf "━%.0s" $(seq 1 "$columns")
    >&2 printf "\n"
    >&2 echo "$@"
    >&2 printf "─%.0s" $(seq 1 "$columns")
    >&2 printf "\n"
}

run() {
    print_header "$*"
    "$@"
    >&2 echo
}

# -----------------------------------------------------------------------------------------------------------
# Since this script may change the current directory, refuse to run in any
# directory that is too close to the root. A user's home directory often is
# /home/user or /Users/user, so we insist on at least one level more.

grandparent=$(cd ../../; pwd)
if [ "$grandparent" = "/" ]; then
    log ERROR "The working directory $(pwd) is too close to root directory."
    log ERROR "Please run in a suitable subdirectory instead."
fi

# ===========================================================================================================
# Ugh! For Linux, I tried making installation work just using APT, since that's
# the package manager shipping with Ubuntu. The fact that package names tend
# towards the baroque is a bit annoying. Python not including venv seems like a
# gratuitous complication. The lack of packages for maturin and mdbook already
# points towards APT being insufficient. That is only reinforced by rust-all
# being five releases or 7.5 months behind the latest release, which means it's
# too old for compiling prettypretty. So much for simplicity and uniformity in
# package management...
#
# Still, I doubt that replacing the Linux distribution's package manager with
# Homebrew is the right approach. So, we split duties between APT, rustup, and
# cargo: APT for generic tools, rustup for core Rust tools, and cargo for
# remaining Rust tools.

if [ -x "$(command -v brew)" ]; then
    PACKAGE_MANAGER=Homebrew
    package_update_manager() {
        brew update
    }
    package_show_name() {
        echo "$1"
    }
    package_show_extras() {
        echo
    }
    package_is_installed() {
        brew ls --versions "$1" > /dev/null
    }
    package_install() {
        brew install "$1"
    }
elif [ -x "$(command -v apt-get)" ]; then
    PACKAGE_MANAGER=APT
    package_update_manager() {
        sudo apt-get -y update
    }
    package_show_name() {
        case "$1" in
            node)   echo "nodejs" ;;
            *)      echo "$1" ;;
        esac
    }
    package_show_extras() {
        # build-essential: includes gcc and linker, required for Rust
        # python3-venv: installs venv module stripped from python3 package
        echo "build-essential python3-venv"
    }
    package_is_installed() {
        dpkg-query -Wf'${db:Status-Abbrev}' "$1" 2>/dev/null | grep -q '^i'
    }
    package_install() {
        sudo apt-get -y install "$1"
    }
else
    # Delay any error until the install option is selected.
    PACKAGE_MANAGER=""
    package_update_manager() {
        log ERROR "Could not find apt-get (APT) or brew (Homebrew) package manager!"
        exit 1
    }
    package_show_name() {
        package_update_manager
    }
    package_show_extras() {
        package_update_manager
    }
    package_is_installed() {
        package_update_manager
    }
    package_install() {
        package_update_manager
    }
fi

install_package() {
    print_header "Installing $1 with $PACKAGE_MANAGER"
    package_install "$1"
    >&2 echo
}

# Install package if tool cannot be found.
install_tool() (
    tool_name="$1"
    pkg_name="$(package_show_name "$tool_name")"

    if [ -x "$(command -v "$tool_name")" ]; then
        log TRACE "Skipping already installed ${BOLD}${pkg_name}${NOBOLD}"
    else
        install_package "$pkg_name"
    fi
)

# -----------------------------------------------------------------------------------------------------------

python_dependencies() {
    extra="$1"
    ./.venv/bin/python -c "$(cat <<EOT
import sys
import tomllib
try:
    with open("pyproject.toml", mode="rb") as file:
        deps = tomllib.load(file)["project"]["optional-dependencies"]
        print(" ".join(deps["$extra"]))
except Exception as x:
    print(str(x), file=sys.stderr)
EOT
)"
}

install() {
    # ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~ Binary Packages
    print_header "Updating $PACKAGE_MANAGER and installed packages"
    package_update_manager
    >&2 echo

    for tool_name in git curl node python3; do
        install_tool "$tool_name"
    done

    for pkg_name in $(package_show_extras); do
        if package_is_installed "$pkg_name"; then
            log TRACE "Skipping already installed ${BOLD}${pkg_name}${NOBOLD}"
        else
            install_package "$pkg_name"
        fi
    done

    # ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~ Rust Tooling
    should_restart=false
    if [ ! -x "$(command -v rustup)" ]; then
        # Rustup modifies .profile, .bashrc,... to source $HOME/.cargo/env,
        # which updates PATH. We do the equivalent to finish installation.
        # However, user should still restart their current shell.
        print_header "Installing rustup"
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s - -y

        # shellcheck source=/dev/null
        . "${CARGO_HOME:-$HOME/.cargo}/env"

        should_restart=true
        >&2 echo
    fi

    for tool in maturin mdbook; do
        run cargo install --locked "$tool"
    done

    # ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~ Python Packages
    if [ -d ./.venv ]; then
        log TRACE "Skipping creation of existing virtual env in ${UNDERLINE}.venv${NOLINE}"
    else
        run python3 -m venv .venv
    fi

    print_header "Install or upgrade Python packages"
    ./.venv/bin/python -m pip install --upgrade pip
    # shellcheck disable=SC2046
    ./.venv/bin/python -m pip install --upgrade $(python_dependencies dev) $(python_dependencies viz)
    echo

    print_header "Install Pyright (via npm)"
    npm install

    # ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~ Done!
    if "${should_restart}"; then
        log WARNING "Please restart your current shell to pick up changes to PATH"
    fi
}

# ===========================================================================================================

build() {
    run cargo fmt
    run maturin dev --all-features
}

check() {
    run cargo fmt --check
    run cargo check
    run cargo check --features f64,gamut
    run cargo check --all-features
    run cargo clippy
    run cargo clippy --features f64,gamut
    run cargo clippy --all-features
    run cargo test --features f64,gamut

    if [ -d prettypretty ]; then
        run npm run pyright -- --pythonpath ./.venv/bin/python
    fi
    if [ -d test ]; then
        run run_python_tests
    fi

    print_header "Testing Rust examples from guide book"
    cargo clean
    cargo build
    mdbook test -L target/debug/deps docs
    >&2 echo

    print_header "Testing Python examples from guide book"
    mdbook build docs
    python docs/book.py
    >&2 echo
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

ooh_special() {
    if [ -z "$1" ]; then code="38;5;16;48;5;45"; else code="$1"; fi
    if [ -z "$BOLD" ]; then
        stl=""
    else
        stl="$(printf "\e[1;%sm" "$code")"
    fi

    if [ -z "$2" ]; then msg='Ooh!'; else msg="$2"; fi
    spc="$(echo "$msg" | tr "[:rune:]" " ")"

    >&2 printf "\n"
    >&2 printf "%s\n" "$stl  $spc $RESET"
    >&2 printf "%s\n" "$stl  $msg  $RESET"
    >&2 printf "%s\n" "$stl  $spc $RESET"
    >&2 printf "\n"
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
    nop) ;;
    ooh)
        ooh_special "$2" "$3"
        exit 0
        ;;
    *)
        log ERROR "target \"$target\" is not help/install/build/check/doc/all!"
        exit 1
        ;;
esac

log SUCCESS "Happy, happy, joy, joy!"
