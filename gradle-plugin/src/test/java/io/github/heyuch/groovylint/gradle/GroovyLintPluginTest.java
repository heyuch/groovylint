package io.github.heyuch.groovylint.gradle;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.attribute.PosixFilePermission;
import java.util.ArrayList;
import java.util.EnumSet;
import java.util.List;

import org.gradle.testkit.runner.BuildResult;
import org.gradle.testkit.runner.GradleRunner;
import org.gradle.testkit.runner.TaskOutcome;
import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

class GroovyLintPluginTest {
    @TempDir
    Path projectDir;

    private void writeProject(String scriptExitCode) throws IOException {
        writeProjectFiles("""
                groovylint {
                    executable.set(layout.projectDirectory.file("fake-groovylint.sh"))
                    configFile.set(layout.projectDirectory.file("codenarc.xml"))
                    threads.set(2)
                }
                """);
        Path script = projectDir.resolve("fake-groovylint.sh");
        Files.writeString(script, "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$PWD/args.txt\"\nexit "
                + scriptExitCode + "\n");
        Files.setPosixFilePermissions(script, EnumSet.of(
                PosixFilePermission.OWNER_READ,
                PosixFilePermission.OWNER_WRITE,
                PosixFilePermission.OWNER_EXECUTE));
    }

    private void writeFakeAnalyzer(String scriptExitCode) throws IOException {
        Path script = projectDir.resolve("fake-groovylint.sh");
        Files.writeString(script, "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$PWD/args.txt\"\nexit "
                + scriptExitCode + "\n");
        Files.setPosixFilePermissions(script, EnumSet.of(
                PosixFilePermission.OWNER_READ,
                PosixFilePermission.OWNER_WRITE,
                PosixFilePermission.OWNER_EXECUTE));
    }

    private void writeProjectFiles(String extensionBlock) throws IOException {
        Files.writeString(projectDir.resolve("settings.gradle.kts"), "rootProject.name = \"sample\"\n");
        Files.writeString(projectDir.resolve("codenarc.xml"), "<ruleset/>\n");
        Files.createDirectories(projectDir.resolve("src/main/groovy"));
        Files.writeString(projectDir.resolve("src/main/groovy/Example.groovy"), "println 'ok'\n");
        Files.writeString(projectDir.resolve("build.gradle.kts"), """
                plugins {
                    groovy
                    id("io.github.heyuch.groovylint")
                }

                """ + extensionBlock);
    }

    private BuildResult run(String... arguments) {
        return GradleRunner.create()
                .withProjectDir(projectDir.toFile())
                .withPluginClasspath()
                .withArguments(arguments)
                .forwardOutput()
                .build();
    }

    @Test
    void createsAndRunsASourceSetTask() throws IOException {
        writeProject("0");

        BuildResult result = run("groovylintMain");

        assertEquals(TaskOutcome.SUCCESS, result.task(":groovylintMain").getOutcome());
        List<String> args = Files.readAllLines(projectDir.resolve("args.txt"));
        assertTrue(args.contains("check"));
        assertTrue(args.contains("--config"));
        assertTrue(args.contains("--threads"));
        assertTrue(args.contains("2"));
        assertTrue(args.stream().anyMatch(arg -> arg.endsWith("src/main/groovy")));
    }

    @Test
    void passesTheChangedFileListToTheAnalyzer() throws IOException {
        writeProjectFiles("""
                groovylint {
                    executable.set(layout.projectDirectory.file("fake-groovylint.sh"))
                    threads.set(2)
                    changedOnly.set(true)
                    changedFiles.from(file("src/main/groovy/Example.groovy"))
                }
                """);
        writeFakeAnalyzer("0");

        BuildResult result = run("groovylintMain");

        assertEquals(TaskOutcome.SUCCESS, result.task(":groovylintMain").getOutcome());
        List<String> args = Files.readAllLines(projectDir.resolve("args.txt"));
        int index = args.indexOf("--changed-files");
        assertTrue(index >= 0, "--changed-files should be passed: " + args);
        List<String> listed = Files.readAllLines(Path.of(args.get(index + 1)));
        assertTrue(listed.stream().anyMatch(path -> path.endsWith("Example.groovy")), listed.toString());
    }

    @Test
    void acceptsAChangedFileListProducedOutsideGradle() throws IOException {
        Files.writeString(projectDir.resolve("changed.txt"), "src/main/groovy/Example.groovy\n");
        writeProjectFiles("""
                groovylint {
                    executable.set(layout.projectDirectory.file("fake-groovylint.sh"))
                    changedFilesList.set(layout.projectDirectory.file("changed.txt"))
                }
                """);
        writeFakeAnalyzer("0");

        BuildResult result = run("groovylintMain");

        assertEquals(TaskOutcome.SUCCESS, result.task(":groovylintMain").getOutcome());
        List<String> args = Files.readAllLines(projectDir.resolve("args.txt"));
        int index = args.indexOf("--changed-files");
        assertTrue(index >= 0, "--changed-files should be passed: " + args);
        assertTrue(args.get(index + 1).endsWith("changed.txt"), args.toString());
    }

    @Test
    void passesExcludedPathSubstringsToTheAnalyzer() throws IOException {
        writeProjectFiles("""
                groovylint {
                    executable.set(layout.projectDirectory.file("fake-groovylint.sh"))
                    exclude.add("/src/test/")
                    exclude.add("Test.groovy")
                }
                """);
        writeFakeAnalyzer("0");

        BuildResult result = run("groovylintMain");

        assertEquals(TaskOutcome.SUCCESS, result.task(":groovylintMain").getOutcome());
        List<String> args = Files.readAllLines(projectDir.resolve("args.txt"));
        assertEquals(List.of("--exclude", "/src/test/", "--exclude", "Test.groovy"), excludeArgs(args), args.toString());
    }

    private List<String> excludeArgs(List<String> args) {
        List<String> found = new ArrayList<>();
        for (int index = 0; index < args.size(); index++) {
            if ("--exclude".equals(args.get(index))) {
                found.add(args.get(index));
                found.add(args.get(index + 1));
            }
        }
        return found;
    }

    @Test
    void skipsTheTaskWhenNothingChanged() throws IOException {
        writeProjectFiles("""
                groovylint {
                    executable.set(layout.projectDirectory.file("fake-groovylint.sh"))
                    changedOnly.set(true)
                }
                """);
        writeFakeAnalyzer("0");

        BuildResult result = run("groovylintMain");

        assertEquals(TaskOutcome.SUCCESS, result.task(":groovylintMain").getOutcome());
        assertTrue(result.getOutput().contains("no changed files"), result.getOutput());
        assertTrue(Files.notExists(projectDir.resolve("args.txt")), "the analyzer must not run");
    }

    @Test
    void createsNullcheckTasks() throws IOException {
        writeProject("0");
        Files.writeString(projectDir.resolve("build.gradle.kts"), """
                plugins {
                    groovy
                    id("io.github.heyuch.groovylint")
                }

                groovylint {
                    executable.set(layout.projectDirectory.file("fake-groovylint.sh"))
                    nullcheckMode.set("inferred")
                }
                """);
        BuildResult result = run("groovylintNullcheckMain");
        assertEquals(TaskOutcome.SUCCESS, result.task(":groovylintNullcheckMain").getOutcome());
        List<String> args = Files.readAllLines(projectDir.resolve("args.txt"));
        assertTrue(args.contains("nullcheck"));
        assertTrue(args.contains("--mode"));
        assertTrue(args.contains("inferred"));
    }

    @Test
    void supportsConfigurationCache() throws IOException {
        writeProject("0");

        BuildResult first = run("groovylintMain", "--configuration-cache");
        assertEquals(TaskOutcome.SUCCESS, first.task(":groovylintMain").getOutcome());

        BuildResult second = run("groovylintMain", "--configuration-cache");
        assertEquals(TaskOutcome.SUCCESS, second.task(":groovylintMain").getOutcome());
        assertTrue(second.getOutput().contains("Reusing configuration cache"));
    }

    @Test
    void runsEmbeddedBinaryWhenNativeResourcesArePresent() throws IOException {
        String resource = "/META-INF/groovylint/" + NativeBinary.detectPlatform() + "/groovylint";
        Assumptions.assumeTrue(
                GroovyLintPluginTest.class.getResource(resource) != null,
                "native binary not staged in this build");

        writeProjectFiles("""
                groovylint {
                    threads.set(2)
                }
                """);
        BuildResult result = run("groovylintMain");
        assertEquals(TaskOutcome.SUCCESS, result.task(":groovylintMain").getOutcome());
    }

    @Test
    void failsOnFindingsAndHonorsIgnoreFailures() throws IOException {
        writeProject("1");

        try {
            run("groovylintMain");
            throw new AssertionError("expected the task to fail");
        } catch (org.gradle.testkit.runner.UnexpectedBuildFailure failure) {
            assertTrue(failure.getMessage().contains("groovylint found violations"));
        }

        Files.writeString(projectDir.resolve("build.gradle.kts"), """
                plugins {
                    groovy
                    id("io.github.heyuch.groovylint")
                }

                groovylint {
                    executable.set(layout.projectDirectory.file("fake-groovylint.sh"))
                    ignoreFailures.set(true)
                }
                """);
        BuildResult result = run("groovylintMain");
        assertEquals(TaskOutcome.SUCCESS, result.task(":groovylintMain").getOutcome());
    }
}
