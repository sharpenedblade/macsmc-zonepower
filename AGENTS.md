# AGENTS.md

Only edit what is directly relevant to your goal. Code should be as short and
simple as possible. Do not add anything the user did not directly ask for. Do
change existing identifiers for no reason. Do not rewrite user comments, docs,
or error messages unless neccessary.

Comments:

- Do not use comments except if absolutely necessary.
- Comments should be short.
- Comments should not describe what is already in the diff.

Commits:

- Each commit should be a single, atomic, self contained change.
- Include yourself as a co-author.
- Existing uncommited changes should remain in the working commit.
- If the user modifies code that you wrote, you should include it with your changes.
- Follow these commit guidelines: https://www.kernel.org/doc/html/latest/process/submitting-patches.html

Commit descriptions:

- Use a short 1 line summary, followed by the full description
- The description is not required for simple changes
- The description explains the why and what, not the how which is already shown by the code diff.

You can build with `nix build .#packages.x86_64-linux.default`.
Run tests with `nix flake test --all-systems --impure --print-build-logs`.
You have to `git add` new files before using any nix commands.

Always read documentation or source code to find information.

You can read documentation in docs/. Write like a human and avoid complex language. Don't document implementation details. Add more documentation whenever you find new information. You can create new files in docs/ if you want.
