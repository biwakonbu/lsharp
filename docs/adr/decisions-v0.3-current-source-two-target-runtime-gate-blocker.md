# ADR: v0.3 current-source two-target runtime gate blocker

- Status: Accepted (verified blocker / partial evidence)
- Date: 2026-08-02
- Scope: `M3-04-N1` / `M3-05-N9` current-source compile/build → artifact → runtime gate
- Related: [`decisions-v0.3-native-official-provider-freshness-binding.md`](decisions-v0.3-native-official-provider-freshness-binding.md), [`decisions-v0.3-native-official-stage0-runtime-smoke.md`](decisions-v0.3-native-official-stage0-runtime-smoke.md)

## Context

The remaining large gate is current-source evidence for the supported Mac Apple
Silicon and Linux x86_64 paths: compile/build, source-bound artifact and
manifest, packaged/stage0 handoff, and runtime smoke. It must be evaluated as
one gate so that provider input, source/artifact provenance, target artifact,
and runtime evidence are not mistaken for separate offline fixture evidence.

At the read-only audit before this documentation change, `HEAD` was
`f36b51539c1d903d89005f02d4bd9a9fe11770f0`. No manifest whose source commit
matched that `HEAD`, and no task-owned expected replay lock, was present under
the inspected `/tmp` and `/Users/biwakonbu/github/tmp` paths. A Lima hostagent,
QEMU process, and `replayd` for `lsharp-linux-x86` were active and owned by
another session.

## Decision

Do not start `native-official-release-local.sh`, stage regeneration, a Linux VM
replay, or a full build while the current-source artifact/lock and resource
ownership prerequisites are absent. This avoids consuming or changing another
session's VM, replay process, artifact, or lock. The docs-only commit that
records this blocker does not provide a current-source artifact and therefore
does not change the gate status.

The gate may resume only when all of the following are true:

1. A fresh manifest identifies the then-current `git rev-parse --verify HEAD`,
   the requested target, and the compiler/stage0 payload with provider
   identity/provenance.
2. An expected replay lock for that same source and artifact exists, and its
   ownership is available to this task.
3. The Lima/QEMU/replayd resource is explicitly released or assigned to this
   task, with no competing owner.
4. One batch records the Mac and Linux stdout/stderr, exit status, artifact
   digest, and runtime bytes before the corresponding `[~]` items are advanced.

## Evidence and reproduction

The audit used the following read-only checks from the dedicated Cloud
worktree:

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 6 -type f \
  -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 6 \
  \( -type d -name 'lsharp-native-linux-x86-hostgen-vm-*.lock' \
     -o -type f -name '*.lock' \)
ps -axo pid=,command= | grep -E 'limactl|qemu-system|replayd|cargo|rustc'
```

The first two checks did not produce a current-`HEAD` manifest/expected replay
lock, while the process check showed the other session's Lima/QEMU/replayd
ownership. Consequently no live provider acquisition, compile/build,
stage-regeneration, Linux replay, current-source runtime, or Mac/Linux
packaged/rollback bytes parity evidence was produced in this batch.

`M3-04-N1`, `M3-05-N2`, `M3-05-N7`, and `M3-05-N9` remain `[~]` in
`TODO.md` and the v0.3 planning document. The next run must repeat the
read-only status/ownership audit, then execute one gate only after the resume
conditions above are satisfied.
