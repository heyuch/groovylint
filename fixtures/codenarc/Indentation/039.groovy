
class MySpec extends Specification {
    void 'some feature'() {
        given: 'something'
        def a = new Object()

        when: 'something is done'
        def b = a.toString()

        then: 'something happens'
        b != ''

        and:
        b != 'raccoon'
    }

    void 'androidLint is run'() {
        given:
        writeAndroidBuildFile(androidVersion)
        useSimpleAndroidLintConfig()
        writeAndroidManifest()
        goodCode()

        when:
        BuildResult result = gradleRunner()
            .withGradleVersion(version)
            .build()

        then:
        if (GradleVersion.version(version) >= GradleVersion.version('2.5')) {
            // Executed task capture is only available in Gradle 2.5+
            result.task(taskName()).outcome == SUCCESS
            result.task(':resolveAndroidLint').outcome == SUCCESS
            result.task(':cleanupAndroidLint').outcome == SUCCESS
        }

        // Make sure report exists and was using the expected tool version
        reportFile().exists()

        where:
        version << ['2.3', '2.4', '2.7', '2.10', '2.14.1'] +
            (Jvm.current.java8Compatible ? ['3.0', '3.1'] : [])
        androidVersion = GradleVersion.version(version) < GradleVersion.version('3.0') ?
            DEFAULT_ANDROID_VERSION : '2.2.0'
    }
}
        