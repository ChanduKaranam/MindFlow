# MindFlow

## Agent skills

This repo is configured for the mattpocock engineering skills (installed under `.agents/skills/`). Setup created the config below — see `docs/agents/*.md` for the details each skill reads.

### Issue tracker

Issues and PRDs live in **GitHub Issues** for `ChanduKaranam/MindFlow`, via the `gh` CLI. External PRs are **not** pulled into the triage queue. See `docs/agents/issue-tracker.md`.

> Note: the `gh` CLI is not yet installed in this environment. Install it (`https://cli.github.com`) and run `gh auth login` before using `to-issues`, `triage`, `to-prd`, or `qa`.

### Triage labels

Five canonical triage roles use their **default label strings**: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

**Single-context** layout: one `CONTEXT.md` + `docs/adr/` at the repo root (created lazily by `domain-modeling`). See `docs/agents/domain.md`.
