Steps to publish a new release.

1. Removed `dev` from version in `cargo.toml`

    A. Remove `dev` from changelog version.

2. Make sure the CI pipeline is clean

3. Do a publish dry run
```sh
cargo publish --dry-run
```

4. Double check package
```sh
cargo package --list
```

5. Publish to crates.io
```sh
cargo publish
```

6. Add git tag `release_vX.Y.Z` 

7. Publish release on Github. Copy the release notes

8. Bump version and add `dev`

    B. Also do this in changelog
