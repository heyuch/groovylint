package io.github.heyuch.groovylint.gradle;

import java.util.Locale;

import org.gradle.api.Plugin;
import org.gradle.api.Project;
import org.gradle.api.Task;
import org.gradle.api.tasks.SourceSetContainer;
import org.gradle.api.tasks.TaskProvider;

/**
 * Gradle integration for the groovylint native analyzer.
 */
public final class GroovyLintPlugin implements Plugin<Project> {
    private static final String EXTENSION_NAME = "groovylint";
    private static final String AGGREGATE_TASK = "groovylint";

    /**
     * Applies the groovylint extension, tasks, and verification hooks.
     *
     * @param project target project
     */
    @Override
    public void apply(Project project) {
        GroovyLintExtension extension = project.getExtensions().create(
                EXTENSION_NAME, GroovyLintExtension.class);
        extension.getThreads().convention(Math.max(1, Runtime.getRuntime().availableProcessors()));
        extension.getIgnoreFailures().convention(false);
        extension.getNullcheckMode().convention("conservative");
        extension.getChangedOnly().convention(false);
        String nativeVersion = pluginVersion();

        TaskProvider<Task> aggregate = project.getTasks().register(AGGREGATE_TASK, task -> {
            task.setGroup("verification");
            task.setDescription("Runs groovylint on all source sets.");
        });
        TaskProvider<Task> nullcheckAggregate = project.getTasks().register("groovylintNullcheck", task -> {
            task.setGroup("verification");
            task.setDescription("Runs nullcheck on all source sets.");
        });

        project.getPluginManager().withPlugin("java", appliedPlugin -> configureSourceSets(
                project, extension, aggregate, nullcheckAggregate, nativeVersion));
    }

    private static void configureSourceSets(
            Project project,
            GroovyLintExtension extension,
            TaskProvider<Task> aggregate,
            TaskProvider<Task> nullcheckAggregate,
            String nativeVersion) {
        SourceSetContainer sourceSets = project.getExtensions().getByType(SourceSetContainer.class);
        sourceSets.all(sourceSet -> {
            String taskName = AGGREGATE_TASK + capitalize(sourceSet.getName());
            TaskProvider<GroovyLintTask> task = project.getTasks().register(
                    taskName, GroovyLintTask.class, lint -> {
                        lint.setGroup("verification");
                        lint.setDescription("Runs groovylint on the "
                                + sourceSet.getName() + " source set.");
                        lint.getCommand().set("check");
                        lint.getNullcheckMode().set(extension.getNullcheckMode());
                        lint.getSourceDirectories().from(sourceSet.getAllSource().getSrcDirs());
                        lint.getConfigFile().set(extension.getConfigFile());
                        lint.getThreads().set(extension.getThreads());
                        lint.getIgnoreFailures().set(extension.getIgnoreFailures());
                        lint.getNativeVersion().set(nativeVersion);
                        lint.getExecutable().set(extension.getExecutable());
                        lint.getChangedOnly().set(extension.getChangedOnly());
                        lint.getChangedFiles().from(extension.getChangedFiles());
                        lint.getChangedFilesList().set(extension.getChangedFilesList());
                        lint.getWorkingDirectory().set(project.getLayout().getProjectDirectory());
                        lint.getNativeCacheDirectory().set(
                                new java.io.File(project.getGradle().getGradleUserHomeDir(), "caches/groovylint"));
                    });

            aggregate.configure(aggregateTask -> aggregateTask.dependsOn(task));
            project.getTasks().named("check").configure(check -> check.dependsOn(task));

            String nullcheckTaskName = "groovylintNullcheck" + capitalize(sourceSet.getName());
            TaskProvider<GroovyLintTask> nullcheckTask = project.getTasks().register(
                    nullcheckTaskName, GroovyLintTask.class, lint -> {
                        lint.setGroup("verification");
                        lint.setDescription("Runs nullcheck on the "
                                + sourceSet.getName() + " source set.");
                        lint.getCommand().set("nullcheck");
                        lint.getNullcheckMode().set(extension.getNullcheckMode());
                        lint.getNullcheckContracts().set(extension.getNullcheckContracts());
                        lint.getSourceDirectories().from(sourceSet.getAllSource().getSrcDirs());
                        lint.getThreads().set(extension.getThreads());
                        lint.getIgnoreFailures().set(extension.getIgnoreFailures());
                        lint.getNativeVersion().set(nativeVersion);
                        lint.getExecutable().set(extension.getExecutable());
                        lint.getChangedOnly().set(extension.getChangedOnly());
                        lint.getChangedFiles().from(extension.getChangedFiles());
                        lint.getChangedFilesList().set(extension.getChangedFilesList());
                        lint.getWorkingDirectory().set(project.getLayout().getProjectDirectory());
                        lint.getNativeCacheDirectory().set(
                                new java.io.File(project.getGradle().getGradleUserHomeDir(), "caches/groovylint"));
                    });
            nullcheckAggregate.configure(aggregateTask -> aggregateTask.dependsOn(nullcheckTask));
        });
    }

    private static String capitalize(String value) {
        if (value.isEmpty()) {
            return value;
        }
        return value.substring(0, 1).toUpperCase(Locale.ROOT) + value.substring(1);
    }

    private static String pluginVersion() {
        Package pluginPackage = GroovyLintPlugin.class.getPackage();
        String version = pluginPackage == null ? null : pluginPackage.getImplementationVersion();
        return version == null ? "dev" : version;
    }
}
