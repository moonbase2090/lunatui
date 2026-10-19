# CI and sign-off

Pull request checks run through the repository runner workflow. The GitHub Actions CI, release, and sign-off workflows are manual dispatches.

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
