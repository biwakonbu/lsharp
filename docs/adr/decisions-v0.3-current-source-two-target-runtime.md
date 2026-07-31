# ADR: v0.3 current-source two-target runtime evidence

## Status

Verified partial evidence (2026-08-01). `EC-M3-05` / `M3-05-N9` remains `[~]` until native MCP and the real provider adapter are closed.

## Context

N9 requires one current source commit to flow through Mac Apple Silicon and Linux x86_64 native producers, packaged App.Cli archives, fetched stage0 packages, source-file smoke, provider identity, and rollback compatibility. A Rust-host success, a stale stage0, or a release manifest without runtime evidence is not sufficient.

## Decision

- The evidence source commit is `1cdbe555f63c909fbfb3940c8462cf4b08ba442d` (the current `origin/main` at the run start).
- App.Cli release artifacts and stage0 compiler artifacts are separate boundaries. The public App.Cli release program accepts CLI commands; the stage0 compiler must accept the positional source/range transport protocol. Linux stage0 therefore uses the replayed `stage2-debug/program.native`, not the target-only App.Cli release program.
- Provider input is an explicit offline fixture boundary. The run hashes the raw trust-store and lifecycle snapshot bytes and passes both paths to every release and stage0 verifier. No network, helper, or implicit provider lookup is allowed by this evidence.
- A live Linux hostgen replay lock is authoritative. The official gate exits `90` before VM/archive work when another task owns the lock; the task is retried only after that owner releases it.

## Evidence

| Boundary | Evidence |
| --- | --- |
| Mac App.Cli producer | actual release E2E passed; `program.native` SHA-256 `29c595726de0668a899c28b8cd7551005b4e8356b26faedd3700eeeb45a21dc1`; source commit in manifest matches. |
| Mac stage0 compiler | package manifest matches target/commit; compiler SHA-256 `8f668a888308df2e84f021cbef785da6377539c8936c524efeb4ccef27a3fcc7`; fetched/source smoke passed. |
| Linux hostgen fixed point | VM stage2/stage3 stdout SHA-256 `dad391cd36df64b6354b1f4429aaf7a4c410697b7ca74606fbb2865dc2186bb1` matched; stage3 target-only App.Cli manifest source commit matches and `program.native` SHA-256 is `733dc7f4320f7b957b2af1380ed5e8e839f4eb20fd16a2411f40dc2c6305e872`; `--version` smoke passed with empty stderr. |
| Linux stage0 compiler | protocol probe with `src/App/Cli.ls 0 64 1 0` passed using stage2 compiler; packaged compiler SHA-256 `daacf2e16e7a0d3272e61d403a2fc01c652d8136715c4b8668cef4b33b8e5106`. |
| Provider identity | trust-store digest `sha256:460b86efa72c4dbd47348204b64d66d9e6c4b58a6ad3b9d1f6a2a9cf665593de`; lifecycle digest `sha256:33edeff993807d2d8180fe016e0fefc56ff473339494ef0709558cc03d03bf3c`; subject digest `sha256:d608aa90aad3e07b272bee50f6c26d4435fc335d82145164d9b7cba2e2d94afc`. |
| Rollback compatibility | Mac archive SHA-256 `c3095df94540448059a03000798ef5eb4e7de7b1ce45a4528f244dad4680a4dc`; Linux archive SHA-256 `c1ae74d30d7a7d013da9b8a0d3f09b1b7493de1afe96c17a89e82e3186a4cefe`; both manifests carry the same source commit and `v0.1.0`. |
| Official packaging | Mac/Linux native release smoke, provider identity verification, stage0 package archive and `fetch-stage0.sh` checksum/provenance passed. Mac fetched-stage0 source smoke passed with exit `0`; the Linux source-file smoke is intentionally pending because another task owns the replay lock and the official retry stopped with exit `90`. That stop is operator safety evidence, not a runtime failure. |

## Failure and correction

The first Linux stage0 package incorrectly used the target-only App.Cli release binary as `bin/compiler`. Its `--version` route passed, but the transport driver received `src/App/Cli.ls` and correctly reported `unknown command`, exit `127`. The package was rejected and not counted as evidence. Repackaging with the stage2 selfhost compiler closed the protocol boundary; the direct VM probe passed before retrying the official gate.

## Remaining boundary

The snapshot bytes are explicit local provider fixtures, not proof of a live provider API/auth adapter. Native MCP parity and the external provider acquisition contract remain active work, so `EC-M3-05` and N9 stay `[~]` in `TODO.md`.
