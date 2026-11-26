# Distribution: Gradle and Maven

Status: **analysis and recommendation**. The Gradle plugin already exists; this
document decides how the binary reaches users and what the plugin has to change to
make onboarding one line.

## 1. What exists today

| Piece | State |
|---|---|
| Gradle plugin `io.github.heyuch.groovylint` | applied per project, creates `groovylint<SourceSet>` and `groovylintNullcheck<SourceSet>` tasks, hooks them into `check` |
| Native binaries | **embedded in the plugin jar** under `/META-INF/groovylint/<platform>/groovylint`, extracted on first use into `~/.gradle/caches/groovylint/<version>/<platform>/` |
| Platforms | `osx-aarch64`, `linux-x86_64` (4.0 MB each, stripped + thin LTO) |
| Channels | Gradle Plugin Portal + Maven Central (plugin, marker, sources, javadoc, signed) + GitHub release tarballs |
| Versioning | one version train: `Cargo.toml` version must equal the plugin version, enforced by the release workflow |
| Task wiring | `ExecOperations`, `@InputFiles` on source dirs, `@DisableCachingByDefault`, no report file, always out of date |

This is already a working distribution. The question is whether it is the *best*
one, and what to change before recommending it to other projects.

## 1.1 How a release is produced

`.github/workflows/release.yml`, triggered by **publishing a GitHub Release** (plus a
manual `workflow_dispatch` for retries). Publishing a Maven Central coordinate is
permanent, so the trigger is the deliberate act, not a tag push.

1. `native` builds `groovylint` for every platform in the matrix and uploads each
   binary as an artifact. This is where the full test suite runs.
2. `publish` runs on the `release` environment — the only place the signing key and
   the Central token exist, so a job that forgets `environment: release` gets empty
   strings and fails at signing rather than publishing something unsigned.
3. It checks that the release tag, `gradle.properties` and `Cargo.toml` all carry the
   same version, copies the native artifacts into `gradle-plugin/build/native/`, runs
   the plugin's TestKit suite, and verifies that every expected platform binary
   arrived.
4. The platform archives and `SHA256SUMS` go onto the release page **before** the
   Central upload: a first submission to Central is reviewed by hand and can take
   days, and a GitHub asset is easier to withdraw than a Central coordinate.
5. Then `publishToMavenCentral` (plugin, marker, sources, javadoc, signed) and
   `publishPlugins` (Plugin Portal).

Secret names on the environment and the names the build reads differ, so the mapping
lives in one place in the workflow:

| Environment secret | Read by the build as |
|---|---|
| `MAVEN_CENTRAL_USERNAME` / `MAVEN_CENTRAL_PASSWORD` | `ORG_GRADLE_PROJECT_mavenCentralUsername` / `…Password` |
| `GPG_PRIVATE_KEY` / `GPG_PASSPHRASE` | `ORG_GRADLE_PROJECT_signingInMemoryKey` / `…KeyPassword` |
| `GRADLE_PUBLISH_KEY` / `GRADLE_PUBLISH_SECRET` | the same names, as environment variables |

Adding a platform means adding it to the matrix **and** to `verifyNativeBinaries` in
`gradle-plugin/build.gradle.kts`; the two lists are meant to be the same list.

## 2. What "simplest for a Gradle user" actually means

Ordered by what breaks adoption, based on the failures other native analyzers hit:

1. **One line, no extra tooling.** `plugins { id("…groovylint") version "x" }` and
   nothing else: no PATH entry, no manual download, no `curl | bash`.
2. **No surprise traffic.** Corporate CI resolves artifacts through a mirror;
   anything that downloads from a raw URL at execution time is a firewall ticket.
3. **Offline after the first resolve**, using Gradle's own dependency cache.
4. **Works where the JVM does**: Linux x86_64 *and* aarch64 (ARM CI runners are
   common now), macOS Intel *and* Apple Silicon, Windows.
5. **Nothing that breaks the build when the tool is absent**: a misconfigured
   platform should fail with a clear message, and `-x groovylintMain` must work.
6. **Reasonable CI ergonomics**: a report file, an exit code, and a way to adopt a
   legacy codebase without a big-bang fix (baseline).

## 3. Options for delivering the binary

| # | How the binary reaches the user | Pros | Cons | Cost |
|---|---|---|---|---|
| A | **Embedded in the plugin jar** (today) | one artifact, one version, works offline after plugin resolve, no extra plugin code | every user downloads every platform (4 platforms ≈ 16 MB, 6 ≈ 24 MB); the artifact is only addressable as "a Gradle plugin", so Maven and plain-CI users must unzip a plugin jar; adding platforms bloats releases | 0 |
| B | **Per-platform artifacts on Maven Central**, resolved lazily by the plugin | users download only their platform; the same artifact serves Maven and CI; platform matrix grows without touching the plugin jar; mirror-friendly (`mavenCentral()` in `pluginManagement`) | plugin carries ~80 lines of resolution code; version skew is possible if the plugin and CLI versions drift (mitigated by the single version train) | ~1.5 d |
| C | **GitHub release download at execution time** | zero publishing infrastructure | needs network at build time, blocked in most CI, no checksum story, version pinning is manual | 0.5 d (but worse UX) |
| D | **System package managers** (Homebrew, apt, npm, cargo) | natural for CLI users | none for Gradle users; Gradle still needs an integration to be "one line" | 2 d each |

**Recommendation: B.** A is acceptable and already shipped, but B strictly
dominates once more than two platforms exist: the plugin jar stops growing, the
binary becomes a first-class artifact for Maven users, and the download is a normal
dependency resolution that mirrors already cache.

Artifact shape matters: publish each platform as its **own artifactId** with
`zip` type — `org.example:groovylint-cli-osx-universal:0.3.0@zip` — not as a
classifier, because that avoids relying on Central's accepted-extension list and
reads unambiguously in a Maven POM. A `.zip` also survives mirrors that mangle
mode bits; the plugin re-applies the executable bit after unpacking (it already
does).

Platform matrix to publish:

| Artifact | Build | Covers |
|---|---|---|
| `groovylint-cli-linux-x86_64` | `x86_64-unknown-linux-musl` | static, no glibc floor |
| `groovylint-cli-linux-aarch64` | `aarch64-unknown-linux-musl` | ARM CI runners, Graviton |
| `groovylint-cli-osx-universal` | `lipo` of `aarch64-apple-darwin` + `x86_64-apple-darwin` | Apple Silicon and Intel |
| `groovylint-cli-windows-x86_64` | `x86_64-pc-windows-msvc` | Windows developers |

macOS universal is one artifact instead of two: same total bytes, one third fewer
coordinates, and no "wrong arch" support tickets. Windows is currently out of the
roadmap's scope; with B it costs one matrix entry, not a new mechanism.

## 4. Options for the Gradle surface

| Option | User writes | Verdict |
|---|---|---|
| **Gradle plugin** (today) | `plugins { id("…groovylint") version "x" }` | primary: only shape that can wire `check`, declare inputs and render reports |
| Dependency + `Exec` snippet | ~8 lines (`configurations.create("groovylint")`, `tasks.register<Exec>`), no plugin | secondary: documents that nothing is hidden, and is the escape hatch for air-gapped CI |
| `exec-maven-plugin` (Maven projects) | see §7 | the Maven answer, no new artifact |
| Gradle build-script `dependencyResolutionManagement` only | — | not applicable: the binary is not a JVM artifact |

Keep the plugin as the primary surface, and publish the snippet in the README so a
reviewer can see exactly what the plugin does.

## 5. What the plugin should change before it is recommended

These are integration quality issues, independent of how the binary ships:

| Priority | Change | Why |
|---|---|---|
| P1 | Resolve the binary through a `groovylintCli` configuration instead of a jar resource (§3 B) | see above |
| P2 | **Report file as a task output**: `build/reports/groovylint/<sourceSet>.<txt\|json>` with `@OutputFile`, so a repeated run is up to date | today the task runs on every build and prints to the console only — no artifact for CI |
| P3 | Lint **main only** by default, `test` opt-in (`groovylint.testSourceSets = [...]`) | test sources legitimately trip rules that main must not; today `check` runs both |
| P4 | `--format json` passthrough plus a SARIF writer (or a documented converter) | code-scanning and Sonar ingestion |
| P5 | Baseline file input: `groovylint.baseline = layout.projectDirectory.file("groovylint-baseline.txt")` | the only way a 24k-file project adopts the gate |
| P6 | Root-level `groovylintArch` task, once, over the whole project | architecture rules need one project-wide run, not one per module (see `docs/ARCH-RULE-MODEL.md`) |
| P7 | `toolVersion` property (default = plugin version) | canary testing without waiting for a plugin release |
| P8 | Wire `markedOnly` / `initializationCheck` / `assertsTrue` through to `nullcheck` | the extension has `nullcheckMode`/`contracts` only; the CLI accepts more |
| P9 | `groovylint.enabled` / `-x groovylintMain` documented | "the tool is down, ship anyway" must be a one-liner |

P2–P5 are what make the plugin trustworthy in CI; P1, P6–P8 are correctness and
coverage. Nothing here changes the user-facing configuration shape, so it can ship
incrementally in patch releases.

## 6. Publishing pipeline

The release workflow already builds, verifies, signs and publishes. With B it gains
one step, not a new pipeline:

1. build the matrix (`musl` targets, `lipo` for macOS), stage `dist/<platform>/groovylint`;
2. `zip` each platform, write `SHA256SUMS`, keep the GitHub release tarballs;
3. publish the zips to Central with `maven-publish`
   (`artifact(taskProvider, { classifier = null; extension = "zip" })` per platform,
   same signing config as the plugin);
4. publish the plugin + marker to Central and the Portal, as today;
5. keep the version lockstep check — it is the thing that keeps "plugin 0.3.0 means
   CLI 0.3.0" true.

One repository for both (`mavenCentral()` in `pluginManagement`) is also the
enterprise-friendly answer: companies already mirror Central, and the Gradle Plugin
Portal is often *not* mirrored.

## 7. Maven build users (no Gradle)

Until a real `maven-plugin` exists, the same Central artifacts give a working setup
in about ten lines:

```xml
<plugin>
  <groupId>org.apache.maven.plugins</groupId>
  <artifactId>maven-dependency-plugin</artifactId>
  <executions><execution><id>unpack-groovylint</id>
    <phase>process-sources</phase>
    <goals><goal>unpack-dependencies</goal></goals>
    <configuration>
      <includeArtifactIds>groovylint-cli-linux-x86_64</includeArtifactIds>
      <outputDirectory>${project.build.directory}/groovylint</outputDirectory>
    </configuration>
  </execution></executions>
</plugin>
<plugin>
  <groupId>org.codehaus.mojo</groupId>
  <artifactId>exec-maven-plugin</artifactId>
  <executions><execution><id>lint</id>
    <phase>verify</phase>
    <goals><goal>exec</goal></goals>
    <configuration>
      <executable>${project.build.directory}/groovylint/groovylint</executable>
      <arguments>
        <argument>check</argument>
        <argument>--format</argument><argument>json</argument>
        <argument>${project.basedir}/src/main/groovy</argument>
      </arguments>
    </configuration>
  </execution></executions>
</plugin>
```

A real `maven-plugin` would duplicate this wiring in Java, need its own release
train, and still shell out to the same binary: ~1 week for cosmetics. Defer it
until a Maven user asks; the artifacts from §3 make it possible without any
re-release.

## 8. Phasing

| Step | Content | Effort |
|---|---|---|
| 1 | Publish per-platform zips (matrix as today: 2 platforms), plugin resolves them | 1.5 d |
| 2 | P2/P3 report file + main-only default; README "what the plugin does" + snippet | 1 d |
| 3 | Add `linux-aarch64`, `osx-universal`, `windows-x86_64` to the matrix | 1 d |
| 4 | P4 SARIF, P5 baseline, P7 `toolVersion` | 1 d |
| 5 | P6 `groovylintArch` root task | after the arch engine exists |
| 6 | Maven snippet in the README (+ a CI example using it) | 0.5 d |

Steps 1–2 are the ones that make the tool adoptable; 3 is cheap and closes the
"ARM CI runner" hole that would otherwise surface as a support issue.

## 9. Risks

| Risk | Mitigation |
|---|---|
| Central rejects an unusual extension | publish `zip`, unpack in the plugin, never `.bin`/`.tar.gz` |
| Exec bit lost in the zip | the plugin already re-applies it after unpacking |
| `noexec` home directory or Gradle cache | `groovylint.executable` override, and fall back to `build/groovylint` when the cache path is not executable |
| Plugin/CLI version drift | single version train enforced in the release workflow; `toolVersion` only as an explicit override |
| Plugin Portal not mirrored in a company | plugin + marker are also on Central; document `pluginManagement { repositories { mavenCentral() } }` |
| Windows/macOS code signing and Gatekeeper | files written by the JVM do not carry the quarantine attribute; a Windows binary may still trip heuristics — a known, documented limitation until we sign |
| `check` becomes slower after adding `@OutputFile` semantics | keep the task `@DisableCachingByDefault` (no remote cache) and only declare the report as output |
