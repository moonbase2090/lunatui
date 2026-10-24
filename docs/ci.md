# CI and sign-off

Pull request checks run through the repository runner workflow. The GitHub Actions CI and sign-off workflows are manual dispatches.

## Release

The Release workflow (`release.yml`) publishes a GitHub release when a `v*` tag is pushed. It runs on hosted runners. The `verify` job refuses a tag that is unsigned, that does not point at a commit on `main`, or whose tag does not match the workspace version in `Cargo.toml`, then runs the test suite. The `publish` job runs only after `verify` passes.

Both release jobs pass the selected full tag ref explicitly to checkout. This preserves the annotated tag object for signature-block checks and release metadata. With the default event inputs, checkout can replace its local annotated tag ref with the peeled commit when it fetches full history.

Before changing release checkout, exercise both the default and explicit-ref configurations in isolated checkouts of an existing signed tag. Confirm that the explicit-ref checkout retains an object of type `tag`, resolves to the expected commit, and preserves the tag subject, body, and signature block. Do not create or move a published tag for this check. This check runs locally and does not publish a release.

The tag subject supplies the release title. Notes use Git `%(contents:body)`, which excludes the signature block. If the annotation has no prose body, the workflow reads the matching bracketed version section from the selected tag's `CHANGELOG.md`. Missing or empty notes stop the job before binary compilation. The release-text gate reads the prepared notes file, and publication uses the same file through `--notes-file`. The gate also scans the title, complete tag message, and built example binary before anything is published. Assets are a source archive, a Linux x86_64 `hello_box` archive, and `SHA256SUMS`.

## Recover publication for an existing tag

A failed run uses its original event ref and commit when rerun. Merging a workflow correction does not update that run. For example, the failed v0.1.3 run cannot acquire the annotated-tag checkout fix through a rerun. See [GitHub rerun behavior](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/re-run-workflows-and-jobs).

After the recovery workflow is merged to main and the owner approves publication, the owner can start the corrected workflow against the existing signed tag:

```sh
gh workflow run release.yml --ref main -f tag=v0.1.3
```

The input must name an existing tag in `vMAJOR.MINOR.PATCH` form. Recovery dispatches from branches other than main fail before checkout. Verification requires an annotated tag with a signature block, a matching workspace version, and a tag commit on main. The signature-block check does not authenticate the signer.

The publish job checks out the same verified tag object and commit. Source archives use that commit, even when the recovery workflow runs from a newer main. Runs for the same tag are serialized. An existing release is left unchanged, including drafts or incomplete releases, and API failures stop publication. A partial existing release requires owner assessment rather than automatic replacement.

Verify recovery logic locally without dispatching or publishing:

```sh
ruby scripts/check-release-workflow-selftest.rb
```

The self-test executes the workflow shell steps in disposable repositories. Tag-check fixtures use synthetic signature blocks. Metadata tests generate a disposable SSH key and real signed empty-body and prose tags, verify those signatures locally, and check literal expected notes. No owner signing key or published ref is used. PGP formatting is not exercised. It tests malformed inputs, non-main dispatches, missing or lightweight tags, unsigned annotated tags, version and commit mismatches, off-main commits, publication checkout changes, API failures, and repeat publication probes. It also rejects missing or empty changelog sections, ignores a dirty worktree changelog in favor of the tagged source, and checks that scanned and published prose match. GitHub API, release-text scanning, and release creation calls use local fixture executables in the wiring tests. Run the real release-text gate separately over the final metadata and artifacts.

## Run sign-off for a pull request

The sign-off workflow needs the pull request's base commit, the exact head commit to check, and whether the pull request has the `mb2090-signoff` label.

Get the current values with:

```sh
gh pr view <number> --json baseRefOid,headRefOid,labels
```

In the GitHub Actions UI, open **Sign-off**, choose **Run workflow**, then provide:

- `base_sha`: the pull request's `baseRefOid` value.
- `head_sha`: the exact commit to validate, normally the pull request's `headRefOid`.
- `signoff_label`: `true` when the pull request has the `mb2090-signoff` label, otherwise `false`.

You can also dispatch it with GitHub CLI:

```sh
gh workflow run signoff.yml --ref main \
  -f base_sha=<base-sha> \
  -f head_sha=<head-sha> \
  -f signoff_label=true
```

Use `signoff_label=false` when the label is absent. The run checks the supplied commits and label state; refresh the values after the pull request changes.
