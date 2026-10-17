# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT

$ErrorActionPreference = 'Stop'
Set-Location (git rev-parse --show-toplevel)
git config core.hooksPath .githooks
Write-Output 'Installed git hooks: core.hooksPath=.githooks'
