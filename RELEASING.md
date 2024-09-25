# Releasing a new version

We are using GitHub Actions to create the release pages, binary artifacts, and Docker images for a given release. In the actions we also use the tool [Knope](https://github.com/knope-dev/knope) to do semantic versioning.

Knope will follow [conventional commits](https://www.conventionalcommits.org/en/v1.0.0/) so if you annotate your PR titles with the appropiate prefix, that will pick up to do a minor or patch version.
If you need to manual increment the version you can make an empty commit following the conventional commit pattern.

## Running the action

* Navigate to the [Build and Release action](https://github.com/apollosolutions/persisted-query-to-rest/actions/workflows/release.yml)
* Trigger a new workflow and pass in the new version you expect to release given all the previous commit history
  * If you have a PR with a `feat:` prefix and we are on version `v0.2.1` run the workflow with the input `v0.3.0`
 