#!/bin/sh
set -eu

cd "$(git rev-parse --show-toplevel)"
git config core.hooksPath .githooks
printf 'Installed git hooks: core.hooksPath=.githooks\n'
