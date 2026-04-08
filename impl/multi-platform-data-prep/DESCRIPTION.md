# Feature: Multi-Platform Web Prep Data

## Problem
Current web prep data is sourced from a dump that only carries `x86_64-linux` platform tags, so browser platform filtering cannot target machines like `aarch64-darwin`.

## Goal
Produce prepared web artifacts that include realistic per-package platform support across Linux/Darwin architectures.

## Approach
Use channel `packages.json.br` (contains `meta.platforms`) as primary prep source and keep existing source as fallback.
