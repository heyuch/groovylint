package io.github.heyuch.groovylint.gradle;

import org.gradle.api.file.ConfigurableFileCollection;
import org.gradle.api.file.RegularFileProperty;
import org.gradle.api.provider.Property;

/**
 * Configuration for the {@code groovylint} Gradle plugin.
 */
public abstract class GroovyLintExtension {
    /**
     * Optional CodeNarc XML ruleset. When absent, groovylint's vendored CodeNarc
     * basic ruleset is used.
     *
     * @return the optional ruleset file
     */
    public abstract RegularFileProperty getConfigFile();

    /**
     * Number of analyzer worker threads. Defaults to the number of available processors.
     *
     * @return the thread count
     */
    public abstract Property<Integer> getThreads();

    /**
     * Whether findings should fail the Gradle task. Defaults to {@code false}.
     *
     * @return the ignore-failures flag
     */
    public abstract Property<Boolean> getIgnoreFailures();

    /**
     * Nullcheck reporting mode: {@code conservative} or {@code inferred}.
     *
     * @return the nullcheck mode
     */
    public abstract Property<String> getNullcheckMode();

    /**
     * Optional TOML contract file for nullcheck.
     *
     * @return the optional contract file
     */
    public abstract RegularFileProperty getNullcheckContracts();

    /**
     * Optional executable override, primarily for local development and tests.
     * When set, no embedded binary is extracted.
     *
     * @return the optional executable
     */
    public abstract RegularFileProperty getExecutable();

    /**
     * Whether to check only {@link #getChangedFiles() changed files}.
     *
     * <p>When enabled and the changed set is empty, the task is skipped and the
     * build stays green — that is what a pull request with no Groovy change needs.
     *
     * @return the changed-only flag
     */
    public abstract Property<Boolean> getChangedOnly();

    /**
     * The files this branch changed, produced by the build (for example from a
     * {@code changed-files.gradle} script). Only read when
     * {@link #getChangedOnly()} is enabled.
     *
     * @return the changed files
     */
    public abstract ConfigurableFileCollection getChangedFiles();

    /**
     * A file holding the changed paths, one per line, produced outside Gradle —
     * for environments where the build cannot run {@code git} itself.
     *
     * <p>When set, it is passed to the analyzer unchanged and implies
     * changed-only mode.
     *
     * @return the optional changed-file list
     */
    public abstract RegularFileProperty getChangedFilesList();
}
