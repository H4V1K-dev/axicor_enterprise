# Publishing Checklist

Use this before making the repository public or accepting outside pull requests.

## Legal

- Confirm the repository-wide default license is AGPL-3.0-only.
- Confirm `LICENSE`, `LICENSE.md`, `CLA.md`, `CONTRIBUTING.md`, and `.github/PULL_REQUEST_TEMPLATE.md` are present.
- Decide what to do with `axicor-master/`. It is tracked and still contains older MIT/Apache notices, legacy contribution text, and possible third-party data. Do not publish it as part of the AGPL repository until it is either removed, archived outside the public repo, or fully audited and relicensed.
- Preserve third-party dependency notices in lockfiles and vendored packages.
- Keep project names, logos, domains, and product identity covered by `TRADEMARKS.md`.
- Have `CLA.md` reviewed by a lawyer before relying on it for substantial outside contributions.

## Repository Hygiene

- Ensure ignored generated files are not tracked: `node_modules`, build artifacts, local storage, private notes, temporary files, and model artifacts.
- Run a secret scan before the first public push.
- Check large tracked files and datasets before publishing.
- Remove personal-only notes that should not be public.

## GitHub Settings

- Protect the default branch.
- Require pull requests for changes to the default branch.
- Require status checks before merge.
- Require the pull request checklist item agreeing to `CLA.md`.
- Consider adding a CLA assistant or required manual maintainer check before merging outside contributions.
- Disable or restrict wiki/discussions until moderation expectations are clear.

## Release Readiness

- Add a real private security contact before announcing the project.
- Add project description, topics, and repository homepage.
- Decide whether `AxiEngine` and `AxiCAD` stay in one monorepo or are split into separate public repositories.
- Tag the first public AGPL version only after the tracked tree is clean and the legacy license status is resolved.

