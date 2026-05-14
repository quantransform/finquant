#!/usr/bin/env bash
# Set up the ml/ Python environment.
#
# Poetry 2.x on Homebrew Python sometimes ignores `virtualenvs.in-project`
# and tries to install into the system interpreter. Forcing VIRTUAL_ENV
# and disabling poetry's own venv-creation gets it to install into a local
# .venv that we create ourselves.

set -euo pipefail
cd "$(dirname "$0")"

if [[ ! -d .venv ]]; then
    /opt/homebrew/opt/python@3.14/bin/python3.14 -m venv .venv
    echo "created .venv (Python 3.14)"
fi

VIRTUAL_ENV="$PWD/.venv" POETRY_VIRTUALENVS_CREATE=false \
    poetry install --no-root "$@"

echo
echo "done. activate with:  source ml/.venv/bin/activate"
