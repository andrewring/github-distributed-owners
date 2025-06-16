# github-distributed-owners

A tool for auto generating GitHub compatible CODEOWNERS files from OWNERS files distributed through the file tree.

Distributing OWNERS configuration throughout the file tree makes it easier to find the appropriate people/teams who own
a given part of the codebase. This is especially useful in a multi-team, monorepo environment. It also has the nice
property of allowing teams to edit their own OWNERS files, with approval required only from the team. With the single
CODEOWNERS file supported by GitHub, you can either grant _everyone_ access to edit owners, or you can set a smaller
group of reviewers for all teams to send changes to, each of which have problems.

> [!NOTE]
> If you're using github-distributed-owners, we want to hear from you!
> Please
> [drop us a comment here](https://github.com/andrewring/github-distributed-owners/discussions/new?category=users).
> :)

## Usage

Create files named `OWNERS` in the directories containing newline separated references to users or groups.

```shell
github_username
user@email.com
@group
```

Once these are in place, you can generate a GitHub compatible CODEOWNERS file by running the following in the root
directory of the git repo

```shell
github-distributed-owners --output-file .github/CODEOWNERS
```

> [!WARNING]
> The generated CODEOWNERS file (`/.github/CODEOWNERS by default) should be set to not have any owners if you are
> enforcing no diff from running this tool. Failure to do so would result in whichever group has ownership of that file
> needing to approve every OWNERS change, which partially defeats the purpose of this process.
> This can be done by adding the following to the OWNERS file adjacent to the CODEOWNERS file, with no owners listed:
>
> ```shell
> [CODEOWNERS]
> set inherit = false
> ```

### Pre-commit

Example pre-commit config:

```yaml
repos:
  - repo: https://github.com/andrewring/github-distributed-owners
    rev: v0.1.10
    hooks:
      - id: github-distributed-owners
```

The default CODEOWNERS location is `.github/CODEOWNERS`. This can be changed via

```yaml
    hooks:
      - id: github-distributed-owners
        args: [ "--output-file=<FILE_PATH>" ]
```

Note that GitHub will only respect CODEOWNERS files in a small number of locations. See
[the documentation](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners#codeowners-file-location)
for details.

You can further optimize the pre-commit behavior by filtering files processed with hook, like so:

```yaml
    hooks:
      - id: github-distributed-owners
        files: (.*/OWNERS|^.github/CODEOWNERS$)
```

NB: The CODEOWNERS path must be updated if specifying the `--output-file`, as above.

### Installation

To install github-distributed-owners independently,
from [crates.io](https://crates.io/crates/github-distributed-owners),
simply run

```shell
cargo install github-distributed-owners --locked
```

## Ownership Inheritance

By default, owners of directories are automatically included as owners of subdirectories. The default behavior can be
changed by setting `--implicit-inherit false`. For individual directories and patterns, this can be overwritten using
the syntax `set inherit = false`.

### Inheritance Example

```shell
# /OWNERS
user0
user1
```

```shell
# /foo/OWNERS
user2
user3
```

```shell
# /foo/bar/OWNERS
set inherit = false
user4
user5
```

In the above, changes files under `/foo` can be approved by any of `user0`, `user1`, `user2`, `user3`.
Changes to files under `/foo/bar` can only be approved by `user4`, and `user5`, however.

## File Patterns

Where listed users/groups at the top of the file are used to define ownership of all files at the directory level, you
can specify patterns within a directory, as well. This is done by providing the pattern in square brackets, like
`[*.rs]`, with owners and set values after.

Example:

```shell
# Directory level owners
user0
user1

# Additional owners for rust source files
[*.rs]
user2
user3

# Separate owners for special files
[special_*]
set inherit = false
user4
user5
```

## Including One OWNERS File From Another

To share OWNERS logic across multiple directories, you can `include` one OWNERS file from another.
The `include` path can either begin with a `/` in which case it's treated as relative to the
root of the repository, or not, in which case it's treated as relative to that OWNERS file path.

Example:

```shell
# /foo/OWNERS
user0
user1

[*.py]
user2
```

```shell
# /bar/OWNERS
include /foo/OWNERS
user3
```

```shell
# /baz/OWNERS
include ../bar/OWNERS
```

This gets unpacked such that `/bar/OWNERS` and `/baz/OWNERS` are effectively:

```shell
user0
user1
user3

[*.py]
user2
```

### Limitations

To mitigate some of the complexity of `include`d OWNERS, there are some language features
that are not available. If you believe you have a use case for one of these features, please
raise an issue and provide context on your use case.

#### Including OWNERS Within A File Pattern

While `include`d OWNERS files may have file pattern rules themselves, which get applied
to the including OWNERS, you may not `include` an OWNERS from within a file pattern rule.

Example:

```shell
user0

[*.py]
# This include will generate an error.
include /python/OWNERS
```

#### Setting Inherit Within Included OWNERS

Currently, `include`d OWNERS files may not `set inherit = ...`. This is to avoid the challenge
of defining semantics around how multiple conflicting `set inherit = ...` should interact.

## License

This Action is distributed under the terms of the MIT license, see [LICENSE](LICENSE) for details.

## Contribute and support

Any contributions are welcomed!
