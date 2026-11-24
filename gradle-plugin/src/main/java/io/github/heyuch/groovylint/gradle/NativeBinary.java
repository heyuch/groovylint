package io.github.heyuch.groovylint.gradle;

import java.io.IOException;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.nio.file.attribute.PosixFilePermission;
import java.util.EnumSet;
import java.util.Locale;
import java.util.Set;

import org.gradle.api.GradleException;

final class NativeBinary {
    private NativeBinary() {
    }

    static Path extract(String version, Path temporaryDir) {
        String platform = detectPlatform();
        String resource = "/META-INF/groovylint/" + platform + "/groovylint";
        Path target = temporaryDir.resolve("native")
                .resolve(version)
                .resolve(platform)
                .resolve("groovylint");

        try {
            if (Files.isRegularFile(target)) {
                return target;
            }
            try (InputStream input = NativeBinary.class.getResourceAsStream(resource)) {
                if (input == null) {
                    throw new GradleException(
                            "groovylint does not contain a native binary for " + platform
                                    + ". Supported platforms: osx-aarch64, linux-x86_64.");
                }
                Files.createDirectories(target.getParent());
                Path temporary = Files.createTempFile(target.getParent(), "groovylint-", ".tmp");
                try {
                    Files.copy(input, temporary, StandardCopyOption.REPLACE_EXISTING);
                    makeExecutable(temporary);
                    try {
                        Files.move(temporary, target, StandardCopyOption.ATOMIC_MOVE);
                    } catch (IOException raceOrNoAtomicMove) {
                        if (!Files.isRegularFile(target)) {
                            Files.move(temporary, target, StandardCopyOption.REPLACE_EXISTING);
                        }
                    }
                } finally {
                    Files.deleteIfExists(temporary);
                }
            }
            return target;
        } catch (IOException exception) {
            throw new GradleException("Unable to extract the groovylint native binary", exception);
        }
    }

    static String detectPlatform() {
        String os = System.getProperty("os.name", "").toLowerCase(Locale.ROOT);
        String arch = System.getProperty("os.arch", "").toLowerCase(Locale.ROOT);
        if (os.contains("mac")) {
            if ("aarch64".equals(arch) || "arm64".equals(arch)) {
                return "osx-aarch64";
            }
            throw unsupported(os, arch);
        }
        if (os.contains("linux")) {
            if ("amd64".equals(arch) || "x86_64".equals(arch)) {
                return "linux-x86_64";
            }
            throw unsupported(os, arch);
        }
        throw unsupported(os, arch);
    }

    private static GradleException unsupported(String os, String arch) {
        return new GradleException(
                "groovylint does not support this platform yet: os=" + os + ", arch=" + arch
                        + ". Supported platforms: osx-aarch64, linux-x86_64.");
    }

    private static void makeExecutable(Path path) throws IOException {
        try {
            Set<PosixFilePermission> permissions = EnumSet.of(
                    PosixFilePermission.OWNER_READ,
                    PosixFilePermission.OWNER_WRITE,
                    PosixFilePermission.OWNER_EXECUTE,
                    PosixFilePermission.GROUP_READ,
                    PosixFilePermission.GROUP_EXECUTE,
                    PosixFilePermission.OTHERS_READ,
                    PosixFilePermission.OTHERS_EXECUTE);
            Files.setPosixFilePermissions(path, permissions);
        } catch (UnsupportedOperationException ignored) {
            path.toFile().setExecutable(true, false);
        }
    }
}
