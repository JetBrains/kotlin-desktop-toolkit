## Publishing

To publish a new version from the current commit:
```bash
git tag 0.0.x
git push --tags
```
where `x` is the version number.

If CI passes, the new version will be published to the registry.
If CI fails, skip the version number and retry with the next version after fixing the issue.

To publish a version from a PR branch, use the format `0.0.x-SNAPSHOT.y`, where `y` is a unique number and `x` is the last published version.