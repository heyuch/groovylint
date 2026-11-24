import org.gradle.plugin.compatibility.compatibility

plugins {
    `java-gradle-plugin`
    id("com.gradle.plugin-publish") version "2.2.1"
    id("com.vanniktech.maven.publish") version "0.37.0"
}

group = providers.gradleProperty("group").get()
version = providers.gradleProperty("version").get()

tasks.jar {
    manifest {
        attributes["Implementation-Version"] = project.version
    }
}

repositories {
    mavenCentral()
}

java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(17)
    }
    withSourcesJar()
    withJavadocJar()
}

tasks.withType<JavaCompile>().configureEach {
    options.release = 17
}

dependencies {
    testImplementation(gradleTestKit())
    testImplementation(platform("org.junit:junit-bom:5.11.4"))
    testImplementation("org.junit.jupiter:junit-jupiter")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
    useJUnitPlatform()
    // TestKit starts nested Gradle builds; keep the daemon overhead predictable.
    maxParallelForks = 1
}

gradlePlugin {
    website = "https://github.com/heyuch/groovylint"
    vcsUrl = "https://github.com/heyuch/groovylint.git"

    plugins {
        create("groovylint") {
            id = "io.github.heyuch.groovylint"
            implementationClass = "io.github.heyuch.groovylint.gradle.GroovyLintPlugin"
            displayName = "groovylint"
            description = "Rust static analysis for Groovy with CodeNarc-compatible rules."
            tags = listOf("groovy", "lint", "static-analysis", "codenarc", "rust")
            compatibility {
                features {
                    configurationCache = true
                    isolatedProjects = false
                }
            }
        }
    }
}

mavenPublishing {
    publishToMavenCentral(automaticRelease = true)
    signAllPublications()
    coordinates(group.toString(), "groovylint-gradle-plugin", version.toString())

    pom {
        name = "groovylint Gradle plugin"
        description = "Gradle integration for the groovylint Groovy static analyzer."
        inceptionYear = "2026"
        url = "https://github.com/heyuch/groovylint"
        licenses {
            license {
                name = "MIT License"
                url = "https://opensource.org/license/mit"
                distribution = "https://opensource.org/license/mit"
            }
        }
        developers {
            developer {
                id = "heyuch"
                name = "heyuch"
                url = "https://github.com/heyuch"
            }
        }
        scm {
            url = "https://github.com/heyuch/groovylint"
            connection = "scm:git:git://github.com/heyuch/groovylint.git"
            developerConnection = "scm:git:ssh://git@github.com/heyuch/groovylint.git"
        }
    }
}

// Native binaries are staged by CI in build/native/<platform>/groovylint.
// Keeping them inside the plugin jar makes Maven Central distribution a single,
// ordinary Java artifact and lets the plugin extract the right executable at
// runtime.
val nativeSourceDir = layout.buildDirectory.dir("native")

val verifyNativeBinaries = tasks.register("verifyNativeBinaries") {
    doLast {
        val platforms = listOf("osx-aarch64", "linux-x86_64")
        platforms.forEach { platform ->
            val file = nativeSourceDir.get().file("$platform/groovylint").asFile
            check(file.isFile) {
                "Missing native binary for $platform at ${file.absolutePath}. " +
                    "CI must download the build-native artifacts into gradle-plugin/build/native before publishing."
            }
        }
    }
}

tasks.processResources {
    from(nativeSourceDir) {
        into("META-INF/groovylint")
    }
}

tasks.withType<AbstractPublishToMaven>().configureEach {
    dependsOn(verifyNativeBinaries)
}

tasks.named("publishPlugins") {
    dependsOn(verifyNativeBinaries)
}
