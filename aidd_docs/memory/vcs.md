# VCS

- `origin`: this public fork (MIT).
- `upstream`: `git@github.com:fmflurry/myna.git`. Fetch only: `git fetch upstream main`.
- `main` mirrors upstream; do not commit fork work there. Work on `win/*` branches.
- Current branch: `win/lightweight`.
- Merge upstream into `main` first, then rebase or merge `main` into the work branch.
- Fetch and check remote branches before creating or pushing a branch.
- Commit messages: conventional, short. Do not commit or push unless asked.
- Keep the upstream `LICENSE` and Parakeet CC-BY-4.0 attribution intact.
