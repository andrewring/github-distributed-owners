# github-distributed-owners
A tool for auto generating GitHub compatible CODEOWNERS files from OWNERS files distributed through the file tree.

Distributing OWNERS configuration throughout the file tree makes it easier to find the appropriate people/teams who own
a given part of the codebase. This is especially useful in a multi-team, monorepo environment. It also has the nice
property of allowing teams to edit their own OWNERS files, with approval required only from the team. With the single
CODEOWNERS file supported by GitHub, you can either grant _everyone_ access to edit owners, or you can set a smaller
group of reviewers for all teams to send changes to, each of which have problems.

## Usage

Create files named `OWNERS` in the directories containing newline separated references to users or groups.

```shell
github_username
user@email.cmo
@group
```

Once these are in place, you can generate a GitHub compatible CODEOWNERS file by running the following in the root
directory of the git repo
```shell
github-distributed-owners --output-file .github/CODEOWNERS
```

### Pre-commit

Example pre-commit config:
```yaml
repos:
  - repo: https://github.com/andrewring/github-distributed-owners
    rev: v0.1.0
    hooks:
      - id: github-distributed-owners
```

The default CODEOWNERS location is `.github/CODEOWNERS`. This can be changed via
```yaml
    hooks:
      - id: github-distributed-owners
        args: ["--output-file=<FILE_PATH>"]
```
Note that GitHub will only respect CODEOWNERS files in a small number of locations. See
[the documentation](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners#codeowners-file-location)
for details.

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

In the above, changes files under `/` or `/foo` can be approved by any of `user0`, `user1`, `user2`, `user3`.
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
