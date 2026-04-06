# xiuxian-git-repo

`xiuxian-git-repo` owns the reusable repository substrate that used to live
inside `xiuxian-wendao`.

Current slice scope:

- ghq-style managed mirror and checkout layout
- local checkout validation
- managed clone/fetch/checkout synchronization
- checkout lock lifecycle and stale-lock reclamation
- checkout metadata and probe-state observation
- backend-neutral public repo contracts and error taxonomy

Current implementation note:

- the public API is backend-neutral
- the crate no longer carries a runtime `git2` dependency
- repository open/probe/revision/drift logic is now backed by `gix`
- managed remote alignment is now persisted through `gix` local-config writes
  and repository reload
- managed remote target probing now uses `gix` remote ref-map inspection with
  explicit HEAD/branch/tag probe refspecs instead of `git ls-remote`
- annotated tag probe results now resolve to the peeled target object so probe
  state stays comparable with checkout and tracking revisions
- the touched materialization regressions now use tmp-backed cwd fixtures
  instead of operator-specific absolute paths
- the remaining bounded native `git` command bridges are clone, fetch, and
  detached checkout where that still keeps behavior aligned with the existing
  contract
