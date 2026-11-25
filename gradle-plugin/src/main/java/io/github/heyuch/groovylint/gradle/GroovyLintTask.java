package io.github.heyuch.groovylint.gradle;

import java.io.File;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;

import javax.inject.Inject;

import org.gradle.api.DefaultTask;
import org.gradle.api.GradleException;
import org.gradle.api.file.ConfigurableFileCollection;
import org.gradle.api.file.DirectoryProperty;
import org.gradle.api.file.RegularFileProperty;
import org.gradle.api.provider.Property;
import org.gradle.api.tasks.Input;
import org.gradle.api.tasks.InputFile;
import org.gradle.api.tasks.InputFiles;
import org.gradle.api.tasks.Internal;
import org.gradle.api.tasks.Optional;
import org.gradle.api.tasks.PathSensitive;
import org.gradle.api.tasks.PathSensitivity;
import org.gradle.api.tasks.TaskAction;
import org.gradle.process.ExecOperations;
import org.gradle.work.DisableCachingByDefault;
import org.gradle.process.ExecResult;

/**
 * Runs the native groovylint executable for one source set.
 */
@DisableCachingByDefault(because = "The lint task delegates to an external analyzer and has no local outputs")
public abstract class GroovyLintTask extends DefaultTask {
    /** Source directories to analyze.
     *
     * @return source directories
     */
    @InputFiles
    @PathSensitive(PathSensitivity.RELATIVE)
    public abstract ConfigurableFileCollection getSourceDirectories();

    /** Analyzer command: {@code check} or {@code nullcheck}.
     *
     * @return the command
     */
    @Input
    public abstract Property<String> getCommand();

    /** Optional CodeNarc XML ruleset.
     *
     * @return the optional ruleset file
     */
    @InputFile
    @Optional
    @PathSensitive(PathSensitivity.RELATIVE)
    public abstract RegularFileProperty getConfigFile();

    /** Optional TOML nullness contract file.
     *
     * @return the optional contract file
     */
    @InputFile
    @Optional
    @PathSensitive(PathSensitivity.NONE)
    public abstract RegularFileProperty getNullcheckContracts();

    /** Nullcheck reporting mode.
     *
     * @return the nullcheck mode
     */
    @Input
    public abstract Property<String> getNullcheckMode();

    /** Number of analyzer worker threads.
     *
     * @return the thread count
     */
    @Input
    public abstract Property<Integer> getThreads();

    /** Whether findings should fail the task.
     *
     * @return the ignore-failures flag
     */
    @Input
    public abstract Property<Boolean> getIgnoreFailures();

    /** Plugin version used to isolate extracted native binaries.
     *
     * @return the native binary cache version
     */
    @Input
    public abstract Property<String> getNativeVersion();

    /** Optional executable override for development and tests.
     *
     * @return the optional executable
     */
    @InputFile
    @Optional
    @PathSensitive(PathSensitivity.NONE)
    public abstract RegularFileProperty getExecutable();

    /** Project directory used as the analyzer working directory.
     *
     * @return the working directory
     */
    @Internal
    public abstract DirectoryProperty getWorkingDirectory();

    /** Shared cache used to store extracted native binaries.
     *
     * @return the native cache directory
     */
    @Internal
    public abstract DirectoryProperty getNativeCacheDirectory();

    /** Whether to check only the changed files.
     *
     * @return the changed-only flag
     */
    @Input
    public abstract Property<Boolean> getChangedOnly();

    /** Files changed by this branch, produced by the build.
     *
     * @return the changed files
     */
    @InputFiles
    @PathSensitive(PathSensitivity.RELATIVE)
    public abstract ConfigurableFileCollection getChangedFiles();

    /** Exec operations used to launch the native analyzer.
     *
     * @return exec operations
     */
    @Inject
    protected abstract ExecOperations getExecOperations();

    /** Runs the analyzer over the configured source directories. */
    @TaskAction
    public void lint() {
        List<File> sources = getSourceDirectories().getFiles().stream()
                .filter(File::isDirectory)
                .sorted(Comparator.comparing(File::getAbsolutePath))
                .toList();
        if (sources.isEmpty()) {
            getLogger().lifecycle("groovylint: no Groovy source directories found");
            return;
        }

        File binary = getExecutable().isPresent()
                ? getExecutable().get().getAsFile()
                : NativeBinary.extract(getNativeVersion().get(), getNativeCacheDirectory().get().getAsFile().toPath()).toFile();
        if (!binary.isFile()) {
            throw new GradleException("groovylint executable does not exist: " + binary);
        }

        List<String> changed = getChangedFiles().getFiles().stream()
                .map(File::getAbsolutePath)
                .sorted()
                .toList();
        if (getChangedOnly().get() && changed.isEmpty()) {
            getLogger().lifecycle("groovylint: no changed files, nothing to check");
            return;
        }

        List<String> arguments = new ArrayList<>();
        arguments.add(getCommand().get());
        if (getChangedOnly().get()) {
            Path list = writeChangedFileList(changed);
            arguments.add("--changed-files");
            arguments.add(list.toAbsolutePath().toString());
        }
        arguments.add("--threads");
        arguments.add(Integer.toString(Math.max(1, getThreads().get())));
        if ("nullcheck".equals(getCommand().get())) {
            arguments.add("--mode");
            arguments.add(getNullcheckMode().get());
            if (getNullcheckContracts().isPresent()) {
                arguments.add("--contracts");
                arguments.add(getNullcheckContracts().get().getAsFile().getAbsolutePath());
            }
        } else if (getConfigFile().isPresent()) {
            arguments.add("--config");
            arguments.add(getConfigFile().get().getAsFile().getAbsolutePath());
        }
        sources.stream().map(File::getAbsolutePath).forEach(arguments::add);

        Path workingDirectory = getWorkingDirectory().get().getAsFile().toPath();
        ExecResult result = getExecOperations().exec(spec -> {
            spec.setExecutable(binary);
            spec.setArgs(arguments);
            spec.setWorkingDir(workingDirectory.toFile());
            spec.setIgnoreExitValue(true);
        });

        int exitCode = result.getExitValue();
        if (exitCode == 1) {
            if (getIgnoreFailures().get()) {
                getLogger().warn("groovylint found violations; ignoring because ignoreFailures=true");
                return;
            }
            throw new GradleException(
                    "groovylint found violations. Fix the findings above or set groovylint.ignoreFailures = true.");
        }
        if (exitCode != 0) {
            throw new GradleException("groovylint failed with exit code " + exitCode);
        }
    }

    private Path writeChangedFileList(List<String> changed) {
        Path list = getTemporaryDir().toPath().resolve("changed-files.txt");
        try {
            Files.write(list, changed, StandardCharsets.UTF_8);
        } catch (IOException exception) {
            throw new UncheckedIOException("Unable to write the changed-file list", exception);
        }
        return list;
    }
}
