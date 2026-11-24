package io.github.heyuch.groovylint.gradle;

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
     * Optional executable override, primarily for local development and tests.
     * When set, no embedded binary is extracted.
     *
     * @return the optional executable
     */
    public abstract RegularFileProperty getExecutable();
}
