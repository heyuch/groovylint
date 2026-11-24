# groovylint Gradle plugin

Gradle integration for the `groovylint` native analyzer. The plugin is the entry
point for Groovy projects: it embeds the analyzer binaries, extracts the correct
one at runtime, creates lint tasks, and hooks them into `check`.

## Usage

```kotlin
// build.gradle.kts
plugins {
    groovy
    id("io.github.heyuch.groovylint") version "0.1.0"
}

groovylint {
    // Optional. Defaults to groovylint's vendored CodeNarc basic ruleset.
    configFile.set(layout.projectDirectory.file("config/codenarc/codenarc.xml"))

    // Optional. Defaults to the number of available processors.
    threads.set(4)

    // Optional. Defaults to false: findings fail the task.
    ignoreFailures.set(false)

    // Nullcheck is opt-in. conservative reports definite null only.
    nullcheckMode.set("conservative") // or "inferred"
    nullcheckContracts.set(layout.projectDirectory.file("config/nullness-contracts.toml"))
}
```

Tasks:

- `groovylintMain` — lint the `main` source set.
- `groovylintTest` — lint the `test` source set.
- `groovylintNullcheckMain` — run annotation-free/annotated nullness analysis on
  the `main` source set (`nullcheckMode`, optional `nullcheckContracts`).
- `groovylintNullcheckTest` — run nullness analysis on the `test` source set.
- `groovylintNullcheck` — aggregate nullness task for all source sets.
- `groovylint` — aggregate CodeNarc task for all source sets.
- CodeNarc source-set tasks are attached to `check`; nullness tasks are opt-in
  until its false-positive rate is measured.

Run:

```bash
./gradlew groovylint
./gradlew check
```

The plugin version and the embedded analyzer version are the same. To run the
CLI directly from the plugin jar, extract
`META-INF/groovylint/<platform>/groovylint`.

## Supported platforms

- macOS Apple Silicon (`osx-aarch64`)
- Linux x86_64 (`linux-x86_64`)

The plugin fails with a clear error on unsupported platforms. Windows and Intel
macOS are not part of the first release.

## Native distribution

The plugin jar is an ordinary Maven artifact containing both native binaries:

```text
META-INF/groovylint/osx-aarch64/groovylint
META-INF/groovylint/linux-x86_64/groovylint
```

At execution, the plugin copies the matching binary to the task's temporary
directory, marks it executable, and runs it. There is no runtime download and no
separate native classifier to resolve, so Maven dependency locking, offline
builds, and corporate mirrors work normally.

## Development

```bash
cd gradle-plugin
./gradlew test validatePlugins
```

For a full local integration test, stage a binary:

```bash
mkdir -p gradle-plugin/build/native/osx-aarch64
cp target/release/groovylint gradle-plugin/build/native/osx-aarch64/groovylint
cd gradle-plugin
./gradlew test
```

The plugin tests use a fake executable by default; the embedded-binary test is
skipped unless native resources are present.

## Publishing

Artifacts are published to both Maven Central and the Gradle Plugin Portal from
the release workflow.

Required GitHub Actions secrets:

| Secret | Purpose |
| --- | --- |
| `MAVEN_CENTRAL_USERNAME` | Central Portal user token username |
| `MAVEN_CENTRAL_PASSWORD` | Central Portal user token password |
| `SIGNING_IN_MEMORY_KEY` | ASCII-armored GPG private key |
| `SIGNING_IN_MEMORY_KEY_PASSWORD` | GPG key password, if any |
| `GRADLE_PUBLISH_KEY` | Gradle Plugin Portal API key |
| `GRADLE_PUBLISH_SECRET` | Gradle Plugin Portal API secret |

The Maven Central namespace must be verified for `io.github.heyuch`, and the
Gradle Plugin Portal account must own the `io.github.heyuch.groovylint` plugin ID.

Local publishing validation:

```bash
cd gradle-plugin
./gradlew publishPlugins --validate-only \
  -x signGroovylintPluginMarkerMavenPublication \
  -x signPluginMavenPublication
```

`publishPlugins --validate-only` requires the native binaries to be staged in
`gradle-plugin/build/native/`.

## Versioning

The release tag must match both versions:

- `gradle-plugin/gradle.properties` → `version`
- workspace `Cargo.toml` → `[workspace.package] version`

The release workflow checks this before publishing.
