# MindFlow

## Agent skills

This repo is configured for the mattpocock engineering skills (installed under `.agents/skills/`). Setup created the config below — see `docs/agents/*.md` for the details each skill reads.

### Issue tracker

Issues and PRDs live in **GitHub Issues** for `ChanduKaranam/MindFlow`, via the `gh` CLI. External PRs are **not** pulled into the triage queue. See `docs/agents/issue-tracker.md`.

> The `gh` CLI is installed at `~/.local/bin/gh` and authenticated as `Purna-Chandra-Rao-Karanam`. `to-issues`, `triage`, `to-prd`, and `qa` are ready to use.

### Triage labels

Five canonical triage roles use their **default label strings**: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

**Single-context** layout: one `CONTEXT.md` + `docs/adr/` at the repo root (created lazily by `domain-modeling`). See `docs/agents/domain.md`.
