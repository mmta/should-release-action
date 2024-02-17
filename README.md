# should-release-action

[![codecov](https://codecov.io/gh/mmta/should-release-action/graph/badge.svg?token=BT7Q8MFDTI)](https://codecov.io/gh/mmta/should-release-action)

<!-- action-docs-description action="action.yml" -->
## Description

This action lets you control releases and tags through the version number in your project files 
(Cargo.toml, package.json, or version.txt). 

It does this by comparing the version on file with the release tag supplied as input, which is meant to be
supplied by another action that reads the latest release tag from the repository.

If the version on file is higher than the release tag supplied, `should-release` output will be set to `true` 
and `current_version` output will be set to the version on file. The next step in the workflow can then use those
outputs to trigger a Github release and build artifacts as needed. 

If the version on file is lower or equal to the `release_tag` input, `should-release` output will be set to `false`.
<!-- action-docs-description action="action.yml" -->

<!-- action-docs-inputs action="action.yml" -->
## Inputs

| name | description | required | default |
| --- | --- | --- | --- |
| `file_path` | <p>path to Cargo.toml, package.json, or version.txt</p> | `true` | `""` |
| `release_tag` | <p>the release tag to compare with, defaults to v0.0.0 if not supplied or empty. That default essentially means always release, which is useful for the first release of a project</p> | `false` | `""` |
<!-- action-docs-inputs action="action.yml" -->

<!-- action-docs-outputs action="action.yml" -->
## Outputs

| name | description |
| --- | --- |
| `should_release` | <p>true if the version on file is higher and release should be made, false otherwise</p> |
| `current_version` | <p>the version on file, should be used to create a release tag by the next step in the workflow</p> |
<!-- action-docs-outputs action="action.yml" -->

<!-- action-docs-runs action="action.yml" -->
## Runs

This action is a `docker` action.
<!-- action-docs-runs action="action.yml" -->

To avoid lengthly build process at runtime, this action's [Dockerfile](./Dockerfile) just downloads a pre-built image from container registry. 
The image itself was built using this [Dockerfile](./docker/Dockerfile) when a release was made from this repo.

## Usage example

Here's how to use this in a release workflow with other actions. It should be easy to adjust this for other actions that perform similar steps as well.

```yaml
name: release
on:
  push:
    branches:
      - master
jobs:
  check-versions:
    runs-on: ubuntu-latest
    outputs:
      should_release: ${{ steps.comp_ver.outputs.should_release }}
      current_version: ${{ steps.comp_ver.outputs.current_version }}
    steps:
      - uses: actions/checkout@v4
      # this will get latest release tag and set it to tag_name on its output
      - uses: cardinalby/git-get-release-action@1.2.5 
        id: check_rel
        name: get latest release
        env:
          GITHUB_TOKEN: ${{ github.token }}
        with:
          latest: true
          prerelease: false
          doNotFailIfNotFound: true
      # this is our action that reads tag_name and produces should_release and current_version
      - name: compare versions
        uses: mmta/should-release-action@v1.0.0
        id: comp_ver
        with:
          file_path: Cargo.toml
          release_tag: ${{ steps.check_rel.outputs.tag_name }}          

  publish-new-version:
    # this job will only run if should_release above is true
    needs: check-versions
    if: needs.check-versions.outputs.should_release == 'true'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      # build artifacts and other pre-release activities here

      - name: create release
        uses: softprops/action-gh-release@v1
        with:
          # use version from file as the tag for the new release          
          tag_name: v${{ needs.check-versions.outputs.current_version }}
          generate_release_notes: true
          draft: false
          prerelease: false
```
