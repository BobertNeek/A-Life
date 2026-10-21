---
name: graphify
description: Optional Graphify navigation for code or document relationships when the user requests it or an existing graph would help answer the question. Source inspection remains available without Graphify.
---

# Graphify navigation

Graphify builds and queries a local graph of imports, calls, concepts, and
cross-file relationships. It is a navigation aid, not a source of repository
policy or proof that a feature works. Check relevant source when graph results
are stale, incomplete, inferred, or important to a code change.

Use the installed CLI and its `--help` for the version available. On Windows:

```powershell
scripts/graphify.ps1 query "How does the save reach the archive?"
scripts/graphify.ps1 path "GpuLiveBrainRuntime" "LineageLibrary"
scripts/graphify.ps1 explain "FoundationWeightAsset"
scripts/graphify.ps1 update --no-cluster
```

The corresponding shell wrapper is `scripts/graphify.sh`. Generated graph data
and reports belong under ignored `graphify-out/`. An existing graph can answer a
navigation question without rebuilding it. Query, path, and explain results
should identify the relevant source locations and distinguish extracted links
from inferences.

Refresh the graph when its inputs changed and a refresh serves the task. For a
new graph, use the CLI's extraction command for the intended directory; choose
structural extraction for code and semantic extraction only when needed. Keep
unrelated files and private data outside the selected input. Report failures or
missing coverage instead of inventing graph edges.

For less common commands such as `extract`, `clone`, `merge-graphs`, `export`,
`watch`, or `serve`, consult the installed CLI help. This repository does not
maintain a second copy of Graphify's extraction engine or platform-specific
orchestration instructions.
