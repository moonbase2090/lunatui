# CI and sign-off

Pull request checks run through the repository runner workflow. The GitHub Actions CI and sign-off workflows are manual dispatches.

## Release

The Release workflow (`release.yml`) publishes a GitHub release when a `v*` tag is pushed. It runs on hosted runners. The `verify` job refuses a tag that is unsigned, that does not point at a commit on `main`, or whose tag does not match the workspace version in `Cargo.toml`, then runs the test suite. The `publish` job runs only after `verify` passes.

The tag message supplies the release title (its first line) and notes (the rest). The release-text gate scans them and the built example binary before anything is published. Assets are a source archive, a Linux x86_64 `hello_box` archive, and `SHA256SUMS`.

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
